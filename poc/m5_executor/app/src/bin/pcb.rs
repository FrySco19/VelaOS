#![no_std]
#![no_main]
#![feature(asm_cfg)]

#[path = "../bridge.rs"]
mod bridge;
#[cfg(feature = "e2e")]
#[path = "../ledger_wire.rs"]
mod ledger_wire;
#[cfg(feature = "e2e")]
#[path = "../guest_clock.rs"]
mod guest_clock;
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
    // Single-flight ownership: PCB writes only before pp_call, then freezes the
    // page until the matching synchronous reply acknowledges completion.
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
    let request_id = u32::from_le_bytes(frame[8..12].try_into().unwrap());
    if expected != bridge::Status::Malformed {
        assert_eq!(
            result[2], request_id as u64,
            "reply acknowledged wrong request"
        );
    }
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

    // R1: a signed infinite loop admitted at MAX_FUEL must exhaust fuel (status 6)
    // without overflowing the Executor's 64 KiB native stack, and the Executor
    // must keep serving afterwards (Wasmi built with `portable-dispatch`).
    let max_fuel_loop = bridge::signed_frame(&key.sk, 9, bridge::LOOP_MODULE, bridge::MAX_FUEL);
    exchange(page, &max_fuel_loop, bridge::Status::Fuel, 4);
    debug_println!("VelaOS Phase 3: MAX_FUEL stack-safe status=6");
    let after_max_fuel = bridge::signed_frame(&key.sk, 10, bridge::RETURN_7_MODULE, bridge::MAX_FUEL);
    exchange(page, &after_max_fuel, bridge::Status::Success, 5);
    debug_println!("VelaOS Phase 2/3: ALL CHECKS PASSED; authenticated Wasm returned 7 via IPC");
    #[cfg(feature = "e2e")]
    e2e::run(page);
    NullHandler::new()
}

/// verify-bridge-e2e: the PCB relays a frame admitted by the host PDP after an
/// explicit UserSelection (Biscuit stays host-side, std-only), then commits the
/// Executor result to the LFS writer domain and notarizes the signed receipt.
#[cfg(feature = "e2e")]
mod e2e {
    use super::{bridge, guest_clock as gc, ledger_wire as lw};
    use core::sync::atomic::{Ordering, fence};
    use ed25519_compact::PublicKey;
    use sel4_microkit::{Channel, MessageInfo, debug_println, with_msg_regs, with_msg_regs_mut};

    const FRAME: &[u8; bridge::PAGE_SIZE] = include_bytes!("../../generated/e2e_frame.bin");
    const META: &[u8; 120] = include_bytes!("../../generated/e2e_meta.bin");
    /// M-CLK-3: second admitted frame whose grant has a 1 s TTL.
    const SHORT: &[u8; bridge::PAGE_SIZE] = include_bytes!("../../generated/e2e_frame_short.bin");
    /// `VCLK` + short request id (u32) + short TTL seconds (u64).
    const CLOCK: &[u8; 16] = include_bytes!("../../generated/e2e_clock.bin");
    /// C5 EXPIRED.
    const EXPIRED: u64 = 7;

    fn u16_at(o: usize) -> u16 {
        u16::from_le_bytes(META[o..o + 2].try_into().unwrap())
    }
    fn u32_at(o: usize) -> u32 {
        u32::from_le_bytes(META[o..o + 4].try_into().unwrap())
    }
    fn u64_at(o: usize) -> u64 {
        u64::from_le_bytes(META[o..o + 8].try_into().unwrap())
    }

    fn execute(page: *mut u8, frame: &[u8; bridge::PAGE_SIZE]) -> [u64; 4] {
        // SAFETY: Microkit maps the complete page rw in this PCB domain (OWN-v2).
        unsafe {
            bridge::publish(page, frame);
        }
        fence(Ordering::Release);
        let reply = Channel::new(0).pp_call(MessageInfo::new(bridge::EXECUTE_LABEL as _, 0));
        assert_eq!(reply.label(), bridge::REPLY_LABEL as _);
        assert_eq!(reply.count(), 4);
        with_msg_regs(|r| [r[0] as u64, r[1] as u64, r[2] as u64, r[3] as u64])
    }

