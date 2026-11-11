#![cfg_attr(docsrs, feature(doc_cfg))]

//! 拆分并还原 Markdown 行首前缀。
//!
//! 前缀包含行首空白与 Markdown 结构标记（标题、引用、列表、任务列表），
//! 正文是其余内容。翻译只处理正文，写回时用 [`restore`] 原样拼回前缀。
//! 引号、逗号等普通标点留在正文里，不当作前缀拆掉。
//!
//! 代码块内可传 `markdown = false`，只拆空白缩进。
//!
//! ```
//! use md_prefix::{Part, restore};
//!
//! let part = Part::new("  ## Title", true);
//! assert_eq!(part.prefix, "  ## ");
//! assert_eq!(part.body, "Title");
//! assert_eq!(restore(part.prefix, part.body), "  ## Title");
//! ```

/// 拆分结果，内容均为原串切片
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part<'a> {
  /// 行首前缀：空白与 Markdown 结构标记
  pub prefix: &'a str,
  /// 去掉前缀与行尾空白的内容，可能为空
  pub body: &'a str,
}

impl<'a> Part<'a> {
  /// 拆一行；`markdown` 为 false 时只拆空白缩进
  pub fn new(line: &'a str, markdown: bool) -> Self {
    let end = line.trim_end();
    let mut start = end.len() - end.trim_start().len();
    if markdown {
      while let Some(n) = md(&end[start..]) {
        start += n;
      }
    }
    Self {
      prefix: &end[..start],
      body: &end[start..],
    }
  }
}

/// 还原行首前缀
pub fn restore(prefix: &str, body: &str) -> String {
  let mut out = String::with_capacity(prefix.len() + body.len());
  out.push_str(prefix);
  out.push_str(body);
  out
}

/// 行首 Markdown 标记的字节数，含标记后的空白
///
/// 支持标题、引用、无序 / 有序列表与任务列表。标记后必须是空白或行尾，
/// 因此 `#tag`、`-1`、`1.0`、`*强调*` 不会被误拆。
fn md(s: &str) -> Option<usize> {
  const MAX_HASH: usize = 6;
  const MAX_DIGIT: usize = 9;
  let b = s.as_bytes();
  let first = *b.first()?;
  let mut i = match first {
    b'#' => {
      let n = b
        .iter()
        .take(MAX_HASH + 1)
        .take_while(|&&c| c == b'#')
        .count();
      if n > MAX_HASH {
        return None;
      }
      n
    }
    b'>' => b.iter().take_while(|&&c| c == b'>').count(),
    b'-' | b'+' | b'*' => 1,
    b'0'..=b'9' => {
      let n = b
        .iter()
        .take(MAX_DIGIT + 1)
        .take_while(|c| c.is_ascii_digit())
        .count();
      if n > MAX_DIGIT || n >= b.len() || !matches!(b[n], b'.' | b')') {
        return None;
      }
      n + 1
    }
    _ => return None,
  };
  // 标记后必须是空白或行尾
  if i < b.len() {
    let rest = &s[i..];
    let ws = rest.len() - rest.trim_start().len();
    if ws == 0 {
      return None;
    }
    i += ws;
  }
  // 任务列表：`- [ ] ` / `- [x] `
  if matches!(first, b'-' | b'+' | b'*')
    && let Some(tail) = s.get(i..)
    && (tail.starts_with("[ ]") || tail.starts_with("[x]") || tail.starts_with("[X]"))
  {
    let rest = &tail[3..];
    let ws = rest.len() - rest.trim_start().len();
    if ws > 0 || rest.is_empty() {
      i += 3 + ws;
    }
  }
  Some(i)
}
