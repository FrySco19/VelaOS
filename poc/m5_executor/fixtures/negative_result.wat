;; EXECUTOR_CONTRACT E5: the only result channel is one i32 reported as u32 bits (-1 -> 0xFFFFFFFF).
(module
  (func (export "run") (result i32) i32.const -1))
