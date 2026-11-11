use serde::Deserialize;

use crate::chat::Think;

/// 单个 API 端点
#[derive(Debug, Clone, Deserialize)]
pub struct Api {
  /// 接口地址
  pub base: String,
  /// 可用 token，按顺序尝试
  pub token: Vec<String>,
  /// 模型名
  pub model: String,
  /// 思考强度，缺省为 `medium`
  #[serde(default)]
  pub think: Think,
}
