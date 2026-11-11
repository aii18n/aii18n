use hipstr::HipStr;

use super::{Chat, Msg, OnReply, OnThink, Opt, Reply, Role};
use crate::{abort::Abort, error::Result};

/// 会话，持有系统提示词、全部历史与打断句柄
///
/// `send` 成功后自动追加助手回复，用完丢弃整个 [`Session`] 即释放历史。
/// 需要中断进行中的请求时，用 [`Session::abort_handle`] 取得句柄，在别的任务里
/// 调用 `abort()`，请求会在下一次 I/O 或流片段处返回 [`crate::Error::Aborted`]，不重试。
pub struct Session<'c, 'a, T: OnThink, R: OnReply> {
  chat: &'c mut Chat<'a, T, R>,
  msg_li: Vec<Msg>,
  abort: Abort,
}

impl<'c, 'a, T: OnThink, R: OnReply> Session<'c, 'a, T, R> {
  pub(super) fn new(chat: &'c mut Chat<'a, T, R>, system: impl Into<String>) -> Self {
    let mut msg_li = Vec::with_capacity(4);
    msg_li.push(Msg::system(system));
    Self {
      chat,
      msg_li,
      abort: Abort::new(),
    }
  }

  /// 取打断句柄，可在别的任务里中断当前请求
  pub fn abort_handle(&self) -> Abort {
    self.abort.clone()
  }

  /// 取回思考回调
  pub fn on_think(&mut self) -> &mut T {
    self.chat.on_think()
  }

  /// 取回正文回调
  pub fn on_reply(&mut self) -> &mut R {
    self.chat.on_reply()
  }

  /// 设置系统提示词，已有则替换
  pub fn system(&mut self, text: impl Into<String>) -> &mut Self {
    match self.msg_li.first_mut() {
      Some(first) if first.role == Role::System => {
        first.content = HipStr::from(text.into());
      }
      _ => self.msg_li.insert(0, Msg::system(text)),
    }
    self
  }

  /// 追加用户消息
  pub fn user(&mut self, text: impl Into<String>) -> &mut Self {
    self.msg_li.push(Msg::user(text));
    self
  }

  /// 追加助手消息，用于预置历史或修正回复
  pub fn assistant(&mut self, text: impl Into<String>) -> &mut Self {
    self.msg_li.push(Msg::assistant(text));
    self
  }

  /// 当前历史
  pub fn msg(&self) -> &[Msg] {
    &self.msg_li
  }

  /// 追加用户消息并发送，返回完整回复
  ///
  /// 发送失败时用户消息保留在历史中，可直接重试。
  pub async fn ask(&mut self, text: impl Into<String>) -> Result<Reply> {
    self.user(text);
    self.send().await
  }

  /// 发送当前历史，成功后追加助手回复
  pub async fn send(&mut self) -> Result<Reply> {
    self.send_with(Opt::default()).await
  }

  /// 按自定义 [`Opt`] 发送
  pub async fn send_with(&mut self, opt: Opt) -> Result<Reply> {
    let reply = self.chat.run(&self.msg_li, opt, &self.abort).await?;
    if !reply.text.is_empty() {
      // 直接复用 HipStr，长文本零拷贝共享
      self.msg_li.push(Msg {
        role: Role::Assistant,
        content: reply.text.clone(),
      });
    }
    Ok(reply)
  }
}
