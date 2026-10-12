//! Real M0 lane for the Intent Engine (host, `std`).
//!
//! ```text
//! IntentEngine --rank(query, corpus)--> M0Process
//!     -> spawn <python> poc/m0/bridge_recall.py   (stdin: vela.recall.cards.v1, one line)
//!        multilingual-e5-small (pinned SHA-256) + LanceDB BM25 + native RRF(K=60)
//!     <- {"status":"RankedCandidates","auto_resolve":false,"embedding_epoch":..,"candidates":[{oid,score}]}
//!     -> oids mapped back to local handles (unknown oids dropped)
//! ```
//!
//! Fail closed: any process error, timeout, schema mismatch, `auto_resolve != false`
//! or unpinned embedding epoch yields an EMPTY lane (BM25 still ranks) and is
//! reported through [`M0Process::status`]. The lane never returns versions,
//! never returns rights, and its output is only fused by RRF in `vela-intent`.
#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vela_intent::{Corpus, Handle, Retriever, Source};

/// Request schema accepted by `bridge_recall.py`.
pub const SCHEMA: &str = "vela.recall.cards.v1";
/// Embedding epoch pinned by `poc/m0/hybrid_recall.py` (`spike.EPOCH`).
pub const PINNED_EPOCH: &str =
    "intfloat/multilingual-e5-small@614241f622f53c4eeff9890bdc4f31cfecc418b3:onnx-fp32:mean:l2:query-passage:v1";
/// `bridge_recall.py` asserts 1..=100 cards.
pub const MAX_CARDS: usize = 100;
/// Cold start loads the ONNX model (~10 s observed on Francesco's Windows host).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct M0Config {
    /// Python interpreter with the M0 dependencies (the `uv` venv of poc/m0).
    pub python: PathBuf,
    /// Directory holding `bridge_recall.py`, `hybrid_recall.py` and `.model-cache/`.
    pub m0_dir: PathBuf,
    pub timeout: Duration,
}

impl M0Config {
    /// `m0_dir` with its own venv (`.venv/Scripts/python.exe` or `.venv/bin/python`).
    pub fn in_dir(m0_dir: impl Into<PathBuf>) -> Self {
        let m0_dir = m0_dir.into();
        let win = m0_dir.join(".venv").join("Scripts").join("python.exe");
        let python = if win.exists() { win } else { m0_dir.join(".venv").join("bin").join("python") };
        Self { python, m0_dir, timeout: DEFAULT_TIMEOUT }
    }

    /// `VELA_M0_DIR` (required) and optional `VELA_M0_PYTHON`.
    pub fn from_env() -> Option<Self> {
        let dir = std::env::var_os("VELA_M0_DIR")?;
        let mut cfg = Self::in_dir(PathBuf::from(dir));
        if let Some(py) = std::env::var_os("VELA_M0_PYTHON") {
            cfg.python = PathBuf::from(py);
        }
        Some(cfg)
    }

