[English](#en) | [中文](#zh)

---

<a name="en"></a>

# ai_i18n

LLM chat client and config loader for Rust, built on the compio async runtime with streaming output, multi-endpoint / multi-token rotation, multi-turn sessions, abort and retries.

- [Crates](#crates)
- [Install](#install)
- [Config](#config)
- [Quick start](#quick-start)
- [Example](#example)
- [Features](#features)
- [Test](#test)
- [License](#license)

## Crates

| crate | Description |
| ----- | ----------- |
| `ai_api` | OpenAI-compatible streaming client: `Chat` / `Session`, separate reasoning and reply callbacks, multi-turn history, abort, exponential backoff |
| `ai_i18n_conf` | NestedText config loader; each endpoint in `api` can set its own `think` |
| `ai_i18n_lang` | Google Translate language codes and localized names |

## Install

```toml
[dependencies]
ai_api = "0.1"
ai_i18n_conf = "0.1"
compio = "0.19"
```

## Config

Reads `~/.config/aii18n.nt` by default; override with `AII18N_CONF_PATH`.

```nt
api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
    model: deepseek-v4.1-flash
    think: medium
  -
    base: https://ollama.com/v1
    token:
      - xxxx
    model: llama3
    # think defaults to medium
```

Per endpoint:

| Field | Description |
| ----- | ----------- |
| `base` | API base, `/chat/completions` is appended |
| `token` | Tokens, tried in rotation |
| `model` | Model name |
| `think` | `off` / `low` / `medium` / `high` / `max`; `medium` by default |

## Quick start

```rust
use ai_api::{Chat, Flow, OnReply, OnThink, fill_line};

struct Gray;

impl OnThink for Gray {
  fn think(&mut self, s: &str) -> Flow {
    eprint!("{s}");
    Flow::Continue
  }

  fn end(&mut self, think: &str) {
    fill_line(think);
  }
}

struct Out;

impl OnReply for Out {
  fn reply(&mut self, s: &str) -> Flow {
    print!("{s}");
    Flow::Continue
  }

  fn end(&mut self, text: &str) {
    fill_line(text);
  }
}

#[compio::main]
async fn main() -> ai_api::Result<()> {
  let conf = ai_i18n_conf::load()?.expect("missing config");
  let mut think = Gray;
  let mut reply = Out;
  let mut chat = Chat::new(&conf.api, &mut think, &mut reply);
  let mut session = chat.session("Be concise");

  let got = session.ask("hello").await?;
  println!("{}", got.text);

  // Multi-turn: history lives in the session
  session.ask("what was my previous question?").await?;
  Ok(())
}
```

`Chat` exposes only `new` and `session`; history, system prompt and the abort handle live on `Session`. Drop it to release history.

## Example

```bash
cargo run -p ai_api --example chat
```

Two fixed turns: `1 + 1 等于几？` then `我上一个问题是什么？` to prove memory.

## Features

- Endpoints tried in order; each starts from a random token and rotates through all of them
- 9 retry rounds by default, 180s idle timeout, exponential backoff for 429/5xx
- Streaming with separate reasoning/reply callbacks; `end` receives the full content
- Abort: `session.abort_handle()` cancels the in-flight request from another task
- `Reply`: `id` / `model` / `think` / `text` / `finish` / `usage` (includes `reasoning` and `cached`; `total()` is prompt + completion)
- `HipStr` strings: inline when short, refcounted when long, shared between history and the returned reply

## Test

```bash
./test.sh
```

## License

Apache-2.0


---

<a name="zh"></a>

# ai_i18n

Rust 的 LLM 对话客户端与配置加载，基于 compio 异步运行时，支持流式输出、多端点与多 token 轮转、多轮对话、打断与重试。

- [Crates](#crates)
- [安装](#安装)
- [配置](#配置)
- [快速开始](#快速开始)
- [示例](#示例)
- [特性](#特性)
- [测试](#测试)
- [License](#license)

## Crates

| crate | 说明 |
| ----- | ---- |
| `ai_api` | OpenAI 兼容的流式对话客户端：`Chat` / `Session`、思考与正文双回调、多轮历史、打断、指数退避重试 |
| `ai_i18n_conf` | NestedText 配置加载，`api` 列表里每个端点可独立配置 `think` |
| `ai_i18n_lang` | Google 翻译语言代码与多语言名称 |

## 安装

```toml
[dependencies]
ai_api = "0.1"
ai_i18n_conf = "0.1"
compio = "0.19"
```

## 配置

默认读取 `~/.config/aii18n.nt`，可用环境变量 `AII18N_CONF_PATH` 指定路径。

```nt
api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
    model: deepseek-v4.1-flash
    think: medium
  -
    base: https://ollama.com/v1
    token:
      - xxxx
    model: llama3
    # 不写 think 则默认 medium
```

每个端点：

| 字段 | 说明 |
| ---- | ---- |
| `base` | 接口地址，拼 `/chat/completions` |
| `token` | 可用 token，按顺序轮转尝试 |
| `model` | 模型名 |
| `think` | 思考强度：`off` / `low` / `medium` / `high` / `max`，缺省 `medium` |

## 快速开始

```rust
use ai_api::{Chat, Flow, OnReply, OnThink, fill_line};

struct Gray;

impl OnThink for Gray {
  fn think(&mut self, s: &str) -> Flow {
    eprint!("{s}");
    Flow::Continue
  }

  fn end(&mut self, think: &str) {
    fill_line(think);
  }
}

struct Out;

impl OnReply for Out {
  fn reply(&mut self, s: &str) -> Flow {
    print!("{s}");
    Flow::Continue
  }

  fn end(&mut self, text: &str) {
    fill_line(text);
  }
}

#[compio::main]
async fn main() -> ai_api::Result<()> {
  let conf = ai_i18n_conf::load()?.expect("缺少配置");
  let mut think = Gray;
  let mut reply = Out;
  let mut chat = Chat::new(&conf.api, &mut think, &mut reply);
  let mut session = chat.session("回答简洁");

  let got = session.ask("你好").await?;
  println!("{}", got.text);

  // 多轮：历史随 session 保留
  session.ask("我上一个问题是什么？").await?;
  Ok(())
}
```

`Chat` 只有 `new` 与 `session` 两个接口；历史、系统提示和打断句柄都在 `Session` 上，用完丢弃即可。

## 示例

```bash
cargo run -p ai_api --example chat
```

固定两轮：先问 `1 + 1 等于几？`，再问 `我上一个问题是什么？`，验证历史记忆。

## 特性

- 端点按顺序尝试，每个端点从随机 token 开始按 `+1` 覆盖全部 token
- 整体重试默认 9 轮，空闲超时默认 180 秒，429/5xx 指数退避
- 流式输出：正文与思考各自回调，`end` 收到完整内容
- 打断：`session.abort_handle()` 可跨任务立即中断
- 返回 `Reply`：`id` / `model` / `think` / `text` / `finish` / `usage`（含思考 `reasoning` 与缓存 `cached`，`total()` 为 prompt + completion）
- 字符串使用 `HipStr`：短串内联、长串引用计数，历史与返回值共享

## 测试

```bash
./test.sh
```

## License

Apache-2.0

