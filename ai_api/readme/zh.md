# ai_api

`ai_i18n` 的 API 端点数据结构，通过 serde 从配置反序列化。

## 安装

```toml
[dependencies]
ai_api = "0.1"
```

## 结构

`Api` 对应一个 API 端点：

| 字段    | 类型          | 说明                   |
| ------- | ------------- | ---------------------- |
| `base`  | `String`      | 接口地址               |
| `token` | `Vec<String>` | 可用 token，按顺序尝试 |
| `model` | `String`      | 模型名                 |
| `think` | `Think`      | 思考强度，缺省 `medium`，可覆盖 |

## 用法

```rust
use ai_api::Api;

let api = Api {
  base: "https://api.example.com/v1".into(),
  token: vec!["sk-xxx".into()],
  model: "deepseek-v4.1-flash".into(),
};
assert_eq!(api.token, ["sk-xxx"]);
```

配置文件加载见 `ai_i18n_conf`。

## 流式请求

基于 compio 的 OpenAI 兼容 `POST {base}/chat/completions`，`stream: true`。

- 端点按顺序尝试，一个失败换下一个
- 每个端点从随机 token 开始，按 `+1` 轮转直到遍历全部 token
- 整轮失败后从头再来，默认 9 轮（`Opt::retry`）
- 连接或任意一次读取超过 `Opt::idle`（默认 180 秒）视为失败
- 已经开始输出后失败不再重试，避免重复内容

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
  let mut session = chat.session("你是简洁的助手");
  let got = session.ask("你好").await?;
  println!("{}", got.text);

  // 多轮：历史随 session 保留
  session.ask("继续").await?;
  Ok(())
}
```

| 类型 | 说明 |
| ---- | ---- |
| `OnThink` | 思考增量回调：`think`；返回 `Flow::Stop` 提前结束；`end(&str)` 在思考段结束（首条正文前或流结束）时收到完整思考 |
| `OnReply` | 正文增量回调：`reply`；`end(&str)` 在流结束时收到完整正文 |
| `fill_line` | 非空且末尾不是换行时补一个换行，供两个 `end` 复用 |
| `Opt` | `idle` 空闲超时（默认 180 秒），`retry` 整体重试轮数（默认 9） |
| `Think` | `Off` / `Low` / `Medium` / `High` / `Max`，映射到 `reasoning_effort`，`Off` 不发送；配置文件里写 `think: medium` 等，缺省 `medium` |
| `Chat` | 客户端，无历史：只有 `new(api_li, &mut on_think, &mut on_reply)` 与 `session(系统提示词)` |
| `Session` | 会话，持有系统提示与历史：`system` / `user` / `assistant` / `msg` / `ask` / `send` / `send_with` / `abort_handle`；丢弃即释放历史 |
| `Session::send_with` | 用自定义 `Opt` 发送 |
| `Session::abort_handle` | 取打断句柄，跨任务中断进行中的请求 |
| `Reply` | 聚合后的完整响应：`id`（`Id`，十六进制存二进制）、`model`、`think`、`text`、`finish`（`Finish` 枚举）、`usage`（`prompt` / `completion` / 思考 `reasoning` / 缓存 `cached`，`total()` 由 prompt + completion 相加；未返回时全 0）；字段均非 `Option`，字符串为 `HipStr`（短串内联、长串引用计数） |

## 错误与重试

`Error::retryable()` 决定是否换下一个 token：

- 可重试：`Io`、`Idle`、`Cut`、`Chunk`、`Head`、`Empty`、`Line`，以及 `Status` 为 429 或 5xx
- 429 与 5xx 指数退避：200ms 起，5s 上限，带抖动
- 立即返回：其余 `Status`，以及 `Api`、`Json`、`Url`、`Scheme`、`Host`、`Port`、`Tls`、`Token`、`NoApi`、`Aborted`

分块流中途断开返回 `Error::Cut`，单行超过 1 MiB 返回 `Error::Line`。
缺少 `[DONE]` 但连接正常关闭时按完成处理并打 `warn`。

复杂度：响应按字节线性扫描，每字节只搜索一次；行完整落在载荷段时零拷贝，跨分块才拼接。额外内存为单次读块与最长一行；`Reply` 持有完整输出。
回调收到的始终是完整 UTF-8 字符串，多字节字符不会跨回调拆分；非法 UTF-8 或无法解析的片段会被丢弃。

只处理第一个 choice（`n=1`），多 choice 响应会忽略其余项。

## 示例

`examples/chat.rs` 读取 `~/.config/aii18n.nt`（或 `AII18N_CONF_PATH`），跑固定两轮对话（第二问「我上一个问题是什么」验证记忆），思考灰色、正文默认色，结束后打印 `Reply` 的 `Debug`：

```bash
cargo run -p ai_api --example chat
```

### 打断

```rust
let handle = session.abort_handle();
// 另一个任务里
handle.abort();
```

进行中的 `send` 会在下一次 I/O 或流片段处返回 `Error::Aborted`，不重试；已经输出的内容与 `end` 回调保留。
每次 `send` 开始都会重置打断标记。
