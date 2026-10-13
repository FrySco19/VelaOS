use vela_storage_core::*;
#[derive(Clone)]
struct Disk {
    bytes: Vec<[u8; SECTOR]>,
    fail: Option<(usize, usize)>,
    writes: usize,
}
impl BlockDevice for Disk {
    type Error = ();
    fn read(&mut self, s: u64, out: &mut [u8; SECTOR]) -> Result<(), ()> {
        *out = *self.bytes.get(s as usize).ok_or(())?;
        Ok(())
    }
    fn write(&mut self, s: u64, b: &[u8; SECTOR]) -> Result<(), ()> {
        let dst = self.bytes.get_mut(s as usize).ok_or(())?;
        let index = self.writes;
        self.writes += 1;
        if let Some((at, prefix)) = self.fail {
            if index == at {
                dst[..prefix].copy_from_slice(&b[..prefix]);
                return Err(());
            }
        }
        *dst = *b;
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
}
#[test]
fn torn_append_never_destroys_previous_objects() {
    let disk = Disk {
        bytes: vec![[0; SECTOR]; ARCHIVE_SECTORS as usize],
        fail: None,
        writes: 0,
    };
    let mut store = ObjectStore::open_archive(disk).unwrap();
    store.save([1; 16], 0, b"original").unwrap();
    let base = store.into_device();
    for write in 0..5 {
        for prefix in [0, 1, 127, 511, 512] {
            let mut disk = base.clone();
            disk.fail = Some((write, prefix));
            disk.writes = 0;
            let mut store = ObjectStore::open_archive(disk).unwrap();
            let result = store.save([2; 16], 1, b"new immutable version");
            let mut disk = store.into_device();
            disk.fail = None;
            let mut reopened = ObjectStore::open_archive(disk).unwrap();
            let mut out = [0; MAX_BYTES];
            let n = reopened.read([1; 16], &mut out).unwrap();
            assert_eq!(&out[..n], b"original");
            let n = reopened.read([2; 16], &mut out).unwrap();
            assert!(n == 0 || &out[..n] == b"new immutable version");
            if result.is_ok() {
                assert_ne!(n, 0);
            }
        }
    }
}
#[test]
fn archive_capacity_and_duplicate_ids_fail_without_overwrite() {
    let disk = Disk {
        bytes: vec![[0; SECTOR]; ARCHIVE_SECTORS as usize],
        fail: None,
        writes: 0,
    };
    let mut store = ObjectStore::open_archive(disk).unwrap();
    for i in 0..ARCHIVE_SLOTS {
        let mut id = [0; 16];
        id[..8].copy_from_slice(&(i + 1).to_le_bytes());
        store.save(id, i, b"note").unwrap();
    }
    assert!(store.save([255; 16], ARCHIVE_SLOTS, b"overflow").is_err());
    let mut id = [0; 16];
    id[0] = 1;
    assert!(store.save(id, ARCHIVE_SLOTS, b"overwrite").is_err());
    let mut out = [0; MAX_BYTES];
    assert_eq!(store.read(id, &mut out).unwrap(), 4);
    assert_eq!(&out[..4], b"note");
}
