//! FALLBACK CONTEXT-SIGNALS for the ciao fixture.

use std::fs;
use std::process::Command;

use vela_intent::{FALLBACK_MODE, RecallMode, Source, proposal_json, propose_from_context};

const FIXTURE: &str = include_str!("../fixtures/ciao-demo-signals.jsonl");

const HOST: &str = "pc:DESKTOP-G2CD2J5:user:tella";
const CARGO: &str = "pc:DESKTOP-G2CD2J5:userprofile:.cargo";
const CLOCK: &str = "pc:DESKTOP-G2CD2J5:local";

fn ciao() -> vela_intent::ContextProposal {
    propose_from_context("ciao", FIXTURE, RecallMode::ObservedInRecall, &[])
}

#[test]
fn ciao_fallback_emits_the_three_observed_handles_and_no_others() {
    let p = ciao();
    assert_eq!(p.utterance, "ciao");
    assert_eq!(p.mode, FALLBACK_MODE);
    assert_eq!(p.mode, "FALLBACK CONTEXT-SIGNALS");
    assert!(!p.llm_invoked);
    assert_eq!(p.bm25_lexical_hits, 0);
    assert!(!p.auto_resolve());
    assert!(!p.is_empty());
    let handles: Vec<_> = p.candidates.iter().map(|c| c.handle.as_str()).collect();
    assert_eq!(handles, vec![HOST, CARGO, CLOCK]);
    assert!(
        p.candidates
            .iter()
            .all(|c| c.source == Source::ContextSignal)
    );
    assert!(
        p.candidates
            .iter()
            .all(|c| c.source.as_str() == "ContextSignal")
    );
    assert_eq!(p.candidates[0].signal_type, "service_up");
    assert_eq!(p.candidates[0].kind, "host_session");
    assert_eq!(p.candidates[0].expected_version.as_deref(), Some("1"));
    assert_eq!(p.candidates[1].signal_type, "file_changed");
    assert_eq!(
        p.candidates[1].expected_version.as_deref(),
        Some("2026-10-12T03:38:52+02:00")
    );
    assert_eq!(p.candidates[2].signal_type, "clock_tick");
    assert_eq!(p.candidates[2].expected_version, None);
    assert!(p.rationale.contains("LLM not invoked"));
}

#[test]
fn strict_recall_drops_handles_that_were_not_already_shown() {
    let empty = propose_from_context("ciao", FIXTURE, RecallMode::ShownOnly, &[]);
    assert!(
        empty.candidates.is_empty(),
        "unshown handles are dropped before ranking"
    );
    let only_cargo = propose_from_context("ciao", FIXTURE, RecallMode::ShownOnly, &[CARGO]);
    assert_eq!(only_cargo.candidates.len(), 1);
    assert_eq!(only_cargo.candidates[0].handle, CARGO);
    let extra = propose_from_context(
        "ciao",
        FIXTURE,
        RecallMode::ObservedInRecall,
        &["pc:not-in-the-fixture"],
    );
    assert!(
        extra
            .candidates
            .iter()
            .all(|c| c.handle != "pc:not-in-the-fixture")
    );
    assert_eq!(extra.candidates.len(), 3);
}

#[test]
fn lines_without_ts_or_intent_are_dropped_and_handles_are_not_invented() {
    let jsonl = r#"
{"type":"clock_tick","object":{"kind":"trusted_clock","handle":"pc:keep:clock"},"source":"os.clock.local","ttl_signal":30,"consumers":["intent"]}
{"type":"service_up","object":{"kind":"host_session","handle":"pc:keep:session"},"source":"os.host.session","ts":"","ttl_signal":10,"consumers":["intent"]}
{"type":"service_up","object":{"kind":"host_session","handle":"pc:drop:glass"},"source":"os.host.session","ts":"2026-10-12T00:00:00Z","ttl_signal":10,"consumers":["glass"]}
{"type":"file_changed","object":{"kind":"local_path"},"source":"os.fs.observe","ts":"2026-10-12T00:00:00Z","ttl_signal":10,"consumers":["intent"]}
not json
{"type":"file_changed","object":{"kind":"local_path","handle":"pc:keep:file","expected_version":null},"source":"os.fs.observe","ts":"2026-10-12T00:00:01Z","ttl_signal":10,"consumers":["glass","intent"],"scenario":"ciao"}
"#;
    let p = propose_from_context("ciao", jsonl, RecallMode::ObservedInRecall, &[]);
    assert_eq!(
        p.candidates
            .iter()
            .map(|c| c.handle.as_str())
            .collect::<Vec<_>>(),
        vec!["pc:keep:file"]
    );
    assert_eq!(
        p.bm25_lexical_hits, 0,
        "scenario tag is not a lexical document"
    );
}

#[test]
fn bm25_counts_a_real_token_without_replacing_the_observed_handle() {
    let jsonl = r#"{"type":"file_changed","object":{"kind":"local_path","handle":"pc:box:ciao","expected_version":null},"source":"os.fs.observe","ts":"2026-10-12T00:00:00Z","ttl_signal":5,"consumers":["intent"]}"#;
    let p = propose_from_context("ciao", jsonl, RecallMode::ObservedInRecall, &[]);
    assert_eq!(p.bm25_lexical_hits, 1);
    assert_eq!(p.candidates.len(), 1);
    assert_eq!(p.candidates[0].handle, "pc:box:ciao");
    assert!(!p.llm_invoked);
}