    /// Synchronous PCB -> LFS commit. Err carries the writer status code.
    fn commit(c: &lw::Commit) -> Result<(lw::Receipt, bool), u64> {
        let regs = c.to_regs();
        with_msg_regs_mut(|m| {
            for (i, v) in regs.iter().enumerate() {
                m[i] = *v as _;
            }
        });
        let reply = Channel::new(1).pp_call(MessageInfo::new(lw::COMMIT_LABEL as _, lw::COMMIT_REGS));
        assert_eq!(reply.label(), lw::RECEIPT_LABEL as _);
        let n = reply.count().min(lw::RECEIPT_REGS);
        let mut out = [0u64; lw::RECEIPT_REGS];
        with_msg_regs(|m| {
            for i in 0..n {
                out[i] = m[i] as u64;
            }
        });
        if out[0] != lw::LfsStatus::Committed as u64 {
            return Err(out[0]);
        }
        assert_eq!(n, lw::RECEIPT_REGS);
        Ok((lw::Receipt::from_reply(c, &out), out[1] == 1))
    }

    /// PCB -> LFS commit gated by the in-guest monotonic deadline: an expired
    /// (or untrustworthy-time) grant never reaches the writer.
    fn commit_if_alive(
        dl: &gc::Deadline,
        c: &lw::Commit,
    ) -> Result<(lw::Receipt, bool, u64), u64> {
        let remaining = dl.remaining_ms(gc::now()).ok_or(EXPIRED)?;
        commit(c).map(|(r, replayed)| (r, replayed, remaining))
    }

    pub fn run(page: *mut u8) {
        assert_eq!(&META[0..4], b"VE2M");
        let epoch = u64_at(4);
        let denied = u32_at(12);
        let admitted = u32_at(16);
        let (deny_code, signed_on_deny) = (u16_at(20), u16_at(22));
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&META[24..56]);
        let mut lfs_key = [0u8; 32];
        lfs_key.copy_from_slice(&META[56..88]);
        let ttl = u64_at(112);

        debug_println!(
            "VelaOS E2E [1]: mock AI intent -> {} candidates (data only, no authority)",
            u32_at(92)
        );
        assert_eq!((deny_code, signed_on_deny), (10, 0));
        debug_println!(
            "VelaOS E2E [2]: AI-only request={} POLICY_DENIED code={} frames_signed=0, nothing published",
            denied,
            deny_code
        );
        assert!(ttl > 0 && ttl <= 120);
        debug_println!(
            "VelaOS E2E [3]: UserSelection id={} version={} -> PDP Biscuit LFS_READ grant bytes={} ttl={}s authority={:02x}{:02x}{:02x}{:02x}",
            u64_at(104),
            u64_at(96),
            u32_at(88),
            ttl,
            digest[0],
            digest[1],
            digest[2],
            digest[3]
        );
        let d = bridge::parse_descriptor(FRAME).unwrap();
        assert_eq!(d.request_id, admitted);
        assert_eq!(
            lw::grant_digest_in(&FRAME[d.payload_offset..d.payload_offset + d.payload_len]),
            Some(digest)
        );
        debug_println!(
            "VelaOS E2E [4]: admitted frame request={} published on 4 KiB RO page",
            admitted
        );
        let boot = gc::now();
        assert!(boot.freq > 0, "fail-closed: no trusted guest counter");
        debug_println!(
            "VelaOS E2E CLK: guest monotonic CNTPCT_EL0 freq={} Hz (in-guest source, not host wall clock)",
            boot.freq
        );
        // Anchor the grant TTL on the guest counter at publication.
        let deadline = gc::Deadline::start(gc::now(), ttl).expect("fail-closed: TTL/clock");
        let r = execute(page, FRAME);
        assert_eq!(r, [0, 21, admitted as u64, 6], "e2e execution");
        debug_println!(
            "VelaOS E2E [5]: PCB reply request={} status={} value={} attempts={} (fuel-safe Wasm)",
            r[2],
            r[0],
            r[1],
            r[3]
        );

