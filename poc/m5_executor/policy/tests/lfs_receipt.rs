//! M-G1c State Ledger: simulated LFS writer receipts, (epoch, request_id)
//! idempotency, PCB notary verification. The writer is the same `no_std`
//! `ledger_wire` code the QEMU `velaos-lfs` domain runs.

mod common;

use common::*;
use ed25519_compact::PublicKey;
use vela_executor_policy::ledger::{
    Commit, CommitOutcome, E2eError, E2eHost, ExecReply, Ledger, LfsStatus, NotaryError,
    SimulatedLfs,
};
use vela_executor_policy::ledger_wire::{Notary, Receipt, ReceiptBody};
use vela_executor_policy::{DenyCode, bridge};

const EPOCH: u64 = 1;

#[test]
fn recovered_history_requires_signature_contiguous_sequence_and_unique_request() {
    let key = lfs_keys();
    let c = Commit { epoch: 1, request_id: 12, status: 0, value: 21, authority_digest: [7;32] };
    let r = Receipt::sign(&key.sk, ReceiptBody::of(&c, 1));
    let mut ledger: Ledger<8> = Ledger::new();
    let mut forged = r; forged.signature[0] ^= 1;
    assert_eq!(ledger.recover(forged, &key.pk), Err(LfsStatus::PolicyDenied));
    let skipped = Receipt::sign(&key.sk, ReceiptBody::of(&c, 2));
    assert_eq!(ledger.recover(skipped, &key.pk), Err(LfsStatus::PolicyDenied));
    assert_eq!(ledger.recover(r, &key.pk), Ok(()));
    assert_eq!(ledger.get(1,12), Some(r));
    assert_eq!(ledger.recover(skipped, &key.pk), Err(LfsStatus::PolicyDenied));
    assert_eq!(ledger.len(), 1);
}

fn host(f: &Fixture) -> E2eHost {
    let lfs = SimulatedLfs::new(lfs_keys().sk, PublicKey::new(*pcb().pk));
    E2eHost::new(gated(&f.root), lfs, EPOCH)
}

fn wasm_executor() -> vela_executor_host::HostExecutor {
    vela_executor_host::HostExecutor::new(PublicKey::new(*pcb().pk))
}

fn run(exec: &mut vela_executor_host::HostExecutor, page: &[u8; bridge::PAGE_SIZE]) -> ExecReply {
    let r = exec.call(page);
    ExecReply {
        status: r.status as u64,
        value: r.value,
        request_id: r.request_id,
        attempts: r.attempts,
    }
}

#[test]
fn success_path_persists_one_receipt_and_pcb_notarizes_it() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, n) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    assert!(!n.replayed);
    assert_eq!((commit.value, commit.request_id, commit.epoch), (21, 1, EPOCH));
    assert_eq!(n.receipt.body.seq, 1);
    assert!(n.receipt.verify(&PublicKey::new(*lfs_keys().pk)));
    assert_eq!((h.lfs.persisted(), h.notary.len()), (1, 1));
    assert_eq!(h.resolve(1), Some(n.receipt));
}

#[test]
fn replayed_commit_returns_same_receipt_without_reexecution_or_new_effect() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, first) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    let attempts = exec.attempts();
    // Lost LFS reply -> PCB retries the commit (same epoch, request_id).
    for _ in 0..3 {
        let again = h.commit_and_notarize(&commit).unwrap();
        assert!(again.replayed);
        assert_eq!(again.receipt, first.receipt, "same seq, same signature");
    }
    assert_eq!(exec.attempts(), attempts, "no re-execution");
    assert_eq!((h.lfs.persisted(), h.notary.len(), h.executions), (1, 1, 1));
    // Ambiguous outcome resolves from the notary, not by re-running.
    assert_eq!(h.resolve(1), Some(first.receipt));
}

#[test]
fn same_key_with_different_result_is_idempotency_conflict_and_not_persisted() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, first) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    let forged = Commit { value: 22, ..commit };
    assert_eq!(
        h.commit_and_notarize(&forged),
        Err(E2eError::Writer(LfsStatus::IdempotencyConflict))
    );
    assert_eq!(LfsStatus::IdempotencyConflict as u64, 13);
    assert_eq!(h.lfs.persisted(), 1);
    assert_eq!(h.resolve(1), Some(first.receipt));
}

#[test]
fn ai_only_attempt_never_executes_and_persists_nothing() {
    let f = fixture();
    let mut h = host(&f);
    let t = task(&f.candidates, NOTES, 1);
    let mut called = false;
    let r = h.run_task_at(&t, f.now, |_| {
        called = true;
        ExecReply { status: 0, value: 21, request_id: 1, attempts: 1 }
    });
    assert_eq!(r.unwrap_err(), E2eError::Denied(DenyCode::PolicyDenied));
    assert!(!called, "no Wasm without UserSelection");
    assert_eq!((h.lfs.persisted(), h.notary.len(), h.executions), (0, 0, 0));
}

