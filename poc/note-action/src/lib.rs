//! Scoped signed capability (NOT Biscuit), Wasm note action and durable receipt.
//! Storage is the repository's LFS ObjectStore, NOT LittleFS.
//! This library does not provision roots, establish sessions or wire Microkit IPC.
#![no_std]

use ed25519_dalek::Signer;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use vela_selection_contract::{UserSelection, verify_selection};
use vela_storage_core::{BlockDevice, MAX_BYTES, ObjectStore};
use wasmi::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

pub const RESOURCE: &[u8] = b"/lfs/notes/note1.txt"; // Logical resource, no POSIX path lookup.
pub const MAX_NOTE: usize = 1024;
pub const CAP_SIZE: usize = 174;
pub const RECEIPT_SIZE: usize = 170;
const CAP_BODY: usize = CAP_SIZE - 64;
const RECEIPT_BODY: usize = RECEIPT_SIZE - 64;
pub const MAX_LEASE_NS: u64 = 120_000_000_000;
const NOTE_WASM: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 12, 2, 96, 2, 127, 127, 1, 127, 96, 1, 127, 1, 127, 2, 13, 1,
    3, 108, 102, 115, 5, 119, 114, 105, 116, 101, 0, 0, 3, 2, 1, 1, 5, 4, 1, 1, 1, 1, 7, 16, 2, 6,
    109, 101, 109, 111, 114, 121, 2, 0, 3, 114, 117, 110, 0, 1, 10, 10, 1, 8, 0, 65, 0, 32, 0, 16,
    0, 11,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Approval,
    Capability,
    Expired,
    Replay,
    Storage,
    Wasm,
    Receipt,
}

pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn object_id() -> [u8; 16] {
    digest(RESOURCE)[..16].try_into().unwrap()
}
fn u64at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn message<const N: usize>(domain: &[u8], body: &[u8], out: &mut [u8; N]) -> usize {
    let n = domain.len() + body.len();
    out[..domain.len()].copy_from_slice(domain);
    out[domain.len()..n].copy_from_slice(body);
    n
}

pub struct Operation {
    pub epoch: u64,
    pub request: u64,
    len: usize,
    note: [u8; MAX_NOTE],
}
impl Operation {
    pub fn new(epoch: u64, request: u64, note: &[u8]) -> Result<Self, Error> {
        if epoch == 0
            || request == 0
            || note.len() > MAX_NOTE
            || core::str::from_utf8(note).is_err()
        {
            return Err(Error::Bounds);
        }
        let mut op = Self {
            epoch,
            request,
            len: note.len(),
            note: [0; MAX_NOTE],
        };
        op.note[..note.len()].copy_from_slice(note);
        Ok(op)
    }
    pub fn note(&self) -> &[u8] {
        &self.note[..self.len]
    }
    pub fn manifest(&self) -> [u8; 98] {
        let mut body = [0; 98];
        body[..2].copy_from_slice(&1u16.to_le_bytes());
        body[2..10].copy_from_slice(&self.epoch.to_le_bytes());
        body[10..18].copy_from_slice(&self.request.to_le_bytes());
        body[18..34].copy_from_slice(&object_id());
        body[34..66].copy_from_slice(&digest(self.note()));
        body[66..98].copy_from_slice(&digest(NOTE_WASM));
        body
    }
}

