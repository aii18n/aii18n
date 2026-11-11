use std::result;

use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
  #[error(transparent)]
  Api(#[from] ai_api::Error),
  #[error(transparent)]
  Conf(#[from] ai_i18n_conf::Error),
  /// 配置文件里没有 api
  #[error("配置缺少 api")]
  NoApi,
  /// 多轮修订后仍有行未完成，值为 1 基行号
  #[error("翻译未完成，行: {0:?}")]
  Format(Vec<usize>),
}

pub type Result<T> = result::Result<T, Error>;
