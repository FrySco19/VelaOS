#![no_std]
#![no_main]
#![feature(asm_cfg)]

mod virtio_rx;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU32, Ordering};
use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use vela_protocol::{Decoder, FrameRef, MAX_FRAME, MessageType};

// Microkit SDF must assign this IRQ channel to the console's actual DTB IRQ,
// map its MMIO uncached, map 8192 contiguous DMA bytes uncached and set
// ingress_dma_phys with region_paddr. Rust cannot provision kernel IRQ caps.
const VIRTIO_IRQ: usize = 0;
const PCB_CHANNEL: usize = 1;
const EMPTY: u32 = 0;
const READY: u32 = 1;

#[unsafe(no_mangle)]
static mut ingress_dma_phys: u64 = 0;

struct Mailbox(*mut u8);
impl Mailbox {
    fn state(&self) -> &AtomicU32 {
        unsafe { &*self.0.cast::<AtomicU32>() }
    }
    fn empty(&self) -> bool {
        self.state().load(Ordering::Acquire) == EMPTY
    }
    fn publish(&self, wire: &[u8]) {
        assert!(self.empty());
        assert!(wire.len() <= MAX_FRAME);
        // Slot header: atomic ownership u32, frame length LE u32, then wire.
        unsafe {
            write_volatile(self.0.add(4).cast::<u32>(), (wire.len() as u32).to_le());
        }
        for (i, byte) in wire.iter().enumerate() {
            unsafe {
                write_volatile(self.0.add(8 + i), *byte);
            }
        }
        self.state().store(READY, Ordering::Release);
        Channel::new(PCB_CHANNEL).notify();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Receiving,
    WaitingForPcb,
    Faulted,
}

struct Ingress {
    rx: virtio_rx::Rx,
    decoder: Decoder,
    mailbox: Mailbox,
    state: State,
}
impl Ingress {
    fn fail(&mut self) {
        self.state = State::Faulted;
        self.rx.fail();
        // Never erase a slot already owned by PCB.
        debug_println!("Vela ingress: rejected stream; restart/session reset required");
    }
    fn drain(&mut self) {
        if self.state == State::Faulted {
            return;
        }
        if self.state == State::WaitingForPcb {
            if !self.mailbox.empty() {
                return;
            }
            self.state = State::Receiving;
        }
        // At most one frame per callback; PCB ACK supplies the next credit.
        for _ in 0..MAX_FRAME {
            if self.decoder.ready() {
                if !self.mailbox.empty() {
                    return;
                }
                let wire = self.decoder.wire().expect("ready decoder");
                let frame = FrameRef::decode(wire).expect("validated decoder");
                if !matches!(
                    frame.header.kind,
                    MessageType::Proposal | MessageType::Approval | MessageType::Cancel
                ) {
                    self.fail();
                    return;
                }
                // Framing validation only. PCB must separately verify signature,
                // pending revision/session, chunk sequencing, policy and replay.
                // This framed mailbox is not the old raw-UserSelection fixture.
                self.mailbox.publish(wire);
                self.decoder.consume().expect("ready decoder");
                self.state = State::WaitingForPcb;
                return;
            }
            match self.rx.next_byte() {
                Ok(Some(byte)) => {
                    if self.decoder.push(byte).is_err() {
                        self.fail();
                        return;
                    }
                }
                Ok(None) => return,
                Err(_) => {
                    self.fail();
                    return;
                }
            }
        }
        // A maximum-sized frame can become ready on the final byte.
        if self.decoder.ready() {
            self.drain();
        }
    }
}

impl Handler for Ingress {
    type Error = core::convert::Infallible;
    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        let irq = channels.contains(Channel::new(VIRTIO_IRQ));
        if irq {
            self.rx.ack_interrupt();
        }
        if irq || channels.contains(Channel::new(PCB_CHANNEL)) {
            self.drain();
        }
        if irq {
            Channel::new(VIRTIO_IRQ)
                .irq_ack()
                .expect("IRQ capability/ack");
        }
        Ok(())
    }
}

#[protection_domain]
fn init() -> impl Handler {
    let mmio = memory_region_symbol!(ingress_mmio_addr: *mut u8).as_ptr();
    // qemu-port0 maps the containing aligned page 0xa003000, not the slot
    // itself; the actual device starts at page offset 0xe00 (verified qtree).
    #[cfg(feature = "qemu-port0")]
    let mmio = unsafe { mmio.add(0xe00) };
    let dma = memory_region_symbol!(ingress_dma_addr: *mut u8).as_ptr();
    let page = memory_region_symbol!(ingress_pcb_addr: *mut u8).as_ptr();
    assert!(!page.is_null() && page as usize & 4095 == 0);
    let mailbox = Mailbox(page);
    assert!(mailbox.empty());
    let physical = unsafe { read_volatile(&raw const ingress_dma_phys) };
    let rx = unsafe { virtio_rx::Rx::init(mmio, dma, physical) }.expect("VirtIO console RX init");
    Channel::new(VIRTIO_IRQ)
        .irq_ack()
        .expect("SDF must provision console IRQ");
    debug_println!("Vela ingress: bounded RX framing ready; no authority issued");
    Ingress {
        rx,
        decoder: Decoder::new(),
        mailbox,
        state: State::Receiving,
    }
}
