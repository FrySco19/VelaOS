;; R1 / EXECUTOR_CONTRACT E8: infinite loop of call/return pairs at MAX_FUEL.
;; Exercises call dispatch millions of times; must end in status 6, not a native overflow.
(module
  (func $leaf (param i32) (result i32)
    local.get 0
    i32.const 1
    i32.add)
  (func (export "run") (result i32)
    (local $acc i32)
    (loop $l
      local.get $acc
      call $leaf
      local.set $acc
      br $l)
    local.get $acc))