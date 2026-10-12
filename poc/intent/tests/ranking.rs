//! BM25 + RRF ranking and the M0 connector.

use vela_intent::*;

fn h(b: u8) -> Handle {
    [b; 32]
}

fn corpus() -> Corpus {
    Corpus::new(vec![
        Doc { handle: h(1), version: 3, oid: "oid-notes".into(), title: "Note progetto VelaOS".into(), body: "bozza architettura microkernel note riunione".into() },
        Doc { handle: h(2), version: 1, oid: "oid-photos".into(), title: "Foto vacanze".into(), body: "mare spiaggia note".into() },
        Doc { handle: h(3), version: 9, oid: "oid-keys".into(), title: "Chiavi private".into(), body: "ed25519 seed".into() },
        Doc { handle: h(4), version: 2, oid: "oid-net".into(), title: "Configurazione rete".into(), body: "wifi dns proxy".into() },
    ])
}

#[test]
fn tokenizer_lowercases_and_drops_short_words() {
    assert_eq!(tokenize("Riassumi le NOTE, di VelaOS!"), vec!["riassumi", "note", "velaos"]);
}

#[test]
fn bm25_prefers_the_document_matching_more_and_rarer_terms() {
    let s = Bm25::default().scores("note progetto velaos", &corpus());
    assert_eq!(s[0].0, h(1));
    assert!(s.iter().all(|(hd, _)| *hd != h(3) && *hd != h(4)), "no lexical hit, no candidate");
    assert!(s[0].1 > s[1].1);
}

#[test]
fn rrf_matches_the_formula() {
    let lanes = vec![(Source::Bm25, vec![h(1), h(2)]), (Source::Title, vec![h(2)])];
    let f = rrf(&lanes, 60.0);
    assert_eq!(f[0].0, h(2));
    assert!((f[0].1 - (1.0 / 62.0 + 1.0 / 61.0)).abs() < 1e-6);
    assert!((f[1].1 - 1.0 / 61.0).abs() < 1e-6);
    assert_eq!(f[0].2, vec![Source::Bm25, Source::Title]);
}

#[test]
fn engine_is_deterministic_and_carries_corpus_versions() {
    let e = IntentEngine::new(corpus());
    let a = e.propose_candidates("riassumi le note progetto VelaOS");
    assert_eq!(a, e.propose_candidates("riassumi le note progetto VelaOS"));
    assert_eq!(a.candidates[0].handle, h(1));
    assert_eq!(a.candidates[0].version, 3);
    assert!(a.candidates.len() <= MAX_CANDIDATES);
    assert!(e.propose_candidates("   ").is_empty());
}

#[test]
fn m0_ranking_connector_maps_oids_and_ignores_unknown_or_other_queries() {
    let m0 = M0Ranking {
        query: "configura la rete".into(),
        ranked: vec![("oid-net".into(), 0.03), ("oid-ghost".into(), 0.02), ("oid-net".into(), 0.01)],
    };
    assert_eq!(m0.rank("configura la rete", &corpus()), vec![h(4)]);
    assert!(m0.rank("altra query", &corpus()).is_empty());
    let p = IntentEngine::new(corpus()).with_retriever(Box::new(m0)).propose_candidates("configura la rete");
    assert_eq!(p.candidates[0].handle, h(4));
    assert!(p.candidates[0].sources.contains(&Source::M0));
}

#[test]
fn version_bump_is_reflected_in_new_proposals_only() {
    let mut e = IntentEngine::new(corpus());
    let before = e.propose_candidates("note progetto");
    assert!(e.corpus_mut().bump_version(&h(1), 4));
    let after = e.propose_candidates("note progetto");
    assert_eq!((before.candidates[0].version, after.candidates[0].version), (3, 4));
}
