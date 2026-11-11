use std::{fmt, str};

use hipstr::HipStr;
use serde::{Deserialize, Deserializer};

/// 响应 id
///
/// 十六进制 id 内部存二进制，内存减半，显示时还原为小写十六进制；
/// 非十六进制（如 `chatcmpl-xxx`）按原字节保存，显示原文。
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Id {
  buf: Box<[u8]>,
  /// 原始 id 是十六进制
  hex: bool,
}

impl Id {
  /// 从服务端字符串构造
  pub fn parse(s: &str) -> Self {
    let raw = s.as_bytes();
    if raw.is_empty() || raw.len() % 2 != 0 || !raw.iter().all(u8::is_ascii_hexdigit) {
      return Self {
        buf: raw.into(),
        hex: false,
      };
    }
    let mut buf = Vec::with_capacity(raw.len() / 2);
    for [hi, lo] in raw.as_chunks::<2>().0 {
      buf.push(hex_val(*hi) << 4 | hex_val(*lo));
    }
    Self {
      buf: buf.into_boxed_slice(),
      hex: true,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.buf.is_empty()
  }

  /// 原始二进制（十六进制 id 已解码）
  pub fn as_bytes(&self) -> &[u8] {
    &self.buf
  }
}

const fn hex_val(c: u8) -> u8 {
  match c {
    b'0'..=b'9' => c - b'0',
    b'a'..=b'f' => c - b'a' + 10,
    _ => c - b'A' + 10,
  }
}

/// 小写十六进制字符表
const HEX: &[u8; 16] = b"0123456789abcdef";

impl fmt::Display for Id {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if !self.hex {
      // 由 `Id::parse(&str)` 写入，必为合法 UTF-8
      return f.write_str(unsafe { str::from_utf8_unchecked(&self.buf) });
    }
    // 每 32 字节编码进栈上 64 字节缓冲，避免逐字节走格式化器
    let mut buf = [0u8; 64];
    for chunk in self.buf.chunks(buf.len() / 2) {
      let n = chunk.len() * 2;
      for (k, &b) in chunk.iter().enumerate() {
        buf[2 * k] = HEX[(b >> 4) as usize];
        buf[2 * k + 1] = HEX[(b & 0xf) as usize];
      }
      // 上表只含 ASCII 十六进制字符，切片必然合法
      f.write_str(unsafe { str::from_utf8_unchecked(&buf[..n]) })?;
    }
    Ok(())
  }
}

impl fmt::Debug for Id {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }
}

/// 结束原因
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Finish {
  /// 服务端未返回
  #[default]
  Unknown,
  Stop,
  Length,
  ToolCalls,
  ContentFilter,
  Other(HipStr<'static>),
}

impl Finish {
  /// 从服务端字符串构造
  pub fn parse(s: &str) -> Self {
    match s {
      "stop" => Self::Stop,
      "length" => Self::Length,
      "tool_calls" => Self::ToolCalls,
      "content_filter" => Self::ContentFilter,
      _ => Self::Other(HipStr::from(s.to_owned())),
    }
  }
}

/// token 用量
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
  #[serde(default, rename = "prompt_tokens", deserialize_with = "null_u32")]
  pub prompt: u32,
  #[serde(default, rename = "completion_tokens", deserialize_with = "null_u32")]
  pub completion: u32,
  /// 思考 token，已包含在 `completion` 内
  #[serde(
    default,
    rename = "completion_tokens_details",
    deserialize_with = "reasoning"
  )]
  pub reasoning: u32,
  /// 命中缓存的 prompt token
  #[serde(default, rename = "prompt_tokens_details", deserialize_with = "cached")]
  pub cached: u32,
}

impl Usage {
  /// 总 token 数，由 prompt + completion 计算，省一个字段
  pub const fn total(&self) -> u32 {
    self.prompt.saturating_add(self.completion)
  }
}

/// `completion_tokens_details`
#[derive(Deserialize, Default)]
struct ReasoningDetails {
  #[serde(default, deserialize_with = "null_u32")]
  reasoning_tokens: u32,
}

/// `prompt_tokens_details`
#[derive(Deserialize, Default)]
struct CachedDetails {
  #[serde(default, deserialize_with = "null_u32")]
  cached_tokens: u32,
}

fn reasoning<'de, D: Deserializer<'de>>(de: D) -> Result<u32, D::Error> {
  Ok(Option::<ReasoningDetails>::deserialize(de)?.map_or(0, |d| d.reasoning_tokens))
}

fn cached<'de, D: Deserializer<'de>>(de: D) -> Result<u32, D::Error> {
  Ok(Option::<CachedDetails>::deserialize(de)?.map_or(0, |d| d.cached_tokens))
}

/// 兼容 `null`，缺失或 `null` 都归 0
fn null_u32<'de, D: Deserializer<'de>>(de: D) -> Result<u32, D::Error> {
  Ok(Option::<u32>::deserialize(de)?.unwrap_or(0))
}

/// 完整响应，流式过程中同步聚合
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reply {
  /// 响应 id
  pub id: Id,
  /// 服务端返回的模型名
  pub model: HipStr<'static>,
  /// 思考全文
  pub think: HipStr<'static>,
  /// 正文全文
  pub text: HipStr<'static>,
  /// 结束原因
  pub finish: Finish,
  /// token 用量，服务端未返回时全为 0
  pub usage: Usage,
}

/// 流式聚合中间态：正文与思考先追加到 `String`，结束时一次性转 [`HipStr`]
#[derive(Default)]
pub(crate) struct Acc {
  pub id: Id,
  pub model: HipStr<'static>,
  pub think: String,
  pub text: String,
  pub finish: Finish,
  pub usage: Usage,
}

impl Acc {
  pub(crate) fn reply(self) -> Reply {
    Reply {
      id: self.id,
      model: self.model,
      think: self.think.into(),
      text: self.text.into(),
      finish: self.finish,
      usage: self.usage,
    }
  }
}
