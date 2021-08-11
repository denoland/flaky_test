# flaky_test

![https://docs.rs/flaky_test](https://docs.rs/flaky_test/badge.svg)

This attribute macro will register and run a test 3 times, erroring only if all
three times fail. Useful for situations when a test is flaky.

```rust
#[flaky_test::flaky_test]
fn my_test() {
  assert_eq!(1, 2);
}
```
