#![no_std]
use core::{
    ptr::{read_volatile, write_volatile},
    sync::atomic::{AtomicU32, Ordering},
};
use vela_protocol::{HEADER_SIZE, MAX_FRAME};
pub const EMPTY: u32 = 0;
pub const READY: u32 = 1;
pub struct Slot(pub *mut u8);
impl Slot {
    pub fn state(&self) -> &AtomicU32 {
        unsafe { &*self.0.cast::<AtomicU32>() }
    }
    pub fn ready(&self) -> bool {
        self.state().load(Ordering::Acquire) == READY
    }
    pub fn empty(&self) -> bool {
        self.state().load(Ordering::Acquire) == EMPTY
    }
    pub fn take(&self, out: &mut [u8; MAX_FRAME]) -> Result<usize, &'static str> {
        if !self.ready() {
            return Err("NOT_READY");
        }
        let n = unsafe { u32::from_le(read_volatile(self.0.add(4).cast::<u32>())) } as usize;
        if !(HEADER_SIZE..=MAX_FRAME).contains(&n) {
            return Err("LENGTH");
        }
        for (i, b) in out[..n].iter_mut().enumerate() {
            *b = unsafe { read_volatile(self.0.add(8 + i)) };
        }
        Ok(n)
    }
    pub fn release(&self) {
        for i in 4..4096 {
            unsafe {
                write_volatile(self.0.add(i), 0);
            }
        }
        self.state().store(EMPTY, Ordering::Release);
    }
    pub fn publish(&self, bytes: &[u8]) -> Result<(), &'static str> {
        if !self.empty() || bytes.len() > MAX_FRAME {
            return Err("BACKPRESSURE");
        }
        for (i, b) in bytes.iter().enumerate() {
            unsafe {
                write_volatile(self.0.add(8 + i), *b);
            }
        }
        unsafe {
            write_volatile(self.0.add(4).cast::<u32>(), (bytes.len() as u32).to_le());
        }
        self.state().store(READY, Ordering::Release);
        Ok(())
    }
}
pub fn now_ns() -> u64 {
    let ticks: u64;
    let frequency: u64;
    unsafe {
        core::arch::asm!("isb","mrs {t}, cntpct_el0","mrs {f}, cntfrq_el0",t=out(reg)ticks,f=out(reg)frequency,options(nomem,nostack));
    }
    assert!(frequency != 0);
    ((ticks as u128 * 1_000_000_000) / frequency as u128)
        .try_into()
        .expect("clock overflow")
}
