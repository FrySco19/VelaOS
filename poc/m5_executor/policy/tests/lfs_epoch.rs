//! M-LFS-3: persistent epoch across process restarts.

mod common;

use common::*;
use ed25519_compact::PublicKey;
use std::fs;
use std::path::{Path, PathBuf};
use vela_executor_policy::durable::{Boot, EPOCH_FILE, EpochStore, FileLedger};
use vela_executor_policy::ledger::{E2eHost, ExecReply, SimulatedLfs};

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("lfs_epoch").join(name);
    let _ = fs::remove_dir_all(&d);
    d
}

const OK: ExecReply = ExecReply { status: 0, value: 21, request_id: 1, attempts: 1 };

fn boot(f: &Fixture, d: &Path) -> (E2eHost<FileLedger>, EpochStore) {
    let epoch = EpochStore::open(d).unwrap();
    let lfs = SimulatedLfs::durable(lfs_keys().sk, PublicKey::new(*pcb().pk), d, 64).unwrap();
    (E2eHost::new(gated(&f.root), lfs, epoch.epoch()), epoch)
}

#[test]
fn lfs_epoch_every_boot_is_a_new_epoch_and_reports_how_last_ended() {
    let d = dir("boots");
    let e1 = EpochStore::open(&d).unwrap();
    assert_eq!((e1.epoch(), e1.boot(), e1.previous()), (1, Boot::Fresh, None));
    drop(e1); // crash: no close_clean
    let e2 = EpochStore::open(&d).unwrap();
    assert_eq!((e2.epoch(), e2.boot(), e2.previous()), (2, Boot::Recovered, Some(1)));
    e2.close_clean().unwrap();
    let e3 = EpochStore::open(&d).unwrap();
    assert_eq!((e3.epoch(), e3.boot()), (3, Boot::Clean));
}

#[test]
fn lfs_epoch_session1_receipt_readable_in_session2_and_ids_do_not_collide() {
    let d = dir("sessions");
    let mut f = fixture();
    let now = f.now;
    let (r1, e1) = {
        let (mut h, ep) = boot(&f, &d);
        let mut t = task(&f.candidates, NOTES, 1);
        t.selection = Some(f.channel.select(NOTES, 3));
        let (_, n) = h.run_task_at(&t, now, |_| OK).unwrap();
        (n.receipt, ep.epoch()) // process dies here (no clean close)
    };
    let mut f2 = fixture();
    let (mut h, ep) = boot(&f2, &d);
    assert_eq!(ep.boot(), Boot::Recovered);
    assert_ne!(ep.epoch(), e1);
    // Ambiguous previous-session request resolves from the durable ledger.
    assert_eq!(h.resolve_in(e1, 1), Some(r1));
    // Fresh PDP restarts request ids at 1: new epoch, so a new effect, no conflict.
    let mut t = task(&f2.candidates, NOTES, 1);
    t.selection = Some(f2.channel.select(NOTES, 3));
    let (c, n) = h.run_task_at(&t, f2.now, |_| OK).unwrap();
    assert!(!n.replayed);
    assert_eq!((c.epoch, n.receipt.body.seq), (ep.epoch(), 2));
    assert_eq!(h.lfs.persisted(), 2);
    ep.close_clean().unwrap();
}

#[test]
fn lfs_epoch_corrupted_file_is_fail_closed() {
    let d = dir("corrupt");
    EpochStore::open(&d).unwrap();
    let p = d.join(EPOCH_FILE);
    let mut b = fs::read(&p).unwrap();
    b[9] ^= 0xFF;
    fs::write(&p, &b).unwrap();
    assert_eq!(EpochStore::open(&d).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}
