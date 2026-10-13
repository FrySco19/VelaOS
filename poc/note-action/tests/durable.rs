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
    let nonce = [9; 32];
    let mut index = [0; sync::MAX_RESPONSE];
    let empty = sync::response(&mut store, &lfs, 77, &nonce, &mut index).unwrap();
    assert!(
        sync::verify_response(77, &nonce, &index[..empty], &lfs.verifying_key())
            .unwrap()
            .is_none()
    );
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
    let (request_bytes, request_len) = sync::request_signing(88, &nonce);
    let mut read_grant = [0; sync::REQUEST_SIZE];
    read_grant[..32].copy_from_slice(&nonce);
    read_grant[32..].copy_from_slice(&broker.sign(&request_bytes[..request_len]).to_bytes());
    assert_eq!(
        sync::verify_request(88, &read_grant, &broker.verifying_key()).unwrap(),
        nonce
    );
    read_grant[40] ^= 1;
    assert!(sync::verify_request(88, &read_grant, &broker.verifying_key()).is_err());
    let n = sync::response(&mut store, &lfs, 88, &nonce, &mut index).unwrap();
    let entry = sync::verify_response(88, &nonce, &index[..n], &lfs.verifying_key())
        .unwrap()
        .unwrap();
    assert_eq!(entry.note.as_bytes(), op.note());
    assert_eq!(entry.receipt.0, receipt.0);
    assert!(sync::verify_response(89, &nonce, &index[..n], &lfs.verifying_key()).is_err());
    assert!(sync::verify_response(88, &[8; 32], &index[..n], &lfs.verifying_key()).is_err());
    index[n - 1] ^= 1;
    assert!(sync::verify_response(88, &nonce, &index[..n], &lfs.verifying_key()).is_err());
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

#[test]
fn immutable_documents_reopen_and_stale_update_is_denied() {
    let broker = SigningKey::from_bytes(&[21; 32]);
    let pcb = SigningKey::from_bytes(&[22; 32]);
    let lfs = SigningKey::from_bytes(&[23; 32]);
    let path = std::env::temp_dir().join(format!(
        "vela-archive-{}-{}.img",
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
    file.set_len(vela_storage_core::ARCHIVE_SECTORS * 512)
        .unwrap();
    let mut store = ObjectStore::open_archive(Disk(file)).unwrap();
    let a = Operation::document(1, 100, [0; 16], [0; 16], b"first document").unwrap();
    let b = Operation::document(1, 101, [0; 16], [0; 16], b"second document").unwrap();
    let c = Operation::document(1, 102, a.root, a.object(), b"first document revised").unwrap();
    let stale = Operation::document(1, 103, a.root, a.object(), b"stale writer").unwrap();
    let issue = |op: &Operation| {
        let mut selection = UserSelection {
            session_binding: [9; 32],
            revision: op.request,
            plan_sha256: digest(&op.manifest()),
            signature: [0; 64],
        };
        selection.signature = broker.sign(&selection.signing_bytes()).to_bytes();
        Capability::issue(
            op,
            &selection,
            &[9; 32],
            &broker.verifying_key().to_bytes(),
            0,
            MAX_LEASE_NS,
            &pcb,
        )
        .unwrap()
    };
    for op in [&a, &b, &c] {
        execute(&mut store, op, &issue(op), 1, &pcb.verifying_key(), &lfs).unwrap();
    }
    assert!(
        execute(
            &mut store,
            &stale,
            &issue(&stale),
            1,
            &pcb.verifying_key(),
            &lfs
        )
        .is_err()
    );
    let wrong = Operation::document(1, 104, b.root, a.object(), b"wrong document").unwrap();
    assert!(
        execute(
            &mut store,
            &wrong,
            &issue(&wrong),
            1,
            &pcb.verifying_key(),
            &lfs
        )
        .is_err()
    );
    drop(store);
    let mut store = ObjectStore::open_archive(Disk(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap(),
    ))
    .unwrap();
    for op in [&a, &b, &c] {
        let mut record = [0; MAX_BYTES];
        let n = store.read(op.object(), &mut record).unwrap();
        let (root, parent, r, note) = document_record(&record[..n], &lfs.verifying_key()).unwrap();
        assert_eq!(root, op.root);
        assert_eq!(parent, op.parent);
        assert_eq!(note, op.note());
        r.verify(op, &issue(op), &lfs.verifying_key()).unwrap();
        let nonce = [7; 32];
        let (signed, n) = catalog::request_bytes(true, 900, &nonce, &op.object());
        let mut request = [0; catalog::REQUEST_SIZE];
        request[..32].copy_from_slice(&nonce);
        request[32..48].copy_from_slice(&op.object());
        request[48..].copy_from_slice(&broker.sign(&signed[..n]).to_bytes());
        assert!(catalog::verify_request(true, 900, &request, &broker.verifying_key()).is_ok());
        assert!(catalog::verify_request(false, 900, &request, &broker.verifying_key()).is_err());
        let mut response = [0; catalog::RESPONSE_SIZE];
        let n = catalog::response(
            &mut store,
            &lfs,
            true,
            900,
            &nonce,
            &op.object(),
            &mut response,
        )
        .unwrap();
        let (_, entry) =
            catalog::verify_response(true, 900, &nonce, &response[..n], &lfs.verifying_key())
                .unwrap();
        assert_eq!(entry.unwrap().note.as_bytes(), op.note());
        response[41] ^= 1;
        assert!(
            catalog::verify_response(true, 900, &nonce, &response[..n], &lfs.verifying_key())
                .is_err()
        );
    }
    assert!(execute(&mut store, &a, &issue(&a), 1, &pcb.verifying_key(), &lfs).is_err());
    let mut redirected =
        Operation::document(1, 105, a.root, c.object(), b"redirect attempt").unwrap();
    let bound = issue(&redirected);
    redirected.root = b.root;
    assert!(
        bound
            .verify_binding(&redirected, &pcb.verifying_key())
            .is_err()
    );
    let mut target = [0; 16];
    let mut count = 0;
    loop {
        let nonce = [6; 32];
        let mut response = [0; catalog::RESPONSE_SIZE];
        let n = catalog::response(&mut store, &lfs, false, 901, &nonce, &target, &mut response)
            .unwrap();
        let (next, entry) =
            catalog::verify_response(false, 901, &nonce, &response[..n], &lfs.verifying_key())
                .unwrap();
        if entry.is_some() {
            count += 1;
        }
        if next == 64 {
            break;
        }
        target[..8].copy_from_slice(&next.to_le_bytes());
    }
    assert_eq!(count, 3);
    drop(store);
    std::fs::remove_file(path).unwrap();
}
