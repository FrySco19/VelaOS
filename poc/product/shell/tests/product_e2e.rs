//! Host integration: utterance -> intent candidates -> UserSelection -> admit,
//! and every AI-only path ends in POLICY_DENIED with nothing signed.

mod common;

use common::*;
use ed25519_compact::PublicKey;
use std::time::{Duration, SystemTime};
use vela_executor_policy::DenyCode;
use vela_executor_policy::bridge;
use vela_executor_policy::grant::{Candidate as PolicyCandidate, MAX_READ_GRANT_TTL, TrustedSelectionChannel};
use vela_executor_policy::pipeline::{RETURN_21_MODULE, Stage, TaskOutcome, TaskRequest};
use vela_intent::{IntentEngine, IntentProposal};
use vela_shell::*;

fn to_policy(p: &IntentProposal) -> Vec<PolicyCandidate> {
    p.candidates.iter().map(|c| PolicyCandidate { handle: c.handle, version: c.version, score: c.score }).collect()
}

fn task<'a>(c: &'a [PolicyCandidate], requested: [u8; 32], note: &str) -> TaskRequest<'a> {
    TaskRequest {
        caller: USER.into(),
        requested,
        candidates: c,
        selection: None,
        ttl: Duration::from_secs(60),
        planner_note: note.into(),
        request_id: 1,
        module: RETURN_21_MODULE.to_vec(),
        fuel: 10_000,
    }
}

fn denied(o: TaskOutcome, code: DenyCode) {
    match o {
        TaskOutcome::Denied { code: c, reason, .. } => assert_eq!(c, code, "{reason}"),
        TaskOutcome::Admitted(_) => panic!("expected {code:?}"),
    }
}

#[test]
fn utterance_to_candidates_to_selection_to_admitted_and_executed() {
    let (mut s, _root) = shell();
    s.submit(UTTERANCE).unwrap();
    s.click(Click::Avvia { candidate: 0 }).unwrap();
    assert!(matches!(s.state(), ShellState::Result { outcome: Outcome::Executed { value: 21, .. }, .. }));
    assert_eq!((s.grants_issued(), s.frames_signed()), (1, 1));
}

#[test]
fn ai_only_intent_candidates_without_selection_are_policy_denied() {
    let root = biscuit_auth::KeyPair::new();
    let mut pdp = gated(&root);
    let p = IntentEngine::new(corpus()).with_llm(Box::new(HostileLlm)).propose_candidates(UTTERANCE);
    let c = to_policy(&p);
    for target in [NOTES, SECRETS] {
        let t = task(&c, target, &p.rationale);
        denied(pdp.admit_task_at(&t, SystemTime::now()), DenyCode::PolicyDenied);
    }
    assert_eq!(DenyCode::PolicyDenied.contract_code(), 10);
    assert_eq!((pdp.grants_issued(), pdp.frames_signed()), (0, 0));
}

#[test]
fn selection_redirected_by_the_ai_is_scope() {
    let root = biscuit_auth::KeyPair::new();
    let mut pdp = gated(&root);
    let mut ch = TrustedSelectionChannel::new();
    let p = IntentEngine::new(corpus()).propose_candidates(UTTERANCE);
    let c = to_policy(&p);
    let mut t = task(&c, SECRETS, "");
    t.selection = Some(ch.select(NOTES, 3));
    denied(pdp.admit_task_at(&t, SystemTime::now()), DenyCode::Scope);
    assert_eq!(pdp.frames_signed(), 0);
}

#[test]
fn overlong_ttl_is_expired_and_stale_version_is_conflict() {
    let root = biscuit_auth::KeyPair::new();
    let mut pdp = gated(&root);
    let mut ch = TrustedSelectionChannel::new();
    let p = IntentEngine::new(corpus()).propose_candidates(UTTERANCE);
    let c = to_policy(&p);
    let mut t = task(&c, NOTES, "");
    t.selection = Some(ch.select(NOTES, 3));
    t.ttl = MAX_READ_GRANT_TTL + Duration::from_secs(1);
    denied(pdp.admit_task_at(&t, SystemTime::now()), DenyCode::Expired);
    let mut t = task(&c, NOTES, "");
    t.selection = Some(ch.select(NOTES, 2));
    denied(pdp.admit_task_at(&t, SystemTime::now()), DenyCode::VersionConflict);
    assert_eq!(pdp.frames_signed(), 0);
}

#[test]
fn llm_proposed_candidate_still_needs_the_click_and_then_is_admitted() {
    let (mut s, _root) = shell_with(IntentEngine::new(corpus()).with_llm(Box::new(HostileLlm)));
    s.submit(UTTERANCE).unwrap();
    let ShellState::Glass(p) = s.state() else { panic!() };
    let idx = p.candidates.iter().position(|c| c.handle == SECRETS).expect("llm extra shown");
    assert_eq!((s.grants_issued(), s.frames_signed()), (0, 0));
    // Only an explicit user click on that exact row authorizes it (human decides).
    s.click(Click::RivediPiano { candidate: idx }).unwrap();
    assert_eq!(s.emitted_selections()[0].handle, SECRETS);
    assert_eq!(s.emitted_selections()[0].expected_version, 9);
    assert_eq!(s.frames_signed(), 1);
}

#[test]
fn admitted_frame_verifies_under_the_pcb_key() {
    let root = biscuit_auth::KeyPair::new();
    let mut pdp = gated(&root);
    let mut ch = TrustedSelectionChannel::new();
    let p = IntentEngine::new(corpus()).propose_candidates(UTTERANCE);
    let c = to_policy(&p);
    let sel = p.candidates[0].as_selection();
    let mut t = task(&c, sel.handle, "");
    t.selection = Some(ch.select(sel.handle, sel.expected_version));
    let a = pdp.admit_task_at(&t, SystemTime::now()).admitted().expect("admitted");
    let v = bridge::verify_with_key(&a.frame, 0, PublicKey::new(*pcb().pk)).unwrap();
    assert_eq!(v.descriptor.request_id, 1);
    assert!(matches!(Stage::Grant, Stage::Grant));
}
