//! Bounded no_std storage primitive, not an LFS authority or signed receipt.
//! The device MUST be a dedicated extent supplied by trusted configuration.
//! Never point it at an arbitrary disk: sector zero is written by this format.
#![no_std]

pub const SECTOR: usize = 512;
pub const DATA_SECTORS: usize = 3;
pub const MAX_BYTES: usize = SECTOR * DATA_SECTORS;
pub const EXTENT_SECTORS: u64 = 2 * (1 + DATA_SECTORS as u64);
pub const ARCHIVE_SLOTS: u64 = 64;
pub const ARCHIVE_SECTORS: u64 = ARCHIVE_SLOTS * 4;
const MAGIC: &[u8; 8] = b"VOBJ0001";

/// Adapter contract: writes may tear. Successful flush must order and persist
/// ALL previous writes, and reads must report device errors rather than zeros.
/// A lying device or corrupt media is outside the crash consistency guarantee.
pub trait BlockDevice {
    type Error;
    fn read(&mut self, sector: u64, out: &mut [u8; SECTOR]) -> Result<(), Self::Error>;
    fn write(&mut self, sector: u64, bytes: &[u8; SECTOR]) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
    fn read_block(&mut self, sector: u64, out: &mut [u8; SECTOR]) -> Result<(), Self::Error> {
        self.read(sector, out)
    }
    fn write_block(&mut self, sector: u64, bytes: &[u8; SECTOR]) -> Result<(), Self::Error> {
        self.write(sector, bytes)
    }
    /// Block-device fsync: stable completion, not a host filesystem call.
    fn fsync(&mut self) -> Result<(), Self::Error> {
        self.flush()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Io(E),
    Corrupt,
    TooLarge,
    Conflict,
    SequenceExhausted,
    Poisoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub generation: u64,
    /// Storage slot identity, NOT the content-addressed LFS OID. The future
    /// LFS adapter stores immutable BLAKE3-addressed objects above this layer.
    pub object: [u8; 16],
    pub len: usize,
}

struct Candidate {
    version: Version,
    checksum: u32,
    slot: u64,
}

fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(c & 1));
        }
    }
    !c
}

/// One opaque object, two alternating snapshots; no heap or POSIX dependency.
/// This is NOT the admission ledger: it does not solve cross-boot token replay.
pub struct ObjectStore<D: BlockDevice> {
    device: D,
    current: Option<Candidate>,
    poisoned: bool,
    archive: bool,
}

impl<D: BlockDevice> ObjectStore<D> {
    pub fn open(mut device: D) -> Result<Self, Error<D::Error>> {
        let mut selected: Option<Candidate> = None;
        let mut damaged_header = false;
        for slot in 0..2 {
            let mut h = [0; SECTOR];
            device.read(slot * 4, &mut h).map_err(Error::Io)?;
            if h.iter().all(|b| *b == 0) {
                continue;
            }
            // A torn publication or interrupted overwrite is not committed.
            if &h[..8] != MAGIC
                || crc(&h[..508]) != u32::from_le_bytes(h[508..].try_into().unwrap())
            {
                damaged_header = true;
                continue;
            }
            let generation = u64::from_le_bytes(h[8..16].try_into().unwrap());
            let len = u32::from_le_bytes(h[32..36].try_into().unwrap()) as usize;
            if generation == 0 || len > MAX_BYTES || h[40..508].iter().any(|b| *b != 0) {
                return Err(Error::Corrupt);
            }
            let object = h[16..32].try_into().unwrap();
            let checksum = u32::from_le_bytes(h[36..40].try_into().unwrap());
            let mut data = [0; MAX_BYTES];
            for i in 0..DATA_SECTORS {
                let mut sector = [0; SECTOR];
                device
                    .read(slot * 4 + 1 + i as u64, &mut sector)
                    .map_err(Error::Io)?;
                data[i * SECTOR..(i + 1) * SECTOR].copy_from_slice(&sector);
            }
            // A valid commit marker with damaged payload is media corruption,
            // not a torn uncommitted update: fail closed instead of rolling back.
            if crc(&data[..len]) != checksum || data[len..].iter().any(|b| *b != 0) {
                return Err(Error::Corrupt);
            }
            let candidate = Candidate {
                version: Version {
                    generation,
                    object,
                    len,
                },
                checksum,
                slot,
            };
            if let Some(ref old) = selected
                && (old.version.object != object
                    || old.version.generation.abs_diff(generation) != 1)
            {
                return Err(Error::Corrupt);
            }
            if selected
                .as_ref()
                .is_none_or(|v| generation > v.version.generation)
            {
                selected = Some(candidate);
            }
        }
        if selected.is_none() && damaged_header {
            return Err(Error::Corrupt);
        }
        Ok(Self {
            device,
            current: selected,
            poisoned: false,
            archive: false,
        })
    }

