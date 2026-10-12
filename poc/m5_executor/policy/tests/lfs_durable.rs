//! M-LFS-2: durable append-only + fsync LFS ledger (host). Same receipts and
//! C4/C5 semantics as the in-memory ledger, plus restart and torn-write recovery.

mod common;

use common::*;
use ed25519_compact::PublicKey;
use std::fs;
use std::path::{Path, PathBuf};
use vela_executor_policy::durable::{
    FileLedger, HEADER_LEN, LEDGER_FILE, LedgerStore, MemoryStore, RECORD_LEN,
};
use vela_executor_policy::ledger::{
    Commit, E2eError, E2eHost, ExecReply, LfsStatus, SimulatedLfs,
};
use vela_executor_policy::{DenyCode, bridge};

const EPOCH: u64 = 7;

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lfs_durable").join(name);
    let _ = fs::remove_dir_all(&d);
    d
}

fn durable_host(f: &Fixture, d: &Path, cap: usize) -> E2eHost<FileLedger> {
    let lfs = SimulatedLfs::durable(lfs_keys().sk, PublicKey::new(*pcb().pk), d, cap).unwrap();
    E2eHost::new(gated(&f.root), lfs, EPOCH)
}

fn file_len(d: &Path) -> u64 {
    fs::metadata(d.join(LEDGER_FILE)).unwrap().len()
}

fn wasm(page: &[u8; bridge::PAGE_SIZE]) -> ExecReply {
    let mut exec = vela_executor_host::HostExecutor::new(PublicKey::new(*pcb().pk));
    let r = exec.call(page);
    ExecReply { status: r.status as u64, value: r.value, request_id: r.request_id, attempts: r.attempts }
}

fn selected_task<'a>(f: &'a mut Fixture, id: u32) -> vela_executor_policy::pipeline::TaskRequest<'a> {
    let sel = f.channel.select(NOTES, 3);
    let mut t = task(&f.candidates, NOTES, id);
    t.selection = Some(sel);
    t
}

#[test]
fn lfs_durable_commit_survives_restart_with_byte_identical_receipt() {
    let d = dir("restart");
    let mut f = fixture();
    let now = f.now;
    let (commit, first, page) = {
        let mut h = durable_host(&f, &d, 64);
        let t = selected_task(&mut f, 1);
        let (c, n) = h.run_task_at(&t, now, wasm).unwrap();
        (c, n.receipt, *h.page)
    };
    assert_eq!(file_len(&d), (HEADER_LEN + RECORD_LEN) as u64);
    // New process: fresh PDP, same store. PCB retries the lost-reply commit.
    let f2 = fixture();
    let mut h2 = durable_host(&f2, &d, 64);
    assert_eq!(h2.lfs.store().recovery().records, 1);
    assert_eq!(h2.lfs.get(EPOCH, 1), Some(first));
    *h2.page = page;
    let again = h2.commit_and_notarize(&commit).unwrap();
    assert!(again.replayed);
    assert_eq!(again.receipt, first, "no re-sign after restart: same seq, same signature");
    assert!(first.verify(&PublicKey::new(*lfs_keys().pk)));
    assert_eq!(file_len(&d), (HEADER_LEN + RECORD_LEN) as u64, "no new effect");
}

#[test]
fn lfs_durable_retry_x3_same_receipt_one_effect() {
    let d = dir("retry");
    let mut f = fixture();
    let now = f.now;
    let mut h = durable_host(&f, &d, 64);
    let t = selected_task(&mut f, 1);
    let (commit, first) = h.run_task_at(&t, now, wasm).unwrap();
    let size = file_len(&d);
    for _ in 0..3 {
        let n = h.commit_and_notarize(&commit).unwrap();
        assert!(n.replayed);
        assert_eq!(n.receipt, first.receipt);
    }
    assert_eq!((file_len(&d), h.lfs.persisted(), h.executions), (size, 1, 1));
}

#[test]
fn lfs_durable_conflict_is_13_and_not_persisted() {
    let d = dir("conflict");
    let mut f = fixture();
    let now = f.now;
    let mut h = durable_host(&f, &d, 64);
    let t = selected_task(&mut f, 1);
    let (commit, _) = h.run_task_at(&t, now, wasm).unwrap();
    let size = file_len(&d);
    assert_eq!(
        h.commit_and_notarize(&Commit { value: 22, ..commit }),
        Err(E2eError::Writer(LfsStatus::IdempotencyConflict))
    );
    assert_eq!(file_len(&d), size);
}

#[test]
fn lfs_durable_ai_only_persists_zero_bytes() {
    let d = dir("ai_only");
    let f = fixture();
    let mut h = durable_host(&f, &d, 64);
    let t = task(&f.candidates, NOTES, 1); // no UserSelection
    let r = h.run_task_at(&t, f.now, wasm);
    assert_eq!(r.unwrap_err(), E2eError::Denied(DenyCode::PolicyDenied));
    assert_eq!(file_len(&d), HEADER_LEN as u64, "header only, no record");
    assert_eq!(h.executions, 0);
}

#[test]
fn lfs_durable_unauthorized_output_persists_nothing() {
    let d = dir("unauthorized");
    let mut f = fixture();
    let now = f.now;
    let mut h = durable_host(&f, &d, 64);
    let t = selected_task(&mut f, 1);
    let (commit, _) = h.run_task_at(&t, now, wasm).unwrap();
    let size = file_len(&d);
    for bad in [
        Commit { request_id: 2, ..commit },
        Commit { authority_digest: [0xEE; 32], epoch: 8, ..commit },
    ] {
        assert_eq!(h.commit_and_notarize(&bad), Err(E2eError::Writer(LfsStatus::PolicyDenied)));
    }
    assert_eq!(
        h.commit_and_notarize(&Commit { status: 6, epoch: 9, ..commit }),
        Err(E2eError::Writer(LfsStatus::Malformed))
    );
    assert_eq!(file_len(&d), size);
}

