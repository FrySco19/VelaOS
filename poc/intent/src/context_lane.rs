//! FALLBACK CONTEXT-SIGNALS.
//!
//! Reads observational envelopes already produced elsewhere. Keeps a line only
//! when it has a non-empty `ts` and `consumers` contains `intent`. Emits
//! proposal rows labeled [`Source::ContextSignal`]. Does not call an LLM.
//!
//! INTENT_CONSUME drops an `object.handle` that is not already in recall.
//! [`RecallMode::ShownOnly`] is that rule.
//!
//! For this fallback demo, [`RecallMode::ObservedInRecall`] treats every
//! `object.handle` on a kept envelope as in-recall. Handles are copied from
//! those envelopes. None are synthesized. With the mode off and an empty shown
//! set, the ciao fixture yields no rows.

use std::collections::BTreeSet;

use crate::{Bm25, Corpus, Doc, Source};

/// Printed on every proposal from this lane.
pub const FALLBACK_MODE: &str = "FALLBACK CONTEXT-SIGNALS";

/// Whether an observed native handle may become a proposal row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecallMode {
    /// Keep a handle only when `shown` already lists it.
    ShownOnly,
    /// Also keep handles copied from consumed envelopes.
    ObservedInRecall,
}

/// One proposal row. Data only: the lane label is not a right.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextCandidate {
    /// Native `object.handle` copied from the envelope.
    pub handle: String,
    pub expected_version: Option<String>,
    pub title: String,
    /// Display ordering only.
    pub score: f32,
    /// Always [`Source::ContextSignal`] on this lane.
    pub source: Source,
    pub signal_type: String,
    pub kind: String,
    /// Envelope `source` (which observer produced the row).
    pub observation_source: String,
    pub ts: String,
    pub ttl_signal: u64,
}

/// Proposal-only result for one utterance. [`Self::auto_resolve`] is always false.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextProposal {
    pub utterance: String,
    pub mode: &'static str,
    /// Always false. This lane has no model adapter.
    pub llm_invoked: bool,
    /// Okapi BM25 hits over observational text. The scenario tag is not indexed.
    pub bm25_lexical_hits: usize,
    pub candidates: Vec<ContextCandidate>,
    pub rationale: String,
}

impl ContextProposal {
    pub fn auto_resolve(&self) -> bool {
        false
    }
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

#[derive(Clone, Debug)]
struct Envelope {
    signal_type: String,
    kind: String,
    handle: String,
    expected_version: Option<String>,
    observation_source: String,
    ts: String,
    ttl_signal: u64,
    attr_name: Option<String>,
    lexical: String,
}

/// Build a proposal from JSONL text. `shown` is the prior recall set (native
/// handle strings). This function does not read the filesystem and does not
/// call [`crate::LlmProposer`].
pub fn propose_from_context(
    utterance: &str,
    jsonl: &str,
    recall: RecallMode,
    shown: &[&str],
) -> ContextProposal {
    let parsed = parse_envelopes(jsonl);
    let observed: BTreeSet<&str> = parsed.iter().map(|e| e.handle.as_str()).collect();
    let mut candidates = Vec::new();
    let mut lexical = Vec::new();
    for env in &parsed {
        let in_shown = shown.iter().any(|s| *s == env.handle);
        let keep = match recall {
            RecallMode::ShownOnly => in_shown,
            RecallMode::ObservedInRecall => in_shown || observed.contains(env.handle.as_str()),
        };
        if !keep {
            continue;
        }
        let title = match &env.attr_name {
            Some(name) => format!("{} {} {}", env.signal_type, env.kind, name),
            None => format!("{} {}", env.signal_type, env.kind),
        };
        lexical.push(env.lexical.clone());
        candidates.push(ContextCandidate {
            handle: env.handle.clone(),
            expected_version: env.expected_version.clone(),
            title,
            score: rank_score(&env.signal_type),
            source: Source::ContextSignal,
            signal_type: env.signal_type.clone(),
            kind: env.kind.clone(),
            observation_source: env.observation_source.clone(),
            ts: env.ts.clone(),
            ttl_signal: env.ttl_signal,
        });
    }
    let bm25_lexical_hits = lexical_hit_count(utterance, &lexical);
    candidates.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.handle.cmp(&b.handle))
            .then(a.signal_type.cmp(&b.signal_type))
    });
    let rationale = format!(
        "{FALLBACK_MODE}. BM25 lexical hits={bm25_lexical_hits}. LLM not invoked. Proposal only."
    );
    ContextProposal {
        utterance: utterance.to_string(),
        mode: FALLBACK_MODE,
        llm_invoked: false,
        bm25_lexical_hits,
        candidates,
        rationale,
    }
}

