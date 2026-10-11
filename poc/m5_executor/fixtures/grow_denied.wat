;; EXECUTOR_CONTRACT E3: memory.grow beyond quota returns -1 (limiter), result is reported as u32 bits.
(module
  (memory 1 2)
  (func (export "run") (result i32) i32.const 1 memory.grow))
