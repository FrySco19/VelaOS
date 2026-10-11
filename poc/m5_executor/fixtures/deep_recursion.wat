;; EXECUTOR_CONTRACT E3: unbounded recursion hits the interpreter stack/recursion limit (status 5), not fuel.
(module
  (func $r (export "run") (result i32) call $r))
