;; EXECUTOR_CONTRACT E4: any import (not only WASI) is rejected before instantiation.
(module
  (import "env" "f" (func))
  (func (export "run") (result i32) i32.const 7))
