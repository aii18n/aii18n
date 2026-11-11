//! 翻译主流程用例：分批、修订、停滞与回调

mod common;

use std::{
  fmt::Write as _,
  sync::{Arc, Mutex},
};

use ai_api::{Api, Flow, OnReply, OnThink};
use ai_i18n_parse::CHAR_LIMIT;
use ai_i18n_tran::{Error, OnPrompt, tran_ask_hook, tran_hook, tran_with};
use aok::{OK, Void};
use common::{api, serve, sse};

/// 收集正文增量
struct Rec(Arc<Mutex<String>>);

impl OnReply for Rec {
  fn reply(&mut self, s: &str) -> Flow {
    self.0.lock().unwrap().push_str(s);
    Flow::Continue
  }
}

/// 收集系统提示与用户消息
#[derive(Default)]
struct Prompt {
  system: String,
  ask: String,
}

impl OnPrompt for Prompt {
  fn system(&mut self, text: &str) {
    self.system.push_str(text);
  }

  fn ask(&mut self, text: &str) {
    self.ask.push_str(text);
  }
}

/// 忽略思考
struct Nil;

impl OnThink for Nil {
  fn think(&mut self, _: &str) -> Flow {
    Flow::Continue
  }
}

/// 正常一轮返回，块故意乱序，验证按行号归位
const GOOD: &str = "\
行: 2
English: World
中文: 世界
-
行: 1
English: Hello
中文: 你好
";

#[compio::test]
async fn tran_basic() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(GOOD)
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await?;
  assert_eq!(out, ["你好", "世界"]);

  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  // 提示词带上语言与行号
  assert!(req[0].contains("English:"), "{}", req[0]);
  assert!(req[0].contains("中文:"), "{}", req[0]);
  assert!(req[0].contains("行:"), "{}", req[0]);
  OK
}

/// 首轮原文错误且缺行，第二轮只修问题行
#[compio::test]
async fn tran_fix_round() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    if n == 0 {
      sse(
        "\
行: 1
English: Hell
中文: 你好
",
      )
    } else {
      sse(GOOD)
    }
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await?;
  assert_eq!(out, ["你好", "世界"]);

  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 2);
  // 第二轮带上一轮的助手回复，构成多轮修订，并带上原文
  assert!(req[1].contains(r#""role":"assistant""#), "{}", req[1]);
  assert!(req[1].contains("Hello"), "{}", req[1]);
  // 修订带正确原文、上一轮错误译文与期望格式演示
  assert!(req[1].contains("原文：Hello"), "{}", req[1]);
  assert!(req[1].contains("错误译文：你好"), "{}", req[1]);
  assert!(req[1].contains("期望格式示例"), "{}", req[1]);
  // 格式演示不写死行号
  assert!(req[1].contains("行: <行号>"), "{}", req[1]);
  OK
}

/// 一直不返回有效块，按错误行号报错
#[compio::test]
async fn tran_format_error() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse("随便说点什么")
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello"]).await;
  assert!(
    matches!(out, Err(Error::Format(ref li)) if li == &[1]),
    "{out:?}"
  );
  // 首轮没有进展，连续 3 轮不下降后停止
  assert_eq!(seen.lock().unwrap().len(), 3);
  OK
}

/// 有进展就继续修订，不限于固定轮数
#[compio::test]
async fn tran_progress_rounds() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    let mut body = String::new();
    for k in 0..=n {
      if k > 0 {
        body.push_str("-\n");
      }
      let src = char::from(b'a' + k as u8);
      let _ = write!(body, "行: {}\nEnglish: {src}\n中文: 译{}\n", k + 1, k + 1);
    }
    sse(&body)
  });
  let api_li = [api(addr)];
  let src_li = ["a", "b", "c", "d", "e", "f"];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["译1", "译2", "译3", "译4", "译5", "译6"]);
  // 每轮只多解决一行，共 6 轮
  assert_eq!(seen.lock().unwrap().len(), 6);
  OK
}

/// 先有进展、随后连续 3 轮不下降才停止
#[compio::test]
async fn tran_stall_stop() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    // 每轮都只给第 1 行，第 2 行永远缺
    sse("行: 1\nEnglish: Hello\n中文: 你好\n")
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await;
  assert!(
    matches!(out, Err(Error::Format(ref li)) if li == &[2]),
    "{out:?}"
  );
  // 首轮有进展，之后连续 3 轮不下降
  assert_eq!(seen.lock().unwrap().len(), 4);
  OK
}

/// 有进展后停滞计数归零，累计停滞不触发停止
#[compio::test]
async fn tran_stall_reset() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    // 停顿两轮、前进一步，交替进行；累计停滞数会超过 3
    let done = 1 + n / 3;
    if n % 3 == 2 && done <= 3 {
      sse("无")
    } else {
      let mut body = String::new();
      for k in 0..done.min(3) {
        if k > 0 {
          body.push_str("-\n");
        }
        let src = char::from(b'a' + k as u8);
        let _ = write!(body, "行: {}\nEnglish: {src}\n中文: 译{}\n", k + 1, k + 1);
      }
      sse(&body)
    }
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["a", "b", "c"]).await?;
  assert_eq!(out, ["译1", "译2", "译3"]);
  // 若停滞计数不归零，第 5 次请求后就会误停
  assert_eq!(seen.lock().unwrap().len(), 7);
  OK
}

