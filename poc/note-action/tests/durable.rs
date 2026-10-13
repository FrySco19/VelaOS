use ed25519_dalek::{Signer, SigningKey};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    time::{SystemTime, UNIX_EPOCH},
};
use vela_note_action::*;
use vela_selection_contract::UserSelection;
use vela_storage_core::{BlockDevice, EXTENT_SECTORS, MAX_BYTES, ObjectStore, SECTOR};

struct Disk(File);
impl BlockDevice for Disk {
    type Error = std::io::Error;
    fn read(&mut self, sector: u64, out: &mut [u8; SECTOR]) -> Result<(), Self::Error> {
        self.0.seek(SeekFrom::Start(sector * SECTOR as u64))?;
        self.0.read_exact(out)
    }
    fn write(&mut self, sector: u64, bytes: &[u8; SECTOR]) -> Result<(), Self::Error> {
        self.0.seek(SeekFrom::Start(sector * SECTOR as u64))?;
        self.0.write_all(bytes)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.sync_all()
    }
}

#[test]
fn wasm_commit_survives_reopen_and_rejects_tampering() {
    // Deterministic keys are TEST ONLY. No deployed domain uses these roots.
    let broker = SigningKey::from_bytes(&[1; 32]);
    let pcb = SigningKey::from_bytes(&[2; 32]);
    let lfs = SigningKey::from_bytes(&[3; 32]);
    let session = [4; 32];
    let op = Operation::new(1, 1, b"Real Wasm note, committed to a dedicated test disk").unwrap();
    let mut approval = UserSelection {
        session_binding: session,
        revision: 1,
        plan_sha256: digest(&op.manifest()),
        signature: [0; 64],
    };
    approval.signature = broker.sign(&approval.signing_bytes()).to_bytes();
    let cap = Capability::issue(
        &op,
        &approval,
        &session,
        &broker.verifying_key().to_bytes(),
        100,
        MAX_LEASE_NS,
        &pcb,
    )
    .unwrap();
    let path = std::env::temp_dir().join(format!(
        "vela-note-{}-{}.img",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.set_len(EXTENT_SECTORS * SECTOR as u64).unwrap();
    let mut store = ObjectStore::open(Disk(file)).unwrap();
    static CLOCK_READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    fn expires_during_execution() -> u64 {
        if CLOCK_READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            101
        } else {
            MAX_LEASE_NS + 101
        }
    }
    assert!(
        execute_timed(
            &mut store,
            &op,
            &cap,
            expires_during_execution,
            &pcb.verifying_key(),
            &lfs
        )
        .is_err()
    );
    assert!(store.version().is_none());
    let receipt = execute(&mut store, &op, &cap, 101, &pcb.verifying_key(), &lfs).unwrap();
    receipt.verify(&op, &cap, &lfs.verifying_key()).unwrap();
    drop(store);
    let mut store = ObjectStore::open(Disk(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap(),
    ))
    .unwrap();
    let mut bytes = [0; MAX_BYTES];
    let n = store.read(object_id(), &mut bytes).unwrap();
    assert_eq!(&bytes[4 + RECEIPT_SIZE..n], op.note());
    let mut recovered = [0; MAX_NOTE];
    let recovered_len = recover(&mut store, &lfs.verifying_key(), &mut recovered).unwrap();
    assert_eq!(&recovered[..recovered_len], op.note());
    let version = store.version().unwrap().generation;
    let mut forged = Capability(cap.0);
    forged.0[26] ^= 1;
    assert!(matches!(
        execute(&mut store, &op, &forged, 102, &pcb.verifying_key(), &lfs),
        Err(Error::Capability)
    ));
    assert!(matches!(
        execute(
            &mut store,
            &op,
            &cap,
            MAX_LEASE_NS + 101,
            &pcb.verifying_key(),
            &lfs
        ),
        Err(Error::Expired)
    ));
    assert!(execute(&mut store, &op, &cap, 103, &pcb.verifying_key(), &lfs).is_err());
    assert_eq!(store.version().unwrap().generation, version);
    drop(store);
    std::fs::remove_file(path).unwrap();
}
