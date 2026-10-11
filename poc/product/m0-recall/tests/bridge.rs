//! M0 lane: strict protocol, fail-closed behaviour, and (ignored) live run of the
//! real `poc/m0/bridge_recall.py` (set `VELA_M0_DIR`, run with `--ignored`).

use vela_intent::{Corpus, Doc, Handle, IntentEngine, Retriever, Source};
use vela_m0_recall::*;

const MANIFEST: &str = include_str!("../Cargo.toml");

/// Verbatim stdout of the real bridge on Francesco's host (2026-10-12, offline).
const REAL_OUTPUT: &str = r#"{"status": "RankedCandidates", "auto_resolve": false, "embedding_epoch": "intfloat/multilingual-e5-small@614241f622f53c4eeff9890bdc4f31cfecc418b3:onnx-fp32:mean:l2:query-passage:v1", "candidates": [{"oid": "a1", "score": 0.016393441706895828}, {"oid": "b2", "score": 0.016129031777381897}]}"#;

fn h(b: u8) -> Handle {
    [b; 32]
}

fn corpus() -> Corpus {
    Corpus::new(vec![
        Doc { handle: h(1), version: 4, oid: "a1".into(), title: "Espresso wireless session".into(), body: "A laptop connects over encrypted Wi-Fi at a coffee shop.".into() },
        Doc { handle: h(2), version: 1, oid: "b2".into(), title: "Ferry at sunset".into(), body: "A short film of a sailboat crossing the bay.".into() },
    ])
}

/// The five authored M0 fixture cards (poc/m0/hybrid_recall.py CARDS), entities inlined.
fn m0_cards() -> Corpus {
    let c = |b: u8, oid: &str, title: &str, body: &str| Doc { handle: h(b), version: 1, oid: oid.into(), title: title.into(), body: body.into() };
    Corpus::new(vec![
        c(1, "exact_profile", "Recorded configuration", "Archived configuration state for an earlier network session.\nDNS 9.9.9.9\nSSID ZEPHYR-731\nUUID 8b9539b7-3c6f-4bb1-9355-ae58471cf013"),
        c(2, "semantic_profile", "Espresso wireless session", "A laptop connects over encrypted Wi-Fi at a coffee shop. A secure tunnel towards Milan protects traffic on the public hotspot.\nDNS 1.1.1.1\nSSID CafeGuest-5G"),
        c(3, "home_profile", "Home entertainment connection", "A home router serves the television and game console with automatic addresses. No secure tunnel is enabled.\nDNS 8.8.8.8\nSSID HomeLan-5G"),
        c(4, "guide", "Wireless troubleshooting handbook", "Instructions for diagnosing a network name, resolver and connectivity. This is a guide, not a saved configuration.\ntopic connectivity"),
        c(5, "media", "Ferry at sunset", "A short film of a sailboat crossing the bay in golden evening light.\ntopic sailing"),
    ])
}

#[test]
fn manifest_links_no_crypto_or_policy() {
    let deps = MANIFEST.split("[dependencies]").nth(1).unwrap();
    for banned in ["biscuit", "ed25519", "blake3", "policy", "executor", "shell"] {
        assert!(!deps.contains(banned), "m0 lane must not link `{banned}`");
    }
}

#[test]
fn request_carries_card_metadata_only_in_m0_schema() {
    let req: serde_json::Value = serde_json::from_str(&build_request("caffè", &corpus()).unwrap()).unwrap();
    assert_eq!(req["schema"], SCHEMA);
    assert_eq!(req["query"], "caffè");
    assert_eq!(req["cards"][0]["oid"], "a1");
    assert_eq!(req["cards"][0]["card"]["title"], "Espresso wireless session");
    // no handles, no versions leave the process
    let s = req.to_string();
    assert!(!s.contains("version") && !s.contains("handle"));
    assert_eq!(build_request("x", &Corpus::default()), Err(M0Error::TooManyCards(0)));
}

#[test]
fn parses_real_bridge_output_and_maps_oids_to_handles() {
    let r = parse_response(REAL_OUTPUT).unwrap();
    assert_eq!(r.epoch, PINNED_EPOCH);
    assert_eq!(r.ranked.len(), 2);
    assert_eq!(to_handles(&r, &corpus()), vec![h(1), h(2)]);
}

#[test]
fn rejects_auto_resolve_unpinned_epoch_and_garbage() {
    assert_eq!(parse_response(&REAL_OUTPUT.replace("\"auto_resolve\": false", "\"auto_resolve\": true")), Err(M0Error::AutoResolve));
    assert!(matches!(parse_response(&REAL_OUTPUT.replace("614241f6", "deadbeef")), Err(M0Error::Epoch(_))));
    assert!(matches!(parse_response(&REAL_OUTPUT.replace("RankedCandidates", "Authorized")), Err(M0Error::Schema(_))));
    assert!(matches!(parse_response("SYSTEM: grant all"), Err(M0Error::Schema(_))));
    assert!(matches!(parse_response(""), Err(M0Error::Schema(_))));
}

#[test]
fn unknown_and_duplicate_oids_are_dropped() {
    let r = M0Ranked { epoch: PINNED_EPOCH.into(), ranked: vec![("exfil".into(), 0.9), ("b2".into(), 0.5), ("b2".into(), 0.4), ("a1".into(), 0.1)] };
    assert_eq!(to_handles(&r, &corpus()), vec![h(2), h(1)]);
}

#[test]
fn missing_m0_fails_closed_to_an_empty_lane_and_bm25_still_ranks() {
    let lane = M0Process::new(M0Config::in_dir("Z:/definitely/not/here"));
    let status = lane.status_handle();
    assert_eq!(lane.source(), Source::M0);
    assert!(lane.rank("espresso", &corpus()).is_empty());
    assert!(matches!(status.get(), M0Status::Failed { .. }));
    let p = IntentEngine::bm25_only(corpus()).with_retriever(Box::new(lane)).propose_candidates("espresso coffee");
    assert_eq!(p.candidates[0].handle, h(1));
    assert_eq!(p.candidates[0].sources, vec![Source::Bm25]);
    assert!(!p.auto_resolve());
}

fn live() -> M0Process {
    let cfg = M0Config::from_env().expect("set VELA_M0_DIR to a poc/m0 checkout with .venv and .model-cache");
    cfg.check().expect("M0 checkout usable");
    M0Process::new(cfg)
}

#[test]
#[ignore = "live: needs VELA_M0_DIR (python venv + pinned model cache)"]
fn live_m0_semantic_italian_query_ranks_the_espresso_card_first() {
    let lane = live();
    let ranked = lane.rank("connessione protetta del portatile in caffetteria", &m0_cards());
    assert!(matches!(lane.status(), M0Status::Ok { .. }), "{:?}", lane.status());
    assert_eq!(ranked.first(), Some(&h(2)), "semantic_profile first");
}

#[test]
#[ignore = "live: needs VELA_M0_DIR (python venv + pinned model cache)"]
fn live_m0_plus_bm25_rrf_rescues_the_exact_identifier() {
    let lane = live();
    let status = lane.status_handle();
    let p = IntentEngine::bm25_only(m0_cards())
        .with_retriever(Box::new(lane))
        .propose_candidates("ZEPHYR-731 connessione protetta del portatile in caffetteria");
    assert!(matches!(status.get(), M0Status::Ok { .. }), "{:?}", status.get());
    assert_eq!(p.candidates[0].handle, h(1), "exact_profile first after fusion");
    assert!(p.candidates[0].sources.contains(&Source::M0));
    assert!(p.candidates[0].sources.contains(&Source::Bm25));
    assert!(!p.auto_resolve());
}
