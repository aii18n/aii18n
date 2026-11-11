//! 行计划与分批用例

use ai_i18n_parse::{CHAR_LIMIT, Plan};

#[test]
fn plan_split() {
  let src_li = ["  ## Title  ", "```js", "});", "```", "", "...", "Hello"];
  let plan = Plan::new(&src_li);
  assert_eq!(plan.line_li[0].prefix, "  ## ");
  assert_eq!(plan.line_li[0].body, "Title");
  assert!(!plan.line_li[0].keep);
  // 围栏行与围栏内纯符号行都要翻译
  assert!(!plan.line_li[1].keep);
  assert!(!plan.line_li[2].keep);
  // 空行与围栏外的纯标点直通
  assert!(plan.line_li[4].keep);
  assert!(plan.line_li[5].keep);
  assert_eq!(plan.todo_li, [0, 1, 2, 3, 6]);
  assert_eq!(plan.line_li[0].restore("标题"), "  ## 标题");
}

#[test]
fn plan_batches() {
  // 两行正好等于上限，合为一批
  let half = CHAR_LIMIT / 2;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let src_li = [a.as_str(), "", b.as_str()];
  let plan = Plan::new(&src_li);
  let got_li: Vec<Vec<usize>> = plan.batches().map(|b| b.idx_li.to_vec()).collect();
  assert_eq!(got_li, [vec![0, 2]]);

  // 超限单行独占一批，后续行另起一批
  let long = "a".repeat(CHAR_LIMIT + 1);
  let src_li = [long.as_str(), "b"];
  let plan = Plan::new(&src_li);
  let got_li: Vec<Vec<usize>> = plan.batches().map(|b| b.idx_li.to_vec()).collect();
  assert_eq!(got_li, [vec![0], vec![1]]);
}

#[test]
fn plan_empty() {
  let plan = Plan::new(&[] as &[&str]);
  assert!(plan.line_li.is_empty());
  assert!(plan.todo_li.is_empty());
  assert_eq!(plan.batches().count(), 0);
}

#[test]
fn batches_cover_todo() {
  let a = "a".repeat(CHAR_LIMIT / 2);
  let b = "b".repeat(CHAR_LIMIT / 2);
  let src_li = [a.as_str(), "", "...", "```", "});", "```", &b];
  let plan = Plan::new(&src_li);
  // 所有批次按序拼起来正好覆盖待译行
  let all_li: Vec<usize> = plan
    .batches()
    .flat_map(|batch| batch.idx_li.iter().copied())
    .collect();
  assert_eq!(all_li, plan.todo_li);
  // 每批非空，字符数不超限，超限单行独占一批
  for batch in plan.batches() {
    assert!(!batch.idx_li.is_empty());
    let cost: usize = batch
      .idx_li
      .iter()
      .map(|&i| plan.line_li[i].body.chars().count())
      .sum();
    assert!(cost <= CHAR_LIMIT || batch.idx_li.len() == 1);
  }
}
