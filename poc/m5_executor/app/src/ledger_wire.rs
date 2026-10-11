//! LFS receipt ledger wire + logic (M-G1c, "State Ledger"), `no_std`.
//!
//! Shared verbatim by the Microkit guest (`velaos-lfs` writer, `velaos-pcb`
//! notary, `velaos-executor` grant echo) and by the host crates via `#[path]`,
//! exactly like `bridge.rs`. No allocation, no clock, no filesystem.
//!
//! * The authority digest (BLAKE3-256 of the serialized, verified Biscuit grant,
//!   CONTRACTS_v0 C1 `authority_digest`) is carried *inside the signed Wasm
//!   payload* as one custom section named `vela.grant`. The wire v2 Ed25519
//!   admission signature covers the payload, so the digest is authenticated
//!   without changing the frame format.
//! * The writer persists a result only for a request whose admission frame is
//!   currently published on the RO page, verifies under the pinned PCB key and
//!   carries the same authority digest (C4.4: unauthorized output is not
//!   persisted).
//! * Idempotency key: `(epoch, request_id)` (C4). Same key + same content returns
//!   the stored receipt (no new effect); same key + different content is
//!   `IDEMPOTENCY_CONFLICT` (13).
//! * The PCB [`Notary`] verifies every receipt signature under the pinned LFS key
//!   and that the body matches what it asked to commit, then stores it.
#![allow(dead_code)]

use crate::bridge;
use ed25519_compact::{PublicKey, SecretKey, Signature};

/// PCB -> LFS protected-call label (8 message registers).
pub const COMMIT_LABEL: u64 = 0x711;
/// LFS -> PCB reply label (11 message registers).
pub const RECEIPT_LABEL: u64 = 0x712;
pub const COMMIT_REGS: usize = 8;
pub const RECEIPT_REGS: usize = 11;
/// Wasm custom section carrying the 32-byte authority digest.
pub const GRANT_SECTION: &[u8] = b"vela.grant";
const DOMAIN: &[u8] = b"VelaOS.LFS.Receipt.v0\0";
pub const BODY_LEN: usize = 8 + 8 + 4 + 4 + 32 + 8;
pub const MSG_LEN: usize = DOMAIN.len() + BODY_LEN;

/// Writer outcome codes (CONTRACTS_v0 C5 numbering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum LfsStatus {
    Committed = 0,
    /// Commit request shape invalid, or the execution did not succeed.
    Malformed = 1,
    /// No matching authenticated admission for this output: not persisted.
    PolicyDenied = 10,
    /// Ledger capacity exhausted.
    QuotaExceeded = 11,
    /// Same (epoch, request_id) with different content.
    IdempotencyConflict = 13,
}

impl LfsStatus {
    pub fn from_u64(v: u64) -> Option<Self> {
        Some(match v {
            0 => Self::Committed,
            1 => Self::Malformed,
            10 => Self::PolicyDenied,
            11 => Self::QuotaExceeded,
            13 => Self::IdempotencyConflict,
            _ => return None,
        })
    }
}

/// What the PCB asks the writer to persist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commit {
    pub epoch: u64,
    pub request_id: u64,
    /// Executor status (only 0 = success is committable).
    pub status: u32,
    /// Bounded result (`i32` bits) returned by the Executor.
    pub value: u32,
    pub authority_digest: [u8; 32],
}

impl Commit {
    pub fn to_regs(&self) -> [u64; COMMIT_REGS] {
        let d = &self.authority_digest;
        let w = |i: usize| u64::from_le_bytes(d[i * 8..i * 8 + 8].try_into().unwrap());
        [
            self.epoch,
            self.request_id,
            self.status as u64,
            self.value as u64,
            w(0),
            w(1),
            w(2),
            w(3),
        ]
    }
    pub fn from_regs(r: &[u64; COMMIT_REGS]) -> Option<Self> {
        if r[2] > u32::MAX as u64 || r[3] > u32::MAX as u64 {
            return None;
        }
        let mut d = [0; 32];
        for i in 0..4 {
            d[i * 8..i * 8 + 8].copy_from_slice(&r[4 + i].to_le_bytes());
        }
        Some(Self {
            epoch: r[0],
            request_id: r[1],
            status: r[2] as u32,
            value: r[3] as u32,
            authority_digest: d,
        })
    }
}

