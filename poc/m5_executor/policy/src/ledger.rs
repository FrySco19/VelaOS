//! M-G1c host orchestration of the State Ledger (LFS writer).
//!
//! Admission ([`SelectionGatedPdp`]) -> Executor (caller-supplied closure: the
//! QEMU guest, `vela-executor-host`, or a test double) -> LFS writer
//! ([`SimulatedLfs`] over a [`LedgerStore`]) -> synchronous receipt -> PCB
//! [`Notary`] verification and storage.
//!
//! Stores: [`MemoryStore`] (the *same* `no_std` `Ledger` the guest `velaos-lfs`
//! runs; default) or [`FileLedger`] (M-LFS-2/3, durable append-only + fsync,
//! host only). Time: [`E2eHost::run_task`] enforces the grant TTL on a
//! monotonic [`GrantDeadline`] before execution **and** before commit
//! (M-CLK-1); the legacy [`E2eHost::run_task_at`] keeps the explicit-`now`
//! behaviour for reproducible bundles. The QEMU guest itself still has no
//! clock: TTL is enforced host-side only (see `docs/ADR_LFS_CLOCK.md`).

pub use crate::durable::{Boot, EpochStore, FileLedger, LedgerStore, MemoryStore, Recovery};
pub use crate::ledger_wire::{
    Commit, CommitOutcome, Ledger, LfsStatus, Notary, NotaryError, Receipt, ReceiptBody,
};
use crate::clock::{Clock, GrantDeadline, SystemClock, TrustedTime};
use crate::pipeline::{SelectionGatedPdp, TaskOutcome, TaskRequest};
use crate::{DenyCode, bridge};
use ed25519_compact::{PublicKey, SecretKey};
use std::io;
use std::path::Path;
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
    /// PDP denied (or grant already dead / time untrusted before execution):
    /// nothing signed or nothing executed.
    Denied(DenyCode),
    /// Executor returned a non-success status: nothing committed.
    Execution(u64),
    /// Executor acknowledged another request: nothing committed.
    AckMismatch,
    /// Grant expired (or time became untrusted) between execution and commit:
    /// the result is discarded, nothing committed. Carries `EXPIRED`.
    GrantExpired(DenyCode),
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

/// LFS writer domain: holds its own signing key; the PCB holds only the public half.
pub struct SimulatedLfs<S: LedgerStore = MemoryStore> {
    key: SecretKey,
    pcb: PublicKey,
    store: S,
}

impl SimulatedLfs<MemoryStore> {
    /// In-memory ledger (simulated; lost on restart).
    pub fn new(key: SecretKey, pcb: PublicKey) -> Self {
        Self::with_store(key, pcb, MemoryStore::new())
    }
}

impl SimulatedLfs<FileLedger> {
    /// Durable append-only ledger under `dir` (recovered at open).
    pub fn durable(
        key: SecretKey,
        pcb: PublicKey,
        dir: &Path,
        capacity: usize,
    ) -> io::Result<Self> {
        let store = FileLedger::open(dir, key.public_key(), capacity)?;
        Ok(Self::with_store(key, pcb, store))
    }
}

impl<S: LedgerStore> SimulatedLfs<S> {
    pub fn with_store(key: SecretKey, pcb: PublicKey, store: S) -> Self {
        Self { key, pcb, store }
    }
    pub fn public_key(&self) -> PublicKey {
        self.key.public_key()
    }
    pub fn persisted(&self) -> usize {
        self.store.len()
    }
    pub fn get(&self, epoch: u64, request_id: u64) -> Option<Receipt> {
        self.store.get(epoch, request_id)
    }
    pub fn store(&self) -> &S {
        &self.store
    }
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }
    /// Synchronous commit against the frame currently on the RO page.
    pub fn commit(
        &mut self,
        page: &[u8; bridge::PAGE_SIZE],
        c: &Commit,
    ) -> Result<CommitOutcome, LfsStatus> {
        // SAFETY: `page` is a valid 4 KiB buffer; snapshot like the guest does.
        let snapshot = unsafe { bridge::snapshot(page.as_ptr()) };
        self.store.commit(&self.key, self.pcb, &snapshot, c)
    }
}

