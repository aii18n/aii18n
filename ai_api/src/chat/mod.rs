use std::time::Duration;

use compio::time::sleep;
use log::warn;

use crate::{
  abort::Abort,
  api::Api,
  error::{Error, Result},
  http,
};

mod hook;
mod msg;
mod reply;
mod session;

pub use hook::{Flow, OnReply, OnThink, fill_line};
pub use msg::{Msg, Role, Think};
pub(crate) use reply::Acc;
pub use reply::{Finish, Id, Reply, Usage};
pub use session::Session;

/// 默认空闲超时：180 秒
pub const IDLE: Duration = Duration::from_secs(180);
/// 默认整体重试轮数
pub const RETRY: usize = 9;

/// 重试与超时设置
#[derive(Debug, Clone, Copy)]
pub struct Opt {
  /// 任意一次 I/O（连接、握手、写请求、读）的最长等待，超时换下一个 token
  pub idle: Duration,
  /// 从头到尾整体重试的轮数，0 视作 1
  pub retry: usize,
}

impl Default for Opt {
  fn default() -> Self {
    Self {
      idle: IDLE,
      retry: RETRY,
    }
  }
}

/// 对话客户端，不持有历史
///
/// 绑定端点与两侧回调，可发单轮请求，也可 [`Chat::session`] 开会话保存历史。
///
/// ```no_run
/// # use ai_api::{Api, Chat, Flow, OnReply, OnThink};
/// # struct Gray;
/// # struct Out;
/// # impl OnThink for Gray {
/// #   fn think(&mut self, s: &str) -> Flow {
/// #     eprint!("{s}");
/// #     Flow::Continue
/// #   }
/// # }
/// # impl OnReply for Out {
/// #   fn reply(&mut self, s: &str) -> Flow {
/// #     print!("{s}");
/// #     Flow::Continue
/// #   }
/// # }
/// # async fn demo(api_li: &[Api]) -> ai_api::Result<()> {
/// let mut think = Gray;
/// let mut reply = Out;
/// let mut chat = Chat::new(api_li, &mut think, &mut reply);
/// let mut session = chat.session("你是简洁的助手");
/// let got = session.ask("你好").await?;
/// println!("{}", got.text);
/// // 多轮：历史保存在 session
/// session.ask("继续").await?;
/// # Ok(())
/// # }
/// ```
pub struct Chat<'a, T: OnThink, R: OnReply> {
  target_li: Vec<(&'a Api, http::Target)>,
  on_think: &'a mut T,
  on_reply: &'a mut R,
}

impl<'a, T: OnThink, R: OnReply> Chat<'a, T, R> {
  /// 不需要某一侧输出时传 `&mut ()`
  pub fn new(api_li: &'a [Api], on_think: &'a mut T, on_reply: &'a mut R) -> Self {
    // 端点只解析一次，后续每轮请求复用；不可用端点在这里提示
    let mut target_li = Vec::with_capacity(api_li.len());
    for api in api_li {
      let base = &api.base;
      if api.token.is_empty() {
        warn!("{base} 没有 token，跳过");
        continue;
      }
      match http::Target::new(api) {
        Ok(target) => target_li.push((api, target)),
        Err(e) => warn!("{base} 跳过: {e}"),
      }
    }
    Self {
      target_li,
      on_think,
      on_reply,
    }
  }

  /// 开会话，历史由 [`Session`] 持有
  pub fn session(&mut self, system: impl Into<String>) -> Session<'_, 'a, T, R> {
    Session::new(self, system)
  }

  /// 取回思考回调，供 [`Session`] 使用
  fn on_think(&mut self) -> &mut T {
    self.on_think
  }

  /// 取回正文回调，供 [`Session`] 使用
  fn on_reply(&mut self) -> &mut R {
    self.on_reply
  }

