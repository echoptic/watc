(module
  (func $fib (param $n i32) (result i32)
    (if (i32.eq (get_local $n) (i32.const 1))
      (then (return (i32.const 1))))
    (if (i32.eq (get_local $n) (i32.const 2))
      (then (return (i32.const 1))))
    (i32.add
      (call $fib (i32.sub (local.get 0) (i32.const 2)))
      (call $fib (i32.sub (local.get 0) (i32.const 1)))))
  (export "fib" (func $fib)))
