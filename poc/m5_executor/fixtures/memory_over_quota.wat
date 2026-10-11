;; EXECUTOR_CONTRACT E3: declares 2 pages (128 KiB) > signed 64 KiB quota; instantiation fails closed.
(module
  (memory 2)
  (func (export "run") (result i32) i32.const 7))