/// Host model of the whole pipeline with the PCB as notary.
pub struct E2eHost<S: LedgerStore = MemoryStore> {
    pub pdp: SelectionGatedPdp,
    pub lfs: SimulatedLfs<S>,
    pub notary: Notary<LEDGER_CAPACITY>,
    pub epoch: u64,
    /// Last published page (PCB-writable / LFS+Executor-readable).
    pub page: Box<[u8; bridge::PAGE_SIZE]>,
    pub executions: u64,
    /// LFS public key pinned by the PCB at construction.
    lfs_pin: PublicKey,
    time: TrustedTime<Box<dyn Clock>>,
}

impl<S: LedgerStore> E2eHost<S> {
    /// Uses the real [`SystemClock`] for [`Self::run_task`].
    pub fn new(pdp: SelectionGatedPdp, lfs: SimulatedLfs<S>, epoch: u64) -> Self {
        Self::with_clock(pdp, lfs, epoch, Box::new(SystemClock::new()))
    }

    pub fn with_clock(
        pdp: SelectionGatedPdp,
        lfs: SimulatedLfs<S>,
        epoch: u64,
        clock: Box<dyn Clock>,
    ) -> Self {
        let lfs_pin = lfs.public_key();
        let notary = Notary::new(lfs_pin);
        Self {
            lfs_pin,
            pdp,
            lfs,
            notary,
            epoch,
            page: Box::new([0; bridge::PAGE_SIZE]),
            executions: 0,
            time: TrustedTime::new(clock),
        }
    }

    /// Full happy path with an explicit wall `now` (no monotonic deadline).
    /// Kept for reproducible bundles/tests; prefer [`Self::run_task`].
    pub fn run_task_at(
        &mut self,
        r: &TaskRequest,
        now: SystemTime,
        execute: impl FnOnce(&[u8; bridge::PAGE_SIZE]) -> ExecReply,
    ) -> Result<(Commit, Notarized), E2eError> {
        self.run(r, now, false, execute)
    }

    /// Full path on the host's trusted clock (M-CLK-1): the grant is minted
    /// at trusted wall time, then its TTL is anchored on the monotonic clock
    /// and re-checked before the Executor runs and before the LFS commit.
    /// Expired / untrusted time -> `EXPIRED`, no commit.
    pub fn run_task(
        &mut self,
        r: &TaskRequest,
        execute: impl FnOnce(&[u8; bridge::PAGE_SIZE]) -> ExecReply,
    ) -> Result<(Commit, Notarized), E2eError> {
        let wall = self
            .time
            .wall()
            .map_err(|e| E2eError::Denied(e.deny_code()))?;
        self.run(r, wall, true, execute)
    }

    fn run(
        &mut self,
        r: &TaskRequest,
        now: SystemTime,
        monotonic: bool,
        execute: impl FnOnce(&[u8; bridge::PAGE_SIZE]) -> ExecReply,
    ) -> Result<(Commit, Notarized), E2eError> {
        let admission = match self.pdp.admit_task_at(r, now) {
            TaskOutcome::Admitted(a) => a,
            TaskOutcome::Denied { code, .. } => return Err(E2eError::Denied(code)),
        };
        let deadline = if monotonic {
            let d = GrantDeadline::start(&self.time, now, admission.not_after)
                .map_err(|e| E2eError::Denied(e.deny_code()))?;
            d.check(&self.time).map_err(E2eError::Denied)?;
            Some(d)
        } else {
            None
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
        if let Some(d) = deadline {
            d.check(&self.time).map_err(E2eError::GrantExpired)?;
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
        self.resolve_in(self.epoch, request_id)
    }

    /// Resolution in any epoch (e.g. a previous boot after [`Boot::Recovered`]).
    /// Notary first; otherwise a durable-ledger receipt is returned only if it
    /// verifies under the pinned LFS key (the notary's own check).
    pub fn resolve_in(&self, epoch: u64, request_id: u64) -> Option<Receipt> {
        self.notary.get(epoch, request_id).or_else(|| {
            self.lfs
                .get(epoch, request_id)
                .filter(|r| r.verify(&self.lfs_pin))
        })
    }
}
