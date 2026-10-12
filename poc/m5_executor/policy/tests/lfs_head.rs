//! Host head anchor: a witnessed LFS prefix cannot be rolled back by truncating
//! or replacing `ledger.vlfs` while `head.vlfs` still names the later tip.

mod common;

use common::*;
use ed25519_compact::PublicKey;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use vela_executor_policy::bridge;
use vela_executor_policy::durable::{
    FileLedger, HEAD_FILE, HEADER_LEN, LEDGER_FILE, LedgerStore, RECORD_LEN, encode_head,
    encode_record, load_head, record_tip,
};
use vela_executor_policy::ledger::{E2eHost, ExecReply, SimulatedLfs};
use vela_executor_policy::ledger_wire::Receipt;

const EPOCH: u64 = 7;

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("lfs_head")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    d
}

fn lfs_pk() -> PublicKey {
    PublicKey::new(*lfs_keys().pk)
}

fn wasm(page: &[u8; bridge::PAGE_SIZE]) -> ExecReply {
    let mut exec = vela_executor_host::HostExecutor::new(PublicKey::new(*pcb().pk));
    let r = exec.call(page);
    ExecReply {
        status: r.status as u64,
        value: r.value,
        request_id: r.request_id,
        attempts: r.attempts,
    }
}

fn selected_task<'a>(
    f: &'a mut Fixture,
    id: u32,
) -> vela_executor_policy::pipeline::TaskRequest<'a> {
    let sel = f.channel.select(NOTES, 3);
    let mut t = task(&f.candidates, NOTES, id);
    t.selection = Some(sel);
    t
}

fn host(f: &Fixture, d: &Path) -> E2eHost<FileLedger> {
    let lfs = SimulatedLfs::durable(lfs_keys().sk, PublicKey::new(*pcb().pk), d, 64).unwrap();
    E2eHost::new(gated(&f.root), lfs, EPOCH)
}

fn tip_of(r: &Receipt) -> [u8; 32] {
    record_tip(&encode_record(r))
}

fn commit_n(d: &Path, n: u32) -> Vec<Receipt> {
    let mut f = fixture();
    let now = f.now;
    let mut h = host(&f, d);
    let mut out = Vec::new();
    for id in 1..=n {
        let t = selected_task(&mut f, id);
        let (_, notarized) = h.run_task_at(&t, now, wasm).unwrap();
        out.push(notarized.receipt);
    }
    out
}

fn refuse_open(d: &Path) {
    let err = FileLedger::open(d, lfs_pk(), 64).err().unwrap();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
}

#[test]
fn lfs_head_commit_witnesses_tip_and_replay_does_not_move_it() {
    let d = dir("witness");
    let mut f = fixture();
    let now = f.now;
    let (commit, receipt, page) = {
        let mut h = host(&f, &d);
        let t = selected_task(&mut f, 1);
        let (c, n) = h.run_task_at(&t, now, wasm).unwrap();
        (c, n.receipt, *h.page)
    };
    let head = load_head(&d).unwrap();
    assert_eq!(head.seq, 1);
    assert_eq!(head.tip, tip_of(&receipt));

    let mut h2 = host(&fixture(), &d);
    *h2.page = page;
    let again = h2.commit_and_notarize(&commit).unwrap();
    assert!(again.replayed);
    assert_eq!(again.receipt, receipt);
    assert_eq!(load_head(&d).unwrap(), head);
}

#[test]
fn lfs_head_truncated_prefix_is_refused() {
    let d = dir("truncate");
    let _ = commit_n(&d, 2);
    let path = d.join(LEDGER_FILE);
    let before = fs::read(&path).unwrap();
    let head_before = fs::read(d.join(HEAD_FILE)).unwrap();
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len((HEADER_LEN + RECORD_LEN) as u64).unwrap();
    drop(file);
    let shortened = fs::read(&path).unwrap();
    refuse_open(&d);
    assert_eq!(
        fs::read(&path).unwrap(),
        shortened,
        "refused open does not rewrite the log"
    );
    assert_ne!(shortened, before);
    assert_eq!(fs::read(d.join(HEAD_FILE)).unwrap(), head_before);
    assert_eq!(load_head(&d).unwrap().seq, 2);
}

