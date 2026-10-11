use anyhow::{Result, ensure};
use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
use std::time::{Duration, Instant};
use vela_control_plane_m3::{Risk, Router, cache::ProposalCache};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(args.len() == 2, "usage: cache-gate REPORT");
    let root = KeyPair::new();
    let token = BiscuitBuilder::new()
        .code("right(\"alice\",\"NetConfig\",\"read\"); right(\"alice\",\"NetConfig\",\"write\");")?
        .build(&root)?
        .to_vec()?;
    let router = Router::new(root.public());
    let mut cache = ProposalCache::new(300, Duration::from_secs(120))?;
    for i in 0..300 {
        cache.remember(
            &format!("Restore network Test-{i}"),
            "alice",
            true,
            "policy-v1",
            "{\"intent\":\"restore\"}",
        )?;
    }
    let mut samples = vec![];
    for _ in 0..20 {
        for i in 0..300 {
            let request = format!("Restore network Test-{i}");
            let start = Instant::now();
            let plan = cache
                .resolve(&router, &request, "alice", true, "policy-v1", &token)?
                .unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            ensure!(
                plan.risk() == Risk::Confirm && plan.query() == request,
                "cached plan differs"
            );
        }
    }
    samples.sort_by(f64::total_cmp);
    let p95 = samples[5699];
    let max = samples[5999];
    let report = serde_json::json!({"samples":6000,"p95_ms":p95,"max_ms":max,
        "all_under_5ms":max < 5.0,"llm_calls":0,"capability_verifications":6000,
        "scope":"ready in-process proposal-cache hit plus fresh Biscuit authorization; no service dispatch or object bytes",
        "m3_gate_closed":false});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&report)?)?;
    println!("p95 {p95:.3} ms; max {max:.3} ms");
    Ok(())
}
