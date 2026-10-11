#![no_std]
#![no_main]
#![feature(asm_cfg)]

mod gate_e;

use sel4_microkit::{Channel, Handler, Infallible, MessageInfo, debug_println, protection_domain};

#[protection_domain]
fn init() -> impl Handler {
    HandlerImpl { last_result: None }
}

struct HandlerImpl {
    last_result: Option<sel4_microkit::MessageLabel>,
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        assert_eq!(channel, Channel::new(0));
        let result = msg_info
            .label()
            .checked_sub(gate_e::WASM_RESULT_LABEL_BASE)
            .expect("LFS rejected a malformed result label");
        assert_eq!(result, gate_e::WASM_RESULT);
        self.last_result = Some(result);
        debug_println!(
            "VelaOS Gate E: LFS recorded volatile result {}",
            self.last_result.expect("result was just recorded")
        );
        Ok(MessageInfo::new(gate_e::LFS_ACK_LABEL_BASE + result, 0))
    }
}
