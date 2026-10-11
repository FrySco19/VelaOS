//! Frozen-corpus live inference measurement. Every case is retained, including failures.
use anyhow::{Result, ensure};
use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
use std::time::Instant;
use vela_control_plane_m3::{CompilerOutput, Intent, Router, backend::LocalCompiler};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 6,
        "usage: planner-gate LOOPBACK_ADDRESS KEY_FILE CORPUS REPORT ROUNDS"
    );
    let rounds: usize = args[5].parse()?;
    ensure!((1..=20).contains(&rounds), "invalid rounds");
    let bytes = std::fs::read(&args[3])?;
    let corpus: serde_json::Value = serde_json::from_slice(&bytes)?;
    let cases = corpus["cases"].as_array().unwrap();
    ensure!(cases.len() == 300, "requires frozen 300-case corpus");
    let compiler = LocalCompiler::new(args[1].parse()?, std::fs::read_to_string(&args[2])?)?;
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
    compiler.compile("Initialize the compiler without executing anything.", false)?;
    let mut rows = vec![];
    let mut round_correct = vec![0_usize; rounds];
    let mut round_raw_correct = vec![0_usize; rounds];
    let mut valid = 0;
    let mut times = vec![];
    let mut first = std::collections::BTreeMap::new();
    let mut identical = true;
    for (round, correct_count) in round_correct.iter_mut().enumerate() {
        for (i, case) in cases.iter().enumerate() {
            let request = case["request"].as_str().unwrap();
            let focus = case["focus"].as_bool().unwrap();
            let expected: Intent = serde_json::from_value(case["expected"].clone())?;
            let allowed = case["policy_allowed"].as_bool().unwrap();
            let started = Instant::now();
            let reply = compiler.compile(request, focus)?;
            let raw = reply["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("");
            let output = serde_json::from_str::<CompilerOutput>(raw);
            let raw_intent = output.as_ref().ok().map(|o| o.intent);
            let token = if allowed { &full } else { &read };
            let plan = router.validate(raw, request, "gate", focus, token);
            let actual = plan.as_ref().ok().map(|p| p.intent());
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            times.push(elapsed);
            if output.is_ok() {
                valid += 1;
            }
            // Report raw classification separately from the composed, deterministic router.
            // A missing trusted focus must become Clarify even if the model proposes Save.
            let raw_correct = raw_intent == Some(expected);
            let correct = if allowed {
                actual == Some(expected)
            } else {
                raw_correct && plan.is_err()
            };
            if raw_correct {
                round_raw_correct[round] += 1;
            }
            if correct {
                *correct_count += 1;
            }
            if round == 0 {
                first.insert(i, raw_intent);
            } else if first.get(&i) != Some(&raw_intent) {
                identical = false;
            }
            rows.push(serde_json::json!({"round":round,"id":i,"request":request,"expected":expected,
                "raw_intent":raw_intent,"actual":actual,"policy_allowed":allowed,"policy_accepted":plan.is_ok(),
                "correct":correct,"raw_correct":raw_correct,"valid":output.is_ok(),"elapsed_ms":elapsed,"raw":raw,"timings":reply["timings"]}));
            if (i + 1) % 25 == 0 {
                println!(
                    "Round {}: {}/300, correct {}",
                    round + 1,
                    i + 1,
                    *correct_count
                );
            }
        }
    }
    times.sort_by(f64::total_cmp);
    let p95 = times[((times.len() - 1) as f64 * 0.95).ceil() as usize];
    let max = *times.last().unwrap();
    let passed = rounds == 20
        && round_correct.iter().all(|c| *c >= 286)
        && valid == 300 * rounds
        && max < 400.0
        && identical;
    let report = serde_json::json!({"corpus_blake3":blake3::hash(&bytes).to_hex().to_string(),"rounds":rounds,
        "kernel_prompt_blake3":blake3::hash(vela_control_plane_m3::PROMPT.as_bytes()).to_hex().to_string(),
        "adapter_version":"trusted-context-message-v1-compact-gbnf",
        "round_correct":round_correct,"round_raw_correct":round_raw_correct,"accuracy_scope":"composed model plus deterministic Router; raw accuracy retained separately",
        "raw_valid":valid,"samples":times.len(),"p95_ms":p95,"max_ms":max,
        "identical_normalized_intents":identical,"planner_corpus_gate_passed":passed,"m3_gate_closed":false,"cases":rows,
        "latency_scope":"route-cache miss, ready model, invariant system prefix may be cached; Rust Biscuit gate included",
        "limit":"synthetic finite-template corpus; no service operations executed; warm route-cache and integrated RAM qualification remain separate"});
    std::fs::write(&args[4], serde_json::to_vec_pretty(&report)?)?;
    println!(
        "correct {:?}; p95 {p95:.2} ms; maximum {max:.2} ms; full gate {passed}",
        round_correct
    );
    Ok(())
}
