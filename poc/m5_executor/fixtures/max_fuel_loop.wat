;; R1 / EXECUTOR_CONTRACT E8: tightest possible infinite loop, admitted at MAX_FUEL.
;; Must end in OutOfFuel (status 6) without growing the native stack.
(module
  (func (export "run") (result i32)
    (loop $l (br $l))
    i32.const 0))