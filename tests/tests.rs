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

#[flaky_test(times = 10)]
#[should_panic]
fn fail_ten_times() {
  assert!(false);
}

#[cfg(feature = "tokio")]
#[flaky_test(tokio)]
async fn tokio_basic() {
  let fut = std::future::ready(42);
  assert_eq!(fut.await, 42);
}

//#[cfg(feature = "tokio")]
//#[flaky_test(tokio(flavor = "multi_thread", worker_threads = 2))]
//async fn tokio_basic() {
//  let fut = std::future::ready(42);
//  assert_eq!(fut.await, 42);
//}

#[cfg(feature = "tokio")]
#[flaky_test(tokio, times = 5)]
async fn tokio_with_times() {
  let fut = std::future::ready(42);
  assert_eq!(fut.await, 42);
}

#[cfg(feature = "tokio")]
#[flaky_test(tokio)]
#[should_panic]
async fn tokio_with_should_panic() {
  let fut = std::future::ready(0);
  assert_eq!(fut.await, 42);
}
