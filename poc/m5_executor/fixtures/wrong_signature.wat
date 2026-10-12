;; EXECUTOR_CONTRACT E5: "run" with a non-(() -> i32) signature is rejected.
(module
  (func (export "run") (result i64) i64.const 7))