#[test]
fn lfs_head_stale_anchor_catches_up_when_prefix_matches() {
    let d = dir("catchup");
    let receipts = commit_n(&d, 2);
    fs::write(d.join(HEAD_FILE), encode_head(1, &tip_of(&receipts[0]))).unwrap();
    let ledger = FileLedger::open(&d, lfs_pk(), 64).unwrap();
    let recovery = ledger.recovery();
    assert_eq!(ledger.len(), 2);
    assert!(recovery.anchor_advanced);
    assert_eq!(recovery.truncated_bytes, 0);
    drop(ledger);
    let head = load_head(&d).unwrap();
    assert_eq!(head.seq, 2);
    assert_eq!(head.tip, tip_of(&receipts[1]));
    let again = FileLedger::open(&d, lfs_pk(), 64).unwrap();
    assert!(!again.recovery().anchor_advanced);
    assert_eq!(again.len(), 2);
}

#[test]
fn lfs_head_stale_anchor_with_wrong_tip_is_refused() {
    let d = dir("bad_prefix_tip");
    let _ = commit_n(&d, 2);
    let path = d.join(LEDGER_FILE);
    let before = fs::read(&path).unwrap();
    fs::write(d.join(HEAD_FILE), encode_head(1, &[0x11; 32])).unwrap();
    refuse_open(&d);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(load_head(&d).unwrap().seq, 1);
}

#[test]
fn lfs_head_substituted_tip_is_refused() {
    let d = dir("substituted");
    let _ = commit_n(&d, 2);
    let path = d.join(LEDGER_FILE);
    let before = fs::read(&path).unwrap();
    fs::write(d.join(HEAD_FILE), encode_head(2, &[0x22; 32])).unwrap();
    refuse_open(&d);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        (HEADER_LEN + 2 * RECORD_LEN) as u64
    );
}

#[test]
fn lfs_head_missing_anchor_with_receipts_is_refused() {
    let d = dir("missing_head");
    let _ = commit_n(&d, 1);
    let path = d.join(LEDGER_FILE);
    let before = fs::read(&path).unwrap();
    fs::remove_file(d.join(HEAD_FILE)).unwrap();
    refuse_open(&d);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn lfs_head_missing_ledger_with_witness_is_refused() {
    let d = dir("missing_ledger");
    let _ = commit_n(&d, 1);
    let head_before = fs::read(d.join(HEAD_FILE)).unwrap();
    fs::remove_file(d.join(LEDGER_FILE)).unwrap();
    refuse_open(&d);
    assert!(
        !d.join(LEDGER_FILE).exists(),
        "open must not recreate a witnessed log"
    );
    assert_eq!(fs::read(d.join(HEAD_FILE)).unwrap(), head_before);
}

#[test]
fn lfs_head_corrupt_anchor_is_refused() {
    let d = dir("corrupt_head");
    let _ = commit_n(&d, 1);
    let path = d.join(LEDGER_FILE);
    let before = fs::read(&path).unwrap();
    let head_path = d.join(HEAD_FILE);
    let mut head = fs::read(&head_path).unwrap();
    head[20] ^= 0x01;
    fs::write(&head_path, &head).unwrap();
    refuse_open(&d);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&head_path).unwrap(), head);
}

#[test]
fn lfs_head_unwitnessed_torn_tail_is_truncated() {
    let d = dir("unwitnessed_torn");
    {
        let f = fixture();
        let _ = host(&f, &d);
    }
    assert_eq!(load_head(&d).unwrap().seq, 0);
    let path = d.join(LEDGER_FILE);
    let mut bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), HEADER_LEN);
    bytes.extend_from_slice(&[0xA5; 20]);
    fs::write(&path, &bytes).unwrap();
    let ledger = FileLedger::open(&d, lfs_pk(), 64).unwrap();
    assert_eq!(ledger.len(), 0);
    assert_eq!(ledger.recovery().truncated_bytes, 20);
    assert!(!ledger.recovery().anchor_advanced);
    assert_eq!(load_head(&d).unwrap().seq, 0);
    assert_eq!(fs::metadata(&path).unwrap().len(), HEADER_LEN as u64);
}