/// JSON object for audit and for the demo PNG companion. Not a token.
pub fn proposal_json(p: &ContextProposal) -> String {
    let mut body = String::new();
    for (i, c) in p.candidates.iter().enumerate() {
        if i > 0 {
            body.push_str(",\n");
        }
        let version = match &c.expected_version {
            Some(v) => format!("\"{}\"", json_escape(v)),
            None => "null".to_string(),
        };
        body.push_str(&format!(
            "    {{\n      \"handle\": \"{}\",\n      \"expected_version\": {version},\n      \"title\": \"{}\",\n      \"score\": {:.2},\n      \"source\": \"{}\",\n      \"signal_type\": \"{}\",\n      \"kind\": \"{}\",\n      \"observation_source\": \"{}\",\n      \"ts\": \"{}\",\n      \"ttl_signal\": {}\n    }}",
            json_escape(&c.handle),
            json_escape(&c.title),
            c.score,
            json_escape(c.source.as_str()),
            json_escape(&c.signal_type),
            json_escape(&c.kind),
            json_escape(&c.observation_source),
            json_escape(&c.ts),
            c.ttl_signal,
            version = version,
        ));
    }
    format!(
        "{{\n  \"utterance\": \"{}\",\n  \"mode\": \"{}\",\n  \"llm_invoked\": {},\n  \"bm25_lexical_hits\": {},\n  \"auto_resolve\": {},\n  \"surface_label\": \"{}\",\n  \"authority_label\": \"no-authority\",\n  \"rationale\": \"{}\",\n  \"candidates\": [\n{body}\n  ]\n}}\n",
        json_escape(&p.utterance),
        json_escape(p.mode),
        if p.llm_invoked { "true" } else { "false" },
        p.bm25_lexical_hits,
        if p.auto_resolve() { "true" } else { "false" },
        json_escape("«nessuna autorità»"),
        json_escape(&p.rationale),
        body = body,
    )
}

fn rank_score(signal_type: &str) -> f32 {
    match signal_type {
        "service_up" => 0.90,
        "file_changed" => 0.80,
        "clock_tick" => 0.70,
        _ => 0.50,
    }
}

fn lexical_hit_count(utterance: &str, texts: &[String]) -> usize {
    let docs: Vec<Doc> = texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let mut handle = [0u8; 32];
            // Probe index only. Never copied onto a candidate handle.
            handle[31] = (i as u8).wrapping_add(1);
            Doc {
                handle,
                version: 0,
                oid: String::new(),
                title: text.clone(),
                body: String::new(),
            }
        })
        .collect();
    Bm25::default().scores(utterance, &Corpus::new(docs)).len()
}

fn parse_envelopes(jsonl: &str) -> Vec<Envelope> {
    let mut out = Vec::new();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(env) = parse_envelope(line) {
            out.push(env);
        }
    }
    out
}

