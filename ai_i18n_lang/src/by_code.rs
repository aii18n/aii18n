use std::str::FromStr;

use crate::Code;

/// 繁体中文的区域 / 文字子标签
const HANT: [&str; 4] = ["TW", "HK", "MO", "Hant"];

/// 解析语言代码，如 `zh-CN` → `Zh`、`zh-TW` → `ZhTW`、`en-US` → `En`
pub fn by_code(code: impl AsRef<str>) -> Option<Code> {
  let code = code.as_ref().trim();
  let (main, rest) = code.split_once(['-', '_']).unwrap_or((code, ""));
  // zh 按区域 / 文字细分繁简
  if main.eq_ignore_ascii_case("zh") {
    let traditional = rest
      .split(['-', '_'])
      .any(|s| HANT.iter().any(|t| s.eq_ignore_ascii_case(t)));
    return Some(if traditional { Code::ZhTW } else { Code::Zh });
  }
  Code::from_str(main).ok()
}
