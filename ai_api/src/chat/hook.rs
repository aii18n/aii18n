/// 非空且不以换行结尾时补一个换行
///
/// 供 [`OnThink::end`] / [`OnReply::end`] 复用，避免各回调重复判断。
pub fn fill_line(text: &str) {
  if !text.is_empty() && !text.ends_with('\n') {
    println!();
  }
}

/// 流式回调控制
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
  Continue,
  Stop,
}

/// 思考增量回调
///
/// 每次收到的都是完整 UTF-8 字符串，多字节字符不会跨回调拆分；
/// 无法解析或含非法 UTF-8 的片段直接丢弃。返回 [`Flow::Stop`] 提前结束。
pub trait OnThink {
  fn think(&mut self, s: &str) -> Flow;
  /// 思考段结束：首条正文之前，或流结束时调用
  ///
  /// `think` 为完整思考内容，用于换行、复位样式或落盘。
  fn end(&mut self, _think: &str) {}
}

/// 正文增量回调
///
/// 语义同 [`OnThink`]。
pub trait OnReply {
  fn reply(&mut self, s: &str) -> Flow;
  /// 流结束（正常或异常）时调用
  ///
  /// `text` 为完整正文，用于补换行、落盘等收尾。
  fn end(&mut self, _text: &str) {}
}

/// 不需要某一侧输出时传 `&mut ()`
impl OnThink for () {
  fn think(&mut self, _s: &str) -> Flow {
    Flow::Continue
  }
}

impl OnReply for () {
  fn reply(&mut self, _s: &str) -> Flow {
    Flow::Continue
  }
}
