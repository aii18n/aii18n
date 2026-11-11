//! 用配置文件里的端点演示逐行翻译，流式输出思考与正文
//!
//! 原文在同目录 `tran.md`，直接编辑即可，编译时自动重新读取。
//!
//! cargo run -p ai_i18n_tran --example tran

use std::io::{self, IsTerminal, Write};

use ai_api::{Flow, OnReply, OnThink, fill_line};
use ai_i18n_tran::OnPrompt;

/// 灰色（亮黑）
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

/// 思考：灰色输出
struct Gray {
  /// 终端时才着色
  tty: bool,
  /// 思考段是否已开始（`end` 复位）
  on: bool,
}

impl OnThink for Gray {
  fn think(&mut self, s: &str) -> Flow {
    if !self.on {
      print!("{}思考：", if self.tty { GRAY } else { "" });
      self.on = true;
    }
    print!("{s}");
    let _ = io::stdout().flush();
    Flow::Continue
  }

  fn end(&mut self, think: &str) {
    if !self.on {
      return;
    }
    if self.tty {
      print!("{RESET}");
    }
    self.on = false;
    fill_line(think);
    let _ = io::stdout().flush();
  }
}

/// 正文：直接打印，结束时补换行
struct Out {
  /// 正文段是否已开始（`end` 复位）
  on: bool,
}

impl OnReply for Out {
  fn reply(&mut self, s: &str) -> Flow {
    if !self.on {
      print!("输出：");
      self.on = true;
    }
    print!("{s}");
    let _ = io::stdout().flush();
    Flow::Continue
  }

  fn end(&mut self, text: &str) {
    self.on = false;
    fill_line(text);
    let _ = io::stdout().flush();
  }
}

/// 提示词：调试时打印实际发送内容，按轮次编号
#[derive(Default)]
struct Echo {
  /// 已发送的用户消息轮数
  round: usize,
}

impl OnPrompt for Echo {
  fn system(&mut self, text: &str) {
    println!("\n系统提示：\n{text}");
  }

  fn ask(&mut self, text: &str) {
    self.round += 1;
    println!("\n第 {} 轮用户消息：\n{text}", self.round);
  }
}

#[compio::main]
async fn main() -> aok::Void {
  log_init::init();
  let conf = ai_i18n_conf::load()?.ok_or_else(|| aok::anyhow!("配置文件为空"))?;
  // 原文取自《双城记》第一卷第一章（公版），整段作为一个元素，保留上下文
  // 空行、纯标点、纯表情直通；Call me Ishmael 验证行尾 trim 与行首还原；
  // Markdown 代码块内的纯符号行也送模型，只翻译注释
  let line_li: Vec<&str> = include_str!("tran.md").lines().collect();

  println!("输入 {} 行：", line_li.len());
  for (i, line) in line_li.iter().enumerate() {
    println!("{}. {line}", i + 1);
  }

  println!("\n请求模型（输入 → 思考 → 输出）：");
  let tty = io::stdout().is_terminal();
  let mut think = Gray { tty, on: false };
  let mut reply = Out { on: false };
  let mut echo = Echo::default();
  let got_li = ai_i18n_tran::tran_ask_hook(
    &conf.api, "English", "中文", &line_li, &mut think, &mut reply, &mut echo,
  )
  .await?;

  println!("\n最终输出（空行/纯标点/表情直通，代码块只翻注释）：");
  for (i, (src, dst)) in line_li.iter().zip(&got_li).enumerate() {
    println!("{}. {src} => {dst}", i + 1);
  }

  println!("\n仅译文：");
  for dst in &got_li {
    println!("{dst}");
  }
  Ok(())
}