#[test]
fn unauthorized_output_is_not_persisted() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, _) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    // (a) output for a request that was never admitted (page holds request 1).
    let never_admitted = Commit { request_id: 2, ..commit };
    assert_eq!(
        h.commit_and_notarize(&never_admitted),
        Err(E2eError::Writer(LfsStatus::PolicyDenied))
    );
    // (b) right request id, wrong authority digest.
    let wrong_grant = Commit { request_id: 1, authority_digest: [0xEE; 32], epoch: 2, ..commit };
    assert_eq!(
        h.commit_and_notarize(&wrong_grant),
        Err(E2eError::Writer(LfsStatus::PolicyDenied))
    );
    // (c) failed execution status is never committable.
    let failed = Commit { status: 6, epoch: 3, ..commit };
    assert_eq!(h.commit_and_notarize(&failed), Err(E2eError::Writer(LfsStatus::Malformed)));
    // (d) a frame forged without the PCB key.
    let mut forged_page = *h.page;
    forged_page[bridge::PAYLOAD_OFFSET] ^= 1;
    let mut lfs = SimulatedLfs::new(lfs_keys().sk, PublicKey::new(*pcb().pk));
    assert_eq!(
        lfs.commit(&forged_page, &Commit { epoch: 9, ..commit }).unwrap_err(),
        LfsStatus::PolicyDenied
    );
    assert_eq!(lfs.persisted(), 0);
    assert_eq!((h.lfs.persisted(), h.notary.len()), (1, 1));
}

#[test]
fn failed_execution_and_wrong_ack_commit_nothing() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let r = h.run_task_at(&t, f.now, |_| ExecReply { status: 6, value: 0, request_id: 1, attempts: 1 });
    assert_eq!(r.unwrap_err(), E2eError::Execution(6));
    let mut t = task(&f.candidates, NOTES, 2);
    t.selection = Some(f.channel.select(NOTES, 3));
    let r = h.run_task_at(&t, f.now, |_| ExecReply { status: 0, value: 21, request_id: 99, attempts: 1 });
    assert_eq!(r.unwrap_err(), E2eError::AckMismatch);
    assert_eq!((h.lfs.persisted(), h.notary.len()), (0, 0));
}

#[test]
fn pcb_notary_rejects_forged_mismatched_and_conflicting_receipts() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, n) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    let mut notary: Notary<4> = Notary::new(PublicKey::new(*lfs_keys().pk));
    // Flipped signature byte.
    let mut bad = n.receipt;
    bad.signature[0] ^= 1;
    assert_eq!(notary.accept(&commit, &bad), Err(NotaryError::Forged));
    // Signed by a key that is not the pinned LFS key (e.g. the PCB key itself).
    let impostor = Receipt::sign(&pcb().sk, n.receipt.body);
    assert_eq!(notary.accept(&commit, &impostor), Err(NotaryError::Forged));
    // Genuine receipt for a different value than the PCB asked to commit.
    let other = Receipt::sign(&lfs_keys().sk, ReceiptBody { value: 99, ..n.receipt.body });
    assert_eq!(notary.accept(&commit, &other), Err(NotaryError::Mismatch));
    assert!(notary.is_empty());
    assert_eq!(notary.accept(&commit, &n.receipt), Ok(true));
    assert_eq!(notary.accept(&commit, &n.receipt), Ok(false));
    // Same key, different seq: conflicting history.
    let rewritten = Receipt::sign(&lfs_keys().sk, ReceiptBody { seq: 7, ..n.receipt.body });
    assert_eq!(notary.accept(&commit, &rewritten), Err(NotaryError::Conflict));
    assert_eq!(notary.len(), 1);
}

#[test]
fn receipt_register_roundtrip_matches_guest_ipc_layout() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, n) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    assert_eq!(Commit::from_regs(&commit.to_regs()), Some(commit));
    let regs = n.receipt.to_reply(true);
    assert_eq!((regs[0], regs[1], regs[2]), (0, 1, 1));
    assert_eq!(Receipt::from_reply(&commit, &regs), n.receipt);
}

#[test]
fn ledger_capacity_is_bounded() {
    let mut f = fixture();
    let mut h = host(&f);
    let mut exec = wasm_executor();
    let mut t = task(&f.candidates, NOTES, 1);
    t.selection = Some(f.channel.select(NOTES, 3));
    let (commit, _) = h.run_task_at(&t, f.now, |p| run(&mut exec, p)).unwrap();
    let mut small: Ledger<1> = Ledger::new();
    let key = lfs_keys().sk;
    let pcb = PublicKey::new(*pcb().pk);
    assert!(matches!(small.commit(&key, pcb, &h.page, &commit), Ok(CommitOutcome::Committed(_))));
    assert_eq!(
        small.commit(&key, pcb, &h.page, &Commit { epoch: 2, ..commit }),
        Err(LfsStatus::QuotaExceeded)
    );
    assert_eq!(small.len(), 1);
}
