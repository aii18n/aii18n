use aok::{OK, Void};
use log::info;

#[ctor::ctor(unsafe)]
fn _log_init() {
  log_init::init();
}

#[test]
fn test() -> Void {
  let n = 123456;
  info!("> test {n}");
  OK
}