/// 空输入不读配置、不发起请求
#[compio::test]
async fn tran_empty() -> Void {
  let out = tran_with(&[] as &[Api], "中文", "English", &[] as &[&str]).await?;
  assert!(out.is_empty());
  // 同语言直接返回原文，不请求模型
  let out = tran_with(&[] as &[Api], "English", "english", &["Hi"]).await?;
  assert_eq!(out, ["Hi"]);
  OK
}

/// 空行与纯标点/表情行直通，不进请求；行尾 trim、行首还原
#[compio::test]
async fn tran_filter() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(GOOD)
  });
  let api_li = [api(addr)];
  let src_li = ["Hello   ", "", "  ", "...", "😀😀", "  World  "];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["你好", "", "", "...", "😀😀", "  世界"]);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  // 只把两行正文送模型，且已 trim
  assert!(req[0].contains("1: Hello"), "{}", req[0]);
  assert!(req[0].contains("2: World"), "{}", req[0]);
  assert!(!req[0].contains("Hello   "), "{}", req[0]);
  // 纯符号行不进请求
  assert!(!req[0].contains("..."), "{}", req[0]);
  assert!(!req[0].contains("😀"), "{}", req[0]);
  OK
}

/// 超过字符预算自动分批，每批独立请求
#[compio::test]
async fn tran_batch() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  // 两行各占一半预算，合起来必然超限
  let half = CHAR_LIMIT / 2 + 1;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    let (src, dst) = if n == 0 { (&ca, "甲") } else { (&cb, "乙") };
    sse(&format!("行: 1\nEnglish: {src}\n中文: {dst}\n"))
  });
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str()];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 全部为空行/纯符号时直接返回，不发起请求
#[compio::test]
async fn tran_all_pass() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse("")
  });
  let api_li = [api(addr)];
  let src_li = [
    "",
    "  ",
    "...",
    "😀😀",
    "！！",
    "——",
    "---",
    "====",
    "| --- | --- |",
    "|---|---|",
    "| :--- | ---: |",
  ];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(
    out,
    [
      "",
      "",
      "...",
      "😀😀",
      "！！",
      "——",
      "---",
      "====",
      "| --- | --- |",
      "|---|---|",
      "| :--- | ---: |",
    ]
  );
  assert!(seen.lock().unwrap().is_empty());
  OK
}

/// 整段被代码围栏包裹（带语言标记）也能解析
#[compio::test]
async fn tran_fenced() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(&format!("```text\n{GOOD}```\n"))
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await?;
  assert_eq!(out, ["你好", "世界"]);
  // 首轮就完整解析，不触发修订
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}

/// tran_hook 把正文增量转发给回调，结果与 tran_with 一致
#[compio::test]
async fn tran_hook_stream() -> Void {
  let addr = serve(|_, _| sse(GOOD));
  let api_li = [api(addr)];
  let seen = Arc::new(Mutex::new(String::new()));
  let mut reply = Rec(seen.clone());
  let mut think = Nil;
  let got = tran_hook(
    &api_li,
    "English",
    "中文",
    &["Hello", "World"],
    &mut think,
    &mut reply,
  )
  .await?;
  assert_eq!(got, ["你好", "世界"]);
  assert!(seen.lock().unwrap().contains("你好"));
  OK
}

/// 提示词回调收到系统提示与实际发送的用户消息
#[compio::test]
async fn tran_prompt() -> Void {
  let addr = serve(|_, _| sse(GOOD));
  let api_li = [api(addr)];
  let mut p = Prompt::default();
  let out = tran_ask_hook(
    &api_li,
    "English",
    "中文",
    &["Hello", "World"],
    &mut (),
    &mut (),
    &mut p,
  )
  .await?;
  assert_eq!(out, ["你好", "世界"]);
  assert!(p.system.contains("行: <行号>"), "{}", p.system);
  // 每块后必须输出分隔线，格式演示也带上
  assert!(
    p.system.contains("每个块之后必须输出单独一行 -"),
    "{}",
    p.system
  );
  assert!(p.system.contains("中文: 译文\n-"), "{}", p.system);
  assert!(p.ask.contains("1: Hello"), "{}", p.ask);
  assert!(p.ask.contains("2: World"), "{}", p.ask);
  OK
}

/// 行号行 key 不符时视为格式错误，不会把寒暄里的数字当行号
#[compio::test]
async fn tran_bad_key() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    if n == 0 {
      sse("第 1 行\nEnglish: Hello\n中文: 你好\n")
    } else {
      sse(GOOD)
    }
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await?;
  assert_eq!(out, ["你好", "世界"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 行号超出 usize 时按格式错误处理，不 panic
#[compio::test]
async fn tran_big_nth() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    if n == 0 {
      sse("行: 99999999999999999999999999\nEnglish: Hello\n中文: 你好\n")
    } else {
      sse(GOOD)
    }
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await?;
  assert_eq!(out, ["你好", "世界"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 第二批失败时错误行号映射回原文行号，而不是批内序号
#[compio::test]
async fn tran_error_second_batch() -> Void {
  let half = CHAR_LIMIT / 2 + 1;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let ca = a.clone();
  let addr = serve(move |n, _| {
    if n == 0 {
      sse(&format!("行: 1\nEnglish: {ca}\n中文: 甲\n"))
    } else {
      sse("无")
    }
  });
  let out = tran_with(&[api(addr)], "English", "中文", &[a.as_str(), b.as_str()]).await;
  assert!(
    matches!(out, Err(Error::Format(ref li)) if li == &[2]),
    "{out:?}"
  );
  OK
}
