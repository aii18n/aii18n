//! Markdown 围栏代码块识别
//!
//! 输入是原始行（未 trim）。逐行推进状态：围栏行（``` 或 ~~~）与围栏内的行
//! 都算代码块，供调用方决定是否跳过「纯符号直通」——代码块里的 `});`、
//! `| --- |` 也要交给模型，由提示词约束只翻译注释。
//!
//! 与 CommonMark 对齐：围栏最多缩进 3 个空格（制表符按 4 列算，直接不算
//! 围栏）；反引号围栏的信息串不能含反引号，否则是行内代码；关闭需同字符、
//! 不短于开启长度、其后只有空白；未闭合的围栏一直延续到文末。

/// 围栏最少字符数
const MIN_LEN: usize = 3;
/// 围栏允许的最大缩进空格数
const MAX_INDENT: usize = 3;

/// 围栏代码块状态，整篇只用一个实例
#[derive(Default)]
pub(crate) struct Fence {
  /// 开启中的围栏：字符与长度
  open: Option<(u8, usize)>,
}

impl Fence {
  /// 推进原始行，返回该行是否属于代码块（含围栏行）
  pub(crate) fn step(&mut self, line: &str) -> bool {
    let Some((c, len, tail)) = fence(line) else {
      return self.open.is_some();
    };
    match self.open {
      // 关闭围栏：同字符、不短于开启围栏、其后没有信息串
      Some((oc, olen)) if c == oc && len >= olen && tail.trim().is_empty() => self.open = None,
      Some(_) => {}
      None => self.open = Some((c, len)),
    }
    true
  }
}

/// 取围栏的字符、长度与剩余部分
///
/// 缩进超过 3 个空格不算围栏；制表符不是空格，首个非空格字符不是反引号或
/// 波浪号，因此制表符缩进行同样不算围栏。
fn fence(s: &str) -> Option<(u8, usize, &str)> {
  let indent = s.bytes().take_while(|&b| b == b' ').count();
  if indent > MAX_INDENT {
    return None;
  }
  // indent 只数 ASCII 空格字节，必然落在字符边界上
  let rest = &s[indent..];
  let c = *rest.as_bytes().first()?;
  if c != b'`' && c != b'~' {
    return None;
  }
  let n = rest.bytes().take_while(|&b| b == c).count();
  if n < MIN_LEN {
    return None;
  }
  // n 只数 ASCII 围栏字节，必然落在字符边界上
  let tail = &rest[n..];
  // 反引号围栏的信息串不能含反引号，如 ```a``` 是行内代码
  if c == b'`' && tail.contains('`') {
    return None;
  }
  Some((c, n, tail))
}