  /// 发送一组消息，供 [`Session`] 使用
  async fn run(&mut self, msg_li: &[Msg], opt: Opt, abort: &Abort) -> Result<Reply> {
    abort.reset();
    request(
      &self.target_li,
      msg_li,
      &mut *self.on_think,
      &mut *self.on_reply,
      abort,
      opt,
    )
    .await
  }
}

/// 拼请求体
///
/// `msg_json` 为历史序列化结果，多个端点共享，避免重复序列化。
fn body(msg_json: &[u8], model: &str, think: Think) -> Result<Vec<u8>> {
  // model 需要 JSON 转义，交给序列化器
  let model = sonic_rs::to_vec(model)?;
  let effort = think.effort();
  let fix = 96 + effort.map_or(0, |e| e.len() + 26);
  let mut out = Vec::with_capacity(fix + model.len() + msg_json.len());
  out.extend_from_slice(b"{\"model\":");
  out.extend_from_slice(&model);
  out.extend_from_slice(b",\"messages\":");
  out.extend_from_slice(msg_json);
  out.extend_from_slice(b",\"stream\":true,\"stream_options\":{\"include_usage\":true}");
  if let Some(effort) = effort {
    out.extend_from_slice(b",\"reasoning_effort\":\"");
    out.extend_from_slice(effort.as_bytes());
    out.push(b'"');
  }
  out.push(b'}');
  Ok(out)
}

/// 逐个端点、逐个 token 尝试，全部失败才返回错误
///
/// 端点按顺序使用；每个端点从随机 token 开始按 `+1` 轮转覆盖全部 token。
/// 整轮结束仍未成功则从头再来，最多 `opt.retry` 轮。
/// 已经开始输出或错误不可重试时立即返回。
async fn request(
  target_li: &[(&Api, http::Target)],
  msg_li: &[Msg],
  on_think: &mut impl OnThink,
  on_reply: &mut impl OnReply,
  abort: &Abort,
  opt: Opt,
) -> Result<Reply> {
  if target_li.is_empty() {
    return Err(Error::NoApi);
  }

  // 历史只序列化一次；每个端点的请求体等到真正尝试时才拼
  let msg_json = sonic_rs::to_vec(msg_li)?;
  let mut body_li: Vec<Option<Vec<u8>>> = vec![None; target_li.len()];
  let mut last = None;
  let mut wait: Option<Duration> = None;
  let mut nth = 0u32;
  for round in 1..=opt.retry.max(1) {
    for ((api, target), slot) in target_li.iter().zip(&mut body_li) {
      let base = &api.base;
      if slot.is_none() {
        *slot = match body(&msg_json, &api.model, api.think) {
          Ok(body) => Some(body),
          Err(e) => {
            warn!("{base} 请求体序列化失败: {e}");
            continue;
          }
        };
      }
      let Some(body) = slot.as_ref() else {
        continue;
      };
      let n = api.token.len();
      let start = fastrand::usize(..n);
      // 从随机 token 起按 +1 轮转，链式遍历免去取模
      for idx in (start..n).chain(0..start) {
        if let Some(d) = wait.take() {
          abort.race(sleep(d)).await?;
        }
        match http::call(
          target,
          &api.token[idx],
          body,
          opt.idle,
          abort,
          on_think,
          on_reply,
        )
        .await
        {
          Ok(reply) => return Ok(reply),
          Err(fail) => {
            let err = fail.err;
            if fail.started || !err.retryable() {
              // 已输出内容重试会重复；永久性错误换谁都没用
              return Err(err);
            }
            warn!("第 {round} 轮 {base} token[{idx}] 失败: {err}");
            if err.throttled() {
              nth += 1;
              wait = Some(backoff(nth));
            }
            last = Some(err);
          }
        }
      }
    }
  }
  Err(last.unwrap_or(Error::NoApi))
}

/// 限流 / 5xx 的指数退避，加抖动避免同时重试
fn backoff(nth: u32) -> Duration {
  let ms = (200u64 << nth.min(5)).min(5_000);
  Duration::from_millis(ms + fastrand::u64(..100))
}
