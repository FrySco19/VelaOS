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
