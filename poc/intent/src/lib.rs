//! VelaOS Intent Engine (host, `std`, zero dependencies).
//!
//! ```text
//! utterance -> [Retriever lanes: BM25, title/dense stand-in, M0 ranking, LLM proposals]
//!           -> Reciprocal Rank Fusion (k = 60, same K as poc/m0 bridge_recall.py)
//!           -> IntentProposal { candidates: [Candidate{handle, version, ...}], rationale }
//! ```
//!
//! "Candidate != Authority". Everything this crate outputs is DATA:
//! * a [`Candidate`] carries no token, no right and no "authorized" flag;
//! * an [`IntentProposal`] never auto-resolves ([`IntentProposal::auto_resolve`] is
//!   always `false`, mirroring M0 `"auto_resolve": false`);
//! * an LLM ([`LlmProposer`]) may only add *handles that already exist in the local
//!   corpus*; versions always come from the corpus, never from the model; its free
//!   text is kept as untrusted `rationale` for display and is never parsed;
//! * there is no execute / grant / admit / sign API here and the crate has no
//!   dependencies (no Biscuit, no Ed25519, no policy crate), see `tests/no_authority.rs`.
//!
//! Turning a candidate into a right requires the shell's trusted input path
//! (`UserSelection` via `TrustedSelectionChannel`) and the host PDP
//! (`vela-executor-policy::pipeline::SelectionGatedPdp`), both outside this crate.
//!
//! When lexical recall is empty, [`propose_from_context`] can still emit a
//! proposal from observational envelopes ([`FALLBACK_MODE`]). That lane does
//! not call [`LlmProposer`].
#![forbid(unsafe_code)]

mod context_lane;

pub use context_lane::{
    ContextCandidate, ContextProposal, FALLBACK_MODE, RecallMode, proposal_json, propose_from_context,
};

use std::collections::{BTreeMap, BTreeSet};

/// Opaque LFS object handle (32 bytes, same width as the policy crate's `Handle`).
pub type Handle = [u8; 32];

/// RRF constant (M0 uses `RRFReranker(K=60)`).
pub const RRF_K: f32 = 60.0;
/// Max candidates in one proposal (glass UI shows a short list).
pub const MAX_CANDIDATES: usize = 8;
/// Max extra handles an LLM may contribute per utterance.
pub const MAX_LLM_PROPOSALS: usize = 8;

/// Which lane ranked a candidate (display/audit only, never consulted by policy).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// Okapi BM25 over title + body.
    Bm25,
    /// Title-overlap lane: deterministic stand-in for the M0 dense (embedding) lane.
    Title,
    /// Ranking produced by poc/m0 `bridge_recall.py` (`RankedCandidates`).
    M0,
    /// Extra handle proposed by an LLM.
    Llm,
    /// Observational lane. Display label is [`Source::as_str`] = `ContextSignal`.
    /// Ranking and UX only.
    ContextSignal,
}

impl Source {
    /// Stable audit label. Not a right.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Bm25 => "Bm25",
            Source::Title => "Title",
            Source::M0 => "M0",
            Source::Llm => "Llm",
            Source::ContextSignal => "ContextSignal",
        }
    }
}

/// One local object the engine may rank. Metadata only (no content handle rights).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub handle: Handle,
    pub version: u64,
    /// M0 object id (`oid`), used to map M0 rankings back to handles.
    pub oid: String,
    pub title: String,
    pub body: String,
}

/// The local recall corpus (card metadata, not object contents).
#[derive(Clone, Debug, Default)]
pub struct Corpus {
    docs: Vec<Doc>,
}

impl Corpus {
    pub fn new(docs: Vec<Doc>) -> Self {
        Self { docs }
    }
    pub fn docs(&self) -> &[Doc] {
        &self.docs
    }
    pub fn get(&self, handle: &Handle) -> Option<&Doc> {
        self.docs.iter().find(|d| &d.handle == handle)
    }
    pub fn by_oid(&self, oid: &str) -> Option<&Doc> {
        self.docs.iter().find(|d| d.oid == oid)
    }
    /// Metadata update (new object version). Does not touch any proposal already shown.
    pub fn bump_version(&mut self, handle: &Handle, version: u64) -> bool {
        match self.docs.iter_mut().find(|d| &d.handle == handle) {
            Some(d) => {
                d.version = version;
                true
            }
            None => false,
        }
    }
}

/// A ranked recall result. DATA ONLY: it confers nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub handle: Handle,
    /// Object version as known to the corpus when proposed (becomes the
    /// `expected_version` of a later user selection).
    pub version: u64,
    pub title: String,
    /// Fused RRF score (display/ranking only).
    pub score: f32,
    /// Lanes that ranked this candidate.
    pub sources: Vec<Source>,
}

