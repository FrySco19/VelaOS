//! Native development broker. `serve` performs consent and receipt verification.
//! Legacy `send` is a TEST harness, never a production approval path.
//! Local plaintext seeds are development custody, not a TPM/FIDO2 enclave.
//! keygen <seed-file> <public-key.rs>, then send <endpoint> <seed-file> <mode>.
//! Endpoints: pipe:<Windows name>, unix:<path>, tcp:127.0.0.1:<port>.
//! TCP is the loopback adapter for QEMU inside WSL, not remote authorization.
mod hello;
mod socket;
use ed25519_compact::{KeyPair, Seed};
use sha2::{Digest, Sha256};
use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};
use vela_protocol::{Frame, MAX_FRAME, MessageType};
use vela_selection_contract::UserSelection;

const PLAN: &[u8] = b"Vela selection IPC probe: save this exact note";
const SESSION: [u8; 32] = [0x39; 32]; // Matches guest TEST fixture, not fresh consent.
const REVISION: u64 = 1;

fn keygen(seed_path: &Path, public_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut seed = [0; 32];
    getrandom::getrandom(&mut seed).map_err(|e| io::Error::other(e.to_string()))?;
    let key = KeyPair::from_seed(Seed::new(seed));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(seed_path)?; // Never overwrite an existing key.
    file.write_all(&seed)?;
    file.sync_all()?;
    fs::write(
        public_path,
        format!(
            "pub const TRUSTED_PUBLIC_KEY: [u8; 32] = {:?};\n",
            key.pk.as_ref()
        ),
    )?;
    println!("TEST key generated; enroll the public key in the guest before sending");
    Ok(())
}

fn write_endpoint(endpoint: &str, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Some(address) = endpoint.strip_prefix("tcp:") {
        let address: SocketAddr = address.parse()?;
        if !address.ip().is_loopback() {
            return Err("TEST TCP transport requires loopback".into());
        }
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(bytes)?;
        stream.flush()?;
        return Ok(());
    }
    #[cfg(unix)]
    if let Some(path) = endpoint.strip_prefix("unix:") {
        let mut stream = std::os::unix::net::UnixStream::connect(path)?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(bytes)?;
        return Ok(());
    }
    #[cfg(windows)]
    if let Some(path) = endpoint.strip_prefix("pipe:") {
        if !path.starts_with(r"\\.\pipe\") {
            return Err("expected a local Windows named pipe".into());
        }
        let mut pipe = OpenOptions::new().read(true).write(true).open(path)?;
        pipe.write_all(bytes)?;
        return Ok(());
    }
    Err("unsupported endpoint for this operating system".into())
}

fn send(endpoint: &str, seed_path: &Path, mode: &str) -> Result<(), Box<dyn Error>> {
    let seed: [u8; 32] = fs::read(seed_path)?
        .try_into()
        .map_err(|_| "seed must contain exactly 32 bytes")?;
    let key = KeyPair::from_seed(Seed::new(seed));
    let mut selection = UserSelection {
        session_binding: SESSION,
        revision: REVISION,
        plan_sha256: Sha256::digest(PLAN).into(),
        signature: [0; 64],
    };
    let signature = key.sk.sign(selection.signing_bytes(), None);
    selection.signature.copy_from_slice(signature.as_ref());
    match mode {
        "valid" => (),
        "bad-signature" => selection.signature[0] ^= 1,
        "bad-hash" => selection.plan_sha256[0] ^= 1,
        _ => return Err("mode: valid | bad-signature | bad-hash".into()),
    }
    let frame = Frame::new(MessageType::Approval, REVISION, 0, &selection.encode())
        .map_err(|e| io::Error::other(format!("frame: {e:?}")))?;
    let mut buffer = [0; MAX_FRAME];
    let wire = frame
        .encode(&mut buffer)
        .map_err(|e| io::Error::other(format!("encode: {e:?}")))?;
    write_endpoint(endpoint, wire)?;
    println!(
        "TEST Approval sent: mode={mode}, bytes={}; delivery is not authorization",
        wire.len()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("serve") if args.len() == 5 || (args.len() == 6 && matches!(args[5].as_str(),"--console-development"|"--mock-unavailable")) => socket::serve(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),args.get(5).is_some_and(|s|s=="--console-development"),args.get(5).is_some_and(|s|s=="--mock-unavailable")).map_err(Into::into),
        Some("keygen") if args.len() == 4 => keygen(Path::new(&args[2]), Path::new(&args[3])),
        Some("public") if args.len() == 4 => {
            let seed:[u8;32]=fs::read(&args[2])?.try_into().map_err(|_|"seed length")?;
            let pair=KeyPair::from_seed(Seed::new(seed));
            let mut file=OpenOptions::new().write(true).create_new(true).open(&args[3])?;
            file.write_all(pair.pk.as_ref())?;file.sync_all()?;Ok(())
        },
        Some("send") if args.len() == 5 => send(&args[2], Path::new(&args[3]), &args[4]),
        _ => Err("usage: serve <broker.seed> <pcb.public> <lfs.public> [--console-development|--mock-unavailable] | keygen <seed> <public.rs> | public <seed> <public.bin> | send <endpoint> <seed> <test-mode>".into()),
    }
}
