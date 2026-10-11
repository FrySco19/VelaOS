//! Host half of `verify-bridge-e2e`: runs the std PDP pipeline (mock AI intent ->
//! candidates -> UserSelection -> Biscuit LFS_READ grant -> execute admission)
//! and writes the artefacts the QEMU guest PCB publishes and checks.
//!
//! cargo run --locked --manifest-path poc/m5_executor/policy/Cargo.toml \
//!   --example e2e_bundle -- PCB_SEED LFS_SEED OUT_DIR
//!
//! * PCB_SEED: the existing local Executor-admission seed (read only).
//! * LFS_SEED: local writer seed; created (32 random bytes) if missing. Never commit it.
//! * OUT_DIR:  usually poc/m5_executor/app/generated (gitignored). Contains only
//!   public material: the signed frame, metadata and the LFS public key.
//!
//! The Biscuit root is ephemeral per run (test-only trust root).

use biscuit_auth::{KeyPair as RootKeys, builder::BiscuitBuilder};
use ed25519_compact::{KeyPair, Seed};
use std::{fs, path::Path, time::{Duration, SystemTime}};
use vela_executor_policy::grant::{Handle, TrustedSelectionChannel};
use vela_executor_policy::ledger_wire::grant_digest_in;
use vela_executor_policy::pipeline::{
    IndexEntry, MockPlanner, RETURN_21_MODULE, SelectionGatedPdp, TaskOutcome, TaskRequest,
};
use vela_executor_policy::{DenyCode, bridge};

pub const META_LEN: usize = 120;
const USER: &str = "vela-user";
const EPOCH: u64 = 1;
const DENIED_ID: u32 = 11;
const ADMITTED_ID: u32 = 12;
const FUEL: u64 = 10_000;

fn seed(path: &Path, create: bool) -> [u8; 32] {
    if !path.exists() {
        assert!(create, "missing seed {}", path.display());
        // OS randomness via the Biscuit key generator (no extra dependency).
        let fresh = RootKeys::new().private().to_bytes();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, fresh.as_slice()).expect("write LFS seed");
    }
    fs::read(path).expect("read seed").try_into().expect("32-byte seed")
}

fn delegation(root: &RootKeys, resource: &str, op: &str) -> Vec<u8> {
    BiscuitBuilder::new()
        .code(format!("right(\"{USER}\", \"{resource}\", \"{op}\");"))
        .unwrap()
        .build(root)
        .unwrap()
        .to_vec()
        .unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4, "e2e_bundle PCB_SEED LFS_SEED OUT_DIR");
    let pcb = KeyPair::from_seed(Seed::new(seed(Path::new(&args[1]), false)));
    assert_eq!(
        pcb.pk.as_ref(),
        bridge::trusted_key::TRUSTED_PUBLIC_KEY.as_slice(),
        "PCB seed does not match the committed Executor trust anchor"
    );
    let lfs = KeyPair::from_seed(Seed::new(seed(Path::new(&args[2]), true)));
    let out = Path::new(&args[3]);
    fs::create_dir_all(out).unwrap();

    let root = RootKeys::new();
    let mut pdp = SelectionGatedPdp::new(
        root.public(),
        delegation(&root, "lfs", "read"),
        delegation(&root, "executor", "execute"),
        pcb.sk.clone(),
    )
    .expect("pdp setup");

    // [1] Mock AI intent -> candidates (data only).
    let index = [
        IndexEntry { handle: [0xA1; 32], version: 3, title: "Note progetto VelaOS (bozza)" },
        IndexEntry { handle: [0xB2; 32], version: 1, title: "Foto vacanze note" },
        IndexEntry { handle: [0xC3; 32], version: 9, title: "Chiavi private" },
    ];
    let intent = "riassumi le note progetto VelaOS";
    let candidates = MockPlanner::propose(intent, &index);
    let top: Handle = candidates[0].handle;
    let now = SystemTime::now();
    let base = TaskRequest {
        caller: USER.into(),
        requested: top,
        candidates: &candidates,
        selection: None,
        ttl: Duration::from_secs(120),
        planner_note: "AI: user surely wants this, proceed".into(),
        request_id: DENIED_ID,
        module: RETURN_21_MODULE.to_vec(),
        fuel: FUEL,
    };

    // [2] AI-only attempt (no UserSelection) -> POLICY_DENIED, nothing signed.
    let deny = pdp.admit_task_at(&base, now);
    let deny_code = deny.deny_code().expect("AI-only attempt must be denied");
    assert_eq!(deny_code, DenyCode::PolicyDenied);
    assert_eq!(pdp.frames_signed(), 0);

    // [3] Mock UserSelection -> grant -> admission.
    let mut channel = TrustedSelectionChannel::new();
    let selection = channel.select(top, candidates[0].version);
    let selection_id = selection.selection_id();
    let selected = TaskRequest {
        selection: Some(selection),
        request_id: ADMITTED_ID,
        planner_note: String::new(),
        ..base.clone()
    };
    let admission = match pdp.admit_task_at(&selected, now) {
        TaskOutcome::Admitted(a) => a,
        TaskOutcome::Denied { code, stage, reason } => panic!("{code:?} at {stage:?}: {reason}"),
    };
    assert_eq!(pdp.frames_signed(), 1);
    let d = bridge::verify_with_key(&admission.frame, 0, pcb.pk).expect("frame verifies");
    let payload = &admission.frame[d.descriptor.payload_offset..][..d.descriptor.payload_len];
    assert_eq!(grant_digest_in(payload), Some(admission.authority_digest));
    let ttl = admission.not_after.duration_since(now).unwrap().as_secs();

    let mut meta = [0u8; META_LEN];
    meta[0..4].copy_from_slice(b"VE2M");
    meta[4..12].copy_from_slice(&EPOCH.to_le_bytes());
    meta[12..16].copy_from_slice(&DENIED_ID.to_le_bytes());
    meta[16..20].copy_from_slice(&ADMITTED_ID.to_le_bytes());
    meta[20..22].copy_from_slice(&deny_code.contract_code().to_le_bytes());
    meta[22..24].copy_from_slice(&0u16.to_le_bytes()); // frames signed on deny
    meta[24..56].copy_from_slice(&admission.authority_digest);
    meta[56..88].copy_from_slice(lfs.pk.as_ref());
    meta[88..92].copy_from_slice(&(admission.grant.len() as u32).to_le_bytes());
    meta[92..96].copy_from_slice(&(candidates.len() as u32).to_le_bytes());
    meta[96..104].copy_from_slice(&admission.selected_version.to_le_bytes());
    meta[104..112].copy_from_slice(&selection_id.to_le_bytes());
    meta[112..120].copy_from_slice(&ttl.to_le_bytes());
    fs::write(out.join("e2e_frame.bin"), admission.frame.as_slice()).unwrap();
    fs::write(out.join("e2e_meta.bin"), meta).unwrap();

    println!("intent: {intent:?} -> {} candidates (data only)", candidates.len());
    println!("AI-only request {DENIED_ID}: {deny_code:?} (code {}), frames_signed=0", deny_code.contract_code());
    println!(
        "UserSelection id={selection_id} version={} -> Biscuit LFS_READ grant {} B, ttl {ttl}s, authority {}",
        admission.selected_version,
        admission.grant.len(),
        admission.authority_digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>()
    );
    println!("admitted request {ADMITTED_ID}: frame {} B payload, fuel {FUEL}", admission.payload.len());
    println!("wrote {}", out.display());
}
