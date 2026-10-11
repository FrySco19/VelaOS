//! ShellRuntime state machine: idle -> interpreting -> glass -> authorizing -> result.

mod common;

use common::*;
use vela_executor_policy::DenyCode;
use vela_intent::{IntentEngine, UserSelection};
use vela_shell::*;

#[test]
fn avvia_walks_every_phase_and_runs_the_task() {
    let (mut s, _root) = shell();
    assert_eq!(s.state().phase(), Phase::Idle);
    s.submit(UTTERANCE).unwrap();
    let ShellState::Glass(p) = s.state() else { panic!("glass expected") };
    assert_eq!(p.candidates[0].handle, NOTES);
    assert!(s.render().contains("[Avvia] [Rivedi piano]"));
    assert_eq!((s.grants_issued(), s.frames_signed()), (0, 0), "glass authorizes nothing");
    s.click(Click::Avvia { candidate: 0 }).unwrap();
    assert_eq!(
        s.trace(),
        &[Phase::Idle, Phase::Interpreting, Phase::Glass, Phase::Authorizing, Phase::Result]
    );
    match s.state() {
        ShellState::Result { selection, action: Action::Avvia, outcome: Outcome::Executed { value, request_id, .. } } => {
            assert_eq!(*selection, UserSelection { handle: NOTES, expected_version: 3 });
            assert_eq!((*value, *request_id), (21, 1));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(s.emitted_selections(), &[UserSelection { handle: NOTES, expected_version: 3 }]);
    assert_eq!((s.grants_issued(), s.frames_signed()), (1, 1));
    s.click(Click::Chiudi).unwrap();
    assert_eq!(s.state(), &ShellState::Idle);
}

#[test]
fn rivedi_piano_authorizes_without_dispatching() {
    let (mut s, _root) = shell();
    s.submit(UTTERANCE).unwrap();
    s.click(Click::RivediPiano { candidate: 0 }).unwrap();
    assert!(matches!(
        s.state(),
        ShellState::Result { action: Action::RivediPiano, outcome: Outcome::Reviewed { request_id: 1, .. }, .. }
    ));
    assert_eq!(s.emitted_selections().len(), 1);
}

#[test]
fn annulla_is_undo_and_nothing_is_authorized() {
    let (mut s, _root) = shell();
    s.submit(UTTERANCE).unwrap();
    s.click(Click::Annulla).unwrap();
    assert_eq!(s.state(), &ShellState::Idle);
    assert!(s.emitted_selections().is_empty());
    assert_eq!((s.grants_issued(), s.frames_signed()), (0, 0));
}

#[test]
fn gestures_outside_the_glass_are_rejected_without_side_effects() {
    let (mut s, _root) = shell();
    assert_eq!(s.click(Click::Avvia { candidate: 0 }).unwrap_err(), UiReject::WrongState(Phase::Idle));
    s.submit(UTTERANCE).unwrap();
    assert_eq!(s.submit("altro").unwrap_err(), UiReject::WrongState(Phase::Glass));
    assert_eq!(s.click(Click::Avvia { candidate: 99 }).unwrap_err(), UiReject::NotOffered(99));
    assert_eq!(s.click(Click::Chiudi).unwrap_err(), UiReject::WrongState(Phase::Glass));
    assert_eq!(s.state().phase(), Phase::Glass);
    s.click(Click::Avvia { candidate: 0 }).unwrap();
    // A second Avvia on the same plan (double click / replay) is not possible.
    assert_eq!(s.click(Click::Avvia { candidate: 0 }).unwrap_err(), UiReject::WrongState(Phase::Result));
    assert_eq!(s.frames_signed(), 1);
}

#[test]
fn nothing_found_is_an_error_state() {
    let (mut s, _root) = shell();
    s.submit("zzzz qqqq").unwrap();
    assert!(matches!(s.state(), ShellState::Error { selection: None, error: ShellError::NothingFound }));
    s.click(Click::Chiudi).unwrap();
    assert_eq!(s.state().phase(), Phase::Idle);
}

#[test]
fn hostile_llm_text_never_moves_past_the_glass() {
    let (mut s, _root) = shell_with(IntentEngine::new(corpus()).with_llm(Box::new(HostileLlm)));
    s.submit(UTTERANCE).unwrap();
    let ShellState::Glass(p) = s.state() else { panic!() };
    assert!(p.rationale.contains("already approved"));
    assert!(p.candidates.iter().all(|c| c.handle != [0xEE; 32]), "unknown handle dropped");
    assert!(s.render().contains("nota IA (non verificata)"));
    assert_eq!((s.grants_issued(), s.frames_signed()), (0, 0));
    assert!(s.emitted_selections().is_empty());
}

#[test]
fn ttl_is_clamped_and_successive_tasks_get_fresh_request_ids() {
    let (s, _root) = shell();
    let mut s = s.with_ttl(std::time::Duration::from_secs(10_000));
    for expected in 1..=2u32 {
        s.submit(UTTERANCE).unwrap();
        s.click(Click::Avvia { candidate: 0 }).unwrap();
        match s.state() {
            ShellState::Result { outcome: Outcome::Executed { request_id, .. }, .. } => assert_eq!(*request_id, expected),
            other => panic!("{other:?}"),
        }
        s.click(Click::Chiudi).unwrap();
    }
    let _ = DenyCode::Expired;
}