/// What the user clicked: "this handle, at the version I was shown".
///
/// Plain data, freely constructible, and therefore NOT authority. The shell
/// re-derives it from the displayed candidate on a real click and hands it to
/// the trusted selection channel; only the policy crate's `UserSelection`
/// (minted by `TrustedSelectionChannel`) is consumed by the PDP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserSelection {
    pub handle: Handle,
    pub expected_version: u64,
}

impl Candidate {
    /// The selection a user would make by clicking this candidate.
    pub fn as_selection(&self) -> UserSelection {
        UserSelection { handle: self.handle, expected_version: self.version }
    }
}

/// Output of [`IntentEngine::propose_candidates`].
#[derive(Clone, Debug, PartialEq)]
pub struct IntentProposal {
    pub utterance: String,
    pub candidates: Vec<Candidate>,
    /// Untrusted free text (LLM rationale, possibly injected). Display only.
    pub rationale: String,
}

impl IntentProposal {
    /// Never true: a proposal is never resolved without a human choice.
    pub fn auto_resolve(&self) -> bool {
        false
    }
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// A recall lane. Returns handles best-first; only handles of `corpus` count.
pub trait Retriever {
    fn source(&self) -> Source;
    fn rank(&self, query: &str, corpus: &Corpus) -> Vec<Handle>;
}

/// Lower-case alphanumeric tokens, length > 2 (drops most IT/EN function words).
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() > 2)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Okapi BM25 over `title + body`.
#[derive(Clone, Copy, Debug)]
pub struct Bm25 {
    pub k1: f32,
    pub b: f32,
}

impl Default for Bm25 {
    fn default() -> Self {
        Self { k1: 1.2, b: 0.75 }
    }
}

impl Bm25 {
    pub fn scores(&self, query: &str, corpus: &Corpus) -> Vec<(Handle, f32)> {
        let docs: Vec<Vec<String>> = corpus
            .docs()
            .iter()
            .map(|d| tokenize(&format!("{} {}", d.title, d.body)))
            .collect();
        let n = docs.len() as f32;
        if docs.is_empty() {
            return Vec::new();
        }
        let avgdl = docs.iter().map(Vec::len).sum::<usize>() as f32 / n;
        let terms: BTreeSet<String> = tokenize(query).into_iter().collect();
        let mut out = Vec::new();
        for (doc, toks) in corpus.docs().iter().zip(&docs) {
            let dl = toks.len() as f32;
            let mut s = 0.0f32;
            for t in &terms {
                let tf = toks.iter().filter(|x| *x == t).count() as f32;
                if tf == 0.0 {
                    continue;
                }
                let df = docs.iter().filter(|d| d.contains(t)).count() as f32;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                s += idf * tf * (self.k1 + 1.0)
                    / (tf + self.k1 * (1.0 - self.b + self.b * dl / avgdl.max(1.0)));
            }
            if s > 0.0 {
                out.push((doc.handle, s));
            }
        }
        out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }
}

impl Retriever for Bm25 {
    fn source(&self) -> Source {
        Source::Bm25
    }
    fn rank(&self, query: &str, corpus: &Corpus) -> Vec<Handle> {
        self.scores(query, corpus).into_iter().map(|(h, _)| h).collect()
    }
}

/// Fraction of query terms present in the title. Deterministic stand-in for the
/// M0 dense lane, used only when the real M0 lane is unavailable on the host.
#[derive(Clone, Copy, Debug, Default)]
pub struct TitleOverlap;

impl Retriever for TitleOverlap {
    fn source(&self) -> Source {
        Source::Title
    }
    fn rank(&self, query: &str, corpus: &Corpus) -> Vec<Handle> {
        let q: BTreeSet<String> = tokenize(query).into_iter().collect();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(Handle, usize)> = corpus
            .docs()
            .iter()
            .map(|d| {
                let t: BTreeSet<String> = tokenize(&d.title).into_iter().collect();
                (d.handle, q.intersection(&t).count())
            })
            .filter(|(_, c)| *c > 0)
            .collect();
        hits.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        hits.into_iter().map(|(h, _)| h).collect()
    }
}

/// Connector for the M0 control-plane output (`poc/m0/bridge_recall.py`,
/// `{"status":"RankedCandidates","auto_resolve":false,"candidates":[{"oid","score"}]}`).
/// Holds a pre-computed ranking for ONE query (e.g. a recorded bridge output);
/// oids unknown to the corpus are dropped. The live process lane is
/// `vela-m0-recall::M0Process` (poc/product/m0-recall), kept out of this crate
/// so the engine itself stays free of process/env/fs access.
#[derive(Clone, Debug, Default)]
pub struct M0Ranking {
    pub query: String,
    /// `(oid, score)` best-first, as printed by bridge_recall.py.
    pub ranked: Vec<(String, f32)>,
}

