;; EXECUTOR_CONTRACT E5: a trapping start function fails the task (status 5) after the ID is consumed.
(module
  (func $s unreachable)
  (start $s)
  (func (export "run") (result i32) i32.const 7))