    /// Dedicated append-only extent. Never overwrites a published record.
    pub fn open_archive(device: D) -> Result<Self, Error<D::Error>> {
        let mut store = Self {
            device,
            current: None,
            poisoned: false,
            archive: true,
        };
        let mut previous = 0;
        for slot in 0..ARCHIVE_SLOTS {
            if let Some(c) = store.candidate(slot)? {
                if c.version.generation > previous {
                    previous = c.version.generation;
                    store.current = Some(c);
                }
            }
        }
        Ok(store)
    }
    fn candidate(&mut self, slot: u64) -> Result<Option<Candidate>, Error<D::Error>> {
        let mut h = [0; SECTOR];
        self.device.read(slot * 4, &mut h).map_err(Error::Io)?;
        if h.iter().all(|b| *b == 0) {
            return Ok(None);
        }
        // An unpublished/torn final header consumes no committed generation.
        if &h[..8] != MAGIC || crc(&h[..508]) != u32::from_le_bytes(h[508..].try_into().unwrap()) {
            return Ok(None);
        }
        let generation = u64::from_le_bytes(h[8..16].try_into().unwrap());
        let len = u32::from_le_bytes(h[32..36].try_into().unwrap()) as usize;
        if generation == 0 || len > MAX_BYTES || h[40..508].iter().any(|b| *b != 0) {
            return Err(Error::Corrupt);
        }
        let checksum = u32::from_le_bytes(h[36..40].try_into().unwrap());
        let mut data = [0; MAX_BYTES];
        for i in 0..DATA_SECTORS {
            let mut b = [0; SECTOR];
            self.device
                .read(slot * 4 + 1 + i as u64, &mut b)
                .map_err(Error::Io)?;
            data[i * SECTOR..(i + 1) * SECTOR].copy_from_slice(&b);
        }
        if crc(&data[..len]) != checksum || data[len..].iter().any(|b| *b != 0) {
            return Err(Error::Corrupt);
        }
        Ok(Some(Candidate {
            version: Version {
                generation,
                object: h[16..32].try_into().unwrap(),
                len,
            },
            checksum,
            slot,
        }))
    }
    pub fn entry(&mut self, slot: u64) -> Result<Option<Version>, Error<D::Error>> {
        if !self.archive || slot >= ARCHIVE_SLOTS {
            return Err(Error::Conflict);
        }
        Ok(self.candidate(slot)?.map(|c| c.version))
    }
    pub fn is_archive(&self) -> bool {
        self.archive
    }
    pub fn version(&self) -> Option<Version> {
        self.current.as_ref().map(|v| v.version)
    }

