use std::{io, path::PathBuf, result::Result as StdResult, str::Utf8Error};

/// crate 统一返回类型。
pub type Result<T, E = Error> = StdResult<T, E>;

/// ai_i18n_db 错误。
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Io(#[from] io::Error),
  #[error("内容不是合法 UTF-8")]
  Utf8(#[from] Utf8Error),
  #[error("路径不能为空")]
  EmptyPath,
  #[error("路径含换行符: {0:?}")]
  BadPath(String),
  #[error("语言不能为空")]
  EmptyLang,
  #[error("语言含非法字符: {0:?}")]
  BadLang(String),
  #[error("内容行含换行符")]
  BadLine,
  #[error("数据库已被其他实例或进程占用: {0}")]
  Locked(PathBuf),
}
