//! 集成测试共用：mock HTTP 服务与 SSE 响应

use std::{
  io::{Read, Write},
  net::{SocketAddr, TcpListener, TcpStream},
  sync::atomic::{AtomicUsize, Ordering},
  thread,
};

use ai_api::{Api, Think};

#[ctor::ctor(unsafe)]
fn _log_init() {
  log_init::init();
}

/// 启动一次性的 mock 服务，`handler` 按第几次请求返回正文
pub fn serve(handler: impl Fn(usize, &str) -> String + Send + 'static) -> SocketAddr {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  thread::spawn(move || {
    let nth = AtomicUsize::new(0);
    for mut s in listener.incoming().flatten() {
      let req = read_req(&mut s);
      let n = nth.fetch_add(1, Ordering::Relaxed);
      let body = handler(n, &req);
      let _ = write!(
        s,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
      );
    }
  });
  addr
}

/// 读到完整请求（按 Content-Length）后返回
fn read_req(s: &mut TcpStream) -> String {
  let mut buf = Vec::new();
  let mut tmp = [0; 4096];
  loop {
    let n = s.read(&mut tmp).unwrap_or(0);
    if n == 0 {
      break;
    }
    buf.extend_from_slice(&tmp[..n]);
    let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
      continue;
    };
    let head = String::from_utf8_lossy(&buf[..pos]);
    let len = head
      .lines()
      .find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case("content-length")
          .then(|| v.trim().parse::<usize>().ok())?
      })
      .unwrap_or(0);
    if buf.len() >= pos + 4 + len {
      break;
    }
  }
  String::from_utf8_lossy(&buf).into_owned()
}

/// 单条正文的 SSE 响应
pub fn sse(text: &str) -> String {
  let content = sonic_rs::to_string(text).unwrap();
  format!("data: {{\"choices\":[{{\"delta\":{{\"content\":{content}}}}}]}}\n\ndata: [DONE]\n\n")
}

/// 指向 mock 服务的端点
pub fn api(addr: SocketAddr) -> Api {
  Api {
    base: format!("http://{addr}/v1"),
    token: vec!["k".into()],
    model: "m".into(),
    think: Think::Off,
  }
}
