#![cfg_attr(docsrs, feature(doc_cfg))]

mod by_code;
mod code;
mod name;

pub use by_code::by_code;
pub use code::Code;
pub use name::{EN, NAME, ZH};