#[test]
fn lfs_durable_quota_exceeded_is_11() {
    let d = dir("quota");
    let mut f = fixture();
    let now = f.now;
    let mut h = durable_host(&f, &d, 1);
    let t = selected_task(&mut f, 1);
    h.run_task_at(&t, now, wasm).unwrap();
    let t2 = selected_task(&mut f, 2);
    let r = h.run_task_at(&t2, now, wasm);
    assert_eq!(r.unwrap_err(), E2eError::Writer(LfsStatus::QuotaExceeded));
    assert_eq!(LfsStatus::QuotaExceeded as u64, 11);
    assert_eq!(h.lfs.persisted(), 1);
}

/// Two committed records, then the second one is torn (crash mid-append,
/// before fsync completed): recovery keeps record 1 only, and a retry of
/// request 2 produces exactly one new effect.
#[test]
fn lfs_durable_torn_tail_is_truncated_without_phantom_receipt() {
    let d = dir("torn");
    let mut f = fixture();
    let now = f.now;
    let (c2, p2, r1) = {
        let mut h = durable_host(&f, &d, 64);
        let t = selected_task(&mut f, 1);
        let (_, n1) = h.run_task_at(&t, now, wasm).unwrap();
        let t2 = selected_task(&mut f, 2);
        let (c2, _) = h.run_task_at(&t2, now, wasm).unwrap();
        (c2, *h.page, n1.receipt)
    };
    let path = d.join(LEDGER_FILE);
    let full = (HEADER_LEN + 2 * RECORD_LEN) as u64;
    assert_eq!(fs::metadata(&path).unwrap().len(), full);
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(full - 37).unwrap();
    drop(file);

    let f2 = fixture();
    let mut h = durable_host(&f2, &d, 64);
    let rec = h.lfs.store().recovery();
    assert_eq!((rec.records, rec.truncated_bytes), (1, (RECORD_LEN - 37) as u64));
    assert_eq!(h.lfs.get(EPOCH, 1), Some(r1));
    assert_eq!(h.lfs.get(EPOCH, 2), None, "no phantom receipt");
    *h.page = p2;
    let n = h.commit_and_notarize(&c2).unwrap();
    assert!(!n.replayed);
    assert_eq!(n.receipt.body.seq, 2);
    for _ in 0..3 {
        assert_eq!(h.commit_and_notarize(&c2).unwrap().receipt, n.receipt);
    }
    assert_eq!(file_len(&d), full);
}

#[test]
fn lfs_durable_bitflip_in_last_record_is_dropped() {
    let d = dir("bitflip_tail");
    let mut f = fixture();
    let now = f.now;
    {
        let mut h = durable_host(&f, &d, 64);
        let t = selected_task(&mut f, 1);
        h.run_task_at(&t, now, wasm).unwrap();
    }
    let path = d.join(LEDGER_FILE);
    let mut bytes = fs::read(&path).unwrap();
    bytes[HEADER_LEN + 40] ^= 0x01;
    fs::write(&path, &bytes).unwrap();
    let f2 = fixture();
    let h = durable_host(&f2, &d, 64);
    assert_eq!(h.lfs.persisted(), 0);
    assert_eq!(file_len(&d), HEADER_LEN as u64);
}

#[test]
fn lfs_durable_corruption_before_tail_refuses_to_open() {
    let d = dir("bitflip_middle");
    let mut f = fixture();
    let now = f.now;
    {
        let mut h = durable_host(&f, &d, 64);
        let t = selected_task(&mut f, 1);
        h.run_task_at(&t, now, wasm).unwrap();
        let t2 = selected_task(&mut f, 2);
        h.run_task_at(&t2, now, wasm).unwrap();
    }
    let path = d.join(LEDGER_FILE);
    let mut bytes = fs::read(&path).unwrap();
    bytes[HEADER_LEN + 40] ^= 0x01; // record 1 damaged, record 2 intact
    fs::write(&path, &bytes).unwrap();
    let err = FileLedger::open(&d, PublicKey::new(*lfs_keys().pk), 64).err().unwrap();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(fs::read(&path).unwrap(), bytes, "nothing truncated");
}

#[test]
fn lfs_durable_wrong_lfs_key_refuses_to_open() {
    let d = dir("wrong_key");
    FileLedger::open(&d, PublicKey::new(*lfs_keys().pk), 8).unwrap();
    let err = FileLedger::open(&d, PublicKey::new(*pcb().pk), 8).err().unwrap();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

/// The durable store and the guest's in-memory ledger give identical
/// outcomes and byte-identical receipts for the same input sequence.
#[test]
fn lfs_durable_parity_with_in_memory_ledger() {
    let d = dir("parity");
    let mut f = fixture();
    let now = f.now;
    let mut h = durable_host(&f, &d, 64);
    let t = selected_task(&mut f, 1);
    let (commit, _) = h.run_task_at(&t, now, wasm).unwrap();
    let page = *h.page;
    let pcb_pk = PublicKey::new(*pcb().pk);
    let sk = lfs_keys().sk;
    let mut mem = MemoryStore::new();
    let mut file = FileLedger::open(&dir("parity_b"), sk.public_key(), 64).unwrap();
    for c in [
        commit,
        commit,
        Commit { value: 22, ..commit },
        Commit { request_id: 2, ..commit },
        Commit { status: 6, ..commit },
        Commit { epoch: 99, ..commit },
    ] {
        assert_eq!(mem.commit(&sk, pcb_pk, &page, &c), file.commit(&sk, pcb_pk, &page, &c));
    }
    assert_eq!(mem.len(), file.len());
}
