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

#[flaky_test(5)]
fn fail_first_four_times() {
  static C: AtomicUsize = AtomicUsize::new(0);
  if C.fetch_add(1, Ordering::SeqCst) < 4 {
    panic!("flaky");
  }
  assert_eq!(5, C.load(Ordering::SeqCst));
}

#[flaky_test(times = 10)]
fn fail_first_nine_times() {
  static C: AtomicUsize = AtomicUsize::new(0);
  if C.fetch_add(1, Ordering::SeqCst) < 9 {
    panic!("flaky");
  }
  assert_eq!(10, C.load(Ordering::SeqCst));
}

#[flaky_test]
#[should_panic]
fn fail_three_times() {
  assert!(false);
}

#[flaky_test(5)]
#[should_panic]
fn fail_five_times() {
  assert!(false);
}

#[flaky_test(times = 10)]
#[should_panic]
fn fail_ten_times() {
  assert!(false);
}
