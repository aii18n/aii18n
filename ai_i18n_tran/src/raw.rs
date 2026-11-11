//! 底层逐行翻译
//!
//! 输入是 [`Batch`]，行内容已预处理，不做过滤与 trim。
//!
//! 修订轮数不设上限：待修订行数只减不增，连续 STALL 轮不下降才放弃，
//! 因此每 3 轮至少少一行，最多 3n 轮收敛。

use ai_api::{Api, Chat, OnReply, OnThink};
use ai_i18n_parse::{Batch, Prompt, Why};

use crate::{
  OnPrompt,
  error::{Error, Result},
};

/// 连续多少轮待修订行数不下降就放弃
const STALL: usize = 3;

/// 翻译一批行，返回与批次等长、按批内顺序对齐的译文
pub(crate) async fn tran<T: OnThink, R: OnReply, P: OnPrompt>(
  api_li: &[Api],
  prompt: &Prompt<'_>,
  batch: Batch<'_, '_>,
  on_think: &mut T,
  on_reply: &mut R,
  on_prompt: &mut P,
) -> Result<Vec<String>> {
  let n = batch.idx_li.len();
  if n == 0 {
    return Ok(Vec::new());
  }
  let mut out_li = vec![None::<String>; n];
  let mut why_li = vec![None::<Why>; n];
  // 上一轮的错误译文，与批内序号对齐
  let mut wrong_li = vec![None::<String>; n];
  let mut pending_li: Vec<usize> = (0..n).collect();

  let sys = prompt.system();
  on_prompt.system(&sys);
  let mut chat = Chat::new(api_li, &mut *on_think, &mut *on_reply);
  let mut session = chat.session(sys);
  let mut bad = false;

  let mut prev = n;
  let mut stall = 0;
  let mut first = true;
  loop {
    let msg = if first {
      prompt.ask(batch)
    } else {
      prompt.fix(batch, &pending_li, &why_li, &wrong_li, bad)
    };
    first = false;
    on_prompt.ask(&msg);
    let got = session.ask(msg).await?;
    let reply = prompt.parse(&got.text);
    bad = reply.bad;
    // 只清理本轮待修订行的旧状态，避免每轮扫全部行
    for &k in &pending_li {
      why_li[k] = None;
      wrong_li[k] = None;
    }

    for b in reply.block_li {
      let Some(k) = b.nth.checked_sub(1).filter(|&k| k < n) else {
        bad = true;
        continue;
      };
      // 已译好的行忽略重复块
      if out_li[k].is_some() {
        continue;
      }
      // 原文比对去首尾空白；提示词里仍用原文
      let orig = batch.line(k).body;
      // 解析阶段已得单行且 trim 过的译文，直接取所有权
      let tran = b.tran.into_owned();
      if b.src != orig {
        why_li[k] = Some(Why::Orig);
        // 非空才值得回给模型看
        if !tran.is_empty() {
          wrong_li[k] = Some(tran);
        }
        continue;
      }
      if tran.is_empty() && !orig.is_empty() {
        why_li[k] = Some(Why::Empty);
        continue;
      }
      out_li[k] = Some(tran);
    }

    pending_li.retain(|&k| out_li[k].is_none());
    if pending_li.is_empty() {
      return Ok(out_li.into_iter().map(Option::unwrap_or_default).collect());
    }
    // 待修订行数只减不增，连续 STALL 轮不降说明模型已无法推进
    if pending_li.len() < prev {
      prev = pending_li.len();
      stall = 0;
    } else {
      stall += 1;
      if stall >= STALL {
        break;
      }
    }
  }

  Err(Error::Format(
    pending_li.iter().map(|&k| batch.idx_li[k] + 1).collect(),
  ))
}