pub struct Capability(pub [u8; CAP_SIZE]);
impl Capability {
    // Caller must obtain epoch/session from authenticated current guest context,
    // and perform policy checks before issuance. No static session defaults.
    pub fn issue(
        op: &Operation,
        approval: &UserSelection,
        session: &[u8; 32],
        broker: &[u8; 32],
        now_ns: u64,
        lease_ns: u64,
        issuer: &SigningKey,
    ) -> Result<Self, Error> {
        verify_selection(approval, &op.manifest(), session, op.request, broker)
            .map_err(|_| Error::Approval)?;
        if lease_ns == 0 || lease_ns > MAX_LEASE_NS {
            return Err(Error::Bounds);
        }
        let expires = now_ns.checked_add(lease_ns).ok_or(Error::Bounds)?;
        let mut bytes = [0; CAP_SIZE];
        bytes[..2].copy_from_slice(&1u16.to_le_bytes());
        bytes[2..10].copy_from_slice(&op.epoch.to_le_bytes());
        bytes[10..18].copy_from_slice(&op.request.to_le_bytes());
        bytes[18..26].copy_from_slice(&expires.to_le_bytes());
        bytes[26..58].copy_from_slice(&digest(op.note()));
        bytes[58..90].copy_from_slice(&digest(NOTE_WASM));
        bytes[90..106].copy_from_slice(&object_id());
        bytes[106..110].copy_from_slice(&(op.len as u32).to_le_bytes());
        // Domain fixes right("write", RESOURCE); no caller-controlled rights.
        let mut signed = [0; 160];
        let n = message(b"VelaOS.write-note.v1\0", &bytes[..CAP_BODY], &mut signed);
        bytes[CAP_BODY..].copy_from_slice(&issuer.sign(&signed[..n]).to_bytes());
        Ok(Self(bytes))
    }
    pub fn verify(&self, op: &Operation, now_ns: u64, issuer: &VerifyingKey) -> Result<(), Error> {
        self.verify_binding(op, issuer)?;
        let expires = u64at(&self.0, 18);
        if expires <= now_ns || expires - now_ns > MAX_LEASE_NS {
            return Err(Error::Expired);
        }
        Ok(())
    }
    /// Host-side receipt validation authenticates bindings, not guest time.
    pub fn verify_binding(&self, op: &Operation, issuer: &VerifyingKey) -> Result<(), Error> {
        let b = &self.0;
        let mut signed = [0; 160];
        let n = message(b"VelaOS.write-note.v1\0", &b[..CAP_BODY], &mut signed);
        let signature = Signature::from_bytes(b[CAP_BODY..].try_into().unwrap());
        if issuer.is_weak()
            || issuer.verify_strict(&signed[..n], &signature).is_err()
            || b[..2] != 1u16.to_le_bytes()
            || u64at(b, 2) != op.epoch
            || u64at(b, 10) != op.request
            || b[26..58] != digest(op.note())
            || b[58..90] != digest(NOTE_WASM)
            || b[90..106] != object_id()
            || u32::from_le_bytes(b[106..110].try_into().unwrap()) as usize != op.len
        {
            return Err(Error::Capability);
        }
        Ok(())
    }
}

pub struct Receipt(pub [u8; RECEIPT_SIZE]);
impl Receipt {
    pub fn verify(
        &self,
        op: &Operation,
        cap: &Capability,
        lfs: &VerifyingKey,
    ) -> Result<(), Error> {
        self.authenticate(lfs)?;
        let b = &self.0;
        if u64at(b, 2) != op.epoch
            || u64at(b, 10) != op.request
            || u64at(b, 18) == 0
            || b[26..58] != digest(op.note())
            || b[58..90] != digest(&cap.0)
            || b[90..106] != object_id()
        {
            return Err(Error::Receipt);
        }
        Ok(())
    }
    fn authenticate(&self, lfs: &VerifyingKey) -> Result<(), Error> {
        let mut signed = [0; 160];
        let n = message(
            b"VelaOS.durable-note.v1\0",
            &self.0[..RECEIPT_BODY],
            &mut signed,
        );
        if self.0[..2] != 1u16.to_le_bytes()
            || lfs.is_weak()
            || lfs
                .verify_strict(
                    &signed[..n],
                    &Signature::from_bytes(self.0[RECEIPT_BODY..].try_into().unwrap()),
                )
                .is_err()
        {
            return Err(Error::Receipt);
        }
        Ok(())
    }
}

