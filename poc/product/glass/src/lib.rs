//! VelaOS Glass (host demo): a clickable local page in front of [`ShellRuntime`].
//!
//! ```text
//! browser (127.0.0.1 only)                     this process
//!   GET  /            -> glass.html (+ per-run token)
//!   GET  /state       -> view model (phase, candidates, result, counters, M0 lane)
//!   POST /submit      {token, utterance}          -> ShellRuntime::submit
//!   POST /click       {token, action, candidate}  -> ShellRuntime::click
//!        action = avvia | rivedi | annulla | chiudi
//! ```
//!
//! The primary gesture is labeled Seleziona (synonym Scegli proposta). Its wire
//! name stays `avvia`. Intent boundary: that click is a UserSelection of a shown
//! index only. Intent does not execute. Glass has no execute route.
//!
//! The ONLY way toward the PDP is `POST /click` with `avvia`/`rivedi` and the
//! INDEX of a candidate currently shown on the glass. The body can carry no
//! handle, version, selection, token or frame: unknown fields are rejected (400)
//! and the shell re-derives `UserSelection{handle, expected_version}` from the
//! displayed row. There is no admit/grant/select route. A zero-match Intent
//! result is still a Proposal: rationale, empty candidates, authority false.
//!
//! Host demo hardening (not an isolation boundary): loopback bind, `Host` header
//! pinned to the bound port, per-run random token, JSON content-type required,
//! foreign `Origin` refused, bounded request size, single-threaded.
//!
//! Allowed surfaces and bans: [`catalog`] and `docs/ALLOWED_COMPONENTS.md`.
//! Glass draws ephemeral panels above the ISO §7 desktop. It does not own that
//! chrome, and it is not an authority.
#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vela_m0_recall::{M0Status, M0StatusHandle};
use vela_shell::{Click, Outcome, ShellError, ShellRuntime, ShellState, UiReject};

pub mod catalog;

pub const PAGE: &str = include_str!("../assets/glass.html");
pub const MAX_BODY: usize = 16 * 1024;
pub const MAX_UTTERANCE: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub host: Option<String>,
    pub origin: Option<String>,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

