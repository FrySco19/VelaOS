use anyhow::{Result, ensure};
use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
use std::time::Instant;
use vela_control_plane_m3::{Intent, Router, backend::LocalCompiler};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 4,
        "usage: planner-probe LOOPBACK_ADDRESS KEY_FILE REPORT"
    );
    let compiler = LocalCompiler::new(args[1].parse()?, std::fs::read_to_string(&args[2])?)?;
    let root = KeyPair::new();
    let token = BiscuitBuilder::new()
        .code("right(\"probe\",\"NetConfig\",\"read\"); right(\"probe\",\"NetConfig\",\"write\");")?
        .build(&root)?
        .to_vec()?;
    let router = Router::new(root.public());
    // Prime the invariant system-prefix/kernel graph. Not a scored route-cache hit.
    compiler.compile("Initialize the compiler without executing anything.", false)?;
    let cases = [
        (
            "Salva la configurazione di rete corrente",
            true,
            Intent::Save,
        ),
        (
            "Trova la configurazione salvata con SSID CafeGuest-5G",
            true,
            Intent::Recall,
        ),
        (
            "Ripristina la configurazione salvata della rete",
            true,
            Intent::Restore,
        ),
        ("Save the current network configuration", true, Intent::Save),
        ("Find the saved network configuration", true, Intent::Recall),
        (
            "Restore the saved network configuration",
            true,
            Intent::Restore,
        ),
        (
            "Installa un programma scaricato da Internet",
            true,
            Intent::Unsupported,
        ),
        ("Salva questa configurazione", false, Intent::Clarify),
    ];
    let mut rows = vec![];
    for (request, focus, expected) in cases {
        let started = Instant::now();
        let response = compiler.compile(request, focus)?;
        let raw = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("");
        let plan = router.validate(raw, request, "probe", focus, &token);
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        let row = match plan {
            Ok(p) => serde_json::json!({"request":request,"expected":expected,"actual":p.intent(),
                "correct":p.intent()==expected,"valid":true,"elapsed_ms":elapsed,"risk":p.risk(),"steps":p.steps(),
                "raw":raw,"timings":response["timings"]}),
            Err(e) => {
                serde_json::json!({"request":request,"expected":expected,"correct":false,"valid":false,
                "elapsed_ms":elapsed,"error":e.to_string(),"raw":raw})
            }
        };
        println!("{}", serde_json::to_string(&row)?);
        rows.push(row);
    }
    let report = serde_json::json!({"kind":"eight-intent live SLM integration probe","cases":rows,
        "m3_gate_closed":false,"limit":"not the frozen 300-intent gate; no operations executed",
        "architecture":"SLM selects finite manifest templates; Rust expands typed DAG and checks Biscuit"});
    std::fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
