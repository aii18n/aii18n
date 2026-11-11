use std::{io, result, time::Duration};

use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
  #[error(transparent)]
  Io(#[from] io::Error),
  #[error(transparent)]
  Url(#[from] url::ParseError),
  #[error(transparent)]
  Json(#[from] sonic_rs::Error),
  /// 不支持的协议
  #[error("不支持的协议: {0}")]
  Scheme(String),
  /// 接口地址缺少 host
  #[error("接口地址缺少 host")]
  Host,
  /// 接口地址缺少端口
  #[error("接口地址缺少端口")]
  Port,
  /// TLS 初始化失败
  #[error("TLS 初始化失败")]
  Tls,
  /// token 含控制字符
  #[error("token 含非法字符")]
  Token,
  /// 响应头不完整或格式错误
  #[error("响应头格式错误")]
  Head,
  /// 响应提前结束且没有内容
  #[error("响应为空")]
  Empty,
  /// 分块流被截断
  #[error("流被截断")]
  Cut,
  /// 单行过长，可能是异常数据
  #[error("流式数据行过长")]
  Line,
  /// 分块编码错误
  #[error("分块编码错误")]
  Chunk,
  /// 单个端点返回非 200
  #[error("HTTP {0}: {1}")]
  Status(u16, String),
  /// 流中返回的错误对象
  #[error("接口错误: {0}")]
  Api(String),
  /// 无数据超过指定时长
  #[error("空闲超时 {0:?}")]
  Idle(Duration),
  /// 被调用方打断
  #[error("已打断")]
  Aborted,
  /// 没有可用的端点
  #[error("没有可用的 Api")]
  NoApi,
}

impl Error {
  /// 限流或服务端错误
  pub fn throttled(&self) -> bool {
    matches!(self, Self::Status(code, _) if transient(*code))
  }

  /// 换 token 或换端点可能成功的错误
  pub fn retryable(&self) -> bool {
    match self {
      Self::Io(_)
      | Self::Idle(_)
      | Self::Cut
      | Self::Chunk
      | Self::Head
      | Self::Empty
      | Self::Line
      // token 含控制字符是单个 token 的问题，换下一个 token 可能成功
      | Self::Token => true,
      // 限流与服务端错误值得重试，其余 4xx 是配置或参数问题
      Self::Status(code, _) => transient(*code),
      _ => false,
    }
  }
}

/// 换 token 或稍后重试可能成功的状态码
const fn transient(code: u16) -> bool {
  code == 429 || code >= 500
}

pub type Result<T> = result::Result<T, Error>;
