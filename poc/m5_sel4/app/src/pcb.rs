#![no_std]
#![no_main]
#![feature(asm_cfg)]

mod gate_e;

use sel4_microkit::{Channel, Handler, Infallible, MessageInfo, debug_println, protection_domain};

#[protection_domain]
fn init() -> impl Handler {
    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if !gate_e::accepts(
            channel,
            gate_e::ORCHESTRATOR_TO_PCB,
            &msg_info,
            gate_e::INTENT_LABEL,
        ) {
            return Ok(gate_e::denied());
        }
        // Boot-fixture probes: malformed frames must not terminate the receiver.
        for malformed in [
            MessageInfo::new(0, 0),
            MessageInfo::new(gate_e::AUTHORIZED_LABEL, 1),
        ] {
            let rejected = Channel::new(gate_e::PCB_TO_BUS).pp_call(malformed);
            assert_eq!(rejected.label(), gate_e::DENIED_LABEL);
        }
        debug_println!("VelaOS Gate E: PCB authorized fixed intent");
        debug_println!("VelaOS Gate F: BEGIN pcb_to_bus");
        let response =
            Channel::new(gate_e::PCB_TO_BUS).pp_call(MessageInfo::new(gate_e::AUTHORIZED_LABEL, 0));
        if response.label() != gate_e::FINAL_RESULT_LABEL_BASE + gate_e::WASM_RESULT
            || response.count() != 0
        {
            return Ok(gate_e::denied());
        }
        debug_println!("VelaOS Gate F: END pcb_to_bus");
        Ok(response)
    }
}