/// Signed receipt body (`VelaOS.LFS.Receipt.v0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptBody {
    pub epoch: u64,
    pub request_id: u64,
    pub status: u32,
    pub value: u32,
    pub authority_digest: [u8; 32],
    /// Ledger sequence number assigned at first commit.
    pub seq: u64,
}

impl ReceiptBody {
    pub fn of(c: &Commit, seq: u64) -> Self {
        Self {
            epoch: c.epoch,
            request_id: c.request_id,
            status: c.status,
            value: c.value,
            authority_digest: c.authority_digest,
            seq,
        }
    }
    pub fn matches(&self, c: &Commit) -> bool {
        self.epoch == c.epoch
            && self.request_id == c.request_id
            && self.status == c.status
            && self.value == c.value
            && self.authority_digest == c.authority_digest
    }
    pub fn message(&self) -> [u8; MSG_LEN] {
        let mut m = [0; MSG_LEN];
        let mut at = 0;
        for part in [
            DOMAIN,
            &self.epoch.to_le_bytes(),
            &self.request_id.to_le_bytes(),
            &self.status.to_le_bytes(),
            &self.value.to_le_bytes(),
            &self.authority_digest,
            &self.seq.to_le_bytes(),
        ] {
            m[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        m
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub body: ReceiptBody,
    pub signature: [u8; 64],
}

impl Receipt {
    pub fn sign(key: &SecretKey, body: ReceiptBody) -> Self {
        let sig = key.sign(body.message(), None);
        let mut signature = [0; 64];
        signature.copy_from_slice(sig.as_ref());
        Self { body, signature }
    }
    pub fn verify(&self, lfs: &PublicKey) -> bool {
        lfs.verify(self.body.message(), &Signature::new(self.signature))
            .is_ok()
    }
    /// Reply registers: [status, replayed, seq, sig0..sig7].
    pub fn to_reply(&self, replayed: bool) -> [u64; RECEIPT_REGS] {
        let mut r = [0; RECEIPT_REGS];
        r[0] = LfsStatus::Committed as u64;
        r[1] = replayed as u64;
        r[2] = self.body.seq;
        for i in 0..8 {
            r[3 + i] = u64::from_le_bytes(self.signature[i * 8..i * 8 + 8].try_into().unwrap());
        }
        r
    }
    /// Rebuild a receipt from reply registers and the commit the PCB sent.
    pub fn from_reply(c: &Commit, r: &[u64; RECEIPT_REGS]) -> Self {
        let mut signature = [0; 64];
        for i in 0..8 {
            signature[i * 8..i * 8 + 8].copy_from_slice(&r[3 + i].to_le_bytes());
        }
        Self {
            body: ReceiptBody::of(c, r[2]),
            signature,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitOutcome {
    /// First commit for this key: one new persisted effect.
    Committed(Receipt),
    /// Key already committed with identical content: stored receipt, no effect.
    Replayed(Receipt),
}

impl CommitOutcome {
    pub fn receipt(&self) -> Receipt {
        match self {
            Self::Committed(r) | Self::Replayed(r) => *r,
        }
    }
}

/// Fixed-capacity, append-only, in-memory ledger (simulated LFS writer state).
pub struct Ledger<const N: usize> {
    entries: [Option<Receipt>; N],
    seq: u64,
}

impl<const N: usize> Default for Ledger<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Ledger<N> {
    pub const fn new() -> Self {
        Self {
            entries: [None; N],
            seq: 0,
        }
    }
    /// Number of persisted effects.
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, epoch: u64, request_id: u64) -> Option<Receipt> {
        self.entries
            .iter()
            .flatten()
            .find(|e| e.body.epoch == epoch && e.body.request_id == request_id)
            .copied()
    }

    /// `frame` is a private snapshot of the RO admission page.
    pub fn commit(
        &mut self,
        key: &SecretKey,
        pcb: PublicKey,
        frame: &[u8; bridge::PAGE_SIZE],
        c: &Commit,
    ) -> Result<CommitOutcome, LfsStatus> {
        if c.status != 0 || c.request_id == 0 || c.request_id > u32::MAX as u64 {
            return Err(LfsStatus::Malformed);
        }
        // C4.4: output is persisted only for an authenticated admission of this
        // exact request carrying this exact authority digest.
        let v = bridge::verify_with_key(frame, 0, pcb).map_err(|_| LfsStatus::PolicyDenied)?;
        let d = v.descriptor;
        if d.request_id as u64 != c.request_id {
            return Err(LfsStatus::PolicyDenied);
        }
        let payload = &frame[d.payload_offset..d.payload_offset + d.payload_len];
        if grant_digest_in(payload) != Some(c.authority_digest) {
            return Err(LfsStatus::PolicyDenied);
        }
        if let Some(existing) = self.get(c.epoch, c.request_id) {
            return if existing.body.matches(c) {
                Ok(CommitOutcome::Replayed(existing))
            } else {
                Err(LfsStatus::IdempotencyConflict)
            };
        }
        let slot = self
            .entries
            .iter_mut()
            .find(|e| e.is_none())
            .ok_or(LfsStatus::QuotaExceeded)?;
        self.seq += 1;
        let receipt = Receipt::sign(key, ReceiptBody::of(c, self.seq));
        *slot = Some(receipt);
        Ok(CommitOutcome::Committed(receipt))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotaryError {
    /// Signature does not verify under the pinned LFS key.
    Forged,
    /// Signed body is not what the PCB asked to commit.
    Mismatch,
    /// A different receipt is already notarized for this key.
    Conflict,
    Full,
}

/// PCB-side notary: verifies and stores receipts (append-only, in memory).
pub struct Notary<const N: usize> {
    lfs: PublicKey,
    stored: [Option<Receipt>; N],
}

impl<const N: usize> Notary<N> {
    pub fn new(lfs: PublicKey) -> Self {
        Self {
            lfs,
            stored: [None; N],
        }
    }
    pub fn len(&self) -> usize {
        self.stored.iter().filter(|e| e.is_some()).count()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, epoch: u64, request_id: u64) -> Option<Receipt> {
        self.stored
            .iter()
            .flatten()
            .find(|e| e.body.epoch == epoch && e.body.request_id == request_id)
            .copied()
    }
    /// Ok(true): newly notarized; Ok(false): identical receipt already stored.
    pub fn accept(&mut self, expected: &Commit, r: &Receipt) -> Result<bool, NotaryError> {
        if !r.verify(&self.lfs) {
            return Err(NotaryError::Forged);
        }
        if !r.body.matches(expected) {
            return Err(NotaryError::Mismatch);
        }
        if let Some(existing) = self.get(expected.epoch, expected.request_id) {
            return if existing == *r {
                Ok(false)
            } else {
                Err(NotaryError::Conflict)
            };
        }
        let slot = self
            .stored
            .iter_mut()
            .find(|e| e.is_none())
            .ok_or(NotaryError::Full)?;
        *slot = Some(*r);
        Ok(true)
    }
}

fn leb_u32(b: &[u8]) -> Option<(u32, usize)> {
    let mut r: u32 = 0;
    for (k, byte) in b.iter().enumerate().take(5) {
        r |= ((byte & 0x7f) as u32) << (7 * k);
        if byte & 0x80 == 0 {
            return Some((r, k + 1));
        }
    }
    None
}

/// Extract the authority digest from exactly one `vela.grant` custom section.
/// Missing, duplicated, truncated or wrongly sized sections yield `None`.
pub fn grant_digest_in(m: &[u8]) -> Option<[u8; 32]> {
    if m.len() < 8 || m[..8] != *b"\0asm\x01\0\0\0" {
        return None;
    }
    let mut i = 8;
    let mut found = None;
    while i < m.len() {
        let id = m[i];
        i += 1;
        let (size, n) = leb_u32(&m[i..])?;
        i += n;
        let end = i.checked_add(size as usize).filter(|e| *e <= m.len())?;
        if id == 0 {
            let (name_len, n2) = leb_u32(&m[i..end])?;
            let ns = i + n2;
            let ne = ns.checked_add(name_len as usize).filter(|e| *e <= end)?;
            if &m[ns..ne] == GRANT_SECTION {
                if found.is_some() || end - ne != 32 {
                    return None;
                }
                let mut d = [0; 32];
                d.copy_from_slice(&m[ne..end]);
                found = Some(d);
            }
        }
        i = end;
    }
    found
}
