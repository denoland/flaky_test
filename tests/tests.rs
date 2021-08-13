use flaky_test::flaky_test;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[flaky_test]
fn assert_true() {
  println!("should pass");
}

#[flaky_test]
fn fail_first_two_times() {
  static C: AtomicUsize = AtomicUsize::new(0);
  if C.fetch_add(1, Ordering::SeqCst) < 2 {
    panic!("flaky");
  }
  assert_eq!(3, C.load(Ordering::SeqCst));
}

/* TODO(ry) should_panic doesn't seem to work
#[flaky_test]
#[should_panic]
fn fail_three_times() {
  assert!(false);
}
*/
