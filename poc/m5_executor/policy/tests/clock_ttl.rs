//! M-CLK-1: trusted clock for grant TTL. Deterministic fake clock; monotonic
//! deadline enforced before execution and before the LFS commit; wall-clock
//! skew cannot extend a grant; missing/regressing time is fail-closed.

mod common;

use common::*;
use ed25519_compact::PublicKey;
use std::sync::Arc;
use std::time::Duration;
use vela_executor_policy::DenyCode;
use vela_executor_policy::clock::{Clock, ClockError, FakeClock, GrantDeadline, TrustedTime};
use vela_executor_policy::ledger::{E2eError, E2eHost, ExecReply, SimulatedLfs};

const OK: ExecReply = ExecReply { status: 0, value: 21, request_id: 1, attempts: 1 };

fn host_with(f: &Fixture, clock: &Arc<FakeClock>) -> E2eHost {
    let lfs = SimulatedLfs::new(lfs_keys().sk, PublicKey::new(*pcb().pk));
    E2eHost::with_clock(gated(&f.root), lfs, 1, Box::new(clock.clone()))
}

fn selected<'a>(f: &'a mut Fixture, ttl: u64) -> vela_executor_policy::pipeline::TaskRequest<'a> {
    let sel = f.channel.select(NOTES, 3);
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(sel);
    t.ttl = Duration::from_secs(ttl);
    t
}

#[test]
fn clock_valid_grant_commits_within_ttl() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let c2 = clock.clone();
    let t = selected(&mut f, 120);
    let (_, n) = h
        .run_task(&t, |_| {
            c2.advance(Duration::from_secs(120)); // exactly at not_after: still alive
            OK
        })
        .unwrap();
    assert_eq!(n.receipt.body.value, 21);
    assert_eq!(h.lfs.persisted(), 1);
}

#[test]
fn clock_expired_between_execution_and_commit_persists_nothing() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let c2 = clock.clone();
    let t = selected(&mut f, 120);
    let r = h.run_task(&t, |_| {
        c2.advance(Duration::from_secs(121)); // not_after + 1 s
        OK
    });
    assert_eq!(r.unwrap_err(), E2eError::GrantExpired(DenyCode::Expired));
    assert_eq!(DenyCode::Expired.contract_code(), 7);
    assert_eq!((h.lfs.persisted(), h.notary.len(), h.executions), (0, 0, 1));
}

#[test]
fn clock_wall_jump_back_cannot_extend_grant() {
    // Attacker/NTP sets the wall clock back during execution: the monotonic
    // deadline still expires the grant.
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let (c2, start) = (clock.clone(), f.now);
    let t = selected(&mut f, 30);
    let r = h.run_task(&t, |_| {
        c2.advance(Duration::from_secs(31));
        c2.set_wall(start); // wall says "no time passed"
        OK
    });
    assert_eq!(r.unwrap_err(), E2eError::GrantExpired(DenyCode::Expired));
    assert_eq!(h.lfs.persisted(), 0);
}

#[test]
fn clock_wall_jump_forward_does_not_matter_for_monotonic_deadline() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let (c2, start) = (clock.clone(), f.now);
    let t = selected(&mut f, 60);
    let r = h.run_task(&t, |_| {
        c2.set_wall(start + Duration::from_secs(3600)); // wall-only step
        OK
    });
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn clock_regression_before_admission_is_fail_closed() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    // First call establishes the high-water mark.
    let t = selected(&mut f, 60);
    h.run_task(&t, |_| OK).unwrap();
    clock.set_wall(f.now - Duration::from_secs(600));
    let sel = f.channel.select(NOTES, 3);
    let mut t2 = task(&f.candidates, NOTES, 2);
    t2.selection = Some(sel);
    let mut called = false;
    let r = h.run_task(&t2, |_| {
        called = true;
        ExecReply { request_id: 2, ..OK }
    });
    assert_eq!(r.unwrap_err(), E2eError::Denied(DenyCode::Expired));
    assert!(!called);
    assert_eq!(h.pdp.frames_signed(), 1, "no frame for regressed clock");
}

#[test]
fn clock_without_monotonic_source_refuses_before_execution() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    clock.set_monotonic_available(false);
    let mut h = host_with(&f, &clock);
    let t = selected(&mut f, 60);
    let mut called = false;
    let r = h.run_task(&t, |_| {
        called = true;
        OK
    });
    assert_eq!(r.unwrap_err(), E2eError::Denied(DenyCode::Expired));
    assert!(!called, "no Wasm without a trusted monotonic source");
    assert_eq!(h.lfs.persisted(), 0);
}

#[test]
fn clock_monotonic_lost_after_execution_persists_nothing() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let c2 = clock.clone();
    let t = selected(&mut f, 60);
    let r = h.run_task(&t, |_| {
        c2.set_monotonic_available(false);
        OK
    });
    assert_eq!(r.unwrap_err(), E2eError::GrantExpired(DenyCode::Expired));
    assert_eq!(h.lfs.persisted(), 0);
}

#[test]
fn clock_ai_only_is_still_policy_denied_on_trusted_clock_path() {
    let f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let t = task(&f.candidates, NOTES, 1); // no UserSelection
    let r = h.run_task(&t, |_| OK);
    assert_eq!(r.unwrap_err(), E2eError::Denied(DenyCode::PolicyDenied));
    assert_eq!((h.executions, h.lfs.persisted(), h.pdp.frames_signed()), (0, 0, 0));
}

#[test]
fn clock_ttl_over_120s_is_expired_on_trusted_clock_path() {
    let mut f = fixture();
    let clock = Arc::new(FakeClock::at(f.now));
    let mut h = host_with(&f, &clock);
    let t = selected(&mut f, 121);
    assert_eq!(h.run_task(&t, |_| OK).unwrap_err(), E2eError::Denied(DenyCode::Expired));
}

#[test]
fn clock_deadline_unit_semantics() {
    let f = fixture();
    let fake = FakeClock::at(f.now);
    let t = TrustedTime::new(fake);
    let wall = t.wall().unwrap();
    let d = GrantDeadline::start(&t, wall, wall + Duration::from_secs(10)).unwrap();
    assert_eq!(d.check(&t), Ok(Duration::from_secs(10)));
    t.clock().advance(Duration::from_secs(10));
    assert_eq!(d.check(&t), Ok(Duration::ZERO));
    t.clock().advance(Duration::from_nanos(1));
    assert_eq!(d.check(&t), Err(DenyCode::Expired));
    // not_after before mint time is nonsense -> fail-closed.
    assert_eq!(
        GrantDeadline::start(&t, wall, wall - Duration::from_secs(1)),
        Err(ClockError::Regressed)
    );
    assert!(t.clock().monotonic().is_some());
}

#[test]
fn clock_system_time_now_only_inside_clock_module() {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    for entry in std::fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        if name == "clock.rs" {
            continue;
        }
        for needle in ["SystemTime::now", "Instant::now"] {
            assert!(!text.contains(needle), "{needle} outside clock.rs in {name}");
        }
    }
}
