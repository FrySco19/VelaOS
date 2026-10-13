//! Modern VirtIO-MMIO console RX, queue 0, single-port mode.
//! Bind QEMU virtconsole port 0. Named MULTIPORT virtserialport control queues
//! are not implemented by this increment and must not be advertised as working.
use core::ptr::{read_volatile as read, write_volatile as write};

const QUEUE: u16 = 8;
const BUFFER: usize = 512;
const AVAIL: usize = 256;
const USED: usize = 512;
const DATA: usize = 4096;
pub const DMA_BYTES: usize = 16384;
const TX_DESC: usize = 8192;
const TX_AVAIL: usize = 8448;
const TX_USED: usize = 8704;
const TX_DATA: usize = 12288;

#[derive(Clone, Copy, Debug)]
pub enum Error {
    Mapping,
    Device,
    Features,
    Queue,
    Descriptor,
}

fn barrier() {
    // This driver is compiled only for the AArch64 Microkit target.
    unsafe {
        core::arch::asm!("dmb sy", options(nostack, preserves_flags));
    }
}

pub struct Rx {
    mmio: *mut u8,
    dma: *mut u8,
    available: u16,
    consumed: u16,
    posted: u16,
    current: Option<(u16, usize, usize)>,
    tx_index: u16,
    tx_busy: bool,
    physical: u64,
}

impl Rx {
    fn reg(&self, offset: usize) -> u32 {
        unsafe { read(self.mmio.add(offset).cast()) }
    }
    fn set(&self, offset: usize, value: u32) {
        unsafe {
            write(self.mmio.add(offset).cast(), value);
        }
    }
    fn address(&self, offset: usize, value: u64) {
        self.set(offset, value as u32);
        self.set(offset + 4, (value >> 32) as u32);
    }
    fn get16(&self, offset: usize) -> u16 {
        unsafe { u16::from_le(read(self.dma.add(offset).cast())) }
    }
    fn get32(&self, offset: usize) -> u32 {
        unsafe { u32::from_le(read(self.dma.add(offset).cast())) }
    }
    fn put16(&self, offset: usize, value: u16) {
        unsafe {
            write(self.dma.add(offset).cast(), value.to_le());
        }
    }

    /// # Safety
    /// Exclusive uncached MMIO mapping >= 0x200 bytes for the selected console.
    /// Exclusive uncached DMA mapping >= DMA_BYTES, physically contiguous and
    /// 4096-byte aligned. The configured IRQ must belong to this exact device.
    pub unsafe fn init(mmio: *mut u8, dma: *mut u8, physical: u64) -> Result<Self, Error> {
        if mmio.is_null()
            || dma.is_null()
            || mmio as usize & 3 != 0
            || dma as usize & 4095 != 0
            || physical == 0
            || physical & 4095 != 0
            || physical.checked_add(DMA_BYTES as u64).is_none()
        {
            return Err(Error::Mapping);
        }
        let rx = Self {
            mmio,
            dma,
            available: QUEUE,
            consumed: 0,
            posted: 0xff,
            current: None,
            tx_index: 0,
            tx_busy: false,
            physical,
        };
        if rx.reg(0) != 0x74726976 || rx.reg(4) != 2 || rx.reg(8) != 3 {
            return Err(Error::Device);
        }
        rx.set(0x70, 0);
        if rx.reg(0x70) != 0 {
            return Err(Error::Device);
        }
        rx.set(0x70, 3); // ACKNOWLEDGE | DRIVER
        rx.set(0x14, 1);
        if rx.reg(0x10) & 1 == 0 {
            return Err(Error::Features);
        }
        rx.set(0x24, 0);
        rx.set(0x20, 0); // no MULTIPORT / EVENT_IDX / INDIRECT
        rx.set(0x24, 1);
        rx.set(0x20, 1); // VIRTIO_F_VERSION_1
        rx.set(0x70, 11);
        if rx.reg(0x70) & 8 == 0 {
            return Err(Error::Features);
        }
        rx.set(0x30, 0);
        if rx.reg(0x34) < u32::from(QUEUE) || rx.reg(0x44) != 0 {
            return Err(Error::Queue);
        }
        for i in 0..DMA_BYTES {
            unsafe {
                write(dma.add(i), 0);
            }
        }
        for id in 0..QUEUE {
            let offset = usize::from(id) * 16;
            let address = physical + (DATA + usize::from(id) * BUFFER) as u64;
            unsafe {
                write(dma.add(offset).cast::<u64>(), address.to_le());
                write(dma.add(offset + 8).cast::<u32>(), (BUFFER as u32).to_le());
            }
            rx.put16(offset + 12, 2); // device-writable, no chained descriptor
            rx.put16(AVAIL + 4 + usize::from(id) * 2, id);
        }
        rx.put16(AVAIL + 2, QUEUE);
        rx.set(0x38, u32::from(QUEUE));
        rx.address(0x80, physical);
        rx.address(0x90, physical + AVAIL as u64);
        rx.address(0xa0, physical + USED as u64);
        barrier();
        rx.set(0x44, 1);
        rx.set(0x30, 1);
        if rx.reg(0x34) < u32::from(QUEUE) || rx.reg(0x44) != 0 {
            return Err(Error::Queue);
        }
        rx.set(0x38, u32::from(QUEUE));
        rx.address(0x80, physical + TX_DESC as u64);
        rx.address(0x90, physical + TX_AVAIL as u64);
        rx.address(0xa0, physical + TX_USED as u64);
        barrier();
        rx.set(0x44, 1);
        rx.set(0x70, 15); // DRIVER_OK
        rx.set(0x50, 0);
        rx.ack_interrupt();
        Ok(rx)
    }

