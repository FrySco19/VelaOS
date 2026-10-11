//! Integrated cold-first + exact-cache repetitions, separate from the all-cold stress report.
use anyhow::{Result, anyhow, ensure};
use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
use std::time::{Duration, Instant};
use vela_control_plane_m3::{
    CompilerOutput, Intent, Router, ValidatedPlan, backend::LocalCompiler, cache::ProposalCache,
};

fn signature(plan: &Result<ValidatedPlan>) -> Result<String> {
    match plan {
        Ok(p) => Ok(blake3::hash(&serde_json::to_vec(&(
            p.intent(),
            p.steps(),
            p.risk(),
            p.query(),
            p.caller(),
        ))?)
        .to_hex()
        .to_string()),
        Err(_) => Ok("policy-denied".into()),
    }
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 5,
        "usage: router-gate ADDRESS KEY CORPUS REPORT"
    );
    let bytes = std::fs::read(&args[3])?;
    let corpus: serde_json::Value = serde_json::from_slice(&bytes)?;
    let cases = corpus["cases"]
        .as_array()
        .ok_or_else(|| anyhow!("missing cases"))?;
    ensure!(cases.len() == 300, "requires 300 frozen inputs");
    let compiler = LocalCompiler::new(args[1].parse()?, std::fs::read_to_string(&args[2])?)?;
    compiler.compile("Initialize the compiler without executing anything.", false)?;
    let root = KeyPair::new();
    let full = BiscuitBuilder::new()
        .code("right(\"gate\",\"NetConfig\",\"read\"); right(\"gate\",\"NetConfig\",\"write\");")?
        .build(&root)?
        .to_vec()?;
    let read = BiscuitBuilder::new()
        .code("right(\"gate\",\"NetConfig\",\"read\");")?
        .build(&root)?
        .to_vec()?;
    let router = Router::new(root.public());
    let mut cache = ProposalCache::new(300, Duration::from_secs(120))?;
    let mut proposals = vec![];
    let mut signatures = vec![];
    let mut cold = vec![];
    let mut warm = vec![];
    let mut rows = vec![];
    let mut scores = vec![0_usize; 20];
    let mut raw_scores = vec![0_usize; 20];
    let mut identical = true;
    for round in 0..20 {
        for (i, case) in cases.iter().enumerate() {
            let request = case["request"].as_str().unwrap();
            let focus = case["focus"].as_bool().unwrap();
            let expected: Intent = serde_json::from_value(case["expected"].clone())?;
            let allowed = case["policy_allowed"].as_bool().unwrap();
            let token = if allowed { &full } else { &read };
            let started = Instant::now();
            let (raw, plan) = if round == 0 {
                let reply = compiler.compile(request, focus)?;
                let raw = reply["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("missing proposal"))?
                    .to_owned();
                cache.remember(request, "gate", focus, "policy-v1", &raw)?;
                let plan = router.validate(&raw, request, "gate", focus, token);
                proposals.push(raw.clone());
                (raw, plan)
            } else {
                ensure!(
                    cache.has_live_proposal(request, "gate", focus, "policy-v1"),
                    "cache entry absent or expired; never count a miss as a policy denial"
                );
                let plan = cache
                    .resolve(&router, request, "gate", focus, "policy-v1", token)
                    .and_then(|p| p.ok_or_else(|| anyhow!("cache entry absent or expired")));
                (proposals[i].clone(), plan)
            };
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            if round == 0 {
                cold.push(elapsed);
            } else {
                warm.push(elapsed);
            }
            let raw_intent = serde_json::from_str::<CompilerOutput>(&raw)?.intent;
            let actual = plan.as_ref().ok().map(|p| p.intent());
            let correct = if allowed {
                actual == Some(expected)
            } else {
                raw_intent == expected && plan.is_err()
            };
            if correct {
                scores[round] += 1;
            }
            if raw_intent == expected {
                raw_scores[round] += 1;
            }
            let sig = signature(&plan)?;
            if round == 0 {
                signatures.push(sig.clone());
            } else if signatures[i] != sig {
                identical = false;
            }
            rows.push(serde_json::json!({"round":round,"id":i,"cache_hit":round>0,"elapsed_ms":elapsed,"correct":correct,"raw_intent":raw_intent,"actual":actual,"plan_hash":sig,"policy_accepted":plan.is_ok()}));
        }
        println!(
            "Round {}: {}/300 composed; {}/300 raw",
            round + 1,
            scores[round],
            raw_scores[round]
        );
    }
    cold.sort_by(f64::total_cmp);
    warm.sort_by(f64::total_cmp);
    let cold_max = *cold.last().unwrap();
    let warm_max = *warm.last().unwrap();
    let passed =
        scores.iter().all(|c| *c >= 286) && identical && cold_max < 400.0 && warm_max < 5.0;
    let report = serde_json::json!({"protocol":"300 real ready-model misses then 5700 exact-cache hits with fresh authorization within 120s TTL",
        "corpus_blake3":blake3::hash(&bytes).to_hex().to_string(),"model_calls":300,"cache_hits":5700,
        "kernel_prompt_blake3":blake3::hash(vela_control_plane_m3::PROMPT.as_bytes()).to_hex().to_string(),
        "round_correct":scores,"round_raw_correct":raw_scores,"identical_full_plan_hashes":identical,
        "cold_p95_ms":cold[284],"cold_max_ms":cold_max,"warm_p95_ms":warm[5414],"warm_max_ms":warm_max,
        "cached_router_protocol_passed":passed,"m3_gate_closed":false,"cases":rows,
        "limitations":["development corpus was used to improve prompt; independent holdout required",
        "cache stability is bounded by TTL/epochs, not cold model determinism",
        "inference adapter is authenticated loopback development ingress; production bus integration pending"]});
    std::fs::write(&args[4], serde_json::to_vec_pretty(&report)?)?;
    println!("protocol passed {passed}; cold max {cold_max:.3} ms, warm max {warm_max:.3} ms");
    Ok(())
}
