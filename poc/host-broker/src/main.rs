//! Native development broker. `serve` performs consent and receipt verification.
//! All signing requires a verified USB hardware assertion.
//! Local plaintext seeds are development custody, not a TPM/FIDO2 enclave.
mod hello;
mod intent;
mod socket;
use ed25519_compact::{KeyPair, Seed};

use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("serve") if args.len()==6 => socket::serve(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&args[5])).map_err(Into::into),
        Some("enroll-fido") if args.len()==3 => hello::enroll(Path::new(&args[2])).map_err(Into::into),
        Some("keygen") if args.len() == 4 => keygen(Path::new(&args[2]), Path::new(&args[3])),
        Some("public") if args.len() == 4 => {
            let seed:[u8;32]=fs::read(&args[2])?.try_into().map_err(|_|"seed length")?;
            let pair=KeyPair::from_seed(Seed::new(seed));
            let mut file=OpenOptions::new().write(true).create_new(true).open(&args[3])?;
            file.write_all(pair.pk.as_ref())?;file.sync_all()?;Ok(())
        },

        _ => Err("usage: serve <broker.seed> <pcb.public> <lfs.public> <fido.json> | enroll-fido <fido.json> | keygen <seed> <public.rs> | public <seed> <public.bin>".into()),
    }
}
