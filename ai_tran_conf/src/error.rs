use std::{io, result};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error(transparent)]
  Io(#[from] io::Error),
  #[error(transparent)]
  Parse(#[from] nested_text::Error),
  #[error("未知语言: {0}")]
  Lang(String),
}

pub type Result<T> = result::Result<T, Error>;
