//! 字符上限与分批相关用例

mod common;

use std::sync::{Arc, Mutex};

use ai_i18n_parse::CHAR_LIMIT;
use ai_i18n_tran::tran_with;
use aok::{OK, Void};
use common::{api, serve, sse};

/// 两行字符数正好等于上限，合为一批
#[compio::test]
async fn tran_limit_exact() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let half = CHAR_LIMIT / 2;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(&format!(
      "行: 1\nEnglish: {ca}\n中文: 甲\n-\n行: 2\nEnglish: {cb}\n中文: 乙\n"
    ))
  });
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str()];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}

/// 超限单行独占一批后，后续行另起一批，行号正确回填
#[compio::test]
async fn tran_limit_tail() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let long = "a".repeat(CHAR_LIMIT + 1);
  let tail = long.clone();
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    if n == 0 {
      sse(&format!("行: 1\nEnglish: {tail}\n中文: 甲\n"))
    } else {
      sse("行: 1\nEnglish: b\n中文: 乙\n")
    }
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &[long.as_str(), "b"]).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 一批正好用完上限后，下一行另起一批，行号仍按原文回填
#[compio::test]
async fn tran_limit_exact_tail() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let half = CHAR_LIMIT / 2;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let addr = serve(move |n, req| {
    got.lock().unwrap().push(req.to_owned());
    if n == 0 {
      sse(&format!(
        "行: 1\nEnglish: {ca}\n中文: 甲\n-\n行: 2\nEnglish: {cb}\n中文: 乙\n"
      ))
    } else {
      // 每批独立会话，批内行号从 1 起
      sse("行: 1\nEnglish: c\n中文: 丙\n")
    }
  });
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str(), "c"];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["甲", "乙", "丙"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 字符数按字符而非字节统计，多字节内容不提前拆批
#[compio::test]
async fn tran_limit_multibyte() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let half = CHAR_LIMIT / 2;
  let a = "中".repeat(half);
  let b = "文".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(&format!(
      "行: 1\n中文: {ca}\nEnglish: 甲\n-\n行: 2\n中文: {cb}\nEnglish: 乙\n"
    ))
  });
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str()];
  let out = tran_with(&api_li, "中文", "English", &src_li).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}

/// 单行超过上限时独占一批，不从行中间切开
#[compio::test]
async fn tran_limit_single() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let a = "a".repeat(CHAR_LIMIT + 1);
  let ca = a.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(&format!("行: 1\nEnglish: {ca}\n中文: 甲\n"))
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &[a.as_str()]).await?;
  assert_eq!(out, ["甲"]);
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}
