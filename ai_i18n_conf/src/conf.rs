use ai_api::Api;
use serde::Deserialize;

/// 配置文件根结构
#[derive(Debug, Clone, Deserialize)]
pub struct Conf {
  /// API 端点列表
  pub api: Vec<Api>,
}
