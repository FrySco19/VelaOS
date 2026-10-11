//! Table-driven contract tests. Each test names the EXECUTOR_CONTRACT.md
//! section (E1..E9) it asserts. Host-only: no Docker/QEMU.

use ed25519_compact::{KeyPair, Seed};
use vela_executor_host::{HostExecutor, MAX_PAYLOAD_BYTES, bridge, bridge::Status};

fn keys() -> KeyPair {
    KeyPair::from_seed(Seed::new([41; 32]))
}
fn frame(id: u32, payload: &[u8], fuel: u64) -> [u8; bridge::PAGE_SIZE] {
    bridge::signed_frame(&keys().sk, id, payload, fuel)
}

/// E1: verify and execute an owned snapshot; later writes to shared storage
/// cannot change what runs.
#[test]
fn e1_owned_snapshot_is_what_executes() {
    let mut exec = HostExecutor::new(keys().pk);
    let mut shared = frame(1, bridge::RETURN_7_MODULE, 1000);
    let captured = unsafe { bridge::snapshot(shared.as_ptr()) };
    // Producer flips the returned constant 7 -> 8 after capture.
    shared[bridge::PAYLOAD_OFFSET + bridge::RETURN_7_MODULE.len() - 2] = 8;
    let reply = exec.call(&captured);
    assert_eq!((reply.status, reply.value), (Status::Success, 7));
    // The mutated shared bytes no longer authenticate.
    let mut fresh = HostExecutor::new(keys().pk);
    assert_eq!(fresh.call(&shared).status, Status::Authentication);
    assert_eq!(fresh.attempts(), 0);
}

/// E2: admission = Ed25519 over domain || header || claims || exact payload,
/// against the pinned PCB key. No Biscuit in the guest.
#[test]
fn e2_admission_rejections_happen_before_instantiation() {
    let valid = frame(5, bridge::RETURN_7_MODULE, 1000);
    let wrong_key = KeyPair::from_seed(Seed::new([42; 32]));
    let t = bridge::TOKEN_OFFSET;
    let cases: Vec<(&str, [u8; bridge::PAGE_SIZE], KeyPair)> = vec![
        ("wrong pinned key", valid, wrong_key),
        ("payload byte", { let mut f = valid; f[bridge::PAYLOAD_OFFSET + 3] ^= 1; f }, keys()),
        ("header request id", { let mut f = valid; f[8] ^= 1; f }, keys()),
        ("claims fuel", { let mut f = valid; f[t + 16] ^= 1; f }, keys()),
        ("claims memory", { let mut f = valid; f[t + 24] ^= 1; f }, keys()),
        ("claims audience", { let mut f = valid; f[t + 4] ^= 1; f }, keys()),
        ("signature", { let mut f = valid; f[t + bridge::CLAIMS_SIZE] ^= 1; f }, keys()),
    ];
    for (name, page, root) in cases {
        let mut exec = HostExecutor::new(root.pk);
        let reply = exec.call(&page);
        assert_eq!(reply.status, Status::Authentication, "{name}");
        assert_eq!(exec.attempts(), 0, "{name}: unauthenticated bytes reached Wasmi");
        assert_eq!(exec.last_consumed(), 0, "{name}: auth failure must not consume the ID");
    }
}

/// E3: signed budget ceilings (fuel 1..=MAX_FUEL) are enforced by the Executor itself.
#[test]
fn e3_fuel_quota_bounds_are_scope_errors() {
    for (fuel, expected) in [
        (0, Status::Scope),
        (bridge::MAX_FUEL + 1, Status::Scope),
        (u64::MAX, Status::Scope),
        (1000, Status::Success),
        (bridge::MAX_FUEL, Status::Success),
    ] {
        let mut exec = HostExecutor::new(keys().pk);
        let reply = exec.call(&frame(1, bridge::RETURN_7_MODULE, fuel));
        assert_eq!(reply.status, expected, "fuel={fuel}");
        if expected == Status::Scope {
            assert_eq!(exec.attempts(), 0);
            assert_eq!(exec.last_consumed(), 0);
        }
    }
}

