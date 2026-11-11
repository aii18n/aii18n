//! 一批待译行的视图

use crate::{Line, Plan};

/// 按字符预算切分后的一批行
#[derive(Debug, Clone, Copy)]
pub struct Batch<'a, 'p> {
  /// 所属计划
  plan: &'p Plan<'a>,
  /// 本批行下标，保持输入顺序
  pub idx_li: &'p [usize],
}

impl<'a, 'p> Batch<'a, 'p> {
  /// 由计划与下标切片构造
  pub(crate) const fn new(plan: &'p Plan<'a>, idx_li: &'p [usize]) -> Self {
    Self { plan, idx_li }
  }

  /// 批内第 `k` 行
  #[inline]
  pub fn line(&self, k: usize) -> &'p Line<'a> {
    &self.plan.line_li[self.idx_li[k]]
  }
}
