//! M-LFS-1..3: durable LFS writer store (ADR-LFS-1 Option A: append-only file +
//! fsync) and a persistent boot epoch.
//!
//! * [`LedgerStore`] is the seam shared by the in-memory ledger
//!   ([`MemoryStore`], identical behaviour to the guest `Ledger<N>`) and the
//!   durable host store ([`FileLedger`]).
//! * Record = `VREC` + the 64-byte `VelaOS.LFS.Receipt.v0` body + 64-byte
//!   Ed25519 signature + 8-byte BLAKE3 checksum (140 bytes, fixed size). The
//!   wire receipt is stored verbatim; nothing about the guest wire changes.
//! * Write order: validate -> sign -> `write_all` -> `sync_data` (fsync) ->
//!   **only then** the receipt is returned to the PCB. A crash before the fsync
//!   returns no receipt; at reopen the torn tail is truncated, so the notary
//!   never sees an "OK" receipt without an integral record behind it.
//! * Recovery accepts only a contiguous prefix of records that pass magic,
//!   checksum, signature under the pinned LFS key and `seq = n + 1`. Damage that
//!   is *not* a torn tail (a bad record followed by a good one) refuses to open
//!   (fail-closed) instead of silently dropping receipts.
//! * Idempotency `(epoch, request_id)` survives restarts: a retry after reopen
//!   returns the byte-identical stored receipt (Ed25519 signing is
//!   deterministic and `seq` is persisted), never a re-signed one.
//! * I/O failure poisons the store (every later commit is refused with
//!   `QUOTA_EXCEEDED` 11) until it is reopened and recovered.
//!
//! Host `std` only. The QEMU guest still runs the in-memory `Ledger<8>`.

use crate::bridge;
use crate::ledger::LEDGER_CAPACITY;
use crate::ledger_wire::{
    BODY_LEN, Commit, CommitOutcome, Ledger, LfsStatus, Receipt, ReceiptBody, grant_digest_in,
};
use ed25519_compact::{PublicKey, SecretKey};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Storage backend of the (simulated) LFS writer domain.
pub trait LedgerStore {
    fn get(&self, epoch: u64, request_id: u64) -> Option<Receipt>;
    /// Number of persisted effects.
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// `frame` is a private snapshot of the RO admission page.
    fn commit(
        &mut self,
        key: &SecretKey,
        pcb: PublicKey,
        frame: &[u8; bridge::PAGE_SIZE],
        c: &Commit,
    ) -> Result<CommitOutcome, LfsStatus>;
}

/// The original simulated in-memory ledger (same code as the guest).
#[derive(Default)]
pub struct MemoryStore(Ledger<LEDGER_CAPACITY>);

impl MemoryStore {
    pub fn new() -> Self {
        Self(Ledger::new())
    }
}

