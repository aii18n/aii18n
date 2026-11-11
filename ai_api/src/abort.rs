use std::{
  future::{Future, poll_fn},
  pin::{Pin, pin},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  task::{Context, Poll},
};

use futures_util::task::AtomicWaker;

use crate::error::{Error, Result};

/// 打断句柄，可跨任务共享
///
/// [`Session::abort_handle`] 取得后，在别的任务里调用 [`Abort::abort`]，
/// 正在进行的请求会在下一次 I/O 或流片段处返回 [`Error::Aborted`]。
///
/// [`Session::abort_handle`]: crate::Session::abort_handle
#[derive(Clone, Default)]
pub struct Abort(Arc<Inner>);

#[derive(Default)]
struct Inner {
  flag: AtomicBool,
  waker: AtomicWaker,
}

impl Abort {
  pub fn new() -> Self {
    Self::default()
  }

  /// 打断当前请求
  pub fn abort(&self) {
    self.0.flag.store(true, Ordering::Release);
    self.0.waker.wake();
  }

  /// 是否已打断
  pub fn is_aborted(&self) -> bool {
    self.0.flag.load(Ordering::Acquire)
  }

  /// 清除标记，供下一次请求使用
  pub(crate) fn reset(&self) {
    self.0.flag.store(false, Ordering::Release);
  }

  /// 等待被打断
  fn wait(&self) -> Wait<'_> {
    Wait(self)
  }

  /// 与任意 future 竞速，打断时立即返回 [`Error::Aborted`]
  pub(crate) async fn race<F: Future>(&self, fut: F) -> Result<F::Output> {
    let mut fut = pin!(fut);
    let mut wait = pin!(self.wait());
    poll_fn(move |cx| {
      if self.is_aborted() {
        return Poll::Ready(Err(Error::Aborted));
      }
      if wait.as_mut().poll(cx).is_ready() {
        return Poll::Ready(Err(Error::Aborted));
      }
      fut.as_mut().poll(cx).map(Ok)
    })
    .await
  }
}

struct Wait<'a>(&'a Abort);

impl Future for Wait<'_> {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
    let abort = self.0;
    if abort.is_aborted() {
      return Poll::Ready(());
    }
    abort.0.waker.register(cx.waker());
    if abort.is_aborted() {
      Poll::Ready(())
    } else {
      Poll::Pending
    }
  }
}
