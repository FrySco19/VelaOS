#![no_std]
#![no_main]
#![feature(asm_cfg)]

#[path = "../bridge.rs"]
mod bridge;
use core::sync::atomic::{Ordering, fence};
use ed25519_compact::{KeyPair, Seed};
use sel4_microkit::{
    Channel, Handler, MessageInfo, NullHandler, debug_println, memory_region_symbol,
    protection_domain, with_msg_regs,
};

fn exchange(
    page: *mut u8,
    frame: &[u8; bridge::PAGE_SIZE],
    expected: bridge::Status,
    attempts: u64,
) {
    // SAFETY: Microkit maps the complete page rw in this PCB domain.
    unsafe {
        bridge::publish(page, frame);
    }
    fence(Ordering::Release);
    let reply = Channel::new(0).pp_call(MessageInfo::new(bridge::EXECUTE_LABEL as _, 0));
    assert_eq!(reply.label(), bridge::REPLY_LABEL as _);
    assert_eq!(reply.count(), 4);
    let result = with_msg_regs(|regs| {
        [
            regs[0] as u64,
            regs[1] as u64,
            regs[2] as u64,
            regs[3] as u64,
        ]
    });
    assert_eq!(result[0], expected as u64);
    assert_eq!(
        result[3], attempts,
        "unauthenticated bytes entered Wasm execution"
    );
    if expected == bridge::Status::Success {
        assert_eq!(result[1], 7);
    }
    debug_println!(
        "VelaOS Phase 2/3: PCB reply request={} status={} value={} attempts={}",
        result[2],
        result[0],
        result[1],
        result[3]
    );
}

#[protection_domain]
fn init() -> impl Handler {
    // BuildKit provides a locally generated seed. No private seed is in Git.
    // This boot-demo embeds the key in PCB only; production needs key provisioning.
    let seed: &[u8; 32] = include_bytes!(env!("VELA_PCB_SEED"));
    let key = KeyPair::from_seed(Seed::new(*seed));
    assert_eq!(
        key.pk.as_ref(),
        bridge::trusted_key::TRUSTED_PUBLIC_KEY.as_slice()
    );
    let page = memory_region_symbol!(shared_page_addr: *mut u8).as_ptr();

    let good = bridge::signed_frame(&key.sk, 1, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    exchange(page, &good, bridge::Status::Success, 1);

    let mut tampered = bridge::signed_frame(&key.sk, 2, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    tampered[bridge::PAYLOAD_OFFSET + bridge::RETURN_7_MODULE.len() - 2] = 8;
    exchange(page, &tampered, bridge::Status::Authentication, 1);
    debug_println!("VelaOS Phase 2: tampered return-8 payload rejected before execution");

    let mut bad_signature =
        bridge::signed_frame(&key.sk, 3, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    bad_signature[bridge::TOKEN_OFFSET + bridge::CLAIMS_SIZE] ^= 1;
    exchange(page, &bad_signature, bridge::Status::Authentication, 1);

    let mut malformed = bridge::signed_frame(&key.sk, 4, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    malformed[16..18].copy_from_slice(&4095u16.to_le_bytes());
    exchange(page, &malformed, bridge::Status::Malformed, 1);

    let overscoped =
        bridge::signed_frame(&key.sk, 5, bridge::RETURN_7_MODULE, bridge::MAX_FUEL + 1);
    exchange(page, &overscoped, bridge::Status::Scope, 1);
    exchange(page, &good, bridge::Status::Replay, 1);

    let loop_task = bridge::signed_frame(&key.sk, 6, bridge::LOOP_MODULE, 64);
    exchange(page, &loop_task, bridge::Status::Fuel, 2);
    debug_println!("VelaOS Phase 3: signed infinite loop stopped by fuel");

    let wasi = bridge::signed_frame(&key.sk, 7, bridge::WASI_IMPORT_MODULE, bridge::MAX_FUEL);
    exchange(page, &wasi, bridge::Status::Wasm, 2);
    debug_println!("VelaOS Phase 3: signed WASI import rejected");

    let final_task = bridge::signed_frame(&key.sk, 8, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    exchange(page, &final_task, bridge::Status::Success, 3);
    debug_println!("VelaOS Phase 2/3: ALL CHECKS PASSED; authenticated Wasm returned 7 via IPC");
    NullHandler::new()
}
