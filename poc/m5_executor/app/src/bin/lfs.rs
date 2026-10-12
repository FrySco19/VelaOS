#![no_std]
#![no_main]
#![feature(asm_cfg)]

//! Simulated LFS writer domain (verify-bridge-e2e only).
//!
//! Maps the PCB->Executor admission page read-only. On a synchronous commit
//! from the PCB it persists a successful result only if the admission frame on
//! the page authenticates under the pinned PCB key, names the same request and
//! carries the same authority digest; then it returns an Ed25519-signed receipt.
//! Idempotency key (epoch, request_id). Default E2E is in-memory; the hardware
//! feature uses the dedicated VirtIO disk and authenticates recovery on boot.

#[path = "../bridge.rs"]
mod bridge;
#[path = "../ledger_wire.rs"]
mod ledger_wire;

use core::sync::atomic::{Ordering, fence};
use ed25519_compact::{KeyPair, PublicKey, Seed};
use ledger_wire as lw;
use sel4_microkit::{
    Channel, Handler, MessageInfo, debug_println, memory_region_symbol, protection_domain,
    with_msg_regs, with_msg_regs_mut,
};

#[cfg(feature = "hardware")]
#[path = "../disk_ledger.rs"]
mod disk_ledger;
#[cfg(feature = "hardware")]
#[allow(unused_attributes, dead_code)]
#[path = "../../../../storage-core/src/lib.rs"]
mod storage_core;
#[cfg(feature = "hardware")]
#[path = "../virtio_block.rs"]
mod virtio_block;
#[cfg(not(feature = "hardware"))]
const CAPACITY: usize = 8;

#[protection_domain]
fn init() -> impl Handler {
    // Local build secret (Docker BuildKit secret), never in Git.
    let seed: &[u8; 32] = include_bytes!(env!("VELA_LFS_SEED"));
    let key = KeyPair::from_seed(Seed::new(*seed));
    #[cfg(not(feature = "hardware"))]
    debug_println!(
        "VelaOS E2E: LFS writer ready (simulated in-memory ledger, capacity={})",
        CAPACITY
    );
    #[cfg(feature = "hardware")]
    debug_println!("VelaOS E2E: LFS writer ready (VirtIO durable bounded ledger, capacity=8)");
    #[cfg(feature = "hardware")]
    let disk = disk_ledger::DiskLedger::new(&key);
    LfsWriter {
        page: memory_region_symbol!(shared_page_addr: *mut u8).as_ptr(),
        key,
        #[cfg(not(feature = "hardware"))]
        ledger: lw::Ledger::new(),
        #[cfg(feature = "hardware")]
        disk,
    }
}

struct LfsWriter {
    page: *mut u8,
    key: KeyPair,
    #[cfg(not(feature = "hardware"))]
    ledger: lw::Ledger<CAPACITY>,
    #[cfg(feature = "hardware")]
    disk: disk_ledger::DiskLedger,
}

impl LfsWriter {
    fn len(&self) -> usize {
        #[cfg(not(feature = "hardware"))]
        {
            self.ledger.len()
        }
        #[cfg(feature = "hardware")]
        {
            self.disk.len()
        }
    }
    fn status(status: lw::LfsStatus) -> MessageInfo {
        with_msg_regs_mut(|m| m[0] = status as u64 as _);
        MessageInfo::new(lw::RECEIPT_LABEL as _, 1)
    }
}

impl Handler for LfsWriter {
    type Error = core::convert::Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        message: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != Channel::new(0)
            || message.label() != lw::COMMIT_LABEL as _
            || message.count() != lw::COMMIT_REGS
        {
            return Ok(Self::status(lw::LfsStatus::Malformed));
        }
        let regs = with_msg_regs(|m| {
            let mut a = [0u64; lw::COMMIT_REGS];
            for (i, v) in a.iter_mut().enumerate() {
                *v = m[i] as u64;
            }
            a
        });
        let Some(c) = lw::Commit::from_regs(&regs) else {
            return Ok(Self::status(lw::LfsStatus::Malformed));
        };
        fence(Ordering::Acquire);
        // SAFETY: Microkit maps the whole 4 KiB page readable here; decide on a
        // private snapshot only.
        let snapshot = unsafe { bridge::snapshot(self.page) };
        let pcb = PublicKey::new(bridge::trusted_key::TRUSTED_PUBLIC_KEY);
        #[cfg(not(feature = "hardware"))]
        let result = self.ledger.commit(&self.key.sk, pcb, &snapshot, &c);
        #[cfg(feature = "hardware")]
        let result = self.disk.commit(&self.key, pcb, &snapshot, &c);
        match result {
            Ok(outcome) => {
                let replayed = matches!(outcome, lw::CommitOutcome::Replayed(_));
                let r = outcome.receipt();
                debug_println!(
                    "VelaOS E2E: LFS {} epoch={} request={} seq={} persisted={}",
                    if replayed { "replayed" } else { "committed" },
                    r.body.epoch,
                    r.body.request_id,
                    r.body.seq,
                    self.len()
                );
                let out = r.to_reply(replayed);
                with_msg_regs_mut(|m| {
                    for (i, v) in out.iter().enumerate() {
                        m[i] = *v as _;
                    }
                });
                Ok(MessageInfo::new(lw::RECEIPT_LABEL as _, lw::RECEIPT_REGS))
            }
            Err(status) => {
                debug_println!(
                    "VelaOS E2E: LFS refused request={} status={} persisted={}",
                    c.request_id,
                    status as u64,
                    self.len()
                );
                Ok(Self::status(status))
            }
        }
    }
}
