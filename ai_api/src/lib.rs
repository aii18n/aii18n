#![cfg_attr(docsrs, feature(doc_cfg))]

//! ai_i18n 的 API 端点配置与流式对话客户端。
//!
//! - [`Api`]：端点（`base` / `token` / `model` / `think`）
//! - [`Chat`]：客户端，只有 `new` 与 `session`
//! - [`Session`]：会话，持有系统提示与历史，按端点顺序、token 轮转、整体重试
//! - [`OnThink`] / [`OnReply`]：思考与正文的流式回调，返回 [`Flow::Stop`] 可提前结束
//! - [`Opt`]：空闲超时与重试轮数，默认 180 秒 / 9 轮
//! - [`Abort`]：打断句柄，从 `Session` 取得，跨任务中断进行中的请求
//!
//! 复杂度：响应按字节线性扫描，每字节只搜索一次；行完整落在载荷段时零拷贝，
//! 跨分块才拼接。额外内存为单次读块与最长一行。
//! 返回的 [`Reply`] 持有完整正文与思考，空间与输出长度成正比。
//!
//! 需在 compio 运行时内调用：
//!
//! ```no_run
//! use ai_api::{Api, Chat, Flow, OnReply, OnThink, Think};
//!
//! struct Gray;
//! struct Out;
//!
//! impl OnThink for Gray {
//!   fn think(&mut self, s: &str) -> Flow {
//!     eprint!("{s}");
//!     Flow::Continue
//!   }
//! }
//!
//! impl OnReply for Out {
//!   fn reply(&mut self, s: &str) -> Flow {
//!     print!("{s}");
//!     Flow::Continue
//!   }
//! }
//!
//! #[compio::main]
//! async fn main() -> ai_api::Result<()> {
//! let api_li = [Api {
//!   base: "https://api.example.com/v1".into(),
//!   token: vec!["sk-xxx".into()],
//!   model: "deepseek-v4.1-flash".into(),
//!   think: Think::Max,
//! }];
//! let mut think = Gray;
//! let mut reply = Out;
//! let mut chat = Chat::new(&api_li, &mut think, &mut reply);
//! let mut session = chat.session("你是简洁的助手");
//! let got = session.ask("你好").await?;
//! println!("\n{}", got.text);
//! Ok(())
//! }
//! ```

mod abort;
mod api;
mod chat;
mod error;
mod http;

pub use abort::Abort;
pub use api::Api;
pub use chat::{
  Chat, Finish, Flow, IDLE, Id, Msg, OnReply, OnThink, Opt, RETRY, Reply, Role, Session, Think,
  Usage, fill_line,
};
pub use error::{Error, Result};