impl LedgerStore for MemoryStore {
    fn get(&self, epoch: u64, request_id: u64) -> Option<Receipt> {
        self.0.get(epoch, request_id)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn commit(
        &mut self,
        key: &SecretKey,
        pcb: PublicKey,
        frame: &[u8; bridge::PAGE_SIZE],
        c: &Commit,
    ) -> Result<CommitOutcome, LfsStatus> {
        self.0.commit(key, pcb, frame, c)
    }
}

/// C4.4 admission check, identical to `Ledger::commit` in `ledger_wire.rs`.
fn authorize(
    pcb: PublicKey,
    frame: &[u8; bridge::PAGE_SIZE],
    c: &Commit,
) -> Result<(), LfsStatus> {
    if c.status != 0 || c.request_id == 0 || c.request_id > u32::MAX as u64 {
        return Err(LfsStatus::Malformed);
    }
    let v = bridge::verify_with_key(frame, 0, pcb).map_err(|_| LfsStatus::PolicyDenied)?;
    let d = v.descriptor;
    if d.request_id as u64 != c.request_id {
        return Err(LfsStatus::PolicyDenied);
    }
    let payload = &frame[d.payload_offset..d.payload_offset + d.payload_len];
    if grant_digest_in(payload) != Some(c.authority_digest) {
        return Err(LfsStatus::PolicyDenied);
    }
    Ok(())
}

pub const LEDGER_FILE: &str = "ledger.vlfs";
pub const EPOCH_FILE: &str = "epoch.bin";
const FILE_MAGIC: &[u8; 8] = b"VLFSLOG1";
const FORMAT_VERSION: u32 = 1;
const SUM_LEN: usize = 8;
/// magic + version + pinned LFS public key + checksum.
pub const HEADER_LEN: usize = 8 + 4 + 32 + SUM_LEN;
const REC_MAGIC: &[u8; 4] = b"VREC";
/// magic + receipt body + signature + checksum.
pub const RECORD_LEN: usize = 4 + BODY_LEN + 64 + SUM_LEN;

fn checksum(domain: &[u8], bytes: &[u8]) -> [u8; SUM_LEN] {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(bytes);
    let mut out = [0; SUM_LEN];
    out.copy_from_slice(&h.finalize().as_bytes()[..SUM_LEN]);
    out
}
const HEADER_DOMAIN: &[u8] = b"VelaOS.LFS.Header.v1\0";
const RECORD_DOMAIN: &[u8] = b"VelaOS.LFS.Record.v1\0";
const EPOCH_DOMAIN: &[u8] = b"VelaOS.LFS.Epoch.v1\0";

fn encode_header(lfs: &PublicKey) -> [u8; HEADER_LEN] {
    let mut h = [0; HEADER_LEN];
    h[..8].copy_from_slice(FILE_MAGIC);
    h[8..12].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    h[12..44].copy_from_slice(&lfs[..]);
    let sum = checksum(HEADER_DOMAIN, &h[..44]);
    h[44..].copy_from_slice(&sum);
    h
}

pub fn encode_record(r: &Receipt) -> [u8; RECORD_LEN] {
    let b = &r.body;
    let mut out = [0; RECORD_LEN];
    let mut at = 0;
    for part in [
        &REC_MAGIC[..],
        &b.epoch.to_le_bytes(),
        &b.request_id.to_le_bytes(),
        &b.status.to_le_bytes(),
        &b.value.to_le_bytes(),
        &b.authority_digest,
        &b.seq.to_le_bytes(),
        &r.signature,
    ] {
        out[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    }
    let sum = checksum(RECORD_DOMAIN, &out[..at]);
    out[at..].copy_from_slice(&sum);
    out
}

/// Structural decode (magic + checksum). Signature/sequence are checked by the caller.
pub fn decode_record(rec: &[u8]) -> Option<Receipt> {
    if rec.len() != RECORD_LEN || rec[..4] != *REC_MAGIC {
        return None;
    }
    let end = RECORD_LEN - SUM_LEN;
    if checksum(RECORD_DOMAIN, &rec[..end]) != rec[end..] {
        return None;
    }
    let u64_at = |i: usize| u64::from_le_bytes(rec[i..i + 8].try_into().unwrap());
    let u32_at = |i: usize| u32::from_le_bytes(rec[i..i + 4].try_into().unwrap());
    let mut authority_digest = [0; 32];
    authority_digest.copy_from_slice(&rec[28..60]);
    let mut signature = [0; 64];
    signature.copy_from_slice(&rec[68..132]);
    Some(Receipt {
        body: ReceiptBody {
            epoch: u64_at(4),
            request_id: u64_at(12),
            status: u32_at(20),
            value: u32_at(24),
            authority_digest,
            seq: u64_at(60),
        },
        signature,
    })
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}
#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    // Windows: directory handles cannot be fsynced through std; NTFS journals
    // the metadata of the create/rename. Documented in ADR-LFS-1.
    Ok(())
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

/// What recovery found at open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Recovery {
    /// Integral records accepted.
    pub records: usize,
    /// Bytes of torn tail discarded (0 on a clean file).
    pub truncated_bytes: u64,
}

/// Fault-injection phases for the crash gate (feature `fault-fixtures`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashPhase {
    /// Abort after signing, before any byte is written.
    BeforeWrite,
    /// Abort after writing half of the record (torn write).
    MidWrite,
    /// Abort after the full record is written, before fsync.
    BeforeSync,
    /// Abort after fsync, before the receipt is returned.
    AfterSync,
}

/// Durable append-only LFS ledger (Option A).
pub struct FileLedger {
    file: File,
    path: PathBuf,
    lfs: PublicKey,
    entries: Vec<Receipt>,
    index: HashMap<(u64, u64), usize>,
    capacity: usize,
    poisoned: bool,
    recovery: Recovery,
    crash: Option<(CrashPhase, usize)>,
}

