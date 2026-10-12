;; EXECUTOR_CONTRACT E4: imported memory = ambient host state; rejected.
(module
  (import "env" "memory" (memory 1))
  (func (export "run") (result i32) i32.const 7))
