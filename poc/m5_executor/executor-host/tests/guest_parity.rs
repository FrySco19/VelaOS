//! Drift guards: the host mirror must stay aligned with the guest sources it
//! models, and the guest PCB sequence (verify-bridge markers) is replayed on
//! the host with the same expected statuses and attempt counts.
//! These tests only READ `app/src/*.rs`; they never modify the guest.

use ed25519_compact::{KeyPair, Seed};
use vela_executor_host::{HostExecutor, bridge, bridge::Status};

const GUEST_MAIN: &str = include_str!("../../app/src/main.rs");
const GUEST_PCB: &str = include_str!("../../app/src/bin/pcb.rs");

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn guest_runtime_configuration_matches_host_mirror() {
    let guest = squash(GUEST_MAIN);
    for needle in [
        ".consume_fuel(true)",
        ".set_max_stack_height(16*1024)",
        ".set_max_recursion_depth(64)",
        "ifmodule.imports().next().is_some(){returnErr(bridge::Status::Wasm);}",
        ".memory_size(memory).memories(1).instances(1).tables(0).table_elements(0)",
        "store.set_fuel(fuel)",
        "Linker::<StoreLimits>::new(&engine);",
        "*attempts+=1;",
        "Some(TrapCode::OutOfFuel)",
        ".get_typed_func::<(),i32>(&store,\"run\")",
        "letsnapshot=unsafe{bridge::snapshot(self.shared_page)};",
        "bridge::verify_frame(&snapshot,self.last_consumed)",
        "self.last_consumed=verified.descriptor.request_id;",
        "regs[1]=valueasu32as_;",
    ] {
        assert!(guest.contains(&squash(needle)), "guest main.rs drifted from host mirror: {needle}");
    }
    // Consumption happens before execution in the guest, as mirrored.
    let consume = guest.find("self.last_consumed=verified.descriptor.request_id;").unwrap();
    let execute = guest.find("run_authenticated(payload").unwrap();
    assert!(consume < execute);
    // No Biscuit in the guest (C3).
    assert!(!GUEST_MAIN.contains("biscuit") && !GUEST_PCB.contains("biscuit"));
}

/// Parses `exchange(page, &X, bridge::Status::Y, N);` lines from pcb.rs.
fn guest_sequence() -> Vec<(String, u64)> {
    GUEST_PCB
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("exchange(page,"))
        .map(|l| {
            let status = l.split("bridge::Status::").nth(1).unwrap();
            let (status, rest) = status.split_once(',').unwrap();
            let attempts = rest.trim().trim_end_matches(");").trim().parse().unwrap();
            (status.to_string(), attempts)
        })
        .collect()
}

#[test]
fn host_replays_guest_pcb_sequence() {
    let keys = KeyPair::from_seed(Seed::new([46; 32]));
    let sk = &keys.sk;
    let good = bridge::signed_frame(sk, 1, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    let mut tampered = bridge::signed_frame(sk, 2, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    tampered[bridge::PAYLOAD_OFFSET + bridge::RETURN_7_MODULE.len() - 2] = 8;
    let mut bad_signature = bridge::signed_frame(sk, 3, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    bad_signature[bridge::TOKEN_OFFSET + bridge::CLAIMS_SIZE] ^= 1;
    let mut malformed = bridge::signed_frame(sk, 4, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    malformed[16..18].copy_from_slice(&4095u16.to_le_bytes());
    let overscoped = bridge::signed_frame(sk, 5, bridge::RETURN_7_MODULE, bridge::MAX_FUEL + 1);
    let loop_task = bridge::signed_frame(sk, 6, bridge::LOOP_MODULE, 64);
    let wasi = bridge::signed_frame(sk, 7, bridge::WASI_IMPORT_MODULE, bridge::MAX_FUEL);
    let final_task = bridge::signed_frame(sk, 8, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    let max_fuel_loop = bridge::signed_frame(sk, 9, bridge::LOOP_MODULE, bridge::MAX_FUEL);
    let after_max_fuel = bridge::signed_frame(sk, 10, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);

    let host: Vec<(&[u8; bridge::PAGE_SIZE], Status, u64)> = vec![
        (&good, Status::Success, 1),
        (&tampered, Status::Authentication, 1),
        (&bad_signature, Status::Authentication, 1),
        (&malformed, Status::Malformed, 1),
        (&overscoped, Status::Scope, 1),
        (&good, Status::Replay, 1),
        (&loop_task, Status::Fuel, 2),
        (&wasi, Status::Wasm, 2),
        (&final_task, Status::Success, 3),
        (&max_fuel_loop, Status::Fuel, 4),
        (&after_max_fuel, Status::Success, 5),
    ];
    let guest = guest_sequence();
    assert_eq!(guest.len(), host.len(), "pcb.rs exchange sequence changed");
    let mut exec = HostExecutor::new(keys.pk);
    for (i, ((page, status, attempts), (g_status, g_attempts))) in host.iter().zip(&guest).enumerate() {
        assert_eq!(format!("{status:?}"), *g_status, "step {i}: status differs from pcb.rs");
        assert_eq!(attempts, g_attempts, "step {i}: attempts differ from pcb.rs");
        let reply = exec.call(page);
        assert_eq!((reply.status, reply.attempts), (*status, *attempts), "step {i}");
        if *status == Status::Success {
            assert_eq!(reply.value, 7);
        }
        if *status != Status::Malformed {
            assert_eq!(reply.request_id, u32::from_le_bytes(page[8..12].try_into().unwrap()));
        }
    }
}
