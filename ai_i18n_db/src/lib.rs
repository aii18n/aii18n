//! 纯文本、可 diff 的 i18n 数据库。
//!
//! 存储格式（只追加）：
//!
//! ```text
//! @path/lang
//! |line
//! |line
//! ```
//!
//! - `@` 开头是记录头，按最后一个 `/` 拆出 path 与 lang
//! - `|` 开头是内容行
//! - 其他行忽略
//! - 同一 `@path/lang` 后写覆盖先写（读取时最后一条生效）
//!
//! 读取与压实都是流式扫描，内存占用与文件大小无关。

mod db;
mod error;
mod line_reader;

use std::borrow::Cow;

pub use db::Db;
pub use error::{Error, Result};

/// 记录头前缀
const HEADER_PREFIX: u8 = b'@';
/// 内容行前缀
const LINE_PREFIX: u8 = b'|';

/// Windows 下把路径分隔符统一成 `/`，其余平台零拷贝。
#[cfg(windows)]
fn norm_path(path: &str) -> Cow<'_, str> {
  if path.as_bytes().contains(&b'\\') {
    Cow::Owned(path.replace('\\', "/"))
  } else {
    Cow::Borrowed(path)
  }
}

#[cfg(not(windows))]
fn norm_path(path: &str) -> Cow<'_, str> {
  Cow::Borrowed(path)
}
