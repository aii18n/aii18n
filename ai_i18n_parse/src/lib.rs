#![cfg_attr(docsrs, feature(doc_cfg))]

//! 翻译文本的解析与过滤。
//!
//! 与模型调用完全解耦：输入原始行，输出确定性的行计划、按字符预算切分的
//! 批次，以及提示词生成与回复解析。行计划与输入下标一一对应，
//! 便于上层把原文与译文逐行对齐、比对人改内容。
//!
//! ```
//! use ai_i18n_parse::Plan;
//!
//! let line_li = ["  ## Title", "```js", "});", "```", "..."];
//! let plan = Plan::new(&line_li);
//! assert_eq!(plan.line_li[0].prefix, "  ## ");
//! assert_eq!(plan.line_li[0].body, "Title");
//! // 围栏内纯符号行也要翻译，围栏行本身也送模型
//! assert!(!plan.line_li[2].keep);
//! // 围栏外的纯标点直通
//! assert!(plan.line_li[4].keep);
//! ```

mod batch;
mod fence;
mod line;
mod noise;
mod prompt;

pub use batch::Batch;
pub use line::{Line, Plan};
pub use prompt::{Block, Prompt, Reply, Why};

/// 单批原文的行字符上限，单行超限时独占一批
pub const CHAR_LIMIT: usize = 10_000;
