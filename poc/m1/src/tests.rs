use super::*;
fn setup() -> Result<(tempfile::TempDir, Lfs, Vec<u8>)> {
    let dir = tempfile::tempdir()?;
    let lfs = Lfs::unlock(
        &dir.path().join("test.redb"),
        fixture::COLLECTION,
        fixture::root().public(),
    )?;
    Ok((
        dir,
        lfs,
        fixture::token("alice", &["read", "write", "list"])?,
    ))
}
#[test]
fn canonical_dedup_replay_and_pages() -> Result<()> {
    let (dir, mut lfs, token) = setup()?;
    let value = fixture::payload(0);
    let a = lfs.put(&token, "alice", "one", &value)?;
    let reordered: Value =
        serde_json::from_str(r#"{"ssid":"M1-0","dns":"9.9.9.9","schema":"vela.net.config.v1"}"#)?;
    assert_eq!(a, lfs.put(&token, "alice", "one", &reordered)?);
    assert!(
        lfs.put(&token, "alice", "one", &fixture::payload(1))
            .is_err()
    );
    let b = lfs.put(&token, "alice", "two", &value)?;
    assert_eq!(a.oid, b.oid);
    assert_eq!(lfs.object_count(), 1);
    assert_eq!(lfs.commit_count(), 2);
    drop(lfs);
    let mut lfs = Lfs::unlock(
        &dir.path().join("test.redb"),
        fixture::COLLECTION,
        fixture::root().public(),
    )?;
    assert_eq!(a, lfs.put(&token, "alice", "one", &value)?);
    let c = lfs.put(&token, "alice", "three", &fixture::payload(2))?;
    assert_eq!(lfs.list(&token, "alice", 0, 1)?[0].oid, a.oid);
    assert_eq!(lfs.list(&token, "alice", a.commit_id, 1)?[0].oid, c.oid);
    let handle = lfs.handle(&token, "alice", &a.oid, 120)?;
    assert_eq!(&*lfs.open(&token, "alice", &handle)?, &canonical(&value)?);
    drop(lfs);
    let lfs = Lfs::unlock(
        &dir.path().join("test.redb"),
        fixture::COLLECTION,
        fixture::root().public(),
    )?;
    assert!(lfs.open(&token, "alice", &handle).is_err());
    drop(lfs);
    assert!(
        Lfs::unlock(
            &dir.path().join("test.redb"),
            [0; 32],
            fixture::root().public()
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn authorization_handles_and_attenuation() -> Result<()> {
    let (_dir, mut lfs, token) = setup()?;
    let result = lfs.put(&token, "alice", "one", &fixture::payload(0))?;
    let mut handle = lfs.handle(&token, "alice", &result.oid, 120)?;
    let read = fixture::token("alice", &["read"])?;
    assert!(
        lfs.put(&read, "alice", "two", &fixture::payload(1))
            .is_err()
    );
    assert!(lfs.list(&read, "alice", 0, 10).is_err());
    assert!(lfs.open(&token, "bob", &handle).is_err());
    assert!(lfs.handle(&token, "alice", &result.oid, 121).is_err());
    handle.mac[0] ^= 1;
    assert!(lfs.open(&token, "alice", &handle).is_err());
    handle.mac[0] ^= 1;
    handle.exp = 0;
    handle.mac = lfs.handle_mac(&handle)?;
    assert!(lfs.open(&token, "alice", &handle).is_err());
    let mut monotonic = lfs.handle(&token, "alice", &result.oid, 120)?;
    monotonic.deadline_ms = 0;
    monotonic.mac = lfs.handle_mac(&monotonic)?;
    assert!(lfs.open(&token, "alice", &monotonic).is_err());
    use biscuit_auth::builder::BlockBuilder;
    let reduced = Biscuit::from(&token, fixture::root().public())?
        .append(BlockBuilder::new().code("check if operation(\"read\");")?)?
        .to_vec()?;
    assert!(
        lfs.put(&reduced, "alice", "three", &fixture::payload(2))
            .is_err()
    );
    let valid = lfs.handle(&reduced, "alice", &result.oid, 120)?;
    assert!(lfs.open(&reduced, "alice", &valid).is_ok());
    let expired = Biscuit::from(&token, fixture::root().public())?
        .append(BlockBuilder::new().code("check if time($t), $t < 2000-01-01T00:00:00Z;")?)?
        .to_vec()?;
    assert!(lfs.handle(&expired, "alice", &result.oid, 120).is_err());
    let mut forged = token;
    forged[10] ^= 1;
    assert!(lfs.handle(&forged, "alice", &result.oid, 120).is_err());
    Ok(())
}
#[test]
fn corruption_fails_closed() -> Result<()> {
    let (dir, mut lfs, token) = setup()?;
    let result = lfs.put(&token, "alice", "one", &fixture::payload(123))?;
    let handle = lfs.handle(&token, "alice", &result.oid, 120)?;
    let tx = lfs.db.begin_write()?;
    {
        let mut table = tx.open_table(BLOBS)?;
        let mut bytes = table.get(result.oid.as_str())?.unwrap().value().to_vec();
        assert!(!String::from_utf8_lossy(&bytes).contains("M1-123"));
        let pos = bytes.len() / 2;
        bytes[pos] ^= 1;
        table.insert(result.oid.as_str(), bytes.as_slice())?;
    }
    tx.commit()?;
    assert!(lfs.open(&token, "alice", &handle).is_err());
    drop(lfs);
    assert!(
        Lfs::unlock(
            &dir.path().join("test.redb"),
            fixture::COLLECTION,
            fixture::root().public()
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn signed_log_tampering_fails_closed() -> Result<()> {
    let (dir, mut lfs, token) = setup()?;
    lfs.put(&token, "alice", "one", &fixture::payload(0))?;
    let tx = lfs.db.begin_write()?;
    {
        let mut table = tx.open_table(LOG)?;
        let mut s: Signed = serde_json::from_slice(table.get(1)?.unwrap().value())?;
        s.signature[0] ^= 1;
        let bytes = canonical(&s)?;
        table.insert(1, bytes.as_slice())?;
    }
    tx.commit()?;
    drop(lfs);
    assert!(
        Lfs::unlock(
            &dir.path().join("test.redb"),
            fixture::COLLECTION,
            fixture::root().public()
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn password_derivation_and_schema_rejection() -> Result<()> {
    assert_eq!(
        *derive_collection_key(b"synthetic-test-password", &[1; 16])?,
        *derive_collection_key(b"synthetic-test-password", &[1; 16])?
    );
    let (_dir, mut lfs, token) = setup()?;
    let mut value = fixture::payload(0);
    value["unexpected"] = Value::Bool(true);
    assert!(lfs.put(&token, "alice", "one", &value).is_err());
    assert_eq!(lfs.commit_count(), 0);
    Ok(())
}

#[test]
fn multi_chunk_crypto_dedup_and_aad() -> Result<()> {
    let (_dir, lfs, _token) = setup()?;
    let plain = vec![b'a'; 256 * 1024];
    let object = blake3::hash(&plain).to_hex().to_string();
    let encoded = encode_blob(&lfs.collection, &object, &plain)?;
    let blob: Blob = serde_json::from_slice(&encoded)?;
    assert!(blob.order.len() > 1 && blob.chunks.len() < blob.order.len());
    let tx = lfs.db.begin_write()?;
    {
        tx.open_table(BLOBS)?
            .insert(object.as_str(), encoded.as_slice())?;
    }
    tx.commit()?;
    let entry = Entry {
        seq: 1,
        oid: object,
        kind: Kind::NetConfig,
        caller: "test".into(),
        idem: "test".into(),
        time: 0,
        blob_digest: blake3::hash(&encoded).to_hex().to_string(),
        card: extract(&fixture::payload(0))?,
    };
    assert_eq!(&*lfs.plaintext(&entry)?, &plain);
    assert!(decrypt(&lfs.collection, &blob.wrapped_key, b"wrong-object-aad").is_err());
    let mut corrupt = blob.wrapped_key;
    corrupt.ciphertext[0] ^= 1;
    assert!(
        decrypt(
            &lfs.collection,
            &corrupt,
            format!("wrap:{}", entry.oid).as_bytes()
        )
        .is_err()
    );
    Ok(())
}