impl FileLedger {
    /// Open (or create) `dir/ledger.vlfs` pinned to the LFS public key `lfs`,
    /// recovering the longest integral prefix.
    pub fn open(dir: &Path, lfs: PublicKey, capacity: usize) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join(LEDGER_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        if bytes.len() < HEADER_LEN {
            // Torn or missing header can only happen at creation, before any
            // receipt existed: start a fresh log.
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&encode_header(&lfs))?;
            file.sync_all()?;
            sync_dir(dir)?;
            bytes = encode_header(&lfs).to_vec();
        }
        if bytes[..HEADER_LEN] != encode_header(&lfs) {
            return Err(invalid(
                "ledger header invalid or pinned to another LFS key (fail-closed)",
            ));
        }
        let body = &bytes[HEADER_LEN..];
        let mut entries: Vec<Receipt> = Vec::new();
        let mut index = HashMap::new();
        let mut good = 0;
        for chunk in body.chunks(RECORD_LEN) {
            let ok = decode_record(chunk).filter(|r| {
                r.verify(&lfs)
                    && r.body.seq == entries.len() as u64 + 1
                    && !index.contains_key(&(r.body.epoch, r.body.request_id))
            });
            let Some(r) = ok else { break };
            index.insert((r.body.epoch, r.body.request_id), entries.len());
            entries.push(r);
            good += 1;
        }
        let good_end = HEADER_LEN + good * RECORD_LEN;
        // Anything after the first bad record must be a torn tail: if a later
        // full record still passes its checksum, this is media corruption.
        let tail = &bytes[good_end..];
        if tail.len() > RECORD_LEN
            && tail[RECORD_LEN..]
                .chunks(RECORD_LEN)
                .any(|c| decode_record(c).is_some())
        {
            return Err(invalid(
                "ledger corrupted before its tail; refusing to drop receipts (fail-closed)",
            ));
        }
        let truncated_bytes = (bytes.len() - good_end) as u64;
        if truncated_bytes > 0 {
            file.set_len(good_end as u64)?;
            file.sync_all()?;
        }
        if entries.len() > capacity {
            return Err(invalid("ledger holds more records than its capacity"));
        }
        file.seek(SeekFrom::End(0))?;
        Ok(Self {
            file,
            path,
            lfs,
            entries,
            index,
            capacity,
            poisoned: false,
            recovery: Recovery {
                records: good,
                truncated_bytes,
            },
            crash: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn recovery(&self) -> Recovery {
        self.recovery
    }
    pub fn receipts(&self) -> &[Receipt] {
        &self.entries
    }
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    pub fn lfs_key(&self) -> PublicKey {
        self.lfs
    }

    /// Fault injection: abort the process at `phase` of the `nth` (1-based)
    /// new commit of this session. Test/crash-gate only.
    #[cfg(feature = "fault-fixtures")]
    pub fn crash_at(&mut self, phase: CrashPhase, nth: usize) {
        self.crash = Some((phase, nth));
    }

    fn probe(&self, phase: CrashPhase, nth: usize) {
        if self.crash == Some((phase, nth)) {
            std::process::abort();
        }
    }

    fn append(&mut self, r: &Receipt, nth: usize) -> io::Result<()> {
        let rec = encode_record(r);
        self.probe(CrashPhase::BeforeWrite, nth);
        if self.crash == Some((CrashPhase::MidWrite, nth)) {
            self.file.write_all(&rec[..RECORD_LEN / 2])?;
            self.file.sync_data()?;
            std::process::abort();
        }
        self.file.write_all(&rec)?;
        self.probe(CrashPhase::BeforeSync, nth);
        self.file.sync_data()?;
        self.probe(CrashPhase::AfterSync, nth);
        Ok(())
    }
}

impl LedgerStore for FileLedger {
    fn get(&self, epoch: u64, request_id: u64) -> Option<Receipt> {
        self.index
            .get(&(epoch, request_id))
            .map(|&i| self.entries[i])
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn commit(
        &mut self,
        key: &SecretKey,
        pcb: PublicKey,
        frame: &[u8; bridge::PAGE_SIZE],
        c: &Commit,
    ) -> Result<CommitOutcome, LfsStatus> {
        if self.poisoned {
            return Err(LfsStatus::QuotaExceeded);
        }
        authorize(pcb, frame, c)?;
        if let Some(existing) = self.get(c.epoch, c.request_id) {
            return if existing.body.matches(c) {
                Ok(CommitOutcome::Replayed(existing))
            } else {
                Err(LfsStatus::IdempotencyConflict)
            };
        }
        if self.entries.len() >= self.capacity {
            return Err(LfsStatus::QuotaExceeded);
        }
        if key.public_key() != self.lfs {
            return Err(LfsStatus::PolicyDenied);
        }
        let seq = self.entries.len() as u64 + 1;
        let receipt = Receipt::sign(key, ReceiptBody::of(c, seq));
        let nth = self.entries.len() + 1 - self.recovery.records;
        if self.append(&receipt, nth).is_err() {
            // Partial bytes may be on disk: refuse everything until reopen+recover.
            self.poisoned = true;
            return Err(LfsStatus::QuotaExceeded);
        }
        self.index
            .insert((c.epoch, c.request_id), self.entries.len());
        self.entries.push(receipt);
        Ok(CommitOutcome::Committed(receipt))
    }
}

/// How the previous session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boot {
    /// No epoch file: first boot, epoch 1.
    Fresh,
    /// Previous session closed cleanly.
    Clean,
    /// Previous session did not close (crash/power loss): its in-flight
    /// requests are AMBIGUOUS until resolved against the ledger/notary.
    Recovered,
}

/// Persistent boot epoch (ADR-LFS-1 §3: **every** boot gets a new epoch, so
/// in-memory request-id counters restarting at 1 can never collide with a
/// previous session's `(epoch, request_id)`; grants of old epochs are dead).
#[derive(Debug)]
pub struct EpochStore {
    dir: PathBuf,
    epoch: u64,
    boot: Boot,
    previous: Option<u64>,
}

const EPOCH_MAGIC: &[u8; 8] = b"VEPOCH01";
const EPOCH_LEN: usize = 8 + 8 + 1 + SUM_LEN;

fn encode_epoch(epoch: u64, clean: bool) -> [u8; EPOCH_LEN] {
    let mut b = [0; EPOCH_LEN];
    b[..8].copy_from_slice(EPOCH_MAGIC);
    b[8..16].copy_from_slice(&epoch.to_le_bytes());
    b[16] = clean as u8;
    let sum = checksum(EPOCH_DOMAIN, &b[..17]);
    b[17..].copy_from_slice(&sum);
    b
}

impl EpochStore {
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join(EPOCH_FILE);
        let (epoch, boot, previous) = match fs::read(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => (1, Boot::Fresh, None),
            Err(e) => return Err(e),
            Ok(b) => {
                if b.len() != EPOCH_LEN
                    || b[..8] != *EPOCH_MAGIC
                    || checksum(EPOCH_DOMAIN, &b[..17]) != b[17..]
                    || b[16] > 1
                {
                    return Err(invalid("epoch file corrupted (fail-closed)"));
                }
                let prev = u64::from_le_bytes(b[8..16].try_into().unwrap());
                let next = prev
                    .checked_add(1)
                    .ok_or_else(|| invalid("epoch exhausted"))?;
                let boot = if b[16] == 1 { Boot::Clean } else { Boot::Recovered };
                (next, boot, Some(prev))
            }
        };
        let store = Self {
            dir: dir.to_path_buf(),
            epoch,
            boot,
            previous,
        };
        store.write(false)?;
        Ok(store)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn boot(&self) -> Boot {
        self.boot
    }
    /// Epoch of the previous session, if any.
    pub fn previous(&self) -> Option<u64> {
        self.previous
    }

    /// Mark a clean shutdown (next boot reports [`Boot::Clean`]).
    pub fn close_clean(self) -> io::Result<()> {
        self.write(true)
    }

    fn write(&self, clean: bool) -> io::Result<()> {
        let tmp = self.dir.join("epoch.bin.tmp");
        {
            let mut f = File::create(&tmp)?;
            f.write_all(&encode_epoch(self.epoch, clean))?;
            f.sync_all()?;
        }
        fs::rename(&tmp, self.dir.join(EPOCH_FILE))?;
        sync_dir(&self.dir)
    }
}
