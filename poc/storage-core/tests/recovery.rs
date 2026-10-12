use vela_storage_core::*;

#[derive(Clone)]
struct Disk {
    bytes: [u8; 4096],
    writes: usize,
    fail: Option<(usize, usize)>,
    flushes: usize,
    fail_flush: Option<usize>,
}
impl Default for Disk {
    fn default() -> Self {
        Self {
            bytes: [0; 4096],
            writes: 0,
            fail: None,
            flushes: 0,
            fail_flush: None,
        }
    }
}
impl BlockDevice for Disk {
    type Error = ();
    fn read(&mut self, n: u64, out: &mut [u8; SECTOR]) -> Result<(), ()> {
        out.copy_from_slice(&self.bytes[n as usize * SECTOR..(n as usize + 1) * SECTOR]);
        Ok(())
    }
    fn write(&mut self, n: u64, data: &[u8; SECTOR]) -> Result<(), ()> {
        let fail = self.fail.filter(|(w, _)| *w == self.writes);
        self.writes += 1;
        let count = fail.map_or(SECTOR, |(_, count)| count);
        self.bytes[n as usize * SECTOR..n as usize * SECTOR + count]
            .copy_from_slice(&data[..count]);
        if fail.is_some() { Err(()) } else { Ok(()) }
    }
    fn flush(&mut self) -> Result<(), ()> {
        let fail = self.fail_flush == Some(self.flushes);
        self.flushes += 1;
        if fail { Err(()) } else { Ok(()) }
    }
}
const HANDLE: [u8; 16] = [7; 16];
fn committed() -> Disk {
    let mut s = ObjectStore::open(Disk::default()).unwrap();
    s.save(HANDLE, 0, b"first draft").unwrap();
    // Exercise reuse too: new update invalidates a previously committed slot.
    s.save(HANDLE, 1, b"second draft").unwrap();
    let mut d = s.into_device();
    d.writes = 0;
    d.flushes = 0;
    d
}

#[test]
fn every_torn_sector_prefix_recovers_a_complete_version() {
    let base = committed();
    for write in 0..5 {
        for prefix in 0..=SECTOR {
            let mut disk = base.clone();
            disk.fail = Some((write, prefix));
            let mut store = ObjectStore::open(disk).unwrap();
            assert_eq!(store.save(HANDLE, 2, b"third draft"), Err(Error::Io(())));
            assert_eq!(store.save(HANDLE, 2, b"retry"), Err(Error::Poisoned));
            let mut recovered = ObjectStore::open(store.into_device()).unwrap();
            let mut out = [0; MAX_BYTES];
            let len = recovered.read(HANDLE, &mut out).unwrap();
            assert!(&out[..len] == b"second draft" || &out[..len] == b"third draft");
            if recovered.version().unwrap().generation == 3 {
                assert_eq!(&out[..len], b"third draft");
            } else {
                assert_eq!(recovered.version().unwrap().generation, 2);
                assert_eq!(&out[..len], b"second draft");
            }
        }
    }
}

#[test]
fn flush_failure_is_ambiguous_and_poisoned() {
    for flush in 0..3 {
        let mut disk = committed();
        disk.fail_flush = Some(flush);
        let mut store = ObjectStore::open(disk).unwrap();
        assert_eq!(store.save(HANDLE, 2, b"next"), Err(Error::Io(())));
        assert_eq!(
            store.read(HANDLE, &mut [0; MAX_BYTES]),
            Err(Error::Poisoned)
        );
        let recovered = ObjectStore::open(store.into_device()).unwrap();
        assert!([2, 3].contains(&recovered.version().unwrap().generation));
    }
}

