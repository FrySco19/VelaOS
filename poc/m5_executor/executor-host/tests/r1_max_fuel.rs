//! Risk R1 (EXECUTOR_CONTRACT E8): a task admitted at `MAX_FUEL` must end in
//! `Status::Fuel` (6) with value 0, never in a native stack overflow.
//!
//! Every case runs the full host mirror (signed v2 frame -> admission -> Wasmi)
//! on a thread with a fixed native stack budget and a MAX_FUEL (10^7) task:
//!
//! - `cargo test` (dev profile, opt-level 0, debug assertions): 1 MiB. Before
//!   `portable-dispatch`, Wasmi 2.0.0's tail-call backend grew one native frame
//!   per executed instruction at opt 0, so these tests aborted with
//!   STATUS_STACK_OVERFLOW; with it they finish in the same budget as a 64-fuel
//!   task (constant stack). 1 MiB rather than 64 KiB because unoptimised
//!   admission code alone (4 KiB snapshot copies, Ed25519) needs > 256 KiB.
//! - `cargo test --release` (the guest's opt-level 3): 64 KiB, the Executor
//!   domain's native stack (`executor.system` stack_size="0x10_000").
//!
//! If the dispatch backend regresses, the process aborts with a stack overflow
//! instead of reporting a failed assertion - which is the R1 failure mode itself.

#[path = "../examples/gen_fixtures.rs"]
#[allow(dead_code)]
mod gen_fixtures;

use ed25519_compact::{KeyPair, Seed};
use vela_executor_host::{HostExecutor, Reply, bridge, bridge::Status};

/// Native stack budget: the Executor domain's 64 KiB (`executor.system`) in
/// optimised builds; 1 MiB for unoptimised admission code (see module docs).
const EXECUTOR_NATIVE_STACK: usize = if cfg!(debug_assertions) { 0x10_0000 } else { 0x10_000 };

const MANIFEST: &str = include_str!("../Cargo.toml");
const GUEST_MANIFEST: &str = include_str!("../../app/Cargo.toml");

fn fixture(name: &str) -> Vec<u8> {
    let path = gen_fixtures::fixtures_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Executes one signed frame on a thread with the Executor's native stack size.
fn run_on_executor_stack(payload: Vec<u8>, fuel: u64) -> (Reply, u64, u32) {
    std::thread::Builder::new()
        .name("executor-stack".into())
        .stack_size(EXECUTOR_NATIVE_STACK)
        .spawn(move || {
            let keys = KeyPair::from_seed(Seed::new([47; 32]));
            let mut exec = HostExecutor::new(keys.pk);
            let reply = exec.call(&bridge::signed_frame(&keys.sk, 9, &payload, fuel));
            // The same session must keep serving after the MAX_FUEL task.
            let after = exec.call(&bridge::signed_frame(&keys.sk, 10, bridge::RETURN_7_MODULE, fuel));
            assert_eq!((after.status, after.value), (Status::Success, 7), "executor unusable after R1 case");
            (reply, exec.attempts(), exec.last_consumed())
        })
        .expect("spawn executor thread")
        .join()
        .expect("executor thread panicked")
}

fn assert_fuel_exhausted(name: &str, payload: Vec<u8>) {
    let (reply, attempts, last) = run_on_executor_stack(payload, bridge::MAX_FUEL);
    assert_eq!(reply.status, Status::Fuel, "{name}: MAX_FUEL task must end in status 6");
    assert_eq!(reply.status as u64, 6);
    assert_eq!(reply.value, 0, "{name}: no partial result on fuel exhaustion");
    assert_eq!(reply.request_id, 9, "{name}: reply acknowledges the request");
    assert_eq!((attempts, last), (2, 10), "{name}: both requests executed and consumed");
}

/// Baseline: the 64-fuel loop already proven in QEMU (request 6) fits the budget.
#[test]
fn r1_loop_at_probe_fuel_fits_same_budget() {
    let (reply, _, _) = run_on_executor_stack(bridge::LOOP_MODULE.to_vec(), 64);
    assert_eq!((reply.status, reply.value), (Status::Fuel, 0));
}

/// The guest PCB's request 9: `bridge::LOOP_MODULE` at MAX_FUEL.
#[test]
fn r1_guest_loop_module_at_max_fuel_returns_status_6() {
    assert_fuel_exhausted("bridge::LOOP_MODULE", bridge::LOOP_MODULE.to_vec());
}

#[test]
fn r1_tight_loop_fixture_at_max_fuel_returns_status_6() {
    assert_fuel_exhausted("max_fuel_loop.wasm", fixture("max_fuel_loop.wasm"));
}

#[test]
fn r1_call_loop_fixture_at_max_fuel_returns_status_6() {
    assert_fuel_exhausted("max_fuel_call_loop.wasm", fixture("max_fuel_call_loop.wasm"));
}

#[test]
fn r1_recursion_churn_fixture_at_max_fuel_returns_status_6() {
    assert_fuel_exhausted("max_fuel_recursion_churn.wasm", fixture("max_fuel_recursion_churn.wasm"));
}

/// Recursion past the limit is still a WASM trap (5) within the same stack budget.
#[test]
fn r1_unbounded_recursion_at_max_fuel_is_status_5_not_overflow() {
    let (reply, _, _) = run_on_executor_stack(fixture("deep_recursion.wasm"), bridge::MAX_FUEL);
    assert_eq!((reply.status, reply.value), (Status::Wasm, 0));
}

fn wasmi_line(manifest: &str) -> String {
    manifest
        .lines()
        .find(|l| l.trim_start().starts_with("wasmi ="))
        .expect("wasmi dependency")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// The mitigation is a build feature: guard it in both manifests.
#[test]
fn r1_portable_dispatch_is_pinned_in_guest_and_host() {
    let guest = wasmi_line(GUEST_MANIFEST);
    let host = wasmi_line(MANIFEST);
    assert!(guest.contains("\"portable-dispatch\""), "guest wasmi must enable portable-dispatch: {guest}");
    assert!(!guest.contains("auto-dispatch") && !guest.contains("default-features=true"));
    assert_eq!(guest, host, "host mirror must use the guest's exact wasmi features");
}