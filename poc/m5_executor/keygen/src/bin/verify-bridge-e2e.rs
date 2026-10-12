//! Host runner for the M5 end-to-end QEMU check; never executes inside the OS.
//!
//! Prerequisite (host std PDP half, writes public artefacts only):
//!   cargo run --locked --manifest-path poc/m5_executor/policy/Cargo.toml \
//!     --example e2e_bundle -- PCB_SEED LFS_SEED poc/m5_executor/app/generated
//! Then:
//!   verify-bridge-e2e DOCKER_CONTEXT PCB_SEED LFS_SEED LOG_FILE [--wsl]
//! With Docker inside WSL pass /mnt/c/... context/seed paths and --wsl.
use std::{
    env, fs,
    process::{Command, Output},
    time::Instant,
};

fn docker(args: &[&str], wsl: bool) -> Output {
    let mut command = if wsl {
        let mut c = Command::new("wsl.exe");
        c.args(["-e", "docker"]);
        c
    } else {
        Command::new("docker")
    };
    command.args(args).output().expect("start Docker")
}

/// Every marker must appear exactly once.
const MARKERS: &[&str] = &[
    // Existing verify-bridge sequence (unchanged, incl. R1 MAX_FUEL survival).
    "PCB reply request=1 status=0 value=7 attempts=1",
    "PCB reply request=2 status=2 value=0 attempts=1",
    "PCB reply request=3 status=2 value=0 attempts=1",
    "PCB reply request=4 status=1 value=0 attempts=1",
    "PCB reply request=5 status=3 value=0 attempts=1",
    "PCB reply request=1 status=4 value=0 attempts=1",
    "PCB reply request=6 status=6 value=0 attempts=2",
    "PCB reply request=7 status=5 value=0 attempts=2",
    "PCB reply request=8 status=0 value=7 attempts=3",
    "PCB reply request=9 status=6 value=0 attempts=4",
    "MAX_FUEL stack-safe status=6",
    "PCB reply request=10 status=0 value=7 attempts=5",
    "fuel exhaustion and 64 KiB memory cap verified",
    "ALL CHECKS PASSED; authenticated Wasm returned 7 via IPC",
    // End-to-end pipeline.
    "LFS writer ready (simulated in-memory ledger, capacity=8)",
    "E2E [1]: mock AI intent -> 2 candidates (data only, no authority)",
    "E2E [2]: AI-only request=11 POLICY_DENIED code=10 frames_signed=0, nothing published",
    "E2E [3]: UserSelection id=1 version=3 -> PDP Biscuit LFS_READ grant bytes=",
    "E2E [4]: admitted frame request=12 published on 4 KiB RO page",
    "Executor authenticated grant-bound payload request=12 authority=",
    "E2E [5]: PCB reply request=12 status=0 value=21 attempts=6 (fuel-safe Wasm)",
    "LFS committed epoch=1 request=12 seq=1 persisted=1",
    "E2E [6]: LFS receipt epoch=1 request=12 seq=1 replayed=0",
    "E2E [7]: PCB notary verified+stored receipt request=12 value=21 seq=1",
    "LFS replayed epoch=1 request=12 seq=1 persisted=1",
    "E2E N1: commit retry same epoch/request -> same receipt seq=1 replayed=1",
    "LFS refused request=12 status=13 persisted=1",
    "E2E N2: same key different result -> status=13 not persisted",
    "LFS refused request=11 status=10 persisted=1",
    "E2E N3: output for AI-only request=11 -> status=10 not persisted",
    "E2E N4: forged receipt rejected by PCB notary",
    "E2E N5: replayed admission request=12 status=4 attempts=6 (no Wasm, no commit)",
    // M-CLK-3: trusted in-guest monotonic clock for grant TTL.
    "E2E CLK: guest monotonic CNTPCT_EL0 freq=",
    "E2E CLK [1]: request=12 grant ttl=120s alive in-guest remaining_ms=",
    "E2E CLK [2]: request=13 grant ttl=1s expired in-guest after ",
    "-> EXPIRED code=7, result discarded, no LFS commit",
    "ALL E2E CHECKS PASSED; intent -> selection -> grant -> Wasm 21 -> receipt notarized",
];

/// Must never appear: an expired grant reaching the LFS writer.
const FORBIDDEN: &[&str] = &["LFS committed epoch=1 request=13", "LFS refused request=13"];

fn main() {
    let args: Vec<_> = env::args().collect();
    assert!(
        args.len() == 5 || (args.len() == 6 && args[5] == "--wsl"),
        "verify-bridge-e2e DOCKER_CONTEXT PCB_SEED LFS_SEED LOG_FILE [--wsl]"
    );
    let wsl = args.len() == 6;
    let started = Instant::now();
    let dockerfile = format!("{}/Dockerfile.e2e", args[1]);
    let pcb = format!("id=pcb_seed,src={}", args[2]);
    let lfs = format!("id=lfs_seed,src={}", args[3]);
    let built = docker(
        &[
            "build", "--secret", &pcb, "--secret", &lfs, "-f", &dockerfile, "-t",
            "velaos-executor-e2e", &args[1],
        ],
        wsl,
    );
    assert!(
        built.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = docker(
        &[
            "run", "--rm", "--entrypoint", "timeout", "velaos-executor-e2e", "30s", "make", "run",
            "E2E=1",
        ],
        wsl,
    );
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    fs::write(&args[4], &log).expect("save complete serial output");
    assert_eq!(
        run.status.code(),
        Some(124),
        "guest terminated unexpectedly; inspect log"
    );
    assert!(
        !log.contains("panicked") && !log.contains("FAULT"),
        "guest fault"
    );
    for marker in MARKERS {
        assert_eq!(
            log.matches(marker).count(),
            1,
            "missing/duplicate marker: {marker}"
        );
    }
    for forbidden in FORBIDDEN {
        assert!(!log.contains(forbidden), "expired grant reached LFS: {forbidden}");
    }
    for line in log.lines().filter(|line| line.starts_with("VelaOS ")) {
        println!("{line}");
    }
    println!(
        "PASS (e2e): {} markers, no panic/FAULT, clean idle timeout, {:.0?} incl. build.",
        MARKERS.len(),
        started.elapsed()
    );
}
