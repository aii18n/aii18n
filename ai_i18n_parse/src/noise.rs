//! 纯标点 / 表情行判定
//!
//! 用 Unicode 通用类别生成判定表：标点、符号（含表情）、组合符、格式符、
//! 空白与控制符都视为无需翻译。字母、数字不命中，`$5`、`C#`、`1.0` 仍会翻译。
//! ASCII 用 128 位图 O(1) 命中，其余码点二分有序区间；零堆分配。

include!(concat!(env!("OUT_DIR"), "/noise.rs"));

/// 单个码点是否无需翻译
#[inline]
fn noise(c: u32) -> bool {
  if c < 128 {
    let bit = c as usize;
    ASCII_NOISE[bit >> 6] >> (bit & 63) & 1 != 0
  } else {
    // 区间有序且互不相交，先定位首个 end >= c 的区间，再比 start
    let i = NOISE.partition_point(|&(_, end)| end < c);
    NOISE.get(i).is_some_and(|&(start, _)| start <= c)
  }
}

/// 全部字符都是标点 / 表情等无需翻译的符号，空串视为真
#[inline]
pub(crate) fn only_noise(s: &str) -> bool {
  s.chars().all(|c| noise(c as u32))
}
