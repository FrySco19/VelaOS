//! Host verification runner; never executes inside the OS data plane.
use std::{
    env, fs,
    process::{Command, Output},
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
fn main() {
    let args: Vec<_> = env::args().collect();
    assert!(
        args.len() == 4 || (args.len() == 5 && args[4] == "--wsl"),
        "verify-bridge DOCKER_CONTEXT PRIVATE_SEED LOG_FILE [--wsl]"
    );
    let wsl = args.len() == 5;
    let dockerfile = format!("{}/Dockerfile", args[1]);
    let secret = format!("id=pcb_seed,src={}", args[2]);
    let built = docker(
        &[
            "build",
            "--secret",
            &secret,
            "-f",
            &dockerfile,
            "-t",
            "velaos-executor-verified",
            &args[1],
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
            "run",
            "--rm",
            "--entrypoint",
            "timeout",
            "velaos-executor-verified",
            "20s",
            "make",
            "run",
        ],
        wsl,
    );
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    fs::write(&args[3], &log).expect("save complete serial output");
    assert_eq!(
        run.status.code(),
        Some(124),
        "guest terminated unexpectedly; inspect log"
    );
    assert!(
        !log.contains("panicked") && !log.contains("FAULT"),
        "guest fault"
    );
    for marker in [
        "PCB reply request=1 status=0 value=7 attempts=1",
        "PCB reply request=2 status=2 value=0 attempts=1",
        "PCB reply request=3 status=2 value=0 attempts=1",
        "PCB reply request=4 status=1 value=0 attempts=1",
        "PCB reply request=5 status=3 value=0 attempts=1",
        "PCB reply request=1 status=4 value=0 attempts=1",
        "PCB reply request=6 status=6 value=0 attempts=2",
        "PCB reply request=7 status=5 value=0 attempts=2",
        "PCB reply request=8 status=0 value=7 attempts=3",
        "fuel exhaustion and 64 KiB memory cap verified",
        "ALL CHECKS PASSED; authenticated Wasm returned 7 via IPC",
    ] {
        assert_eq!(
            log.matches(marker).count(),
            1,
            "missing/duplicate marker: {marker}"
        );
    }
    for line in log.lines().filter(|line| line.starts_with("VelaOS ")) {
        println!("{line}");
    }
    println!(
        "PASS: fresh build, matched acknowledgements, all guest rejection cases, clean idle timeout."
    );
}