    fn repost(&mut self, id: u16) {
        self.put16(AVAIL + 4 + usize::from(self.available % QUEUE) * 2, id);
        self.posted |= 1 << id;
        self.available = self.available.wrapping_add(1);
        barrier();
        self.put16(AVAIL + 2, self.available);
        barrier();
        self.set(0x50, 0);
    }

    pub fn next_byte(&mut self) -> Result<Option<u8>, Error> {
        // Bound zero-length descriptors too: no unbounded IRQ handler loop.
        for _ in 0..=QUEUE {
            if let Some((id, position, length)) = self.current {
                let byte =
                    unsafe { read(self.dma.add(DATA + usize::from(id) * BUFFER + position)) };
                if position + 1 == length {
                    self.current = None;
                    self.repost(id);
                } else {
                    self.current = Some((id, position + 1, length));
                }
                return Ok(Some(byte));
            }
            let produced = self.get16(USED + 2);
            barrier();
            let pending = produced.wrapping_sub(self.consumed);
            if pending == 0 {
                return Ok(None);
            }
            if pending > QUEUE {
                return Err(Error::Descriptor);
            }
            let offset = USED + 4 + usize::from(self.consumed % QUEUE) * 8;
            let id = self.get32(offset);
            let length = self.get32(offset + 4) as usize;
            if id >= u32::from(QUEUE) || length > BUFFER || self.posted & (1 << id) == 0 {
                return Err(Error::Descriptor);
            }
            self.posted &= !(1 << id);
            self.consumed = self.consumed.wrapping_add(1);
            if length == 0 {
                self.repost(id as u16);
                continue;
            }
            self.current = Some((id as u16, 0, length));
        }
        Ok(None)
    }
    pub fn ack_interrupt(&self) {
        let pending = self.reg(0x60);
        if pending != 0 {
            self.set(0x64, pending);
        }
    }
    pub fn try_send(&mut self, bytes: &[u8]) -> Result<bool, Error> {
        if bytes.len() > 2048 || bytes.is_empty() {
            return Err(Error::Descriptor);
        }
        if self.tx_busy {
            let completed = self.get16(TX_USED + 2);
            barrier();
            if completed == self.tx_index.wrapping_sub(1) {
                return Ok(false);
            }
            let slot = TX_USED + 4 + usize::from(self.tx_index.wrapping_sub(1) % QUEUE) * 8;
            if completed != self.tx_index || self.get32(slot) != 0 {
                return Err(Error::Descriptor);
            }
            self.tx_busy = false;
        }
        for (i, b) in bytes.iter().enumerate() {
            unsafe {
                write(self.dma.add(TX_DATA + i), *b);
            }
        }
        unsafe {
            write(
                self.dma.add(TX_DESC).cast::<u64>(),
                (self.physical + TX_DATA as u64).to_le(),
            );
            write(
                self.dma.add(TX_DESC + 8).cast::<u32>(),
                (bytes.len() as u32).to_le(),
            );
        }
        self.put16(TX_DESC + 12, 0);
        self.put16(TX_AVAIL + 4 + usize::from(self.tx_index % QUEUE) * 2, 0);
        self.tx_index = self.tx_index.wrapping_add(1);
        barrier();
        self.put16(TX_AVAIL + 2, self.tx_index);
        barrier();
        self.tx_busy = true;
        self.set(0x50, 1);
        Ok(true)
    }
    pub fn fail(&self) {
        self.set(0x70, self.reg(0x70) | 128);
    }
}
