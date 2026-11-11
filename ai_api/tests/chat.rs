use std::{
  io::{Read, Write},
  net::{SocketAddr, TcpListener, TcpStream},
  sync::{Arc, Mutex},
  thread,
  time::{Duration, Instant},
};

use ai_api::{
  Abort, Api, Chat, Error, Finish, Flow, Id, OnReply, OnThink, Opt, Role, Think, Usage,
};
use aok::{OK, Void};
use compio::{runtime::Runtime, time::sleep};

/// 启动 mock 服务器
fn serve(handler: impl Fn(&str) -> (u16, String, bool) + Send + 'static) -> SocketAddr {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  thread::spawn(move || {
    for stream in listener.incoming().flatten() {
      let mut s = stream;
      let req = read_req(&mut s);
      let (status, body, chunked) = handler(&req);
      if chunked {
        let _ = write!(
          s,
          "HTTP/1.1 {status} OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n"
        );
        // 按 5 字节切块，故意切断多字节字符
        for piece in body.as_bytes().chunks(5) {
          let _ = write!(s, "{:x}\r\n", piece.len());
          let _ = s.write_all(piece);
          let _ = s.write_all(b"\r\n");
        }
        let _ = s.write_all(b"0\r\n\r\n");
      } else {
        let len = body.len();
        let _ = write!(
          s,
          "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {len}\r\n\r\n{body}"
        );
      }
    }
  });
  addr
}

/// 原样写回响应字节，用于构造异常场景
fn serve_raw(handler: impl Fn(&str) -> Vec<u8> + Send + 'static) -> SocketAddr {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  thread::spawn(move || {
    for stream in listener.incoming().flatten() {
      let mut s = stream;
      let req = read_req(&mut s);
      let _ = s.write_all(&handler(&req));
    }
  });
  addr
}

fn read_req(s: &mut TcpStream) -> String {
  let mut buf = Vec::new();
  let mut tmp = [0; 4096];
  loop {
    let n = s.read(&mut tmp).unwrap_or(0);
    if n == 0 {
      break;
    }
    buf.extend_from_slice(&tmp[..n]);
    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
      let head = String::from_utf8_lossy(&buf[..i]).to_ascii_lowercase();
      let len = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
      while buf.len() < i + 4 + len {
        let n = s.read(&mut tmp).unwrap_or(0);
        if n == 0 {
          break;
        }
        buf.extend_from_slice(&tmp[..n]);
      }
      break;
    }
  }
  String::from_utf8_lossy(&buf).into_owned()
}

const SSE: &str = concat!(
  "data: {\"id\":\"cmpl-1\",\"model\":\"m\",\"choices\":[{\"delta\":{\"reasoning_content\":\"思考\"}}]}\n\n",
  "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n",
  "data: {\"choices\":[{\"delta\":{\"content\":\"好\"},\"finish_reason\":\"stop\"}]}\n\n",
  "data: {\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3,\"prompt_tokens_details\":{\"cached_tokens\":1},\"completion_tokens_details\":{\"reasoning_tokens\":1}}}\n\n",
  "data: [DONE]\n\n",
);

/// 收集思考
#[derive(Default)]
struct Thinker {
  inner: String,
}

impl OnThink for Thinker {
  fn think(&mut self, s: &str) -> Flow {
    self.inner.push_str(s);
    Flow::Continue
  }
}

/// 收集正文
#[derive(Default)]
struct Replier {
  inner: String,
}

impl OnReply for Replier {
  fn reply(&mut self, s: &str) -> Flow {
    self.inner.push_str(s);
    Flow::Continue
  }
}

fn api(addr: SocketAddr, token: &[&str]) -> Api {
  Api {
    base: format!("http://{addr}/v1"),
    token: token.iter().map(|s| s.to_string()).collect(),
    model: "test-model".into(),
    think: Think::default(),
  }
}

