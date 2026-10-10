//! Real process termination using synthetic credentials and objects only.
use anyhow::{Result, ensure};
use rand::{Rng, SeedableRng};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use vela_lfs_m1::{Lfs, Phase, fixture};
fn worker(backing: &std::path::Path, trial: u64) -> Result<()> {
    let token = fixture::token("crash-test", &["read", "write", "list"])?;
    let mut lfs = Lfs::unlock(backing, fixture::COLLECTION, fixture::root().public())?;
    lfs.put_with_probe(
        &token,
        "crash-test",
        &format!("trial-{trial}"),
        &fixture::payload(trial + 1),
        |phase| {
            println!("{phase:?}");
            std::io::stdout().flush().unwrap();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
        },
    )?;
    loop {
        std::thread::park();
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|v| v == "worker") {
        return worker(
            std::path::Path::new(&args[2]),
            args[3].to_str().unwrap().parse()?,
        );
    }
    let trials = args
        .get(1)
        .and_then(|v| v.to_str())
        .unwrap_or("10000")
        .parse::<usize>()?;
    ensure!(
        (1..=10000).contains(&trials),
        "trial count must be 1..10000"
    );
    let output = args
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "crash-report.json".into());
    let jobs = args
        .get(3)
        .and_then(|v| v.to_str())
        .unwrap_or("8")
        .parse::<usize>()?;
    ensure!((1..=8).contains(&jobs), "jobs must be 1..8");
    let started = Instant::now();
    let mut tasks = vec![];
    for job in 0..jobs.min(trials) {
        let count = trials / jobs + usize::from(job < trials % jobs);
        tasks.push(std::thread::spawn(move || {
            run_trials(count, 0x56454c414d31 + job as u64)
        }));
    }
    let mut parts = vec![];
    for task in tasks {
        parts.push(
            task.join()
                .map_err(|_| anyhow::anyhow!("harness thread panicked"))??,
        );
    }
    let mut phase_counts = [0u64; 5];
    for part in &parts {
        for (i, count) in phase_counts.iter_mut().enumerate() {
            *count += part["phase_counts"][i].as_u64().unwrap();
        }
    }
    let report = serde_json::json!({"trials":trials,"jobs":jobs.min(trials),"phase_counts":phase_counts,
        "lost_commits":0,"duplicate_commits":0,"invalid_reads":0,"elapsed_seconds":started.elapsed().as_secs_f64(),
        "process_kill_gate_passed":trials==10000,"power_loss_tested":false,"bus_integrated":false,"parts":parts});
    std::fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!("Verified {trials} process kills; report saved");
    Ok(())
}
fn run_trials(trials: usize, seed: u64) -> Result<serde_json::Value> {
    let scratch = tempfile::tempdir()?;
    let backing = scratch.path().join("fixture.redb");
    let token = fixture::token("crash-test", &["read", "write", "list"])?;
    let phases = [
        Phase::BeforeBlob,
        Phase::BlobDurable,
        Phase::BeforeLogCommit,
        Phase::LogDurable,
        Phase::IndexApplied,
    ];
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut distribution = [0usize; 5];
    let start = Instant::now();
    let mut recovered_inflight = 0;
    for trial in 0..trials {
        let (baseline, base_head) = {
            let mut lfs = Lfs::unlock(&backing, fixture::COLLECTION, fixture::root().public())?;
            let baseline = lfs.put(&token, "crash-test", "baseline", &fixture::payload(0))?;
            (baseline, lfs.commit_count())
        };
        let which = rng.gen_range(0..phases.len());
        ensure!(
            base_head == (trial % 32) as u64 + 1,
            "previous acknowledged commits lost between trials"
        );
        distribution[which] += 1;
        let mut child = Command::new(std::env::current_exe()?)
            .arg("worker")
            .arg(&backing)
            .arg(trial.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        loop {
            let mut line = String::new();
            ensure!(
                reader.read_line(&mut line)? > 0,
                "worker failed before target phase"
            );
            if line.trim() == format!("{:?}", phases[which]) {
                if phases[which] == Phase::BeforeLogCommit && rng.gen_bool(0.5) {
                    writeln!(child.stdin.as_mut().unwrap(), "continue")?;
                    child.stdin.as_mut().unwrap().flush()?;
                    std::thread::sleep(Duration::from_micros(rng.gen_range(0..=2500)));
                }
                child.kill()?;
                child.wait()?;
                break;
            }
            writeln!(child.stdin.as_mut().unwrap(), "continue")?;
            child.stdin.as_mut().unwrap().flush()?;
        }
        let mut lfs = Lfs::unlock(&backing, fixture::COLLECTION, fixture::root().public())?;
        ensure!(
            lfs.put(&token, "crash-test", "baseline", &fixture::payload(0))? == baseline,
            "lost acknowledged baseline"
        );
        let before = lfs.commit_count();
        ensure!(
            (base_head..=base_head + 1).contains(&before),
            "duplicated or lost log entry"
        );
        if which >= 3 {
            ensure!(before == base_head + 1, "lost durable operation");
        }
        if which <= 1 {
            ensure!(before == base_head, "uncommitted operation visible");
        }
        if which == 2 && before == base_head + 1 {
            recovered_inflight += 1;
        }
        let result = lfs.put(
            &token,
            "crash-test",
            &format!("trial-{trial}"),
            &fixture::payload(trial as u64 + 1),
        )?;
        ensure!(
            lfs.commit_count() == base_head + 1 && lfs.object_count() as u64 == base_head + 1,
            "retry duplicated commit/object"
        );
        ensure!(
            lfs.put(
                &token,
                "crash-test",
                &format!("trial-{trial}"),
                &fixture::payload(trial as u64 + 1)
            )? == result,
            "retry changed committed reply"
        );
        for row in lfs.list(&token, "crash-test", 0, 100)? {
            let h = lfs.handle(&token, "crash-test", &row.oid, 120)?;
            let value: serde_json::Value =
                serde_json::from_slice(&lfs.open(&token, "crash-test", &h)?)?;
            ensure!(vela_lfs_m1::oid(&value)? == row.oid, "read hash mismatch");
        }
        drop(lfs);
        if (trial + 1) % 32 == 0 || trial + 1 == trials {
            std::fs::remove_file(&backing)?;
        }
        if (trial + 1) % 250 == 0 {
            eprintln!("Verified {} / {trials} process kills", trial + 1);
        }
    }
    let report = serde_json::json!({"trials":trials,"seed":seed,"phase_counts":distribution,
        "phases":phases.iter().map(|p| format!("{p:?}")).collect::<Vec<_>>(),"inflight_commits_recovered":recovered_inflight,
        "lost_commits":0,"duplicate_commits":0,"invalid_reads":0,"elapsed_seconds":start.elapsed().as_secs_f64(),
        "process_kill_gate_passed":trials==10000,"power_loss_tested":false,"bus_integrated":false,
        "batch_size":32,"note":"Randomized real child-process termination at transaction phases and racing log commit. Distinct requests accumulate in groups of 32; all earlier objects, baseline acknowledgements, durable operations, retry replies and handle reads checked after every recovery. Not power-loss testing."});
    Ok(report)
}
