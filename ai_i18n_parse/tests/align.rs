//! 原文与译文分别解析后按行对齐

use ai_i18n_parse::Plan;

#[test]
fn align_source_and_translation() {
  let src_li = ["  ## Title", "Hello", "", "...", "World"];
  let dst_li = ["  ## 标题", "你好", "", "...", "世界"];
  let a = Plan::new(&src_li);
  let b = Plan::new(&dst_li);
  assert_eq!(a.line_li.len(), b.line_li.len());
  // 行下标一一对应，直通判定一致
  for (x, y) in a.line_li.iter().zip(&b.line_li) {
    assert_eq!(x.keep, y.keep);
  }
  assert_eq!(b.line_li[0].prefix, "  ## ");

  // 模拟用户修改译文第 5 行
  let user_li = ["  ## 标题", "你好", "", "...", "世界啊"];
  let c = Plan::new(&user_li);
  let changed_li: Vec<usize> = b
    .line_li
    .iter()
    .zip(&c.line_li)
    .enumerate()
    .filter(|(_, (x, y))| x.body != y.body)
    .map(|(i, _)| i)
    .collect();
  assert_eq!(changed_li, [4]);
}
