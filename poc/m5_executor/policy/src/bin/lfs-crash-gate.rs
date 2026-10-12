//! M-LFS-2 crash gate (modelled on `poc/m1` `crash-gate`): real process aborts
//! at every write phase of the durable LFS ledger, then recovery checks.
//!
//! `lfs-crash-gate <iterations> <report.json> [<work-dir>]`
//!
//! Each iteration spawns this binary as a child "boot" (new epoch) that commits
//! up to 3 receipts and aborts at one probe phase (before write / mid write /
//! before fsync / after fsync) of commit #1..#3. The child prints every receipt
//! the writer *returned*. The parent then reopens the ledger and requires:
//! * every announced receipt is present, byte-identical (no lost "OK");
//! * the log is a contiguous, signature-valid prefix (checked by `open`);
//! * a retry of the aborted request yields exactly one effect (replay if the
//!   record was integral, new commit otherwise, never both).

use ed25519_compact::{KeyPair, PublicKey, Seed};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use vela_executor_policy::bridge;
use vela_executor_policy::durable::{CrashPhase, FileLedger, LedgerStore};
use vela_executor_policy::ledger_wire::{Commit, CommitOutcome};
use vela_executor_policy::pipeline::{RETURN_21_MODULE, bind_grant};

const PER_BOOT: u32 = 3;
const PHASES: [CrashPhase; 4] = [
    CrashPhase::BeforeWrite,
    CrashPhase::MidWrite,
    CrashPhase::BeforeSync,
    CrashPhase::AfterSync,
];

fn pcb() -> KeyPair {
    KeyPair::from_seed(Seed::new([41; 32]))
}
fn lfs() -> KeyPair {
    KeyPair::from_seed(Seed::new([53; 32]))
}

fn digest(epoch: u64, rid: u32) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"crash-gate");
    h.update(&epoch.to_le_bytes());
    h.update(&rid.to_le_bytes());
    *h.finalize().as_bytes()
}

fn frame_and_commit(epoch: u64, rid: u32) -> ([u8; bridge::PAGE_SIZE], Commit) {
    let d = digest(epoch, rid);
    let payload = bind_grant(RETURN_21_MODULE, &d);
    let frame = bridge::signed_frame(&pcb().sk, rid, &payload, 10_000);
    let c = Commit {
        epoch,
        request_id: rid as u64,
        status: 0,
        value: 21,
        authority_digest: d,
    };
    (frame, c)
}

fn open(dir: &Path, cap: usize) -> FileLedger {
    FileLedger::open(dir, lfs().pk, cap).expect("ledger must reopen after crash")
}

fn child(dir: &Path, epoch: u64, phase: usize, nth: usize, cap: usize) {
    let mut ledger = open(dir, cap);
    ledger.crash_at(PHASES[phase], nth);
    let pcb_pk = pcb().pk;
    let out = std::io::stdout();
    for rid in 1..=PER_BOOT {
        let (frame, c) = frame_and_commit(epoch, rid);
        match ledger.commit(&lfs().sk, pcb_pk, &frame, &c) {
            Ok(CommitOutcome::Committed(r)) | Ok(CommitOutcome::Replayed(r)) => {
                let mut o = out.lock();
                writeln!(o, "RECEIPT {} {} {}", epoch, rid, hex(&r.signature)).unwrap();
                o.flush().unwrap();
            }
            Err(e) => panic!("unexpected commit error {e:?}"),
        }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("child") {
        let dir = PathBuf::from(&args[2]);
        let p = |i: usize| args[i].parse::<u64>().unwrap();
        child(&dir, p(3), p(4) as usize, p(5) as usize, p(6) as usize);
        return;
    }
    let iterations: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let report = PathBuf::from(args.get(2).cloned().unwrap_or(".local/lfs-crash.json".into()));
    let dir = PathBuf::from(
        args.get(3)
            .cloned()
            .unwrap_or(".local/lfs-crash-ledger".into()),
    );
    let _ = std::fs::remove_dir_all(&dir);
    let cap = iterations * PER_BOOT as usize + 16;
    let me = std::env::current_exe().unwrap();
    let pcb_pk: PublicKey = pcb().pk;
    let (mut aborted, mut announced_total, mut replays, mut recommits, mut torn) = (0, 0, 0, 0, 0u64);
    for i in 0..iterations {
        let epoch = i as u64 + 1;
        let phase = i % PHASES.len();
        let nth = 1 + (i / PHASES.len()) % PER_BOOT as usize;
        let out = Command::new(&me)
            .args([
                "child",
                dir.to_str().unwrap(),
                &epoch.to_string(),
                &phase.to_string(),
                &nth.to_string(),
                &cap.to_string(),
            ])
            .stderr(Stdio::null())
            .output()
            .unwrap();
        if !out.status.success() {
            aborted += 1;
        }
        let mut ledger = open(&dir, cap);
        torn += ledger.recovery().truncated_bytes;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut announced = 0;
        for line in text.lines().filter(|l| l.starts_with("RECEIPT ")) {
            let f: Vec<&str> = line.split(' ').collect();
            let (e, rid): (u64, u64) = (f[1].parse().unwrap(), f[2].parse().unwrap());
            let r = ledger
                .get(e, rid)
                .unwrap_or_else(|| panic!("iteration {i}: announced receipt ({e},{rid}) lost"));
            assert_eq!(hex(&r.signature), f[3], "iteration {i}: receipt re-signed");
            announced += 1;
        }
        assert_eq!(announced, nth - 1, "iteration {i}: abort at commit #{nth}");
        announced_total += announced;
        // PCB retry of the aborted request: exactly one effect.
        let rid = nth as u32;
        let before = ledger.len();
        let had = ledger.get(epoch, rid as u64).is_some();
        let (frame, c) = frame_and_commit(epoch, rid);
        match ledger.commit(&lfs().sk, pcb_pk, &frame, &c).unwrap() {
            CommitOutcome::Replayed(_) => {
                assert!(had);
                assert_eq!(ledger.len(), before);
                replays += 1;
            }
            CommitOutcome::Committed(_) => {
                assert!(!had);
                assert_eq!(ledger.len(), before + 1);
                recommits += 1;
            }
        }
        // Finish the boot's remaining requests so the next epoch starts clean.
        for rid in (nth as u32 + 1)..=PER_BOOT {
            let (frame, c) = frame_and_commit(epoch, rid);
            ledger.commit(&lfs().sk, pcb_pk, &frame, &c).unwrap();
        }
    }
    let ledger = open(&dir, cap);
    let expected = iterations * PER_BOOT as usize;
    assert_eq!(ledger.len(), expected, "exactly one effect per (epoch, request_id)");
    let json = format!(
        "{{\n  \"iterations\": {iterations},\n  \"aborted_children\": {aborted},\n  \"announced_receipts_verified\": {announced_total},\n  \"retry_replayed\": {replays},\n  \"retry_recommitted\": {recommits},\n  \"torn_bytes_truncated\": {torn},\n  \"final_records\": {},\n  \"expected_records\": {expected},\n  \"lost_announced_receipts\": 0,\n  \"double_effects\": 0\n}}\n",
        ledger.len()
    );
    if let Some(p) = report.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(&report, &json).unwrap();
    print!("{json}");
    println!("LFS CRASH GATE PASS");
}