/// E3/E8: infinite loop is stopped by fuel (status 6); the ID is consumed.
#[test]
fn e3_loop_exhausts_fuel_and_consumes_id() {
    let mut exec = HostExecutor::new(keys().pk);
    let reply = exec.call(&frame(3, bridge::LOOP_MODULE, 64));
    assert_eq!(reply.status, Status::Fuel);
    assert_eq!(reply.value, 0);
    assert_eq!(exec.attempts(), 1);
    assert_eq!(exec.last_consumed(), 3);
    assert_eq!(exec.call(&frame(3, bridge::LOOP_MODULE, 64)).status, Status::Replay);
}

/// E4: the WASI probe is rejected by the import check before instantiation.
#[test]
fn e4_wasi_import_rejected_without_instantiation() {
    let mut exec = HostExecutor::new(keys().pk);
    let reply = exec.call(&frame(2, bridge::WASI_IMPORT_MODULE, 1000));
    assert_eq!(reply.status, Status::Wasm);
    assert_eq!(exec.attempts(), 0, "import check must precede instantiation");
    assert_eq!(exec.last_consumed(), 2, "authenticated ID is consumed even on rejection");
}

/// E5: the only result channel is one i32, reported as u32 bits in regs[1].
#[test]
fn e5_result_is_single_u32_register() {
    let mut exec = HostExecutor::new(keys().pk);
    let reply = exec.call(&frame(1, bridge::RETURN_7_MODULE, 1000));
    assert_eq!(reply, vela_executor_host::Reply {
        status: Status::Success,
        value: 7,
        request_id: 1,
        attempts: 1,
    });
}

/// E6: Executor status codes are exactly C5 codes 0..=6. Codes >= 7
/// (EXPIRED .. UNAVAILABLE, incl. POLICY_DENIED = 10 from the host PDP) are
/// never produced by the Executor today.
#[test]
fn e6_status_codes_match_c5_and_exclude_pdp_codes() {
    let table = [
        (Status::Success, 0u64, "OK"),
        (Status::Malformed, 1, "MALFORMED"),
        (Status::Authentication, 2, "AUTH"),
        (Status::Scope, 3, "SCOPE"),
        (Status::Replay, 4, "REPLAY"),
        (Status::Wasm, 5, "WASM_REJECT_OR_TRAP"),
        (Status::Fuel, 6, "FUEL_EXHAUSTED"),
    ];
    for (status, code, name) in table {
        assert_eq!(status as u64, code, "{name}");
        assert!(code < 7, "{name} collides with a C5 code reserved for PCB/LFS/supervisor");
    }
    const POLICY_DENIED: u64 = 10;
    assert!(table.iter().all(|(s, _, _)| *s as u64 != POLICY_DENIED));
}

/// E6: wrong channel/label/count is Malformed with request_id 0 (untrusted).
#[test]
fn e6_bad_call_shape_is_malformed() {
    let page = frame(1, bridge::RETURN_7_MODULE, 1000);
    for (channel, label, count) in [(1, bridge::EXECUTE_LABEL, 0), (0, 0x700, 0), (0, bridge::EXECUTE_LABEL, 1)] {
        let mut exec = HostExecutor::new(keys().pk);
        let reply = exec.protected(channel, label, count, &page);
        assert_eq!((reply.status, reply.request_id, reply.attempts), (Status::Malformed, 0, 0));
        assert_eq!(exec.last_consumed(), 0);
    }
}

/// E6 caveat: on AUTH/SCOPE errors regs[2] echoes the *unauthenticated*
/// request-id bytes of the snapshot. PCB must correlate via single-flight,
/// not trust this value.
#[test]
fn e6_error_reply_request_id_is_unauthenticated() {
    let mut forged = frame(1, bridge::RETURN_7_MODULE, 1000);
    forged[8..12].copy_from_slice(&999u32.to_le_bytes());
    let mut exec = HostExecutor::new(keys().pk);
    let reply = exec.call(&forged);
    assert_eq!(reply.status, Status::Authentication);
    assert_eq!(reply.request_id, 999, "echoed, not authenticated");
}

