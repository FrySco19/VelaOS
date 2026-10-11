//! M-G1c host orchestration of the State Ledger (simulated LFS writer).
//!
//! Admission ([`SelectionGatedPdp`]) -> Executor (caller-supplied closure: the
//! QEMU guest, `vela-executor-host`, or a test double) -> simulated LFS writer
//! ([`Ledger`], the *same* `no_std` code the guest `velaos-lfs` runs) ->
//! synchronous receipt -> PCB [`Notary`] verification and storage.
//!
//! **Simulated**: the ledger is in memory, has no durable storage, no
//! cross-boot epoch and no `result_handle`/`expected_version` write check yet.

pub use crate::ledger_wire::{
    Commit, CommitOutcome, Ledger, LfsStatus, Notary, NotaryError, Receipt, ReceiptBody,
};
use crate::pipeline::{SelectionGatedPdp, TaskOutcome, TaskRequest};
use crate::{DenyCode, bridge};
use ed25519_compact::{PublicKey, SecretKey};
use std::time::SystemTime;

pub const LEDGER_CAPACITY: usize = 64;

/// The four Executor reply registers (label 0x702).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecReply {
    pub status: u64,
    pub value: u32,
    pub request_id: u32,
    pub attempts: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum E2eError {
    /// PDP denied: nothing signed, executor never called.
    Denied(DenyCode),
    /// Executor returned a non-success status: nothing committed.
    Execution(u64),
    /// Executor acknowledged another request: nothing committed.
    AckMismatch,
    /// Writer refused the commit: nothing persisted.
    Writer(LfsStatus),
    /// PCB refused the receipt.
    Notary(NotaryError),
}

/// Result of one notarized task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notarized {
    pub receipt: Receipt,
    pub replayed: bool,
}

/// Simulated LFS writer domain: holds its own signing key; the PCB holds only
/// the public half.
pub struct SimulatedLfs {
    key: SecretKey,
    pcb: PublicKey,
    ledger: Ledger<LEDGER_CAPACITY>,
}

impl SimulatedLfs {
    pub fn new(key: SecretKey, pcb: PublicKey) -> Self {
        Self {
            key,
            pcb,
            ledger: Ledger::new(),
        }
    }
    pub fn public_key(&self) -> PublicKey {
        self.key.public_key()
    }
    pub fn persisted(&self) -> usize {
        self.ledger.len()
    }
    /// Synchronous commit against the frame currently on the RO page.
    pub fn commit(
        &mut self,
        page: &[u8; bridge::PAGE_SIZE],
        c: &Commit,
    ) -> Result<CommitOutcome, LfsStatus> {
        // SAFETY: `page` is a valid 4 KiB buffer; snapshot like the guest does.
        let snapshot = unsafe { bridge::snapshot(page.as_ptr()) };
        self.ledger.commit(&self.key, self.pcb, &snapshot, c)
    }
}

/// Host model of the whole pipeline with the PCB as notary.
pub struct E2eHost {
    pub pdp: SelectionGatedPdp,
    pub lfs: SimulatedLfs,
    pub notary: Notary<LEDGER_CAPACITY>,
    pub epoch: u64,
    /// Last published page (PCB-writable / LFS+Executor-readable).
    pub page: Box<[u8; bridge::PAGE_SIZE]>,
    pub executions: u64,
}

impl E2eHost {
    pub fn new(pdp: SelectionGatedPdp, lfs: SimulatedLfs, epoch: u64) -> Self {
        let notary = Notary::new(lfs.public_key());
        Self {
            pdp,
            lfs,
            notary,
            epoch,
            page: Box::new([0; bridge::PAGE_SIZE]),
            executions: 0,
        }
    }

    /// Full happy path. `execute` sees only the published page.
    pub fn run_task_at(
        &mut self,
        r: &TaskRequest,
        now: SystemTime,
        execute: impl FnOnce(&[u8; bridge::PAGE_SIZE]) -> ExecReply,
    ) -> Result<(Commit, Notarized), E2eError> {
        let admission = match self.pdp.admit_task_at(r, now) {
            TaskOutcome::Admitted(a) => a,
            TaskOutcome::Denied { code, .. } => return Err(E2eError::Denied(code)),
        };
        *self.page = *admission.frame;
        self.executions += 1;
        let reply = execute(&self.page);
        if reply.status != 0 {
            return Err(E2eError::Execution(reply.status));
        }
        if reply.request_id != admission.request_id {
            return Err(E2eError::AckMismatch);
        }
        let commit = Commit {
            epoch: self.epoch,
            request_id: admission.request_id as u64,
            status: 0,
            value: reply.value,
            authority_digest: admission.authority_digest,
        };
        let notarized = self.commit_and_notarize(&commit)?;
        Ok((commit, notarized))
    }

    /// PCB -> LFS commit, then notary verification. Safe to retry (idempotent).
    pub fn commit_and_notarize(&mut self, c: &Commit) -> Result<Notarized, E2eError> {
        let outcome = self.lfs.commit(&self.page, c).map_err(E2eError::Writer)?;
        let replayed = matches!(outcome, CommitOutcome::Replayed(_));
        let receipt = outcome.receipt();
        self.notary.accept(c, &receipt).map_err(E2eError::Notary)?;
        Ok(Notarized { receipt, replayed })
    }

    /// OWN-v3 resolution of an ambiguous outcome: ask the notary, never re-execute.
    pub fn resolve(&self, request_id: u64) -> Option<Receipt> {
        self.notary.get(self.epoch, request_id)
    }
}
