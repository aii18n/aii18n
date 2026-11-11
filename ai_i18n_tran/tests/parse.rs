//! 回复解析相关用例：寒暄、特殊语言名与折行译文

mod common;

use ai_i18n_tran::tran_with;
use aok::{OK, Void};
use common::{api, serve, sse};

/// 寒暄、Markdown 强调、正文与标签同形、值里带冒号都不影响解析
#[compio::test]
async fn tran_noise() -> Void {
  let addr = serve(|_, _| {
    sse(
      "\
好的，以下是翻译：

**行**: 1
**English**: English:
**中文**: 英文冒号
-
行: 2
English: World
中文: 时间: 12:00
",
    )
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["English:", "World"]).await?;
  assert_eq!(out, ["英文冒号", "时间: 12:00"]);
  OK
}

/// 语言名带 `#` 不能被当成 Markdown 强调剥掉
#[compio::test]
async fn tran_hash_lang() -> Void {
  let addr = serve(|_, _| {
    sse(
      "\
行: 1
C#: var x = 1;
中文: 变量 x 等于 1;
",
    )
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "C#", "中文", &["var x = 1;"]).await?;
  assert_eq!(out, ["变量 x 等于 1;"]);
  OK
}

/// 模型自行折行时，译文续行并入并删除换行
#[compio::test]
async fn tran_multiline() -> Void {
  let addr = serve(|_, _| {
    sse(
      "\
行: 1
English: Hello
中文: 你
好
-
行: 2
English: World
中文:
世
界
",
    )
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &["Hello", "World"]).await?;
  assert_eq!(out, ["你好", "世界"]);
  OK
}
