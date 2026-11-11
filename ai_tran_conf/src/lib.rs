//! 解析 `aii18n.nt` 任务配置。
//!
//! 配置文件是一个 NestedText 列表，每项是一个翻译任务：
//!
//! ```text
//! -
//!   dir: ./i18n
//!   from: zh
//!   to:
//!     - en
//! ```
//!
//! - `dir`：待翻译目录，必填；缺失时忽略该任务并向 stderr 输出警告
//! - `from`：源语言，必填；缺失时同样忽略并警告
//! - `to`：目标语言，可为空；为空时展开为除源语言外的全部语言
//!
//! 语言代码用 [`ai_i18n_lang::by_code`] 解析，大小写不敏感，`-` 与 `_` 等价。

use std::{fs, io, path::Path};

mod conf;
mod error;

pub use conf::{Conf, Task};
pub use error::{Error, Result};

/// 解析配置文本，空文档（只有空行/注释）返回 `None`
pub fn from_str(text: &str) -> Result<Option<Conf>> {
  if only_comment(text) {
    return Ok(None);
  }
  Ok(Some(conf::parse(text)?))
}

/// 读取并解析配置文件，文件不存在或为空返回 `Ok(None)`
pub fn load_from(path: &Path) -> Result<Option<Conf>> {
  match fs::read_to_string(path) {
    Ok(text) => from_str(&text),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(e.into()),
  }
}

/// 只有空行与注释，与 NestedText 的词法规则一致
fn only_comment(text: &str) -> bool {
  let text = text.strip_prefix('\u{feff}').unwrap_or(text);
  text.split(['\n', '\r']).all(|line| {
    let line = line.trim_start_matches(' ');
    line.is_empty() || line.starts_with('#')
  })
}
