use std::path::PathBuf;

use ai_i18n_lang::{Code, NAME, by_code};
use serde::Deserialize;
use strum::VariantArray;

use crate::{Error, Result};

/// 配置根：任务列表
#[derive(Debug, Clone, Default)]
pub struct Conf {
  pub task: Vec<Task>,
}

/// 单个翻译任务
#[derive(Debug, Clone)]
pub struct Task {
  /// 待翻译目录（必填）
  pub dir: PathBuf,
  /// 源语言
  pub from: Code,
  /// 目标语言；配置为空时展开为全部语言
  pub to: Vec<Code>,
}

impl Task {
  /// 源语言的自称
  pub fn from_name(&self) -> &'static str {
    NAME[self.from as usize]
  }

  /// 目标语言的自称，与 `to` 一一对应
  pub fn to_name(&self) -> impl Iterator<Item = &'static str> + '_ {
    self.to.iter().map(|code| NAME[*code as usize])
  }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct RawConf(Vec<RawTask>);

#[derive(Deserialize)]
struct RawTask {
  dir: Option<PathBuf>,
  from: Option<String>,
  #[serde(default)]
  to: ToRaw,
}

/// `to` 支持列表，也容忍空值
#[derive(Deserialize)]
#[serde(untagged)]
enum ToRaw {
  One(String),
  Many(Vec<String>),
}

impl Default for ToRaw {
  fn default() -> Self {
    Self::Many(Vec::new())
  }
}

impl ToRaw {
  fn into_li(self) -> Vec<String> {
    match self {
      Self::Many(li) => li,
      // `to:` 后没有内容时，NestedText 会给出空字符串
      Self::One(text) if text.trim().is_empty() => Vec::new(),
      Self::One(text) => vec![text],
    }
  }
}

pub(crate) fn parse(text: &str) -> Result<Conf> {
  let raw: RawConf = nested_text::from_str(text)?;
  let mut task = Vec::with_capacity(raw.0.len());
  for (i, raw) in raw.0.into_iter().enumerate() {
    let Some(dir) = raw.dir.filter(|dir| !dir.as_os_str().is_empty()) else {
      warn_skip(i, "dir");
      continue;
    };
    let Some(from) = raw.from.filter(|from| !from.trim().is_empty()) else {
      warn_skip(i, "from");
      continue;
    };
    let from = lang(&from)?;
    let mut to = code_li(raw.to.into_li())?;
    // 为空即全部语言（排除源语言）
    if to.is_empty() {
      to.extend(Code::VARIANTS.iter().copied().filter(|code| *code != from));
    }
    task.push(Task { dir, from, to });
  }
  Ok(Conf { task })
}

fn lang(text: &str) -> Result<Code> {
  by_code(text).ok_or_else(|| Error::Lang(text.to_owned()))
}

/// 配置缺字段时跳过该任务并告警
fn warn_skip(index: usize, field: &str) {
  eprintln!("ai_tran_conf: 第 {} 个任务缺少 {field}，已忽略", index + 1);
}

fn code_li(text_li: Vec<String>) -> Result<Vec<Code>> {
  text_li.into_iter().map(|text| lang(&text)).collect()
}
