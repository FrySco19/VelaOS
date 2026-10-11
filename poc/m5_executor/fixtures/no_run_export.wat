;; EXECUTOR_CONTRACT E5: entry point must be export "run" : () -> i32.
(module
  (func (export "main") (result i32) i32.const 7))
