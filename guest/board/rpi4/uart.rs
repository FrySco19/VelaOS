#![no_std]
#![no_main]
#![feature(asm_cfg)]
use core::ptr::{read_volatile, write_volatile};
use sel4_microkit::{NullHandler, memory_region_symbol, protection_domain};

#[protection_domain(heap_size = 16 * 1024)]
fn init() -> NullHandler {
    let base = memory_region_symbol!(uart_addr: *mut u8).as_ptr();
    // Firmware/U-Boot must configure UART0 pins and clock/baud before handoff.
    // Never change GPIO, clock or firmware console state from this PD.
    for &byte in b"Vela RPi4: isolated PL011 domain started\r\n" {
        let mut ready = false;
        for _ in 0..100_000 {
            // SAFETY: SDF maps an exclusive uncached UART register page.
            if unsafe { read_volatile(base.add(0x18).cast::<u32>()) } & (1 << 5) == 0 {
                ready = true;
                break;
            }
            core::hint::spin_loop();
        }
        if !ready { break; }
        unsafe { write_volatile(base.cast::<u32>(), byte as u32) };
    }
    NullHandler::new()
}
