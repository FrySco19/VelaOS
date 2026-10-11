#![no_std]
#![no_main]
#![feature(asm_cfg)]

mod gate_e;

use sel4_microkit::{Channel, Handler, MessageInfo, NullHandler, debug_println, protection_domain};

#[protection_domain]
fn init() -> impl Handler {
    for label in 0..100 {
        let rejected =
            Channel::new(gate_e::ORCHESTRATOR_TO_PCB).pp_call(MessageInfo::new(label, 0));
        assert_eq!(rejected.label(), gate_e::DENIED_LABEL);
    }
    let rejected = Channel::new(gate_e::ORCHESTRATOR_TO_PCB)
        .pp_call(MessageInfo::new(gate_e::INTENT_LABEL, 1));
    assert_eq!(rejected.label(), gate_e::DENIED_LABEL);
    debug_println!("VelaOS Gate E: 101 malformed PCB frames rejected; domain remains live");
    debug_println!("VelaOS Gate E: orchestrator submitting fixed intent");
    debug_println!("VelaOS Gate F: BEGIN end_to_end");
    let response = Channel::new(gate_e::ORCHESTRATOR_TO_PCB)
        .pp_call(MessageInfo::new(gate_e::INTENT_LABEL, 0));
    assert_eq!(
        response.label(),
        gate_e::FINAL_RESULT_LABEL_BASE + gate_e::WASM_RESULT,
        "orchestrator received an unexpected flow result"
    );
    debug_println!("VelaOS Gate F: END end_to_end");
    debug_println!("VelaOS Gate E: intent completed with result 42");
    NullHandler::new()
}