/// Read authenticated saved bytes after restart, without granting new rights.
pub fn recover<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    key: &VerifyingKey,
    out: &mut [u8; MAX_NOTE],
) -> Result<usize, Error> {
    let mut record = [0; MAX_BYTES];
    let n = store
        .read(object_id(), &mut record)
        .map_err(|_| Error::Storage)?;
    if n == 0 {
        return Ok(0);
    }
    if n < 4 + RECEIPT_SIZE || n > 4 + RECEIPT_SIZE + MAX_NOTE || record[..4] != *b"VNOT" {
        return Err(Error::Storage);
    }
    let receipt = Receipt(record[4..4 + RECEIPT_SIZE].try_into().unwrap());
    receipt.authenticate(key)?;
    let note = &record[4 + RECEIPT_SIZE..n];
    if receipt.0[26..58] != digest(note)
        || receipt.0[90..106] != object_id()
        || store.version().map(|v| v.generation) != Some(u64at(&receipt.0, 18))
    {
        return Err(Error::Receipt);
    }
    out[..note.len()].copy_from_slice(note);
    Ok(note.len())
}

fn commit<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    op: &Operation,
    cap: &Capability,
    lfs: &SigningKey,
) -> Result<Receipt, Error> {
    let mut previous = [0; MAX_BYTES];
    let count = store
        .read(object_id(), &mut previous)
        .map_err(|_| Error::Storage)?;
    if count != 0 {
        if count < 4 + RECEIPT_SIZE || previous[..4] != *b"VNOT" {
            return Err(Error::Storage);
        }
        let old = Receipt(previous[4..4 + RECEIPT_SIZE].try_into().unwrap());
        old.authenticate(&lfs.verifying_key())?;
        if old.0[26..58] != digest(&previous[4 + RECEIPT_SIZE..count]) {
            return Err(Error::Storage);
        }
        if op.epoch < u64at(&old.0, 2)
            || (op.epoch == u64at(&old.0, 2) && op.request <= u64at(&old.0, 10))
        {
            return Err(Error::Replay);
        }
    }
    let generation = store.version().map_or(0, |v| v.generation);
    let next = generation.checked_add(1).ok_or(Error::Storage)?;
    let mut receipt = [0; RECEIPT_SIZE];
    receipt[..2].copy_from_slice(&1u16.to_le_bytes());
    receipt[2..10].copy_from_slice(&op.epoch.to_le_bytes());
    receipt[10..18].copy_from_slice(&op.request.to_le_bytes());
    receipt[18..26].copy_from_slice(&next.to_le_bytes());
    receipt[26..58].copy_from_slice(&digest(op.note()));
    receipt[58..90].copy_from_slice(&digest(&cap.0));
    receipt[90..106].copy_from_slice(&object_id());
    let mut signed = [0; 160];
    let n = message(
        b"VelaOS.durable-note.v1\0",
        &receipt[..RECEIPT_BODY],
        &mut signed,
    );
    receipt[RECEIPT_BODY..].copy_from_slice(&lfs.sign(&signed[..n]).to_bytes());
    let mut record = [0; MAX_BYTES];
    record[..4].copy_from_slice(b"VNOT");
    record[4..4 + RECEIPT_SIZE].copy_from_slice(&receipt);
    record[4 + RECEIPT_SIZE..4 + RECEIPT_SIZE + op.len].copy_from_slice(op.note());
    let length = 4 + RECEIPT_SIZE + op.len;
    // ObjectStore saves through BlockDevice::flush; no receipt is returned before
    // stable completion and exact read-back. Atomicity follows that backend only.
    store
        .save(object_id(), generation, &record[..length])
        .map_err(|_| Error::Storage)?;
    let mut read_back = [0; MAX_BYTES];
    let count = store
        .read(object_id(), &mut read_back)
        .map_err(|_| Error::Storage)?;
    if count != length || read_back[..count] != record[..length] {
        return Err(Error::Storage);
    }
    Ok(Receipt(receipt))
}