    /// Read only the configured opaque object. An undersized buffer is refused.
    pub fn read(&mut self, object: [u8; 16], out: &mut [u8]) -> Result<usize, Error<D::Error>> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        if self.archive {
            for slot in (0..ARCHIVE_SLOTS).rev() {
                if let Some(c) = self.candidate(slot)? {
                    if c.version.object == object {
                        if out.len() < c.version.len {
                            return Err(Error::TooLarge);
                        }
                        let mut data = [0; MAX_BYTES];
                        for i in 0..DATA_SECTORS {
                            let mut b = [0; SECTOR];
                            self.device
                                .read(slot * 4 + 1 + i as u64, &mut b)
                                .map_err(Error::Io)?;
                            data[i * SECTOR..(i + 1) * SECTOR].copy_from_slice(&b);
                        }
                        out[..c.version.len].copy_from_slice(&data[..c.version.len]);
                        return Ok(c.version.len);
                    }
                }
            }
            return Ok(0);
        }
        let Some(ref c) = self.current else {
            return Ok(0);
        };
        if c.version.object != object {
            return Err(Error::Conflict);
        }
        if out.len() < c.version.len {
            return Err(Error::TooLarge);
        }
        let mut data = [0; MAX_BYTES];
        for i in 0..DATA_SECTORS {
            let mut sector = [0; SECTOR];
            self.device
                .read(c.slot * 4 + 1 + i as u64, &mut sector)
                .map_err(Error::Io)?;
            data[i * SECTOR..(i + 1) * SECTOR].copy_from_slice(&sector);
        }
        if crc(&data[..c.version.len]) != c.checksum
            || data[c.version.len..].iter().any(|b| *b != 0)
        {
            return Err(Error::Corrupt);
        }
        out[..c.version.len].copy_from_slice(&data[..c.version.len]);
        Ok(c.version.len)
    }

    /// Compare-and-swap by generation. Caller must authorize before invoking.
    /// On I/O failure the result is ambiguous; reopen and inspect before retry.
    pub fn save(
        &mut self,
        object: [u8; 16],
        expected: u64,
        bytes: &[u8],
    ) -> Result<Version, Error<D::Error>> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        if bytes.len() > MAX_BYTES {
            return Err(Error::TooLarge);
        }
        let old = self.version();
        if old.map_or(0, |v| v.generation) != expected
            || (!self.archive && old.is_some_and(|v| v.object != object))
        {
            return Err(Error::Conflict);
        }
        let generation = expected.checked_add(1).ok_or(Error::SequenceExhausted)?;
        let slot = if self.archive {
            let mut old_bytes = [0; MAX_BYTES];
            if self.read(object, &mut old_bytes)? != 0 {
                return Err(Error::Conflict);
            }
            let mut slot = 0;
            for i in 0..ARCHIVE_SLOTS {
                if self.candidate(i)?.is_some() {
                    slot = i + 1;
                }
            }
            if slot >= ARCHIVE_SLOTS {
                return Err(Error::TooLarge);
            }
            slot
        } else {
            self.current.as_ref().map_or(0, |c| 1 - c.slot)
        };
        let checksum = crc(bytes);
        let mut header = [0; SECTOR];
        header[..8].copy_from_slice(MAGIC);
        header[8..16].copy_from_slice(&generation.to_le_bytes());
        header[16..32].copy_from_slice(&object);
        header[32..36].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        header[36..40].copy_from_slice(&checksum.to_le_bytes());
        let sum = crc(&header[..508]);
        header[508..].copy_from_slice(&sum.to_le_bytes());
        let result = (|| {
            // Invalidate inactive slot durably BEFORE modifying its payload.
            self.device.write(slot * 4, &[0; SECTOR])?;
            self.device.flush()?;
            for i in 0..DATA_SECTORS {
                let mut sector = [0; SECTOR];
                let start = i * SECTOR;
                if start < bytes.len() {
                    let end = bytes.len().min(start + SECTOR);
                    sector[..end - start].copy_from_slice(&bytes[start..end]);
                }
                self.device.write(slot * 4 + 1 + i as u64, &sector)?;
            }
            self.device.flush()?;
            self.device.write(slot * 4, &header)?;
            self.device.flush()
        })();
        if let Err(e) = result {
            self.poisoned = true;
            return Err(Error::Io(e));
        }
        let version = Version {
            generation,
            object,
            len: bytes.len(),
        };
        self.current = Some(Candidate {
            version,
            checksum,
            slot,
        });
        Ok(version)
    }

    pub fn into_device(self) -> D {
        self.device
    }
}
