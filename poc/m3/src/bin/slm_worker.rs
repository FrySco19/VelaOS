use anyhow::{Result, ensure};
use std::path::Path;
use vela_control_plane_m3::worker::LocalWorker;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    ensure!(
        args.len() == 5,
        "usage: slm-worker PINNED_SERVER PINNED_MODEL PROTECTED_KEY_FILE LOOPBACK_ADDRESS"
    );
    let address = args[4]
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid address"))?
        .parse()?;
    let mut worker = LocalWorker::start(
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
        address,
    )?;
    eprintln!("owned SLM worker PID={} idle unload=5s", worker.id());
    ensure!(worker.wait()?.success(), "SLM worker failed");
    Ok(())
}
