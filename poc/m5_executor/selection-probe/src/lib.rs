//! Test-only IPC harness. No human input, Biscuit issuance or production policy.
#![no_std]

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU32, Ordering};

pub const PLAN: &[u8] = b"Vela selection IPC probe: save this exact note";
pub const SESSION: [u8; 32] = [0x39; 32];
pub const REVISION: u64 = 1;
pub const EMPTY: u32 = 0;
pub const READY: u32 = 1;
pub const ACK: u32 = 2;
pub const DATA_OFFSET: usize = 8;
pub const PAGE_SIZE: usize = 4096;

pub struct Page(*mut u8);
impl Page {
    /// # Safety
    /// Address must reference a live, aligned, coherent 4096-byte shared mapping.
    pub unsafe fn new(address: *mut u8) -> Self {
        assert!(!address.is_null());
        assert_eq!(address as usize % core::mem::align_of::<AtomicU32>(), 0);
        Self(address)
    }
    pub fn state(&self) -> &AtomicU32 {
        // SAFETY: established by new; both peers use atomic ownership words.
        unsafe { &*self.0.cast::<AtomicU32>() }
    }
    pub fn write(&self, bytes: &[u8]) {
        assert!(bytes.len() <= PAGE_SIZE - DATA_OFFSET);
        for (i, b) in bytes.iter().enumerate() {
            unsafe {
                write_volatile(self.0.add(DATA_OFFSET + i), *b);
            }
        }
    }
    pub fn read<const N: usize>(&self) -> [u8; N] {
        assert!(N <= PAGE_SIZE - DATA_OFFSET);
        let mut result = [0; N];
        for (i, b) in result.iter_mut().enumerate() {
            *b = unsafe { read_volatile(self.0.add(DATA_OFFSET + i)) };
        }
        result
    }
    pub fn clear(&self) {
        for i in DATA_OFFSET..PAGE_SIZE {
            unsafe {
                write_volatile(self.0.add(i), 0);
            }
        }
    }
    pub fn set(&self, state: u32) {
        self.state().store(state, Ordering::Release);
    }
    pub fn get(&self) -> u32 {
        self.state().load(Ordering::Acquire)
    }
}
