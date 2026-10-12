//! Ciao demo: utterance «ciao» → FALLBACK CONTEXT-SIGNALS.
//!
//! BM25 is reported and is not treated as a model. No LLM is called.

mod font;
mod png;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use vela_intent::{RecallMode, proposal_json, propose_from_context};

const DEMO_COMMAND: &str = "cargo run --manifest-path poc/intent/Cargo.toml --bin ciao-fallback -- poc/intent/fixtures/ciao-demo-signals.jsonl";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let fixture = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(default_fixture);
    let out_dir = args.next().map(PathBuf::from).unwrap_or_else(default_out);
    let jsonl = match fs::read_to_string(&fixture) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("cannot read {}: {err}", fixture.display());
            return ExitCode::from(1);
        }
    };
    let proposal = propose_from_context("ciao", &jsonl, RecallMode::ObservedInRecall, &[]);
    println!("command: {DEMO_COMMAND}");
    println!("mode: {}", proposal.mode);
    println!("llm_invoked: {}", proposal.llm_invoked);
    println!("LLM not invoked");
    println!("utterance: {}", proposal.utterance);
    println!("bm25_lexical_hits: {}", proposal.bm25_lexical_hits);
    println!("auto_resolve: {}", proposal.auto_resolve());
    println!("rows: {}", proposal.candidates.len());
    for (i, row) in proposal.candidates.iter().enumerate() {
        let version = row.expected_version.as_deref().unwrap_or("null");
        println!(
            "{}  source={}  type={}  handle={}  version={}  score={:.2}",
            i + 1,
            row.source.as_str(),
            row.signal_type,
            row.handle,
            version,
            row.score,
        );
    }
    if proposal.candidates.is_empty() {
        eprintln!("no proposal rows");
        return ExitCode::from(2);
    }
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("cannot create {}: {err}", out_dir.display());
        return ExitCode::from(1);
    }
    let list_path = out_dir.join("proposal-list.png");
    let glass_path = out_dir.join("glass-no-authority.png");
    let json_path = out_dir.join("proposal.json");
    let wrote = fs::write(&list_path, png::proposal_list_png(&proposal))
        .and_then(|_| fs::write(&glass_path, png::glass_surface_png(&proposal)))
        .and_then(|_| fs::write(&json_path, proposal_json(&proposal)));
    if let Err(err) = wrote {
        eprintln!("cannot write outputs: {err}");
        return ExitCode::from(1);
    }
    println!("wrote: {}", list_path.display());
    println!("wrote: {}", glass_path.display());
    println!("wrote: {}", json_path.display());
    ExitCode::SUCCESS
}

fn default_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/ciao-demo-signals.jsonl")
}

fn default_out() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo-out")
}
