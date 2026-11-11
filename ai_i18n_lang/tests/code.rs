use ai_i18n_lang::{Code, NAME, by_code};
use strum::VariantArray as _;

#[test]
fn every_code_round_trips() {
  for &code in Code::VARIANTS {
    let s = code.as_str();
    assert_eq!(by_code(s), Some(code), "{s}");
    assert_eq!(by_code(s.to_ascii_uppercase()), Some(code), "{s}");
  }
}

#[test]
fn region_and_case() {
  assert_eq!(by_code("zh-CN"), Some(Code::Zh));
  assert_eq!(by_code("zh-TW"), Some(Code::ZhTW));
  assert_eq!(by_code("zh_HK"), Some(Code::ZhTW));
  assert_eq!(by_code("zh-Hant"), Some(Code::ZhTW));
  assert_eq!(by_code("zh-Hans"), Some(Code::Zh));
  assert_eq!(by_code("ZH-cn"), Some(Code::Zh));
  assert_eq!(by_code("en-US"), Some(Code::En));
  assert_eq!(by_code("pt_BR"), Some(Code::Pt));
  assert_eq!(by_code("fil-PH"), Some(Code::Fil));
  assert_eq!(by_code(" xx "), None);
  assert_eq!(by_code(""), None);
}

#[test]
fn to_str() {
  assert_eq!(Code::ZhTW.as_str(), "zh-TW");
  assert_eq!(Code::ZhTW.as_ref(), "zh-TW");
  for &code in Code::VARIANTS {
    assert_eq!(code.as_str(), code.to_string());
    assert_eq!(AsRef::<str>::as_ref(&code), code.as_str());
    assert!(!NAME[code as usize].is_empty());
  }
}