/// E7: replay is monotonic per session; IDs are consumed before execution.
#[test]
fn e7_replay_rules_within_session() {
    let mut exec = HostExecutor::new(keys().pk);
    assert_eq!(exec.call(&frame(10, bridge::RETURN_7_MODULE, 1000)).status, Status::Success);
    for id in [10, 9, 1] {
        let reply = exec.call(&frame(id, bridge::RETURN_7_MODULE, 1000));
        assert_eq!(reply.status, Status::Replay, "id={id}");
    }
    assert_eq!(exec.attempts(), 1, "replayed frames never reach Wasmi");
    // Request ID 0 is never valid (scope), and a gap is accepted (monotonic, not contiguous).
    assert_eq!(exec.call(&frame(0, bridge::RETURN_7_MODULE, 1000)).status, Status::Scope);
    assert_eq!(exec.call(&frame(50, bridge::RETURN_7_MODULE, 1000)).status, Status::Success);
}

/// E7 KNOWN GAP (Gate 2): replay state is boot-local. A fresh Executor session
/// (reboot) accepts a previously executed frame. This test pins the gap so a
/// future durable-epoch implementation must consciously flip it.
#[test]
fn e7_known_gap_replay_state_is_boot_local() {
    let page = frame(1, bridge::RETURN_7_MODULE, 1000);
    let mut boot1 = HostExecutor::new(keys().pk);
    assert_eq!(boot1.call(&page).status, Status::Success);
    assert_eq!(boot1.call(&page).status, Status::Replay);
    let mut boot2 = HostExecutor::new(keys().pk);
    assert_eq!(boot2.call(&page).status, Status::Success, "cross-boot replay not yet prevented");
}

/// E8: fuel is a deterministic instruction budget, not a wall-clock deadline.
/// The minimum sufficient fuel for return-7 is stable across runs.
#[test]
fn e8_fuel_is_deterministic_budget_not_time() {
    let min_fuel = |round: u32| {
        (1..=10_000u64)
            .find(|&fuel| {
                let mut exec = HostExecutor::new(keys().pk);
                exec.call(&frame(round + 1, bridge::RETURN_7_MODULE, fuel)).status == Status::Success
            })
            .expect("return-7 succeeds within 10k fuel")
    };
    let first = min_fuel(0);
    for round in 1..5 {
        assert_eq!(min_fuel(round), first, "fuel threshold must not depend on timing");
    }
    if first > 1 {
        let mut exec = HostExecutor::new(keys().pk);
        assert_eq!(exec.call(&frame(1, bridge::RETURN_7_MODULE, first - 1)).status, Status::Fuel);
    }
    eprintln!("return-7 minimum fuel = {first}");
}

/// E9: payload size is bounded by the page; oversized descriptors fail closed.
#[test]
fn e9_payload_size_bound() {
    assert_eq!(MAX_PAYLOAD_BYTES, 3980);
    // The PCB-side helper refuses to build an oversized frame at all.
    let too_big = vec![0u8; MAX_PAYLOAD_BYTES + 1];
    let built = std::panic::catch_unwind(|| bridge::signed_frame(&keys().sk, 1, &too_big, 1000));
    assert!(built.is_err(), "signed_frame must refuse payload > page");
    // A descriptor claiming a payload that runs past the page is Malformed.
    let mut page = frame(1, bridge::RETURN_7_MODULE, 1000);
    page[18..20].copy_from_slice(&((MAX_PAYLOAD_BYTES + 1) as u16).to_le_bytes());
    let mut exec = HostExecutor::new(keys().pk);
    let reply = exec.call(&page);
    assert_eq!(reply.status, Status::Malformed);
    assert_eq!((exec.attempts(), exec.last_consumed()), (0, 0));
}
