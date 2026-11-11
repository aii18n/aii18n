//! Markdown 行首前缀拆分用例

mod common;

use std::sync::{Arc, Mutex};

use ai_i18n_tran::tran_with;
use aok::{OK, Void};
use common::{api, serve, sse};

/// 标题、引用、列表、任务列表前缀拆分后还原；引号留在正文
#[compio::test]
async fn tran_prefix() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(
      "\
行: 1
English: Title
中文: 标题
-
行: 2
English: quote
中文: 引用
-
行: 3
English: item
中文: 项目
-
行: 4
English: first
中文: 第一
-
行: 5
English: nested
中文: 嵌套
-
行: 6
English: task
中文: 任务
-
行: 7
English: \"Hello\"
中文: \"你好\"
",
    )
  });
  let api_li = [api(addr)];
  let src_li = [
    "## Title",
    "> quote",
    "- item",
    "1. first",
    "> - nested",
    "- [ ] task",
    "\"Hello\"",
  ];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(
    out,
    [
      "## 标题",
      "> 引用",
      "- 项目",
      "1. 第一",
      "> - 嵌套",
      "- [ ] 任务",
      "\"你好\"",
    ]
  );
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  for line in [
    "1: Title",
    "2: quote",
    "3: item",
    "4: first",
    "5: nested",
    "6: task",
  ] {
    assert!(req[0].contains(line), "{line}\n{}", req[0]);
  }
  // 请求体是 JSON，引号会被转义
  assert!(req[0].contains(r#"7: \"Hello\""#), "{}", req[0]);
  for (i, raw) in [
    "## Title",
    "> quote",
    "- item",
    "1. first",
    "> - nested",
    "- [ ] task",
  ]
  .into_iter()
  .enumerate()
  {
    assert!(!req[0].contains(&format!("{}: {raw}", i + 1)), "{}", req[0]);
  }
  OK
}

/// 代码块内不拆 Markdown 标记
#[compio::test]
async fn tran_prefix_code() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |_, req| {
    got.lock().unwrap().push(req.to_owned());
    sse(
      "\
行: 1
English: ```
中文: ```
-
行: 2
English: - item
中文: - item
-
行: 3
English: ## title
中文: ## title
-
行: 4
English: ```
中文: ```
",
    )
  });
  let api_li = [api(addr)];
  let src_li = ["```", "- item", "## title", "```"];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, src_li);
  let req = seen.lock().unwrap();
  assert!(req[0].contains("2: - item"), "{}", req[0]);
  assert!(req[0].contains("3: ## title"), "{}", req[0]);
  OK
}
