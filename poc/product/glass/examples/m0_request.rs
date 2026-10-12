//! Reproduce `tests/fixtures/m0_demo_output.json`:
//! `cargo run --offline --example m0_request -- "<query>" | <m0>/.venv/Scripts/python.exe bridge_recall.py`
//! (run python with cwd = poc/m0 checkout holding .model-cache, HF_HUB_OFFLINE=1).
fn main() {
    let q = std::env::args().nth(1).unwrap_or_else(|| "ZEPHYR-731 connessione protetta del portatile in caffetteria".into());
    println!("{}", vela_m0_recall::build_request(&q, &vela_glass::demo::corpus()).expect("request"));
}