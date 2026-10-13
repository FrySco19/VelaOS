//! QEMU virt modern VirtIO-MMIO block adapter. Single request, bounded polling.
//! DMA/MMIO mappings belong only to the LFS domain in hardware.system.
use crate::storage_core::{BlockDevice, SECTOR};
use core::ptr::{read_volatile as rv, write_volatile as wv};

pub struct VirtioBlock {
    mmio: *mut u8,
    dma: *mut u8,
    phys: u64,
    index: u16,
    capacity: u64,
    poisoned: bool,
}

fn barrier() {
    unsafe {
        core::arch::asm!("dmb sy", options(nostack, preserves_flags));
    }
}

impl VirtioBlock {
    unsafe fn reg(&self, offset: usize) -> u32 {
        unsafe { rv(self.mmio.add(offset).cast()) }
    }
    unsafe fn set(&self, offset: usize, value: u32) {
        unsafe {
            wv(self.mmio.add(offset).cast(), value);
        }
    }
    unsafe fn address(&self, offset: usize, value: u64) {
        unsafe {
            self.set(offset, value as u32);
            self.set(offset + 4, (value >> 32) as u32);
        }
    }
    /// # Safety
    /// Exclusive uncached writable mappings: 0x4000 MMIO scanning window and
    /// 0x1000 contiguous DMA, with the actual DMA physical address from Microkit.
    pub unsafe fn new(mmio: *mut u8, dma: *mut u8, phys: u64) -> Result<Self, &'static str> {
        unsafe { Self::new_slots(mmio, dma, phys, 32) }
    }
    /// Exclusive single selected slot; never scans outside its mapped page.
    pub unsafe fn new_selected(
        mmio: *mut u8,
        dma: *mut u8,
        phys: u64,
    ) -> Result<Self, &'static str> {
        unsafe { Self::new_slots(mmio, dma, phys, 1) }
    }
    unsafe fn new_slots(
        mmio: *mut u8,
        dma: *mut u8,
        phys: u64,
        slots: usize,
    ) -> Result<Self, &'static str> {
        let mut s = Self {
            mmio,
            dma,
            phys,
            index: 0,
            capacity: 0,
            poisoned: false,
        };
        let base = mmio;
        let mut found = false;
        for i in 0..slots {
            s.mmio = unsafe { base.add(i * 0x200) };
            if unsafe { s.reg(0) } == 0x74726976 && unsafe { s.reg(8) } == 2 {
                found = true;
                break;
            }
        }
        if !found {
            return Err("no VirtIO block device");
        }
        unsafe {
            if s.reg(4) != 2 {
                return Err("modern MMIO required");
            }
            s.set(0x70, 0);
            s.set(0x70, 1);
            s.set(0x70, 3);
            s.set(0x14, 0);
            let low = s.reg(0x10);
            if low & (1 << 9) == 0 || low & (1 << 5) != 0 {
                return Err("flush missing or read-only disk");
            }
            s.set(0x14, 1);
            if s.reg(0x10) & 1 == 0 {
                return Err("VERSION_1 missing");
            }
            s.set(0x24, 0);
            s.set(0x20, 1 << 9);
            s.set(0x24, 1);
            s.set(0x20, 1);
            s.set(0x70, 11);
            if s.reg(0x70) & 8 == 0 {
                return Err("features rejected");
            }
            s.set(0x30, 0);
            if s.reg(0x34) < 8 {
                return Err("queue too small");
            }
            // desc 0..128; avail at128; used at256; header512; data1024; status1536.
            for i in 0..4096 {
                wv(dma.add(i), 0);
            }
            s.set(0x38, 8);
            s.address(0x80, phys);
            s.address(0x90, phys + 128);
            s.address(0xa0, phys + 256);
            barrier();
            s.set(0x44, 1);
            s.set(0x70, 15);
            let mut stable = false;
            for _ in 0..100 {
                let generation = s.reg(0xfc);
                s.capacity = s.reg(0x100) as u64 | ((s.reg(0x104) as u64) << 32);
                if generation == s.reg(0xfc) {
                    stable = true;
                    break;
                }
            }
            if !stable || s.capacity != 8 {
                return Err("expected a dedicated eight-sector test disk");
            }
        }
        Ok(s)
    }
    unsafe fn descriptor(&self, i: usize, addr: u64, len: u32, flags: u16, next: u16) {
        unsafe {
            let p = self.dma.add(i * 16);
            wv(p.cast::<u64>(), addr);
            wv(p.add(8).cast::<u32>(), len);
            wv(p.add(12).cast::<u16>(), flags);
            wv(p.add(14).cast::<u16>(), next);
        }
    }
    fn request(
        &mut self,
        op: u32,
        sector: u64,
        data: &mut [u8; SECTOR],
    ) -> Result<(), &'static str> {
        if self.poisoned {
            return Err("device poisoned");
        }
        if op != 4 && sector >= 8 {
            return Err("outside dedicated extent");
        }
        unsafe {
            wv(self.dma.add(512).cast::<u32>(), op);
            wv(self.dma.add(516).cast::<u32>(), 0);
            wv(self.dma.add(520).cast::<u64>(), sector);
            wv(self.dma.add(1536), 0xff);
            if op == 1 {
                for (i, b) in data.iter().enumerate() {
                    wv(self.dma.add(1024 + i), *b);
                }
            }
            self.descriptor(0, self.phys + 512, 16, 1, 1);
            if op == 4 {
                self.descriptor(1, self.phys + 1536, 1, 2, 0);
            } else {
                self.descriptor(1, self.phys + 1024, 512, if op == 0 { 3 } else { 1 }, 2);
                self.descriptor(2, self.phys + 1536, 1, 2, 0);
            }
            wv(
                self.dma
                    .add(132 + (self.index as usize % 8) * 2)
                    .cast::<u16>(),
                0,
            );
            barrier();
            self.index = self.index.wrapping_add(1);
            wv(self.dma.add(130).cast::<u16>(), self.index);
            barrier();
            self.set(0x50, 0);
            for _ in 0..20_000_000 {
                if rv(self.dma.add(258).cast::<u16>()) == self.index {
                    barrier();
                    let entry = 260 + ((self.index.wrapping_sub(1) as usize % 8) * 8);
                    if rv(self.dma.add(entry).cast::<u32>()) != 0
                        || rv(self.dma.add(entry + 4).cast::<u32>())
                            != if op == 0 { 513 } else { 1 }
                        || rv(self.dma.add(1536)) != 0
                    {
                        self.poisoned = true;
                        return Err("bad completion");
                    }
                    if op == 0 {
                        for (i, b) in data.iter_mut().enumerate() {
                            *b = rv(self.dma.add(1024 + i));
                        }
                    }
                    self.set(0x64, self.reg(0x60));
                    return Ok(());
                }
                core::hint::spin_loop();
            }
        }
        self.poisoned = true;
        Err("completion timeout; outcome ambiguous")
    }
}
impl BlockDevice for VirtioBlock {
    type Error = &'static str;
    fn read(&mut self, sector: u64, out: &mut [u8; SECTOR]) -> Result<(), Self::Error> {
        self.request(0, sector, out)
    }
    fn write(&mut self, sector: u64, bytes: &[u8; SECTOR]) -> Result<(), Self::Error> {
        let mut copy = *bytes;
        self.request(1, sector, &mut copy)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.request(4, 0, &mut [0; SECTOR])
    }
}
