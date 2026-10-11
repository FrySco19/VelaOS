;; R1 / EXECUTOR_CONTRACT E8: forever re-enter recursion up to depth 60 (< limit 64), then unwind.
;; Combines deep Wasm call stacks with MAX_FUEL; must end in status 6, not a native overflow.
(module
  (func $down (param $n i32) (result i32)
    local.get $n
    i32.eqz
    if (result i32)
      i32.const 0
    else
      local.get $n
      i32.const 1
      i32.sub
      call $down
      i32.const 1
      i32.add
    end)
  (func (export "run") (result i32)
    (loop $l
      i32.const 60
      call $down
      drop
      br $l)
    i32.const 0))