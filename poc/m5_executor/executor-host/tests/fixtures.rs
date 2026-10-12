//! Fail-closed tests over committed Wasm fixtures in `poc/m5_executor/fixtures/`.
//! Every rejection is checked to come from an Executor rule (limit, linker,
//! entry point), not from a fixture that simply fails to validate.

#[path = "../examples/gen_fixtures.rs"]
#[allow(dead_code)]
mod gen_fixtures;

use ed25519_compact::{KeyPair, Seed};
use vela_executor_host::{HostExecutor, MAX_PAYLOAD_BYTES, bridge, bridge::Status, import_count, validates};

fn fixture(name: &str) -> Vec<u8> {
    let path = gen_fixtures::fixtures_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn run(name: &str, fuel: u64) -> (vela_executor_host::Reply, HostExecutor) {
    let keys = KeyPair::from_seed(Seed::new([43; 32]));
    let bytes = fixture(name);
    let page = bridge::signed_frame(&keys.sk, 1, &bytes, fuel);
    let mut exec = HostExecutor::new(keys.pk);
    let reply = exec.call(&page);
    (reply, exec)
}

#[test]
fn fixtures_match_sources() {
    for (name, bytes) in gen_fixtures::generate() {
        assert_eq!(fixture(&name), bytes, "{name} drifted; rerun the gen_fixtures example");
    }
}

#[test]
fn fixtures_are_valid_wasm_so_rejections_are_policy() {
    for (name, _) in gen_fixtures::generate() {
        assert!(validates(&fixture(&name)), "{name} must validate under the guest feature set");
    }
}

/// Table: fixture, expected status, expected value, whether Wasmi instantiation was attempted.
#[test]
fn fixture_outcomes_fail_closed() {
    let cases: &[(&str, Status, u32, u64, &str)] = &[
        ("import_env_func.wasm", Status::Wasm, 0, 0, "E4 any import rejected"),
        ("import_memory.wasm", Status::Wasm, 0, 0, "E4 imported memory rejected"),
        ("memory_over_quota.wasm", Status::Wasm, 0, 1, "E3 128 KiB > 64 KiB quota"),
        ("table_declared.wasm", Status::Wasm, 0, 1, "E3 tables(0)"),
        ("deep_recursion.wasm", Status::Wasm, 0, 1, "E3 recursion/stack limit, not fuel"),
        ("start_trap.wasm", Status::Wasm, 0, 1, "E5 trapping start"),
        ("no_run_export.wasm", Status::Wasm, 0, 1, "E5 missing run export"),
        ("wrong_signature.wasm", Status::Wasm, 0, 1, "E5 run must be () -> i32"),
        ("grow_denied.wasm", Status::Success, u32::MAX, 1, "E3 grow past quota returns -1"),
        ("negative_result.wasm", Status::Success, u32::MAX, 1, "E5 i32 -1 as u32 bits"),
        ("max_payload.wasm", Status::Success, 7, 1, "E9 3980-byte payload fits"),
    ];
    for &(name, status, value, attempts, why) in cases {
        let (reply, exec) = run(name, bridge::MAX_FUEL);
        assert_eq!(reply.status, status, "{name}: {why}");
        assert_eq!(reply.value, value, "{name}: {why}");
        assert_eq!(exec.attempts(), attempts, "{name}: {why}");
        assert_eq!(exec.last_consumed(), 1, "{name}: authenticated ID consumed before execution");
    }
    assert!(import_count(&fixture("import_env_func.wasm")) > 0);
    assert!(import_count(&fixture("import_memory.wasm")) > 0);
}

/// E9: a module one byte larger than the frame capacity can never be admitted.
#[test]
fn oversized_module_fails_closed() {
    let bytes = fixture("oversized.wasm");
    assert_eq!(bytes.len(), MAX_PAYLOAD_BYTES + 1);
    assert!(validates(&bytes), "oversized fixture is valid Wasm; rejection is size-only");
    let keys = KeyPair::from_seed(Seed::new([44; 32]));
    assert!(std::panic::catch_unwind(|| bridge::signed_frame(&keys.sk, 1, &bytes, 1000)).is_err());
    // A hand-built page whose descriptor claims the oversized length is Malformed.
    let mut page = bridge::signed_frame(&keys.sk, 1, &bytes[..MAX_PAYLOAD_BYTES], 1000);
    page[18..20].copy_from_slice(&(bytes.len() as u16).to_le_bytes());
    let mut exec = HostExecutor::new(keys.pk);
    assert_eq!(exec.call(&page).status, Status::Malformed);
    assert_eq!((exec.attempts(), exec.last_consumed()), (0, 0));
}

/// E3/E8: deep recursion is stopped by the recursion limit even with max fuel,
/// and a loop is stopped by fuel; both consume the ID and return value 0.
#[test]
fn bounded_by_limits_not_by_time() {
    let (reply, _) = run("deep_recursion.wasm", bridge::MAX_FUEL);
    assert_eq!(reply.status, Status::Wasm);
    let keys = KeyPair::from_seed(Seed::new([45; 32]));
    let mut exec = HostExecutor::new(keys.pk);
    let reply = exec.call(&bridge::signed_frame(&keys.sk, 1, bridge::LOOP_MODULE, bridge::MAX_FUEL));
    assert_eq!((reply.status, reply.value), (Status::Fuel, 0));
}