        let c = lw::Commit {
            epoch,
            request_id: admitted as u64,
            status: 0,
            value: r[1] as u32,
            authority_digest: digest,
        };
        let mut notary: lw::Notary<4> = lw::Notary::new(PublicKey::new(lfs_key));
        let (receipt, replayed, remaining) =
            commit_if_alive(&deadline, &c).expect("grant alive in-guest -> LFS commit");
        assert!(!replayed);
        debug_println!(
            "VelaOS E2E CLK [1]: request={} grant ttl={}s alive in-guest remaining_ms={} -> commit allowed",
            admitted,
            ttl,
            remaining
        );
        debug_println!(
            "VelaOS E2E [6]: LFS receipt epoch={} request={} seq={} replayed=0",
            receipt.body.epoch,
            receipt.body.request_id,
            receipt.body.seq
        );
        assert_eq!(notary.accept(&c, &receipt), Ok(true));
        debug_println!(
            "VelaOS E2E [7]: PCB notary verified+stored receipt request={} value={} seq={}",
            receipt.body.request_id,
            receipt.body.value,
            receipt.body.seq
        );

        let (again, replayed) = commit(&c).expect("idempotent retry");
        assert!(replayed && again == receipt);
        assert_eq!(notary.accept(&c, &again), Ok(false));
        debug_println!(
            "VelaOS E2E N1: commit retry same epoch/request -> same receipt seq={} replayed=1",
            again.body.seq
        );
        let conflict = lw::Commit { value: 22, ..c };
        assert_eq!(commit(&conflict).unwrap_err(), lw::LfsStatus::IdempotencyConflict as u64);
        debug_println!("VelaOS E2E N2: same key different result -> status=13 not persisted");
        let unauthorized = lw::Commit {
            request_id: denied as u64,
            ..c
        };
        assert_eq!(commit(&unauthorized).unwrap_err(), lw::LfsStatus::PolicyDenied as u64);
        debug_println!(
            "VelaOS E2E N3: output for AI-only request={} -> status=10 not persisted",
            denied
        );
        let mut forged = receipt;
        forged.signature[5] ^= 1;
        assert_eq!(notary.accept(&c, &forged), Err(lw::NotaryError::Forged));
        debug_println!("VelaOS E2E N4: forged receipt rejected by PCB notary");
        let replay = execute(page, FRAME);
        assert_eq!(replay, [4, 0, admitted as u64, 6], "replayed admission");
        debug_println!(
            "VelaOS E2E N5: replayed admission request={} status=4 attempts=6 (no Wasm, no commit)",
            admitted
        );
        // M-CLK-3: grant with a 1 s TTL; the result arrives after the deadline
        // measured by the guest counter -> EXPIRED, never committed.
        assert_eq!(&CLOCK[0..4], b"VCLK");
        let short_id = u32::from_le_bytes(CLOCK[4..8].try_into().unwrap());
        let short_ttl = u64::from_le_bytes(CLOCK[8..16].try_into().unwrap());
        let sd = bridge::parse_descriptor(SHORT).unwrap();
        assert_eq!(sd.request_id, short_id);
        let short_digest =
            lw::grant_digest_in(&SHORT[sd.payload_offset..sd.payload_offset + sd.payload_len])
                .expect("short frame grant-bound");
        let short_deadline =
            gc::Deadline::start(gc::now(), short_ttl).expect("fail-closed: TTL/clock");
        let r = execute(page, SHORT);
        assert_eq!(r, [0, 21, short_id as u64, 7], "short-ttl execution");
        // Slow path (e.g. long task / stalled reply): wait on the guest counter
        // until the grant is dead, bounded so a stuck counter cannot hang.
        loop {
            let n = gc::now();
            if !short_deadline.alive(n) {
                break;
            }
            assert!(
                short_deadline.elapsed_ms(n) <= 10_000 * short_ttl,
                "guest counter not advancing"
            );
        }
        let elapsed = short_deadline.elapsed_ms(gc::now());
        let late = lw::Commit {
            epoch,
            request_id: short_id as u64,
            status: 0,
            value: r[1] as u32,
            authority_digest: short_digest,
        };
        assert_eq!(commit_if_alive(&short_deadline, &late).unwrap_err(), EXPIRED);
        debug_println!(
            "VelaOS E2E CLK [2]: request={} grant ttl={}s expired in-guest after {}ms -> EXPIRED code=7, result discarded, no LFS commit",
            short_id,
            short_ttl,
            elapsed
        );
        assert_eq!(notary.len(), 1);
        assert!(notary.get(epoch, short_id as u64).is_none());
        debug_println!(
            "VelaOS E2E: ALL E2E CHECKS PASSED; intent -> selection -> grant -> Wasm 21 -> receipt notarized"
        );
    }
}
