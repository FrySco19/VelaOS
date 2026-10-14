#![no_std]
#![no_main]
#![feature(asm_cfg)]
use core::ptr::read_volatile;
use sel4_microkit::{NullHandler, memory_region_symbol, protection_domain};

#[protection_domain(heap_size = 16 * 1024)]
fn init() -> NullHandler {
    let page = memory_region_symbol!(mailbox_page_addr: *mut u8).as_ptr();
    // BCM2711 ARM property mailbox starts at page offset 0x880.
    // Read STATUS only: don't consume firmware messages or issue DMA.
    // Framebuffer allocation/rendering is deliberately not claimed here.
    let status = unsafe { read_volatile(page.add(0x898).cast::<u32>()) };
    core::hint::black_box(status);
    NullHandler::new()
}