    /// What must exist before a run can work (no process is started).
    pub fn check(&self) -> Result<(), M0Error> {
        for p in [self.python.clone(), self.m0_dir.join("bridge_recall.py"), self.m0_dir.join(".model-cache")] {
            if !p.exists() {
                return Err(M0Error::Missing(p));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum M0Error {
    Missing(PathBuf),
    Spawn(String),
    Timeout(Duration),
    Exit { code: Option<i32>, stderr: String },
    Schema(String),
    /// The bridge claimed `auto_resolve != false`: a ranking can never resolve itself.
    AutoResolve,
    /// Embeddings from an unpinned model epoch are not evidence.
    Epoch(String),
    TooManyCards(usize),
}

impl std::fmt::Display for M0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(p) => write!(f, "M0 non disponibile: manca {}", p.display()),
            Self::Spawn(e) => write!(f, "M0 spawn fallito: {e}"),
            Self::Timeout(d) => write!(f, "M0 timeout dopo {} s", d.as_secs()),
            Self::Exit { code, stderr } => write!(f, "M0 exit {code:?}: {}", stderr.lines().last().unwrap_or("")),
            Self::Schema(e) => write!(f, "M0 risposta non valida: {e}"),
            Self::AutoResolve => write!(f, "M0 ha dichiarato auto_resolve: rifiutato"),
            Self::Epoch(e) => write!(f, "M0 epoch non pinnata: {e}"),
            Self::TooManyCards(n) => write!(f, "M0 accetta max {MAX_CARDS} card, corpus {n}"),
        }
    }
}

/// One M0 ranking: `(oid, rrf score)` best-first.
#[derive(Clone, Debug, PartialEq)]
pub struct M0Ranked {
    pub epoch: String,
    pub ranked: Vec<(String, f64)>,
}

/// Request line for `bridge_recall.py` (card metadata only, never object bytes).
pub fn build_request(query: &str, corpus: &Corpus) -> Result<String, M0Error> {
    let n = corpus.docs().len();
    if n == 0 || n > MAX_CARDS {
        return Err(M0Error::TooManyCards(n));
    }
    let cards: Vec<Value> = corpus
        .docs()
        .iter()
        .map(|d| json!({"oid": d.oid, "card": {"title": d.title, "summary": d.body}}))
        .collect();
    Ok(json!({"schema": SCHEMA, "query": query, "cards": cards}).to_string())
}

/// Strict parse of the bridge's stdout.
pub fn parse_response(stdout: &str) -> Result<M0Ranked, M0Error> {
    let line = stdout.lines().rev().find(|l| !l.trim().is_empty()).ok_or_else(|| M0Error::Schema("stdout vuoto".into()))?;
    let v: Value = serde_json::from_str(line).map_err(|e| M0Error::Schema(e.to_string()))?;
    if v.get("status").and_then(Value::as_str) != Some("RankedCandidates") {
        return Err(M0Error::Schema("status != RankedCandidates".into()));
    }
    if v.get("auto_resolve").and_then(Value::as_bool) != Some(false) {
        return Err(M0Error::AutoResolve);
    }
    let epoch = v.get("embedding_epoch").and_then(Value::as_str).unwrap_or_default().to_string();
    if epoch != PINNED_EPOCH {
        return Err(M0Error::Epoch(epoch));
    }
    let arr = v.get("candidates").and_then(Value::as_array).ok_or_else(|| M0Error::Schema("candidates mancante".into()))?;
    let mut ranked = Vec::with_capacity(arr.len());
    for c in arr {
        let oid = c.get("oid").and_then(Value::as_str).ok_or_else(|| M0Error::Schema("oid".into()))?;
        let score = c.get("score").and_then(Value::as_f64).ok_or_else(|| M0Error::Schema("score".into()))?;
        ranked.push((oid.to_string(), score));
    }
    Ok(M0Ranked { epoch, ranked })
}

/// Map an M0 ranking to local handles, best-first; unknown/duplicate oids dropped.
pub fn to_handles(r: &M0Ranked, corpus: &Corpus) -> Vec<Handle> {
    let mut seen = BTreeSet::new();
    r.ranked
        .iter()
        .filter_map(|(oid, _)| corpus.by_oid(oid).map(|d| d.handle))
        .filter(|h| seen.insert(*h))
        .collect()
}

/// Run the bridge once (blocking, bounded by `cfg.timeout`).
pub fn run_bridge(cfg: &M0Config, request: &str) -> Result<M0Ranked, M0Error> {
    cfg.check()?;
    let mut child = Command::new(&cfg.python)
        .arg("bridge_recall.py")
        .current_dir(&cfg.m0_dir)
        .env("HF_HUB_OFFLINE", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| M0Error::Spawn(e.to_string()))?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| M0Error::Spawn("stdin".into()))?;
        stdin.write_all(request.as_bytes()).and_then(|_| stdin.write_all(b"\n")).map_err(|e| M0Error::Spawn(e.to_string()))?;
    }
    let mut out = child.stdout.take().ok_or_else(|| M0Error::Spawn("stdout".into()))?;
    let mut err = child.stderr.take().ok_or_else(|| M0Error::Spawn("stderr".into()))?;
    let out_t = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let err_t = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > cfg.timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(M0Error::Timeout(cfg.timeout));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(M0Error::Spawn(e.to_string())),
        }
    };
    let stdout = out_t.join().unwrap_or_default();
    let stderr = err_t.join().unwrap_or_default();
    if !status.success() {
        return Err(M0Error::Exit { code: status.code(), stderr });
    }
    parse_response(&stdout)
}

/// Last run of the lane, for the glass status bar / audit.
#[derive(Clone, Debug, PartialEq)]
pub enum M0Status {
    NotRun,
    Ok { query: String, ranked: usize, mapped: usize, millis: u128 },
    Failed { query: String, error: String },
}

/// Read-only view of the lane's last run (kept by the UI after the engine owns the lane).
#[derive(Clone, Debug, Default)]
pub struct M0StatusHandle(Arc<Mutex<Option<M0Status>>>);

impl M0StatusHandle {
    pub fn get(&self) -> M0Status {
        self.0.lock().ok().and_then(|g| g.clone()).unwrap_or(M0Status::NotRun)
    }
    fn set(&self, s: M0Status) {
        if let Ok(mut g) = self.0.lock() {
            *g = Some(s);
        }
    }
}

/// The real M0 dense+BM25 lane as an intent [`Retriever`] (`Source::M0`).
pub struct M0Process {
    cfg: M0Config,
    status: M0StatusHandle,
}

impl M0Process {
    pub fn new(cfg: M0Config) -> Self {
        Self { cfg, status: M0StatusHandle::default() }
    }
    pub fn config(&self) -> &M0Config {
        &self.cfg
    }
    pub fn status(&self) -> M0Status {
        self.status.get()
    }
    pub fn status_handle(&self) -> M0StatusHandle {
        self.status.clone()
    }
    fn set(&self, s: M0Status) {
        self.status.set(s);
    }
}

impl Retriever for M0Process {
    fn source(&self) -> Source {
        Source::M0
    }
    fn rank(&self, query: &str, corpus: &Corpus) -> Vec<Handle> {
        let t = Instant::now();
        let result = build_request(query, corpus).and_then(|req| run_bridge(&self.cfg, &req));
        match result {
            Ok(r) => {
                let handles = to_handles(&r, corpus);
                self.set(M0Status::Ok { query: query.into(), ranked: r.ranked.len(), mapped: handles.len(), millis: t.elapsed().as_millis() });
                handles
            }
            Err(e) => {
                self.set(M0Status::Failed { query: query.into(), error: e.to_string() });
                Vec::new() // fail closed: no M0 evidence, BM25 still ranks
            }
        }
    }
}

/// Convenience: is `dir` a usable M0 checkout (no process started)?
pub fn available(dir: &Path) -> bool {
    M0Config::in_dir(dir).check().is_ok()
}