impl Request {
    /// Convenience for tests: a JSON POST from the page.
    pub fn post(path: &str, host: &str, body: Value) -> Self {
        Self {
            method: "POST".into(),
            path: path.into(),
            host: Some(host.into()),
            origin: Some(format!("http://{host}")),
            content_type: Some("application/json".into()),
            body: body.to_string().into_bytes(),
        }
    }
    pub fn get(path: &str, host: &str) -> Self {
        Self { method: "GET".into(), path: path.into(), host: Some(host.into()), origin: None, content_type: None, body: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Response {
    fn json(status: u16, v: Value) -> Self {
        Self { status, content_type: "application/json; charset=utf-8", body: v.to_string() }
    }
    fn err(status: u16, msg: &str) -> Self {
        Self::json(status, json!({"error": msg}))
    }
    pub fn json_body(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

/// Which recall lanes feed RRF (shown on the glass).
#[derive(Clone, Debug)]
pub enum Lanes {
    /// BM25 (Rust) + real M0 (e5-small dense + LanceDB BM25, RRF) via bridge_recall.py.
    Bm25PlusM0(M0StatusHandle),
    /// BM25 + deterministic title stand-in (M0 not available on this host).
    Bm25PlusStandIn(String),
    /// Anything else wired by a test.
    Custom(String),
}

pub struct GlassApp {
    shell: ShellRuntime,
    token: String,
    hosts: Vec<String>,
    lanes: Lanes,
    /// Utterance of the latest accepted submit. The shell drops it on NothingFound.
    last_utterance: String,
}

fn random_token() -> String {
    let a = RandomState::new().hash_one(SystemTime::now());
    let b = RandomState::new().hash_one(std::process::id());
    format!("{a:016x}{b:016x}")
}

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn unix(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl GlassApp {
    pub fn new(shell: ShellRuntime, lanes: Lanes, port: u16) -> Self {
        Self::with_token(shell, lanes, port, random_token())
    }
    pub fn with_token(shell: ShellRuntime, lanes: Lanes, port: u16, token: String) -> Self {
        Self {
            shell,
            token,
            hosts: vec![format!("127.0.0.1:{port}"), format!("localhost:{port}")],
            lanes,
            last_utterance: String::new(),
        }
    }
    pub fn token(&self) -> &str {
        &self.token
    }
    pub fn shell(&self) -> &ShellRuntime {
        &self.shell
    }

    /// Route one request. Exhaustive: anything not listed is 404/405.
    pub fn handle(&mut self, req: &Request) -> Response {
        if !req.host.as_ref().is_some_and(|h| self.hosts.contains(h)) {
            return Response::err(403, "host non consentito");
        }
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/") => Response {
                status: 200,
                content_type: "text/html; charset=utf-8",
                body: PAGE.replace("__VELA_TOKEN__", &self.token),
            },
            ("GET", "/state") => Response::json(200, self.view()),
            ("POST", "/submit") | ("POST", "/click") => self.post(req),
            (_, "/" | "/state" | "/submit" | "/click") => Response::err(405, "metodo non consentito"),
            _ => Response::err(404, "nessuna route"),
        }
    }

    fn post(&mut self, req: &Request) -> Response {
        if let Some(o) = &req.origin
            && !self.hosts.iter().any(|h| o == &format!("http://{h}"))
        {
            return Response::err(403, "origin non consentita");
        }
        if !req.content_type.as_deref().is_some_and(|c| c.starts_with("application/json")) {
            return Response::err(415, "serve application/json");
        }
        if req.body.len() > MAX_BODY {
            return Response::err(413, "body troppo grande");
        }
        let Ok(Value::Object(body)) = serde_json::from_slice::<Value>(&req.body) else {
            return Response::err(400, "json non valido");
        };
        if body.get("token").and_then(Value::as_str) != Some(self.token.as_str()) {
            return Response::err(403, "token mancante o errato");
        }
        let allowed: &[&str] = if req.path == "/submit" { catalog::SUBMIT_FIELDS } else { catalog::CLICK_FIELDS };
        if let Some(k) = body.keys().find(|k| !allowed.contains(&k.as_str())) {
            return Response::err(400, &format!("campo non ammesso: {k}"));
        }
        let result = if req.path == "/submit" {
            let Some(u) = body.get("utterance").and_then(Value::as_str) else {
                return Response::err(400, "utterance mancante");
            };
            let u = u.trim();
            if u.is_empty() || u.len() > MAX_UTTERANCE {
                return Response::err(400, "utterance vuota o troppo lunga");
            }
            match self.shell.submit(u) {
                Ok(_) => {
                    self.last_utterance = u.to_string();
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let idx = body.get("candidate").and_then(Value::as_u64).map(|n| n as usize);
            let action = body.get("action").and_then(Value::as_str);
            let known = action.is_some_and(|a| catalog::ALLOWED_CLICK_ACTIONS.contains(&a));
            let click = match (action, idx) {
                (Some("avvia"), Some(candidate)) if known => Click::Avvia { candidate },
                (Some("rivedi"), Some(candidate)) if known => Click::RivediPiano { candidate },
                (Some("annulla"), None) if known => Click::Annulla,
                (Some("chiudi"), None) if known => Click::Chiudi,
                _ => return Response::err(400, "action/candidate non validi"),
            };
            self.shell.click(click).map(|_| ())
        };
        match result {
            Ok(()) => Response::json(200, self.view()),
            Err(UiReject::WrongState(p)) => Response::json(409, json!({"error": format!("gesto non valido nello stato {p:?}"), "state": self.view()})),
            Err(UiReject::NotOffered(i)) => Response::json(409, json!({"error": format!("candidato {i} non mostrato"), "state": self.view()})),
        }
    }

    fn lanes_view(&self) -> Value {
        match &self.lanes {
            Lanes::Bm25PlusM0(h) => {
                let m0 = match h.get() {
                    M0Status::NotRun => json!({"state": "non eseguito"}),
                    M0Status::Ok { ranked, mapped, millis, .. } => json!({"state": "ok", "ranked": ranked, "mapped": mapped, "millis": millis as u64}),
                    M0Status::Failed { error, .. } => json!({"state": "errore", "error": error}),
                };
                json!({"label": "BM25 + M0 reale (e5-small + LanceDB BM25) -> RRF k=60", "m0": m0})
            }
            Lanes::Bm25PlusStandIn(why) => json!({"label": "BM25 + title stand-in -> RRF k=60 (M0 dense non attivo)", "m0": {"state": "assente", "error": why}}),
            Lanes::Custom(l) => json!({"label": l, "m0": {"state": "n/a"}}),
        }
    }

    /// View model. Exposes handles as hex for display/audit only; the page never sends them back.
    pub fn view(&self) -> Value {
        let state = match self.shell.state() {
            ShellState::Idle => json!({"phase": "idle"}),
            ShellState::Glass(p) => json!({
                "phase": "glass",
                "utterance": p.utterance,
                "auto_resolve": p.auto_resolve(),
                "authority": false,
                "rationale": p.rationale,
                "candidates": p.candidates.iter().enumerate().map(|(i, c)| json!({
                    "index": i, "title": c.title, "version": c.version, "score": c.score,
                    "sources": c.sources.iter().map(|s| format!("{s:?}")).collect::<Vec<_>>(),
                    "handle": hex(&c.handle),
                })).collect::<Vec<_>>(),
            }),
            ShellState::Result { selection, action, outcome } => {
                let o = match outcome {
                    Outcome::Executed { request_id, value, not_after } => json!({"kind": "executed", "request_id": request_id, "value": value, "not_after_unix": unix(*not_after)}),
                    Outcome::Reviewed { request_id, not_after } => json!({"kind": "reviewed", "request_id": request_id, "not_after_unix": unix(*not_after)}),
                };
                json!({"phase": "result", "action": format!("{action:?}"),
                       "selection": {"handle": hex(&selection.handle), "expected_version": selection.expected_version}, "outcome": o})
            }
            ShellState::Error { selection, error } => match error {
                // Intent returned no real row. Still a Proposal: no invented app, URL, or channel.
                ShellError::NothingFound => json!({
                    "phase": "glass",
                    "utterance": self.last_utterance,
                    "auto_resolve": false,
                    "authority": false,
                    "rationale": catalog::ZERO_MATCH_RATIONALE,
                    "candidates": [],
                    "dismiss": "chiudi",
                }),
                ShellError::Denied { code, stage, reason } => json!({
                    "phase": "error",
                    "error": {"kind": "denied", "code": format!("{code:?}"), "contract_code": code.contract_code(), "stage": format!("{stage:?}"), "reason": reason},
                    "selection": selection.map(|s| json!({"handle": hex(&s.handle), "expected_version": s.expected_version})),
                }),
                ShellError::Executor { status } => json!({
                    "phase": "error",
                    "error": {"kind": "executor", "status": status},
                    "selection": selection.map(|s| json!({"handle": hex(&s.handle), "expected_version": s.expected_version})),
                }),
            },
        };
        json!({
            "state": state,
            "trace": self.shell.trace().iter().map(|p| format!("{p:?}")).collect::<Vec<_>>(),
            "selections_emitted": self.shell.emitted_selections().len(),
            "grants_issued": self.shell.grants_issued(),
            "frames_signed": self.shell.frames_signed(),
            "lanes": self.lanes_view(),
        })
    }
}

/// Parse one HTTP/1.1 request (bounded). `None` on malformed input.
pub fn read_request(stream: &mut TcpStream) -> Option<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut r = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    r.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.split('?').next()?.to_string();
    let (mut host, mut origin, mut ct, mut len) = (None, None, None, 0usize);
    for _ in 0..64 {
        let mut h = String::new();
        if r.read_line(&mut h).ok()? == 0 {
            break;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        let (k, v) = h.split_once(':')?;
        let v = v.trim().to_string();
        match k.trim().to_ascii_lowercase().as_str() {
            "host" => host = Some(v),
            "origin" => origin = Some(v),
            "content-type" => ct = Some(v),
            "content-length" => len = v.parse().ok()?,
            _ => {}
        }
    }
    if len > MAX_BODY {
        return Some(Request { method, path, host, origin, content_type: ct, body: vec![0; MAX_BODY + 1] });
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body).ok()?;
    Some(Request { method, path, host, origin, content_type: ct, body })
}

pub fn write_response(stream: &mut TcpStream, resp: &Response) {
    let reason = match resp.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nConnection: close\r\n\r\n",
        resp.status,
        resp.content_type,
        resp.body.len()
    );
    let _ = stream.write_all(head.as_bytes()).and_then(|_| stream.write_all(resp.body.as_bytes()));
}

/// Serve sequentially on `listener` (must be bound to loopback). `max` bounds the
/// number of requests (tests); `None` serves forever.
pub fn serve(listener: &TcpListener, app: &mut GlassApp, max: Option<usize>) {
    let mut n = 0;
    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        let resp = match read_request(&mut s) {
            Some(req) => app.handle(&req),
            None => Response::err(400, "richiesta malformata"),
        };
        write_response(&mut s, &resp);
        n += 1;
        if max.is_some_and(|m| n >= m) {
            break;
        }
    }
}

/// Host demo wiring (demo keys, demo corpus). NOT production key management.
pub mod demo {
    use biscuit_auth::{KeyPair as RootKeys, builder::BiscuitBuilder};
    use ed25519_compact::{KeyPair as EdKeys, PublicKey, Seed};
    use vela_executor_host::HostExecutor;
    use vela_executor_policy::pipeline::SelectionGatedPdp;
    use vela_intent::{Corpus, Doc, Handle, IntentEngine};
    use vela_m0_recall::{M0Config, M0Process};
    use vela_shell::ShellRuntime;

    pub const USER: &str = "vela-user";
    pub const NOTES: Handle = [0xA1; 32];
    pub const EXACT_PROFILE: Handle = [0x11; 32];
    pub const ESPRESSO: Handle = [0x12; 32];
    pub const SECRETS: Handle = [0xC3; 32];

    /// Local card metadata: the five authored M0 fixture cards (oid = M0 label,
    /// entities inlined like `hybrid_recall.card_text`) + two workspace items.
    pub fn corpus() -> Corpus {
        let d = |handle: Handle, version: u64, oid: &str, title: &str, body: &str| Doc {
            handle,
            version,
            oid: oid.into(),
            title: title.into(),
            body: body.into(),
        };
        Corpus::new(vec![
            d(EXACT_PROFILE, 2, "exact_profile", "Recorded configuration", "Archived configuration state for an earlier network session.\nDNS 9.9.9.9\nSSID ZEPHYR-731\nUUID 8b9539b7-3c6f-4bb1-9355-ae58471cf013"),
            d(ESPRESSO, 4, "semantic_profile", "Espresso wireless session", "A laptop connects over encrypted Wi-Fi at a coffee shop. A secure tunnel towards Milan protects traffic on the public hotspot.\nDNS 1.1.1.1\nSSID CafeGuest-5G"),
            d([0x13; 32], 1, "home_profile", "Home entertainment connection", "A home router serves the television and game console with automatic addresses. No secure tunnel is enabled.\nDNS 8.8.8.8\nSSID HomeLan-5G"),
            d([0x14; 32], 1, "guide", "Wireless troubleshooting handbook", "Instructions for diagnosing a network name, resolver and connectivity. This is a guide, not a saved configuration.\ntopic connectivity"),
            d([0x15; 32], 1, "media", "Ferry at sunset", "A short film of a sailboat crossing the bay in golden evening light.\ntopic sailing"),
            d(NOTES, 3, "oid-notes", "Note progetto VelaOS (bozza)", "architettura microkernel note riunione seL4 LFS Biscuit"),
            d(SECRETS, 9, "oid-keys", "Chiavi private", "seed wallet backup"),
        ])
    }

    pub fn delegation(root: &RootKeys, resource: &str, op: &str) -> Vec<u8> {
        BiscuitBuilder::new()
            .code(format!("right(\"{USER}\", \"{resource}\", \"{op}\");"))
            .expect("datalog")
            .build(root)
            .expect("build")
            .to_vec()
            .expect("serialize")
    }

    /// Demo PCB key (fixed seed, NOT a production key).
    pub fn pcb() -> EdKeys {
        EdKeys::from_seed(Seed::new([41; 32]))
    }

    pub fn shell(engine: IntentEngine) -> ShellRuntime {
        let root = RootKeys::new();
        let pdp = SelectionGatedPdp::new(root.public(), delegation(&root, "lfs", "read"), delegation(&root, "executor", "execute"), pcb().sk)
            .expect("pdp");
        ShellRuntime::new(USER, engine, pdp, HostExecutor::new(PublicKey::new(*pcb().pk)))
    }

    /// Real M0 lane when `cfg` is usable, else BM25 + stand-in (reason reported).
    pub fn engine(cfg: Option<M0Config>) -> (IntentEngine, super::Lanes) {
        match cfg.map(|c| c.check().map(|_| c)) {
            Some(Ok(c)) => {
                let lane = M0Process::new(c);
                let status = lane.status_handle();
                (IntentEngine::bm25_only(corpus()).with_retriever(Box::new(lane)), super::Lanes::Bm25PlusM0(status))
            }
            Some(Err(e)) => (IntentEngine::new(corpus()), super::Lanes::Bm25PlusStandIn(e.to_string())),
            None => (IntentEngine::new(corpus()), super::Lanes::Bm25PlusStandIn("VELA_M0_DIR / --m0 non impostato".into())),
        }
    }
}
