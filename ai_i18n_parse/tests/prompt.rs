//! 提示词与回复解析用例

use ai_i18n_parse::{Plan, Prompt, Why};

fn prompt() -> Prompt<'static> {
  Prompt::new("English", "中文")
}

#[test]
fn ask_compact() {
  let src_li = ["Hello", "", "World"];
  let plan = Plan::new(&src_li);
  let batch = plan.batches().next().unwrap();
  assert_eq!(prompt().ask(batch), "1: Hello\n2: World\n");
  assert!(prompt().system().contains("行号: 原文"));
  assert!(prompt().system().contains("中文: 译文\n-"));
}

#[test]
fn parse_reply() {
  let msg = "\
好的，以下是翻译：

**行**: 1
**English**: English:
**中文**: 英文冒号
-
行: 2
English: World
中文: 时间: 12:00
";
  let reply = prompt().parse(msg);
  assert!(!reply.bad);
  assert_eq!(reply.block_li.len(), 2);
  assert_eq!(reply.block_li[0].nth, 1);
  assert_eq!(reply.block_li[0].src, "English:");
  assert_eq!(&*reply.block_li[0].tran, "英文冒号");
  assert_eq!(&*reply.block_li[1].tran, "时间: 12:00");
}

#[test]
fn parse_multiline() {
  let msg = "```text\n行: 1\nEnglish: Hello\n中文: 你\n好\n```\n";
  let reply = prompt().parse(msg);
  assert!(!reply.bad);
  assert_eq!(&*reply.block_li[0].tran, "你好");
}

#[test]
fn parse_bad() {
  let reply = prompt().parse("行: 1\nEnglish: Hello\n");
  assert!(reply.bad);
  assert!(reply.block_li.is_empty());
}

#[test]
fn fix_compact() {
  let src_li = ["a", "b", "c"];
  let plan = Plan::new(&src_li);
  let batch = plan.batches().next().unwrap();
  let why_li = [None, Some(Why::Orig), None];
  let wrong_li = [None, Some("误".to_owned()), None];
  let got = prompt().fix(batch, &[1, 2], &why_li, &wrong_li, true);
  assert!(got.contains("- 第 2 行（原文与输入不一致）原文：b，错误译文：误"));
  assert!(got.contains("2: b\n"));
  assert!(got.contains("3: c\n"));
}
