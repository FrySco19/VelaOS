#![allow(dead_code)]

pub use sel4_microkit::MessageLabel;

pub const ORCHESTRATOR_TO_PCB: usize = 0;
pub const PCB_TO_BUS: usize = 1;
pub const BUS_TO_EXECUTOR: usize = 1;
pub const BUS_TO_LFS: usize = 2;

pub const INTENT_LABEL: MessageLabel = 0x100;
pub const AUTHORIZED_LABEL: MessageLabel = 0x200;
pub const EXECUTE_LABEL: MessageLabel = 0x300;
pub const WASM_RESULT_LABEL_BASE: MessageLabel = 0x400;
pub const LFS_ACK_LABEL_BASE: MessageLabel = 0x500;
pub const FINAL_RESULT_LABEL_BASE: MessageLabel = 0x600;
pub const WASM_RESULT: MessageLabel = 42;
pub const DENIED_LABEL: MessageLabel = 0x7ff;

pub fn accepts(
    channel: sel4_microkit::Channel,
    expected_channel: usize,
    message: &sel4_microkit::MessageInfo,
    label: MessageLabel,
) -> bool {
    channel == sel4_microkit::Channel::new(expected_channel)
        && message.label() == label
        && message.count() == 0
}
pub fn denied() -> sel4_microkit::MessageInfo {
    sel4_microkit::MessageInfo::new(DENIED_LABEL, 0)
}