#[compio::test]
async fn stream_text_and_think() -> Void {
  let seen = Arc::new(Mutex::new(String::new()));
  let got = seen.clone();
  let addr = serve(move |req| {
    *got.lock().unwrap() = req.to_string();
    (200, SSE.to_string(), true)
  });

  let mut api_li = [api(addr, &["sk-1"])];
  api_li[0].think = Think::Max;
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("系统提示");
  session.user("你好");
  let reply = session.send().await?;

  assert_eq!(session.on_think().inner, "思考");
  assert_eq!(session.on_reply().inner, "你好");
  // 返回值是聚合后的完整响应
  assert_eq!(reply.think, "思考");
  assert_eq!(reply.text, "你好");
  assert_eq!(reply.id.to_string(), "cmpl-1");
  assert_eq!(reply.model, "m");
  assert_eq!(reply.finish, Finish::Stop);
  assert_eq!(reply.usage.total(), 3);
  // 思考 token 含在 completion 内，单独给出
  assert_eq!(reply.usage.reasoning, 1);
  assert_eq!(reply.usage.cached, 1);

  let req = seen.lock().unwrap();
  // 思考设置取端点配置（最大）
  assert!(req.contains(r#""reasoning_effort":"high""#), "{req}");
  assert!(req.contains(r#""stream":true"#), "{req}");
  assert!(req.contains(r#""include_usage":true"#), "{req}");
  assert!(req.contains("Authorization: Bearer sk-1"), "{req}");
  assert!(req.contains(r#""role":"system""#), "{req}");
  OK
}

#[compio::test]
async fn multi_turn_keeps_history() -> Void {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let got = seen.clone();
  let addr = serve(move |req| {
    got.lock().unwrap().push(req.to_string());
    (200, SSE.to_string(), true)
  });

  let api_li = [api(addr, &["k"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("旧提示");
  session.system("新提示");
  session.ask("第一问").await?;
  session.ask("第二问").await?;

  let msg_li = session.msg();
  assert_eq!(msg_li.len(), 5, "{msg_li:?}");
  assert_eq!(msg_li[0].role, Role::System);
  assert_eq!(msg_li[0].content, "新提示");
  assert_eq!(msg_li[1].content, "第一问");
  assert_eq!(msg_li[2].role, Role::Assistant);
  assert_eq!(msg_li[2].content, "你好");
  assert_eq!(msg_li[3].content, "第二问");
  assert_eq!(msg_li[4].role, Role::Assistant);
  assert_eq!(msg_li[4].content, "你好");

  let req_li = seen.lock().unwrap();
  assert_eq!(req_li.len(), 2);
  // 第二轮请求带上第一轮回复
  assert!(req_li[1].contains(r#""role":"assistant""#), "{}", req_li[1]);
  assert!(req_li[1].contains("你好"), "{}", req_li[1]);
  // session 只替换，不追加
  assert_eq!(req_li[1].matches(r#""role":"system""#).count(), 1);
  OK
}

#[compio::test]
async fn fail_then_next_token() -> Void {
  let addr = serve(|req| {
    if req.contains("Bearer bad") {
      (500, r#"{"error":"bad token"}"#.to_string(), false)
    } else {
      (200, SSE.to_string(), true)
    }
  });

  let api_li = [api(addr, &["bad", "good"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let reply = session.send().await?;
  assert_eq!(session.on_reply().inner, "你好");
  assert_eq!(reply.text, "你好");
  OK
}

#[compio::test]
async fn all_fail_returns_status() -> Void {
  let addr = serve(|_| (500, "boom".to_string(), false));
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let ret = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await;
  assert!(matches!(ret, Err(Error::Status(500, _))), "{ret:?}");
  OK
}

#[compio::test]
async fn idle_timeout() -> Void {
  let addr = serve(|_| {
    thread::sleep(Duration::from_millis(400));
    (200, SSE.to_string(), false)
  });
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let ret = session
    .send_with(Opt {
      idle: Duration::from_millis(50),
      retry: 1,
    })
    .await;
  assert!(matches!(ret, Err(Error::Idle(_))), "{ret:?}");
  OK
}

#[compio::test]
async fn null_choices_keeps_usage() -> Void {
  // choices: null 不应丢整行，同行的 usage 要保留
  let body = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n",
    "data: {\"choices\":null,\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":null}}\n\n",
    "data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\n",
    "data: [DONE]\n\n",
  );
  let addr = serve_raw(move |_| body.as_bytes().to_vec());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  let reply = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await?;
  assert_eq!(reply.text, "好");
  assert_eq!(reply.usage.prompt, 3);
  assert_eq!(reply.usage.completion, 4);
  assert_eq!(reply.usage.total(), 7);
  OK
}

#[test]
fn think_effort() -> Void {
  assert_eq!(Think::Off.effort(), None);
  assert_eq!(Think::Low.effort(), Some("low"));
  assert_eq!(Think::Medium.effort(), Some("medium"));
  assert_eq!(Think::Max.effort(), Some("high"));
  // 默认 medium
  assert_eq!(Think::default(), Think::Medium);
  OK
}

/// 分块响应头
const CHUNKED: &str =
  "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";

fn chunk(data: &str) -> String {
  format!("{:x}\r\n{data}\r\n", data.len())
}

#[compio::test]
async fn truncated_stream_fails() -> Void {
  // 声明 100 字节的块，只发 5 字节就断开
  let raw = format!("{CHUNKED}64\r\nabcde");
  let addr = serve_raw(move |_| raw.clone().into_bytes());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let ret = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await;
  assert!(matches!(ret, Err(Error::Cut)), "{ret:?}");
  OK
}

#[compio::test]
async fn broken_after_output_fails_without_retry() -> Void {
  // 一个完整的正文块，接着一个声明的块只发一半
  let part = chunk("data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n");
  let raw = format!("{CHUNKED}{part}a\r\nabc");
  let addr = serve_raw(move |_| raw.clone().into_bytes());
  let api_li = [api(addr, &["a", "b"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let ret = session
    .send_with(Opt {
      retry: 3,
      ..Opt::default()
    })
    .await;
  assert!(matches!(ret, Err(Error::Cut)), "{ret:?}");
  assert_eq!(session.on_reply().inner, "你");
  OK
}

#[compio::test]
async fn plain_close_stream_ok() -> Void {
  // 非分块，靠关连接结束，且没有 [DONE]
  let raw = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n{SSE}");
  let addr = serve_raw(move |_| raw.clone().into_bytes());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let reply = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await?;
  assert_eq!(session.on_reply().inner, "你好");
  assert_eq!(reply.text, "你好");
  OK
}

#[compio::test]
async fn invalid_utf8_not_emitted() -> Void {
  // 非法字节出现在 JSON 字符串里，整行应被丢弃，不进入回调
  let mut body = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n".to_vec();
  body.extend_from_slice(b"data: {\"choices\":[{\"delta\":{\"content\":\"\xff\"}}]}\n\n");
  body.extend_from_slice(b"data: [DONE]\n\n");
  let addr = serve_raw(move |_| body.clone());

  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let ret = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await;
  assert!(session.on_reply().inner.is_empty());
  assert!(session.on_think().inner.is_empty());
  assert!(matches!(ret, Err(Error::Empty)), "{ret:?}");
  OK
}

#[compio::test]
async fn trailing_line_without_newline_once() -> Void {
  // 末行没有换行、也没有 [DONE]，只应投递一次
  let body = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n",
    "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}",
  );
  let addr = serve_raw(move |_| body.as_bytes().to_vec());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let reply = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await?;
  assert_eq!(session.on_reply().inner, "你");
  assert_eq!(reply.text, "你");
  OK
}

#[compio::test]
async fn escaped_content_kept() -> Void {
  // JSON 转义（换行、引号）不能丢内容
  let body = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n",
    "data: {\"choices\":[{\"delta\":{\"content\":\"一\\n二\\\"三\"}}]}\n\n",
    "data: [DONE]\n\n",
  );
  let addr = serve_raw(move |_| body.as_bytes().to_vec());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let reply = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await?;
  let want = "一\n二\"三";
  assert_eq!(session.on_reply().inner, want);
  assert_eq!(reply.text, want);
  OK
}

/// 在思考增量里立刻打断
struct Aborter {
  abort: Abort,
  inner: String,
}

impl OnThink for Aborter {
  fn think(&mut self, s: &str) -> Flow {
    self.inner.push_str(s);
    self.abort.abort();
    Flow::Continue
  }
}

#[compio::test]
async fn abort_stops_stream() -> Void {
  let addr = serve(|_| (200, SSE.to_string(), true));
  let api_li = [api(addr, &["a"])];
  let mut think = Aborter {
    abort: Abort::new(),
    inner: String::new(),
  };
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  // 回调里用同一份打断句柄
  let handle = session.abort_handle();
  session.on_think().abort = handle.clone();
  let ret = session
    .send_with(Opt {
      retry: 3,
      ..Opt::default()
    })
    .await;
  assert!(matches!(ret, Err(Error::Aborted)), "{ret:?}");
  assert_eq!(session.on_think().inner, "思考");
  // 打断后不再有正文，也不重试
  assert!(session.on_reply().inner.is_empty());
  assert!(handle.is_aborted());
  OK
}

#[compio::test]
async fn abort_interrupts_pending_read() -> Void {
  // 服务端迟迟不响应，打断应立刻结束等待，而不是等空闲超时
  let addr = serve(|_| {
    thread::sleep(Duration::from_millis(400));
    (200, SSE.to_string(), false)
  });
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  let handle = session.abort_handle();
  Runtime::current()
    .spawn(async move {
      sleep(Duration::from_millis(50)).await;
      handle.abort();
    })
    .detach();
  let started = Instant::now();
  let ret = session
    .send_with(Opt {
      idle: Duration::from_secs(5),
      retry: 1,
    })
    .await;
  assert!(matches!(ret, Err(Error::Aborted)), "{ret:?}");
  assert!(started.elapsed() < Duration::from_millis(300));
  OK
}

#[test]
fn hex_id_is_binary() -> Void {
  // 十六进制 id 内部为二进制，显示还原
  let id = Id::parse("00ff10");
  assert_eq!(id.as_bytes(), [0x00, 0xff, 0x10]);
  assert_eq!(id.to_string(), "00ff10");
  // 非十六进制按原文
  let id = Id::parse("cmpl-1");
  assert_eq!(id.to_string(), "cmpl-1");
  OK
}

#[compio::test]
async fn bad_endpoint_skipped() -> Void {
  // 第一个端点地址非法，应跳过并继续后面的端点
  let addr = serve(|_| (200, SSE.to_string(), true));
  let mut bad = api(addr, &["x"]);
  bad.base = "not a url".into();
  let api_li = [bad, api(addr, &["good"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  let got = session.ask("hi").await?;
  assert_eq!(got.text, "你好");
  OK
}

#[compio::test]
async fn null_usage_defaults_zero() -> Void {
  let body = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n",
    "data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}],\"usage\":{\"prompt_tokens\":null,\"total_tokens\":null}}\n\n",
    "data: [DONE]\n\n",
  );
  let addr = serve_raw(move |_| body.as_bytes().to_vec());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.user("hi");
  let got = session
    .send_with(Opt {
      retry: 1,
      ..Opt::default()
    })
    .await?;
  assert_eq!(got.text, "好");
  assert_eq!(got.usage, Usage::default());
  OK
}

#[compio::test]
async fn second_endpoint_used() -> Void {
  // 第一个端点 500，第二个端点成功；请求体按端点模型分别构造
  let seen = Arc::new(Mutex::new(String::new()));
  let got = seen.clone();
  let addr = serve(move |req| {
    if req.contains("first-model") {
      (500, "bad".to_string(), false)
    } else {
      *got.lock().unwrap() = req.to_string();
      (200, SSE.to_string(), true)
    }
  });
  let mut first = api(addr, &["t"]);
  first.model = "first-model".into();
  let mut second = api(addr, &["t"]);
  second.model = "second-model".into();
  let api_li = [first, second];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  let got = session.ask("hi").await?;
  assert_eq!(got.text, "你好");
  assert_eq!(got.model, "m");

  let req = seen.lock().unwrap();
  assert!(req.contains(r#""model":"second-model""#), "{req}");
  assert!(req.contains(r#""role":"user""#), "{req}");
  OK
}

#[compio::test]
async fn think_end_before_reply() -> Void {
  // 思考段结束时（首条正文前）立即回调 end，且只回调一次
  type Log = Arc<Mutex<Vec<&'static str>>>;

  struct T(Log);
  impl OnThink for T {
    fn think(&mut self, _: &str) -> Flow {
      self.0.lock().unwrap().push("think");
      Flow::Continue
    }

    fn end(&mut self, think: &str) {
      assert!(!think.is_empty());
      self.0.lock().unwrap().push("think_end");
    }
  }

  struct R(Log);
  impl OnReply for R {
    fn reply(&mut self, _: &str) -> Flow {
      self.0.lock().unwrap().push("reply");
      Flow::Continue
    }

    fn end(&mut self, text: &str) {
      assert!(!text.is_empty());
      self.0.lock().unwrap().push("reply_end");
    }
  }

  let addr = serve(|_| (200, SSE.to_string(), true));
  let api_li = [api(addr, &["a"])];
  let log: Log = Arc::default();
  let mut think = T(log.clone());
  let mut reply = R(log.clone());
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  session.ask("hi").await?;
  drop(session);

  assert_eq!(
    *log.lock().unwrap(),
    ["think", "think_end", "reply", "reply", "reply_end"]
  );
  OK
}

#[compio::test]
async fn history_shares_reply_text() -> Void {
  // 长文本回复与历史共享同一份堆内存，不整段复制
  let long = "a".repeat(100);
  let body = format!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n\
     data: {{\"choices\":[{{\"delta\":{{\"content\":\"{long}\"}}}}]}}\n\n\
     data: [DONE]\n\n"
  );
  let addr = serve_raw(move |_| body.clone().into_bytes());
  let api_li = [api(addr, &["a"])];
  let mut think = Thinker::default();
  let mut reply = Replier::default();
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("");
  let got = session.ask("hi").await?;
  let msg = &session.msg()[2];
  assert_eq!(msg.role, Role::Assistant);
  assert_eq!(got.text.as_ptr(), msg.content.as_ptr());
  assert_eq!(got.text.len(), msg.content.len());
  OK
}