#[test]
fn proposal_json_is_explicitly_not_a_model_run() {
    let json = proposal_json(&ciao());
    assert!(json.contains("\"mode\": \"FALLBACK CONTEXT-SIGNALS\""));
    assert!(json.contains("\"llm_invoked\": false"));
    assert!(json.contains("\"auto_resolve\": false"));
    assert!(json.contains("\"authority_label\": \"no-authority\""));
    assert!(json.contains("«nessuna autorità»"));
    assert!(json.contains(HOST));
    assert!(json.contains(CARGO));
    assert!(json.contains(CLOCK));
    assert!(json.contains("\"source\": \"ContextSignal\""));
    assert!(!json.contains("\"llm_invoked\": true"));
}

#[test]
fn ciao_demo_writes_two_real_pngs() {
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ciao-demo");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/ciao-demo-signals.jsonl");
    let bin = demo_bin();
    let run = Command::new(&bin)
        .arg(&fixture)
        .arg(&out)
        .output()
        .unwrap_or_else(|err| panic!("demo bin {}: {err}", bin.display()));
    let stdout = String::from_utf8(run.stdout).unwrap();
    let stderr = String::from_utf8(run.stderr).unwrap();
    assert!(run.status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("command: cargo run --manifest-path poc/intent/Cargo.toml --bin ciao-fallback -- poc/intent/fixtures/ciao-demo-signals.jsonl"));
    assert!(stdout.contains("mode: FALLBACK CONTEXT-SIGNALS"));
    assert!(stdout.contains("llm_invoked: false"));
    assert!(stdout.contains("LLM not invoked"));
    assert!(stdout.contains("utterance: ciao"));
    assert!(stdout.contains("bm25_lexical_hits: 0"));
    assert!(stdout.contains("rows: 3"));
    assert!(stdout.contains("source=ContextSignal"));
    assert!(stdout.contains(HOST) && stdout.contains(CARGO) && stdout.contains(CLOCK));
    let list = decode_png(&fs::read(out.join("proposal-list.png")).unwrap());
    let glass = decode_png(&fs::read(out.join("glass-no-authority.png")).unwrap());
    assert!(list.0 >= 200 && list.1 >= 100, "list {}x{}", list.0, list.1);
    assert!(
        glass.0 >= 200 && glass.1 >= 100,
        "glass {}x{}",
        glass.0,
        glass.1
    );
    assert!(distinct_colors(&list.2) >= 3);
    assert!(distinct_colors(&glass.2) >= 3);
    assert!(
        has_pixel(&glass.2, 242, 196, 96),
        "amber no-authority banner"
    );
    assert_ne!(list.2, glass.2);
    let json = fs::read_to_string(out.join("proposal.json")).unwrap();
    assert!(json.contains("\"llm_invoked\": false"));
    assert!(json.contains("no-authority"));
}

fn demo_bin() -> std::path::PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_ciao_fallback") {
        return std::path::PathBuf::from(path);
    }
    let mut path = std::env::current_exe().expect("test executable");
    path.pop();
    path.pop();
    path.push("ciao-fallback");
    path
}

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut i = 8usize;
    let mut w = 0u32;
    let mut h = 0u32;
    let mut idat = Vec::new();
    while i + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        let kind = &bytes[i + 4..i + 8];
        let data = &bytes[i + 8..i + 8 + len];
        let crc = u32::from_be_bytes(bytes[i + 8 + len..i + 12 + len].try_into().unwrap());
        assert_eq!(
            crc,
            crc32_ieee(&[kind, data].concat()),
            "bad chunk crc {kind:?}"
        );
        if kind == b"IHDR" {
            w = u32::from_be_bytes(data[0..4].try_into().unwrap());
            h = u32::from_be_bytes(data[4..8].try_into().unwrap());
            assert_eq!(data[8], 8);
            assert_eq!(data[9], 2);
        } else if kind == b"IDAT" {
            idat.extend_from_slice(data);
        } else if kind == b"IEND" {
            break;
        }
        i += 12 + len;
    }
    let raw = inflate_stored(&idat);
    let stride = 1 + w as usize * 3;
    assert_eq!(raw.len(), stride * h as usize);
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    for row in 0..h as usize {
        let start = row * stride;
        assert_eq!(raw[start], 0);
        rgb.extend_from_slice(&raw[start + 1..start + stride]);
    }
    (w, h, rgb)
}

fn inflate_stored(z: &[u8]) -> Vec<u8> {
    assert!(z.len() >= 6);
    assert_eq!(z[0], 0x78);
    let mut i = 2usize;
    let mut out = Vec::new();
    loop {
        let header = z[i];
        i += 1;
        assert_eq!(header & 0b110, 0, "stored block");
        let len = u16::from_le_bytes([z[i], z[i + 1]]) as usize;
        let nlen = u16::from_le_bytes([z[i + 2], z[i + 3]]);
        assert_eq!(nlen, !(len as u16));
        i += 4;
        out.extend_from_slice(&z[i..i + len]);
        i += len;
        if header & 1 == 1 {
            break;
        }
    }
    out
}

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

fn distinct_colors(rgb: &[u8]) -> usize {
    let mut colors = std::collections::BTreeSet::new();
    for px in rgb.chunks_exact(3) {
        colors.insert([px[0], px[1], px[2]]);
        if colors.len() > 8 {
            break;
        }
    }
    colors.len()
}

fn has_pixel(rgb: &[u8], r: u8, g: u8, b: u8) -> bool {
    rgb.chunks_exact(3).any(|px| px == [r, g, b])
}
