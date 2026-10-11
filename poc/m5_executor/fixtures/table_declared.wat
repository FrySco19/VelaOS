;; EXECUTOR_CONTRACT E3: tables(0) limit; a module declaring a table is rejected at instantiation.
(module
  (table 1 funcref)
  (func (export "run") (result i32) i32.const 7))
