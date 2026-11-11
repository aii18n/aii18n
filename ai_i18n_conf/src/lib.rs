#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
  env, fs, io,
  path::{Path, PathBuf},
  sync::OnceLock,
};

use serde::de::DeserializeOwned;

mod conf;
mod error;

pub use conf::Conf;
pub use error::{Error, Result};

/// 直接指定配置文件路径的环境变量
const CONF_PATH: &str = "AII18N_CONF_PATH";
/// 配置文件名
const FILE: &str = "aii18n.nt";

fn resolve() -> PathBuf {
  match env::var_os(CONF_PATH) {
    Some(path) if !path.is_empty() => PathBuf::from(path),
    _ => confdir::confdir().join(FILE),
  }
}

/// 配置文件路径，`AII18N_CONF_PATH` 优先于系统配置目录
pub fn conf() -> &'static Path {
  static CONF: OnceLock<PathBuf> = OnceLock::new();
  CONF.get_or_init(resolve)
}

/// 解析配置文本为指定类型，空文档返回 `None`
pub fn from_str<T: DeserializeOwned>(text: &str) -> Result<Option<T>> {
  if only_comment(text) {
    return Ok(None);
  }
  Ok(Some(nested_text::from_str(text)?))
}

/// 只有空行与注释，与 NestedText 的词法规则一致
///
/// 只按 ASCII 空格计算缩进；制表符等其他空白视为内容，交给解析器报错。
fn only_comment(text: &str) -> bool {
  let text = text.strip_prefix('\u{feff}').unwrap_or(text);
  text.split(['\n', '\r']).all(|line| {
    let line = line.trim_start_matches(' ');
    line.is_empty() || line.starts_with('#')
  })
}

/// 读取并解析指定配置文件，文件不存在或为空时返回 `Ok(None)`
pub fn load_from<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
  match fs::read_to_string(path) {
    Ok(text) => from_str(&text),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(e.into()),
  }
}

/// 读取并解析配置文件，文件不存在或为空时返回 `Ok(None)`
pub fn load() -> Result<Option<Conf>> {
  load_from(conf())
}
