use std::{env, fs};

use ai_api::Think;
use ai_i18n_conf::{Conf, Error};
use aok::{OK, Void};
use log::info;

#[ctor::ctor(unsafe)]
fn _log_init() {
  log_init::init();
}

#[test]
fn test() -> Void {
  let n = 123456;
  info!("> test {n}");
  OK
}

#[test]
fn test_conf_path() -> Void {
  let path = env::temp_dir().join("aii18n_conf_test.nt");
  unsafe { env::set_var("AII18N_CONF_PATH", &path) };
  assert_eq!(ai_i18n_conf::conf(), path.as_path());
  OK
}

#[test]
fn test_conf_default() -> Void {
  unsafe { env::remove_var("AII18N_CONF_PATH") };
  assert_eq!(
    ai_i18n_conf::conf(),
    confdir::confdir().join("aii18n.nt").as_path()
  );
  OK
}

const TEXT: &str = concat!(
  "# 注释\n",
  "api:\n",
  "  -\n",
  "    base: https://api.example.com/v1\n",
  "    token:\n",
  "      - sk-1\n",
  "      - sk-2\n",
  "    model: deepseek-v4.1-flash\n",
  "    think: max\n",
  "  -\n",
  "    base: https://ollama.com/v1\n",
  "    token:\n",
  "      - sk-3\n",
  "    model: llama3\n",
);

#[test]
fn test_from_str() -> Void {
  let conf: Conf = ai_i18n_conf::from_str(TEXT)?.unwrap();
  assert_eq!(conf.api.len(), 2);
  let api = &conf.api[0];
  assert_eq!(api.base, "https://api.example.com/v1");
  assert_eq!(api.token, ["sk-1", "sk-2"]);
  assert_eq!(api.model, "deepseek-v4.1-flash");
  assert_eq!(api.think, Think::Max);
  // 未配置时默认 medium
  assert_eq!(conf.api[1].model, "llama3");
  assert_eq!(conf.api[1].think, Think::Medium);
  OK
}

#[test]
fn test_from_str_empty() -> Void {
  assert!(ai_i18n_conf::from_str::<Conf>("# 只有注释\n\n")?.is_none());
  assert!(ai_i18n_conf::from_str::<Conf>("# a\r\n  # b\r\n")?.is_none());
  assert!(ai_i18n_conf::from_str::<Conf>("\u{feff}# bom\n")?.is_none());
  OK
}

#[test]
fn test_from_str_err() -> Void {
  assert!(matches!(
    ai_i18n_conf::from_str::<Conf>("key: a\nkey: b\n"),
    Err(Error::Parse(_))
  ));
  OK
}

#[test]
fn test_load_from() -> Void {
  let path = env::temp_dir().join("aii18n_conf_load_test.nt");
  fs::write(&path, TEXT)?;
  let conf: Conf = ai_i18n_conf::load_from(&path)?.unwrap();
  fs::remove_file(&path)?;
  assert_eq!(conf.api.len(), 2);
  assert_eq!(conf.api[0].token[0], "sk-1");
  OK
}

#[test]
fn test_load_from_missing() -> Void {
  let path = env::temp_dir().join("aii18n_conf_missing_test.nt");
  let _ = fs::remove_file(&path);
  assert!(ai_i18n_conf::load_from::<Conf>(&path)?.is_none());
  OK
}

#[test]
fn test_load_conf() -> Void {
  let path = env::temp_dir().join("aii18n_conf_env_test.nt");
  fs::write(&path, TEXT)?;
  unsafe { env::set_var("AII18N_CONF_PATH", &path) };
  let conf = ai_i18n_conf::load()?.unwrap();
  fs::remove_file(&path)?;
  assert_eq!(conf.api[1].base, "https://ollama.com/v1");
  OK
}
