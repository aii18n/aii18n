# ai_api

API endpoint data structure for `ai_i18n`, deserialized from config via serde.

## Install

```toml
[dependencies]
ai_api = "0.1"
```

## Struct

`Api` describes one API endpoint:

| Field   | Type          | Description               |
| ------- | ------------- | ------------------------- |
| `base`  | `String`      | API endpoint              |
| `token` | `Vec<String>` | Tokens, tried in order    |
| `model` | `String`      | Model name                |

## Usage

```rust
use ai_api::Api;

let api = Api {
  base: "https://api.example.com/v1".into(),
  token: vec!["sk-xxx".into()],
  model: "deepseek-v4.1-flash".into(),
};
assert_eq!(api.token, ["sk-xxx"]);
```

See `ai_i18n_conf` for config file loading.

## Streaming

OpenAI-compatible `POST {base}/chat/completions` with `stream: true`, powered by compio.

- Endpoints are tried in order; a failure moves to the next one
- Each endpoint starts from a random token, then `+1` until all tokens are covered
- After a full round fails, the whole list is retried; 9 rounds by default (`Opt::retry`)
- Connect or any single read exceeding `Opt::idle` (180s by default) counts as a failure
- Once output has started, failures are not retried, to avoid duplicated content

```rust,no_run
use ai_api::{Api, Chat, Flow, OnReply, OnThink, Think};

/// 思考
struct Gray;

impl OnThink for Gray {
  fn think(&mut self, s: &str) -> Flow {
    eprint!("{s}");
    Flow::Continue
  }
}

/// 正文
struct Out;

impl OnReply for Out {
  fn reply(&mut self, s: &str) -> Flow {
    print!("{s}");
    Flow::Continue
  }
}

#[compio::main]
async fn main() -> ai_api::Result<()> {
  let api_li = vec![Api {
    base: "https://api.example.com/v1".into(),
    token: vec!["sk-xxx".into()],
    model: "deepseek-v4.1-flash".into(),
    think: Think::Max,
  }];
  let mut think = Gray;
  let mut reply = Out;
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("You are concise.");
  let got = session.ask("hello").await?;
  println!("{}", got.text);

  // Multi-turn: history is kept by the session
  session.ask("continue").await?;
  Ok(())
}
```

| Item | Description |
| ---- | ----------- |
| `OnThink` | Reasoning delta callback: `think`; return `Flow::Stop` to finish early; `end(&str)` receives the full reasoning when reasoning ends, before the first content delta or at stream end |
| `OnReply` | Content delta callback: `reply`; `end(&str)` receives the full text when the stream ends |
| `fill_line` | Append a newline when the text is non-empty and does not end with one; shared by both `end` callbacks |
| `Opt` | `idle` timeout (180s default), `retry` full rounds (9 default) |
| `Think` | `Off` / `Low` / `Medium` / `High` / `Max`, mapped to `reasoning_effort`; `Off` sends nothing. In config: `think: medium`; `medium` by default |
| `Chat` | Client without history: only `new(api_li, &mut on_think, &mut on_reply)` and `session(system_prompt)` |
| `Session` | Conversation holding the system prompt and history: `system` / `user` / `assistant` / `msg` / `ask` / `send` / `send_with` / `abort_handle`; drop it to release history |
| `Session::send_with` | Send with a custom `Opt` |
| `Session::abort_handle` | Get an abort handle to cancel the in-flight request from another task |
| `Reply` | Aggregated full response: `id` (`Id`, hex stored as binary), `model`, `think`, `text`, `finish` (`Finish` enum), `usage` (`prompt` / `completion` / `reasoning` / `cached`, `total()` is prompt + completion; zeros when absent). No field is an `Option`; strings are `HipStr` (inline when short, refcounted when long) |

## Errors and retries

`Error::retryable()` decides whether to move on to the next token:

- Retryable: `Io`, `Idle`, `Cut`, `Chunk`, `Head`, `Empty`, `Line`, and `Status` 429 or 5xx
- Exponential backoff for 429/5xx: 200ms start, 5s cap, with jitter
- Returned at once: other `Status`, plus `Api`, `Json`, `Url`, `Scheme`, `Host`, `Port`, `Tls`, `Token`, `NoApi`, `Aborted`

A chunked stream cut mid-chunk yields `Error::Cut`; a line over 1 MiB yields `Error::Line`.
A clean close without `[DONE]` is treated as complete, with a `warn`.

Complexity: one linear byte scan with a single search per byte; lines inside one payload segment are zero-copy, only lines spanning chunks are assembled. Extra memory is one read chunk plus the longest line. `Reply` holds the full output.
Callbacks always receive complete UTF-8 strings; multi-byte characters are never split across callbacks. Invalid UTF-8 or unparsable fragments are dropped.

Only the first choice (`n=1`) is handled; extra choices are ignored.

## Example

`examples/chat.rs` loads `~/.config/aii18n.nt` (or `AII18N_CONF_PATH`), runs a fixed two-turn chat (the second turn asks what the previous question was, to prove memory), with gray reasoning and plain content, then prints the `Debug` of `Reply`:

```bash
cargo run -p ai_api --example chat
```

### Abort

```rust
let handle = session.abort_handle();
// from another task
handle.abort();
```

The in-flight `send` returns `Error::Aborted` at the next I/O or stream fragment, without retrying; already emitted content and the `end` callback are kept.
The abort flag is reset at the start of every `send`.