impl Retriever for M0Ranking {
    fn source(&self) -> Source {
        Source::M0
    }
    fn rank(&self, query: &str, corpus: &Corpus) -> Vec<Handle> {
        if query != self.query {
            return Vec::new(); // a ranking for another query is not evidence
        }
        let mut seen = BTreeSet::new();
        self.ranked
            .iter()
            .filter_map(|(oid, _)| corpus.by_oid(oid).map(|d| d.handle))
            .filter(|h| seen.insert(*h))
            .collect()
    }
}

/// What an LLM returns: handles it thinks are relevant + free text.
#[derive(Clone, Debug, Default)]
pub struct LlmOutput {
    pub handles: Vec<Handle>,
    pub rationale: String,
}

/// An LLM adapter. It may only PROPOSE extra candidates.
pub trait LlmProposer {
    fn propose(&self, utterance: &str, corpus: &Corpus) -> LlmOutput;
}

/// Reciprocal Rank Fusion: `score(d) = sum_lanes 1 / (k + rank)`, rank 1-based.
pub fn rrf(lanes: &[(Source, Vec<Handle>)], k: f32) -> Vec<(Handle, f32, Vec<Source>)> {
    let mut acc: BTreeMap<Handle, (f32, BTreeSet<Source>)> = BTreeMap::new();
    for (src, ranked) in lanes {
        let mut seen = BTreeSet::new();
        for (i, h) in ranked.iter().filter(|h| seen.insert(**h)).enumerate() {
            let e = acc.entry(*h).or_default();
            e.0 += 1.0 / (k + (i + 1) as f32);
            e.1.insert(*src);
        }
    }
    let mut out: Vec<(Handle, f32, Vec<Source>)> = acc
        .into_iter()
        .map(|(h, (s, src))| (h, s, src.into_iter().collect()))
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

/// Local intent engine: retrievers (+ optional LLM) -> RRF -> candidates.
pub struct IntentEngine {
    corpus: Corpus,
    retrievers: Vec<Box<dyn Retriever>>,
    llm: Option<Box<dyn LlmProposer>>,
}

impl IntentEngine {
    /// Default lexical pipeline: BM25 + title lane, fused with RRF.
    pub fn new(corpus: Corpus) -> Self {
        Self {
            corpus,
            retrievers: vec![Box::new(Bm25::default()), Box::new(TitleOverlap)],
            llm: None,
        }
    }
    /// BM25 only (no stand-in lane). Add the real M0 lane with [`Self::with_retriever`].
    pub fn bm25_only(corpus: Corpus) -> Self {
        Self { corpus, retrievers: vec![Box::new(Bm25::default())], llm: None }
    }
    pub fn with_retriever(mut self, r: Box<dyn Retriever>) -> Self {
        self.retrievers.push(r);
        self
    }
    pub fn with_llm(mut self, llm: Box<dyn LlmProposer>) -> Self {
        self.llm = Some(llm);
        self
    }
    pub fn corpus(&self) -> &Corpus {
        &self.corpus
    }
    pub fn corpus_mut(&mut self) -> &mut Corpus {
        &mut self.corpus
    }

    /// Rank candidates for `utterance`. Pure function of (utterance, corpus, lanes).
    pub fn propose_candidates(&self, utterance: &str) -> IntentProposal {
        let mut lanes: Vec<(Source, Vec<Handle>)> = self
            .retrievers
            .iter()
            .map(|r| (r.source(), r.rank(utterance, &self.corpus)))
            .collect();
        let mut rationale = String::new();
        if let Some(llm) = &self.llm {
            let out = llm.propose(utterance, &self.corpus);
            rationale = out.rationale;
            // Only handles that exist locally; capped; the LLM never sets versions.
            let known: Vec<Handle> = out
                .handles
                .into_iter()
                .filter(|h| self.corpus.get(h).is_some())
                .take(MAX_LLM_PROPOSALS)
                .collect();
            lanes.push((Source::Llm, known));
        }
        let candidates = rrf(&lanes, RRF_K)
            .into_iter()
            .filter_map(|(h, score, sources)| {
                let d = self.corpus.get(&h)?;
                Some(Candidate {
                    handle: h,
                    version: d.version,
                    title: d.title.clone(),
                    score,
                    sources,
                })
            })
            .take(MAX_CANDIDATES)
            .collect();
        IntentProposal { utterance: utterance.to_string(), candidates, rationale }
    }
}
