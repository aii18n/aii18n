#![cfg_attr(docsrs, feature(doc_cfg))]

//! 批量逐行翻译。
//!
//! 上游先用 [`ai_i18n_parse::Plan`] 做行预处理：去掉行尾空白，拆出行首前缀
//! （空白 + 标题、引用、列表等 Markdown 标记），只把正文交给模型，译文写回时
//! 还原前缀；空行与纯标点/表情行直接原样返回，不进入模型。Markdown 围栏代码块
//! （含围栏行本身）除外：`});`、`| --- |` 这类纯符号行也要交给模型，由提示词
//! 约束只翻译注释。其余行按 [`ai_i18n_parse::CHAR_LIMIT`] 逐行累计分批，装满即
//! 调用底层逐行翻译，每批独立会话，避免上下文过长。
//!
//! 提示词生成、回复解析与分批都在 `ai_i18n_parse`，本 crate 只负责模型调用与
//! 多轮修订编排。
//!
//! 需在 compio 运行时内调用：
//!
//! ```no_run
//! # async fn demo() -> ai_i18n_tran::Result<()> {
//! let got = ai_i18n_tran::tran("中文", "English", &["你好", "世界"]).await?;
//! assert_eq!(got, ["Hello", "World"]);
//! # Ok(())
//! # }
//! ```

mod error;
mod raw;

use ai_api::{Api, OnReply, OnThink};
use ai_i18n_parse::{Batch, Plan, Prompt};
pub use error::{Error, Result};

/// 请求提示词回调，便于调试实际发送内容
pub trait OnPrompt {
  /// 会话系统提示，每批建立会话时调用一次
  fn system(&mut self, _text: &str) {}
  /// 实际发送的用户消息，含修订轮
  fn ask(&mut self, text: &str);
}

/// 不需要提示词时传 `&mut ()`
impl OnPrompt for () {
  fn ask(&mut self, _text: &str) {}
}

/// 用配置文件里的端点逐行翻译
///
/// 空输入直接返回空表，不读配置、不请求模型。
pub async fn tran(
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
) -> Result<Vec<String>> {
  // 空输入与同语言都不需要端点，避免无配置时报错
  if line_li.is_empty() || from_lang.as_ref().eq_ignore_ascii_case(to_lang.as_ref()) {
    return tran_with(&[], from_lang, to_lang, line_li).await;
  }
  let conf = ai_i18n_conf::load()?.ok_or(Error::NoApi)?;
  tran_with(&conf.api, from_lang, to_lang, line_li).await
}

/// 指定端点逐行翻译，逻辑同 [`tran`]
///
/// 返回与 `line_li` 等长、按下标对齐的译文。
pub async fn tran_with(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
) -> Result<Vec<String>> {
  tran_hook(api_li, from_lang, to_lang, line_li, &mut (), &mut ()).await
}

/// 带流式回调的逐行翻译，逻辑同 [`tran_with`]
///
/// `on_think` / `on_reply` 收到模型的思考与正文增量，便于在终端演示或落盘。
pub async fn tran_hook<T: OnThink, R: OnReply>(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
  on_think: &mut T,
  on_reply: &mut R,
) -> Result<Vec<String>> {
  tran_ask_hook(
    api_li,
    from_lang,
    to_lang,
    line_li,
    on_think,
    on_reply,
    &mut (),
  )
  .await
}

/// 带提示词回调的逐行翻译，逻辑同 [`tran_hook`]
///
/// `on_prompt` 额外收到每批的系统提示与每次实际发送的用户消息。
pub async fn tran_ask_hook<T: OnThink, R: OnReply, P: OnPrompt>(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
  on_think: &mut T,
  on_reply: &mut R,
  on_prompt: &mut P,
) -> Result<Vec<String>> {
  let from = from_lang.as_ref();
  let to = to_lang.as_ref();
  if line_li.is_empty() {
    return Ok(Vec::new());
  }
  // 同语言无需翻译，也避免两个标签同名导致解析歧义
  if from.eq_ignore_ascii_case(to) {
    return Ok(
      line_li
        .iter()
        .map(|line| line.as_ref().trim_end().to_owned())
        .collect(),
    );
  }
  let plan = Plan::new(line_li);

  // 直通行先写回，待译行按批请求后写回
  let mut out_li: Vec<String> = plan
    .line_li
    .iter()
    .map(|line| {
      if line.keep {
        line.restore(line.body)
      } else {
        String::new()
      }
    })
    .collect();
  let mut job = Job {
    api_li,
    prompt: Prompt::new(from, to),
    on_think,
    on_reply,
    on_prompt,
  };
  for batch in plan.batches() {
    job.flush(&mut out_li, batch).await?;
  }
  Ok(out_li)
}

/// 一次批量翻译的上下文
struct Job<'a, T: OnThink, R: OnReply, P: OnPrompt> {
  api_li: &'a [Api],
  prompt: Prompt<'a>,
  on_think: &'a mut T,
  on_reply: &'a mut R,
  on_prompt: &'a mut P,
}

impl<T: OnThink, R: OnReply, P: OnPrompt> Job<'_, T, R, P> {
  /// 翻译并写回一批
  async fn flush(&mut self, out_li: &mut [String], batch: Batch<'_, '_>) -> Result<()> {
    let got_li = raw::tran(
      self.api_li,
      &self.prompt,
      batch,
      &mut *self.on_think,
      &mut *self.on_reply,
      &mut *self.on_prompt,
    )
    .await?;
    for (k, text) in got_li.into_iter().enumerate() {
      let i = batch.idx_li[k];
      let line = batch.line(k);
      // 解析阶段已 trim，无前缀时直接复用译文，省一次分配
      out_li[i] = if line.prefix.is_empty() {
        text
      } else {
        line.restore(&text)
      };
    }
    Ok(())
  }
}