struct State<'a, D: BlockDevice> {
    store: &'a mut ObjectStore<D>,
    op: &'a Operation,
    cap: &'a Capability,
    key: &'a SigningKey,
    receipt: Option<Receipt>,
    limits: StoreLimits,
    clock: Option<fn() -> u64>,
    started: u64,
}

pub fn execute<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    op: &Operation,
    cap: &Capability,
    now_ns: u64,
    issuer: &VerifyingKey,
    lfs: &SigningKey,
) -> Result<Receipt, Error> {
    execute_inner(store, op, cap, now_ns, None, issuer, lfs)
}
/// Native caller supplies the guest clock; check expiry again at the side effect.
pub fn execute_timed<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    op: &Operation,
    cap: &Capability,
    clock: fn() -> u64,
    issuer: &VerifyingKey,
    lfs: &SigningKey,
) -> Result<Receipt, Error> {
    execute_inner(store, op, cap, clock(), Some(clock), issuer, lfs)
}
fn execute_inner<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    op: &Operation,
    cap: &Capability,
    now_ns: u64,
    clock: Option<fn() -> u64>,
    issuer: &VerifyingKey,
    lfs: &SigningKey,
) -> Result<Receipt, Error> {
    cap.verify(op, now_ns, issuer)?;
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, NOTE_WASM).map_err(|_| Error::Wasm)?;
    let mut wasm = Store::new(
        &engine,
        State {
            store,
            op,
            cap,
            key: lfs,
            receipt: None,
            clock,
            started: now_ns,
            limits: StoreLimitsBuilder::new()
                .memory_size(65536)
                .memories(1)
                .instances(1)
                .tables(0)
                .build(),
        },
    );
    wasm.limiter(|state| &mut state.limits);
    wasm.set_fuel(10_000).map_err(|_| Error::Wasm)?;
    let mut linker = Linker::new(&engine);
    linker
        .func_wrap(
            "lfs",
            "write",
            |mut caller: Caller<'_, State<'_, D>>,
             ptr: i32,
             length: i32|
             -> Result<i32, wasmi::Error> {
                if ptr < 0
                    || length < 0
                    || length as usize > MAX_NOTE
                    || caller.data().receipt.is_some()
                {
                    return Err(wasmi::Error::new("invalid write"));
                }
                let memory = caller
                    .get_export("memory")
                    .and_then(|v| v.into_memory())
                    .ok_or_else(|| wasmi::Error::new("memory missing"))?;
                let mut bytes = [0; MAX_NOTE];
                memory
                    .read(&caller, ptr as usize, &mut bytes[..length as usize])
                    .map_err(|_| wasmi::Error::new("bounds"))?;
                if &bytes[..length as usize] != caller.data().op.note() {
                    return Err(wasmi::Error::new("approved bytes changed"));
                }
                let state = caller.data_mut();
                let at = state.clock.map_or(state.started, |read| read());
                if at < state.started || at >= u64at(&state.cap.0, 18) {
                    return Err(wasmi::Error::new("expired before write"));
                }
                state.receipt = Some(
                    commit(state.store, state.op, state.cap, state.key)
                        .map_err(|_| wasmi::Error::new("LFS commit failed"))?,
                );
                Ok(0)
            },
        )
        .map_err(|_| Error::Wasm)?;
    let instance = linker
        .instantiate_and_start(&mut wasm, &module)
        .map_err(|_| Error::Wasm)?;
    let memory = instance.get_memory(&wasm, "memory").ok_or(Error::Wasm)?;
    memory
        .write(&mut wasm, 0, op.note())
        .map_err(|_| Error::Wasm)?;
    let run = instance
        .get_typed_func::<i32, i32>(&wasm, "run")
        .map_err(|_| Error::Wasm)?;
    if run
        .call(&mut wasm, op.len as i32)
        .map_err(|_| Error::Wasm)?
        != 0
    {
        return Err(Error::Wasm);
    }
    let receipt = wasm.data_mut().receipt.take().ok_or(Error::Receipt)?;
    receipt.verify(op, cap, &lfs.verifying_key())?;
    Ok(receipt)
}