fn parse_envelope(line: &str) -> Option<Envelope> {
    let value = parse_json(line)?;
    let Json::Obj(map) = value else { return None };
    let ts = match get(&map, "ts")? {
        Json::Str(s) if !s.is_empty() => s.clone(),
        _ => return None,
    };
    let consumers = get(&map, "consumers")?;
    let Json::Arr(items) = consumers else {
        return None;
    };
    let for_intent = items
        .iter()
        .any(|item| matches!(item, Json::Str(s) if s == "intent"));
    if !for_intent {
        return None;
    }
    let signal_type = nonempty_str(get(&map, "type")?)?.to_string();
    let observation_source = nonempty_str(get(&map, "source")?)?.to_string();
    let ttl_signal = match get(&map, "ttl_signal") {
        None => 0,
        Some(Json::Num(n)) => *n,
        Some(_) => return None,
    };
    let Json::Obj(object) = get(&map, "object")? else {
        return None;
    };
    let kind = nonempty_str(get(object, "kind")?)?.to_string();
    let handle = nonempty_str(get(object, "handle")?)?.to_string();
    let expected_version = match get(object, "expected_version") {
        None | Some(Json::Null) => None,
        Some(Json::Str(s)) => Some(s.clone()),
        Some(_) => return None,
    };
    let (attr_name, attr_path) = match get(&map, "attrs") {
        Some(Json::Obj(attrs)) => (
            get(attrs, "name")
                .and_then(nonempty_str)
                .map(str::to_string),
            get(attrs, "path_hint")
                .and_then(nonempty_str)
                .map(str::to_string),
        ),
        _ => (None, None),
    };
    let mut lexical_parts = vec![
        signal_type.clone(),
        kind.clone(),
        handle.clone(),
        observation_source.clone(),
    ];
    if let Some(v) = &expected_version {
        lexical_parts.push(v.clone());
    }
    if let Some(n) = &attr_name {
        lexical_parts.push(n.clone());
    }
    if let Some(p) = &attr_path {
        lexical_parts.push(p.clone());
    }
    Some(Envelope {
        signal_type,
        kind,
        handle,
        expected_version,
        observation_source,
        ts,
        ttl_signal,
        attr_name,
        lexical: lexical_parts.join(" "),
    })
}

fn get<'a>(map: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn nonempty_str(v: &Json) -> Option<&str> {
    match v {
        Json::Str(s) if !s.is_empty() => Some(s.as_str()),
        _ => None,
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

fn parse_json(input: &str) -> Option<Json> {
    let mut p = Parser {
        s: input.as_bytes(),
        i: 0,
    };
    let value = p.parse_value()?;
    p.skip();
    if p.i != p.s.len() {
        return None;
    }
    Some(value)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i += 1;
        Some(b)
    }
    fn parse_value(&mut self) -> Option<Json> {
        self.skip();
        match self.peek()? {
            b'n' => self.parse_lit(b"null", Json::Null),
            b't' => self.parse_lit(b"true", Json::Bool(true)),
            b'f' => self.parse_lit(b"false", Json::Bool(false)),
            b'"' => self.parse_string().map(Json::Str),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'0'..=b'9' => self.parse_number(),
            _ => None,
        }
    }
    fn parse_lit(&mut self, lit: &[u8], value: Json) -> Option<Json> {
        for expected in lit {
            if self.bump()? != *expected {
                return None;
            }
        }
        Some(value)
    }
    fn parse_number(&mut self) -> Option<Json> {
        let start = self.i;
        if !self.peek()?.is_ascii_digit() {
            return None;
        }
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.i += 1;
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E' | b'+' | b'-')) {
            return None;
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).ok()?;
        Some(Json::Num(text.parse().ok()?))
    }
    fn parse_string(&mut self) -> Option<String> {
        if self.bump() != Some(b'"') {
            return None;
        }
        let mut out = Vec::new();
        loop {
            match self.bump()? {
                b'"' => break,
                b'\\' => match self.bump()? {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'/' => out.push(b'/'),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'u' => {
                        let mut hex = [0u8; 4];
                        for slot in &mut hex {
                            *slot = self.bump()?;
                        }
                        let digits = std::str::from_utf8(&hex).ok()?;
                        let code = u32::from_str_radix(digits, 16).ok()?;
                        let ch = char::from_u32(code)?;
                        let mut buf = [0u8; 4];
                        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                    }
                    _ => return None,
                },
                b if b < 0x20 => return None,
                b => out.push(b),
            }
        }
        String::from_utf8(out).ok()
    }
    fn parse_array(&mut self) -> Option<Json> {
        if self.bump() != Some(b'[') {
            return None;
        }
        self.skip();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Some(Json::Arr(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip();
            match self.bump()? {
                b']' => break,
                b',' => continue,
                _ => return None,
            }
        }
        Some(Json::Arr(items))
    }
    fn parse_object(&mut self) -> Option<Json> {
        if self.bump() != Some(b'{') {
            return None;
        }
        self.skip();
        let mut pairs = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Some(Json::Obj(pairs));
        }
        loop {
            self.skip();
            let key = self.parse_string()?;
            self.skip();
            if self.bump() != Some(b':') {
                return None;
            }
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip();
            match self.bump()? {
                b'}' => break,
                b',' => continue,
                _ => return None,
            }
        }
        Some(Json::Obj(pairs))
    }
}
