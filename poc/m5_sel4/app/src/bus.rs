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
        assert_eq!(channel, Channel::new(0));
        assert_eq!(msg_info.label(), gate_e::AUTHORIZED_LABEL);

        debug_println!("VelaOS Gate F: BEGIN bus_to_executor");
        let execution = Channel::new(gate_e::BUS_TO_EXECUTOR)
            .pp_call(MessageInfo::new(gate_e::EXECUTE_LABEL, 0));
        assert_eq!(
            execution.label(),
            gate_e::WASM_RESULT_LABEL_BASE + gate_e::WASM_RESULT
        );
        debug_println!("VelaOS Gate F: END bus_to_executor");
        debug_println!("VelaOS Gate E: bus routed result toward LFS");

        debug_println!("VelaOS Gate F: BEGIN bus_to_lfs");
        let log_ack =
            Channel::new(gate_e::BUS_TO_LFS).pp_call(MessageInfo::new(execution.label(), 0));
        assert_eq!(
            log_ack.label(),
            gate_e::LFS_ACK_LABEL_BASE + gate_e::WASM_RESULT
        );
        debug_println!("VelaOS Gate F: END bus_to_lfs");
        Ok(MessageInfo::new(
            gate_e::FINAL_RESULT_LABEL_BASE + gate_e::WASM_RESULT,
            0,
        ))
    }
}
