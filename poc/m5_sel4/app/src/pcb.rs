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
        assert_eq!(channel, Channel::new(gate_e::ORCHESTRATOR_TO_PCB));
        assert_eq!(
            msg_info.label(),
            gate_e::INTENT_LABEL,
            "PCB rejected an unknown intent"
        );
        debug_println!("VelaOS Gate E: PCB authorized fixed intent");
        let response =
            Channel::new(gate_e::PCB_TO_BUS).pp_call(MessageInfo::new(gate_e::AUTHORIZED_LABEL, 0));
        assert_eq!(
            response.label(),
            gate_e::FINAL_RESULT_LABEL_BASE + gate_e::WASM_RESULT
        );
        Ok(response)
    }
}
