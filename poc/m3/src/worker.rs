//! Owned development inference worker. Production ingress remains Neural Bus work.
//! Bootstrap paths identify pinned artifacts, never user LFS objects.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    net::SocketAddr,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
};

const MODEL_SHA256: &str = "fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634";
const SERVER_SHA256: &str = "5b92164b26a620b56d38470d29ff9b36390b87ee19bc21f25b487d61482f6723";

fn verify_artifact(path: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == expected,
        "bootstrap artifact hash mismatch"
    );
    Ok(())
}

/// One GPU slot, bounded context/scratch and no persistent host prompt snapshots.
pub const MEMORY_ARGS: &[&str] = &[
    "--gpu-layers",
    "999",
    // Gemma input/per-layer embeddings are not covered by layer count alone.
    "--override-tensor",
    ".*=CUDA0",
    "--fit",
    "off",
    "--load-mode",
    // The mapped 512-token profile retained ~3.94 GB resident host pages in
    // live inference despite CUDA tensor placement. Stream uploads instead.
    "none",
    "--lazy-mode",
    "off",
    "--no-host",
    "--ctx-size",
    "512",
    "--kv-offload",
    "--batch-size",
    "512",
    "--ubatch-size",
    "128",
    "--parallel",
    "1",
    "--cache-ram",
    "0",
    "--ctx-checkpoints",
    "2",
    "--no-cache-idle-slots",
    "-lv",
    "4",
    "--sleep-idle-seconds",
    "5",
];

/// Owns only the child it starts. Shutdown releases backend/driver allocations.
pub struct LocalWorker {
    child: Child,
}
impl LocalWorker {
    pub fn start(
        executable: &Path,
        model: &Path,
        key_file: &Path,
        address: SocketAddr,
    ) -> Result<Self> {
        ensure!(
            address.ip().is_loopback() && address.port() != 0,
            "local endpoint required"
        );
        ensure!(
            executable.is_file() && model.is_file() && key_file.is_file(),
            "missing bootstrap artifact"
        );
        // Verify with bounded staging; never read the GGUF into a host Vec.
        verify_artifact(executable, SERVER_SHA256)?;
        verify_artifact(model, MODEL_SHA256)?;
        let mut command = Command::new(executable);
        // Isolate configuration from inherited llama/CUDA tuning. In particular,
        // unified-memory presence (even value "0") enables managed allocations.
        for (name, _) in std::env::vars_os() {
            let upper = name.to_string_lossy().to_ascii_uppercase();
            if upper.starts_with("LLAMA_ARG_") || upper.starts_with("GGML_CUDA_") {
                command.env_remove(name);
            }
        }
        command
            .args(MEMORY_ARGS)
            .arg("--model")
            .arg(model)
            .arg("--api-key-file")
            .arg(key_file)
            .arg("--host")
            .arg(address.ip().to_string())
            .arg("--port")
            .arg(address.port().to_string())
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        Ok(Self {
            child: command.spawn()?,
        })
    }
    pub fn id(&self) -> u32 {
        self.child.id()
    }
    pub fn wait(&mut self) -> Result<ExitStatus> {
        Ok(self.child.wait()?)
    }
}
impl Drop for LocalWorker {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MEMORY_ARGS;

    fn value_for(option: &str) -> &'static str {
        MEMORY_ARGS
            .windows(2)
            .find(|pair| pair[0] == option)
            .map(|pair| pair[1])
            .expect("required memory setting")
    }

    #[test]
    fn pins_model_and_kv_cache_to_cuda_with_bounded_context() {
        assert_eq!(value_for("--gpu-layers"), "999");
        assert_eq!(value_for("--load-mode"), "none");
        assert_eq!(value_for("--ctx-size"), "512");
        assert!(MEMORY_ARGS.contains(&"--kv-offload"));
        assert!(!MEMORY_ARGS.contains(&"--no-kv-offload"));
    }
}
