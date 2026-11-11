# ai_i18n

LLM chat client and config loader for Rust, built on the compio async runtime with streaming output, multi-endpoint / multi-token rotation, multi-turn sessions, abort and retries.

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
