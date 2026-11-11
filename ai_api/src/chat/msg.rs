use hipstr::HipStr;
use serde::{Deserialize, Serialize};

/// 对话角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
  System,
  User,
  Assistant,
}

/// 单条消息
#[derive(Debug, Clone, Serialize)]
pub struct Msg {
  pub role: Role,
  /// 短内容内联，长内容引用计数，克隆 O(1)
  pub content: HipStr<'static>,
}

impl Msg {
  fn new(role: Role, content: impl Into<String>) -> Self {
    Self {
      role,
      content: HipStr::from(content.into()),
    }
  }

  pub fn system(content: impl Into<String>) -> Self {
    Self::new(Role::System, content)
  }

  pub fn user(content: impl Into<String>) -> Self {
    Self::new(Role::User, content)
  }

  pub fn assistant(content: impl Into<String>) -> Self {
    Self::new(Role::Assistant, content)
  }
}

/// 思考强度，对应 OpenAI 兼容接口的 `reasoning_effort`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Think {
  /// 不发送思考参数，由服务端决定
  Off,
  Low,
  #[default]
  Medium,
  High,
  /// 语义上的最高档，线上仍映射 `high`
  Max,
}

impl Think {
  /// 请求体中的取值，`Off` 不发送该字段
  pub const fn effort(self) -> Option<&'static str> {
    match self {
      Self::Off => None,
      Self::Low => Some("low"),
      Self::Medium => Some("medium"),
      Self::High | Self::Max => Some("high"),
    }
  }
}