#[test]
fn bounds_scope_generation_and_corruption_are_checked() {
    let mut store = ObjectStore::open(committed()).unwrap();
    assert_eq!(store.save([8; 16], 2, b"bad"), Err(Error::Conflict));
    assert_eq!(store.save(HANDLE, 1, b"stale"), Err(Error::Conflict));
    assert_eq!(
        store.save(HANDLE, 2, &[0; MAX_BYTES + 1]),
        Err(Error::TooLarge)
    );
    assert_eq!(store.read(HANDLE, &mut [0; 1]), Err(Error::TooLarge));
    store.save(HANDLE, 2, &[]).unwrap();
    assert_eq!(store.read(HANDLE, &mut []), Ok(0));
    let mut disk = committed();
    disk.bytes[5 * SECTOR] ^= 1;
    assert!(matches!(ObjectStore::open(disk), Err(Error::Corrupt)));
    let mut disk = committed();
    disk.bytes[0] ^= 1;
    disk.bytes[4 * SECTOR] ^= 1;
    assert!(matches!(ObjectStore::open(disk), Err(Error::Corrupt)));
}

// A volatile write-back adapter: unflushed bytes are lost after a crash.
struct CachedDisk {
    durable: Disk,
    pending: Disk,
    flushes: usize,
    cut: usize,
}
impl BlockDevice for CachedDisk {
    type Error = ();
    fn read(&mut self, n: u64, out: &mut [u8; SECTOR]) -> Result<(), ()> {
        self.pending.read(n, out)
    }
    fn write(&mut self, n: u64, bytes: &[u8; SECTOR]) -> Result<(), ()> {
        self.pending.write(n, bytes)
    }
    fn flush(&mut self) -> Result<(), ()> {
        if self.flushes == self.cut {
            return Err(());
        }
        self.flushes += 1;
        self.durable = self.pending.clone();
        Ok(())
    }
}

#[test]
fn losing_unflushed_writeback_data_preserves_previous_commit() {
    for cut in 0..3 {
        let disk = committed();
        let cache = CachedDisk {
            durable: disk.clone(),
            pending: disk,
            flushes: 0,
            cut,
        };
        let mut s = ObjectStore::open(cache).unwrap();
        assert_eq!(s.save(HANDLE, 2, b"unacknowledged"), Err(Error::Io(())));
        let mut recovered = ObjectStore::open(s.into_device().durable).unwrap();
        let mut bytes = [0; MAX_BYTES];
        let n = recovered.read(HANDLE, &mut bytes).unwrap();
        assert_eq!(&bytes[..n], b"second draft");
        assert_eq!(recovered.version().unwrap().generation, 2);
    }
}

struct FileDisk(std::fs::File);
impl BlockDevice for FileDisk {
    type Error = std::io::Error;
    fn read(&mut self, n: u64, out: &mut [u8; SECTOR]) -> Result<(), Self::Error> {
        use std::io::{Read, Seek, SeekFrom};
        self.0.seek(SeekFrom::Start(n * SECTOR as u64))?;
        self.0.read_exact(out)
    }
    fn write(&mut self, n: u64, bytes: &[u8; SECTOR]) -> Result<(), Self::Error> {
        use std::io::{Seek, SeekFrom, Write};
        self.0.seek(SeekFrom::Start(n * SECTOR as u64))?;
        self.0.write_all(bytes)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.sync_data()
    }
}

#[test]
fn child_process_storage_probe() {
    let Ok(path) = std::env::var("VELA_STORAGE_PROBE") else {
        return;
    };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut s = ObjectStore::open(FileDisk(file)).unwrap();
    match std::env::var("VELA_STORAGE_ACTION").unwrap().as_str() {
        "save" => {
            s.save(HANDLE, 0, b"IPC bridge design survives process restart")
                .unwrap();
        }
        "read" => {
            let mut out = [0; MAX_BYTES];
            let n = s.read(HANDLE, &mut out).unwrap();
            assert_eq!(&out[..n], b"IPC bridge design survives process restart");
            assert_eq!(s.version().unwrap().generation, 1);
        }
        _ => panic!("unknown probe action"),
    }
}

#[test]
fn real_file_survives_writer_process_exit_and_reader_process_restart() {
    let path = std::env::temp_dir().join(format!("vela-storage-{}.img", std::process::id()));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(EXTENT_SECTORS * SECTOR as u64).unwrap();
    drop(file);
    for action in ["save", "read"] {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_process_storage_probe", "--nocapture"])
            .env("VELA_STORAGE_PROBE", &path)
            .env("VELA_STORAGE_ACTION", action)
            .status()
            .unwrap();
        assert!(result.success());
    }
    std::fs::remove_file(path).unwrap();
}
