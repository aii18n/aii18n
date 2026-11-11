use std::{
  borrow::Cow, future::Future, io::Write, mem::take, result, str, sync::OnceLock, time::Duration,
};

use compio::{
  BufResult,
  io::{AsyncRead, AsyncWrite, AsyncWriteExt},
  net::TcpStream,
  time::timeout,
};
use compio_tls::{TlsConnector, native_tls};
use log::{debug, warn};
use memchr::memmem;
use native_tls::TlsConnector as NativeTlsTlsConnector;
use serde::Deserialize;
use smallvec::SmallVec;
use url::Url;

use crate::{
  abort::Abort,
  api::Api,
  chat::{Acc, Finish, Flow, Id, OnReply, OnThink, Reply, Usage},
  error::{Error, Result},
};

/// 单次读取上限
const CAP: usize = 8 * 1024;
/// 响应头上限
const MAX_HEAD: usize = 64 * 1024;
/// 错误体最多保留的字节数
const MAX_ERR: usize = 512;
/// SSE 单行上限
const MAX_LINE: usize = 1 << 20;
/// 单个分块上限
const MAX_CHUNK: usize = 1 << 26;
/// 分块长度行上限
const MAX_SIZE_LINE: usize = 64;
/// 已消费数据超过该长度就压缩缓冲区
const COMPACT: usize = 8 * 1024;
/// 流结束标记
const DONE: &[u8] = b"[DONE]";

/// 复用的 TLS 连接器，避免每次请求重建；初始化失败不缓存
static TLS: OnceLock<TlsConnector> = OnceLock::new();

fn tls() -> Result<&'static TlsConnector> {
  if let Some(conn) = TLS.get() {
    return Ok(conn);
  }
  let conn = NativeTlsTlsConnector::new().map_err(|_| Error::Tls)?;
  let _ = TLS.set(TlsConnector::from(conn));
  TLS.get().ok_or(Error::Tls)
}

/// 单次请求的失败信息
pub(crate) struct Fail {
  pub err: Error,
  /// 是否已经向回调输出过内容
  pub started: bool,
}

impl Fail {
  fn of(err: Error) -> Self {
    Self {
      err,
      started: false,
    }
  }
}

impl From<Error> for Fail {
  fn from(err: Error) -> Self {
    Self::of(err)
  }
}

type Res<T> = result::Result<T, Fail>;

/// 两侧流式回调
///
/// 思考段结束时（首条正文之前或流结束）立刻调用 [`OnThink::end`]，
/// 保证思考与正文之间的换行、颜色复位发生在正确时机。
pub(crate) struct Emit<'a, T: OnThink, R: OnReply> {
  think: &'a mut T,
  reply: &'a mut R,
  think_open: bool,
}

impl<T: OnThink, R: OnReply> Emit<'_, T, R> {
  fn think(&mut self, s: &str) -> Flow {
    self.think_open = true;
    self.think.think(s)
  }

  fn reply(&mut self, s: &str, think: &str) -> Flow {
    self.close_think(think);
    self.reply.reply(s)
  }

  fn close_think(&mut self, think: &str) {
    if self.think_open {
      self.think_open = false;
      self.think.end(think);
    }
  }

  /// 流结束：收尾两侧
  fn end(&mut self, think: &str, text: &str) {
    self.close_think(think);
    self.reply.end(text);
  }
}

/// 正常结束：通知回调收尾
fn ok(emit: &mut Emit<'_, impl OnThink, impl OnReply>, acc: Acc) -> Res<Reply> {
  let out = acc.reply();
  emit.end(&out.think, &out.text);
  Ok(out)
}

/// 失败：同样通知回调收尾，避免界面上残留思考样式
fn fail(
  emit: &mut Emit<'_, impl OnThink, impl OnReply>,
  acc: &Acc,
  err: Error,
  started: bool,
) -> Res<Reply> {
  emit.end(&acc.think, &acc.text);
  Err(Fail { err, started })
}

/// 带空闲超时的等待
async fn idle<F: Future>(d: Duration, f: F) -> Result<F::Output> {
  timeout(d, f).await.map_err(|_| Error::Idle(d))
}

/// 空闲超时 + 打断
async fn io_wait<T, F: Future<Output = T>>(abort: &Abort, dur: Duration, f: F) -> Result<T> {
  abort.race(idle(dur, f)).await?
}

/// 端点的解析结果，避免每次重试重复解析 URL
pub(crate) struct Target {
  /// 连接用主机名，IPv6 去方括号
  host: String,
  /// Host 头，IPv6 保留方括号
  head: String,
  path: String,
  port: u16,
  https: bool,
}

impl Target {
  pub(crate) fn new(api: &Api) -> Result<Self> {
    let base = &api.base;
    let url = Url::parse(base)?;
    let https = match url.scheme() {
      "https" => true,
      "http" => false,
      s => return Err(Error::Scheme(s.into())),
    };
    let raw = url.host_str().ok_or(Error::Host)?;
    let port = url.port_or_known_default().ok_or(Error::Port)?;
    let host = raw
      .trim_start_matches('[')
      .trim_end_matches(']')
      .to_string();
    let mut path = url.path().trim_end_matches('/').to_string();
    path.push_str("/chat/completions");
    let mut head = raw.to_string();
    if let Some(p) = url.port() {
      head.push(':');
      head.push_str(&p.to_string());
    }
    Ok(Self {
      host,
      head,
      path,
      port,
      https,
    })
  }
}

/// 请求一个端点的一个 token
pub(crate) async fn call(
  target: &Target,
  token: &str,
  body: &[u8],
  dur: Duration,
  abort: &Abort,
  on_think: &mut impl OnThink,
  on_reply: &mut impl OnReply,
) -> Res<Reply> {
  if token.bytes().any(|b| b.is_ascii_control()) {
    return Err(Fail::of(Error::Token));
  }
  let tcp = io_wait(
    abort,
    dur,
    TcpStream::connect((target.host.as_str(), target.port)),
  )
  .await?
  .map_err(|e| Fail::of(e.into()))?;

  let mut emit = Emit {
    think: on_think,
    reply: on_reply,
    think_open: false,
  };
  if target.https {
    let io = io_wait(abort, dur, tls()?.connect(&target.host, tcp))
      .await?
      .map_err(|e| Fail::of(e.into()))?;
    stream(io, target, token, body, dur, abort, &mut emit).await
  } else {
    stream(tcp, target, token, body, dur, abort, &mut emit).await
  }
}

async fn stream<S>(
  mut io: S,
  target: &Target,
  token: &str,
  body: &[u8],
  dur: Duration,
  abort: &Abort,
  emit: &mut Emit<'_, impl OnThink, impl OnReply>,
) -> Res<Reply>
where
  S: AsyncRead + AsyncWrite + Unpin,
{
  let Target {
    head: host, path, ..
  } = target;
  // 一次分配直接拼出请求，避免中间 String
  let len = body.len();
  let mut req = Vec::with_capacity(256 + host.len() + path.len() + token.len() + len);
  let _ = write!(
    req,
    "POST {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nAccept: text/event-stream\r\nUser-Agent: ai_api\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
  );
  req.extend_from_slice(body);
  let BufResult(res, _) = io_wait(abort, dur, io.write_all(req)).await?;
  res.map_err(|e| Fail::of(e.into()))?;

  // 读到响应头结束，复用同一个读缓冲；只扫描新增部分，避免 O(n^2)
  let mut head = Vec::with_capacity(CAP);
  let mut buf = Vec::with_capacity(CAP);
  let mut scan = 0;
  let sep = loop {
    if let Some(i) = memmem::find(&head[scan..], b"\r\n\r\n") {
      break scan + i;
    }
    // 先找分隔符再判上限，避免同一批数据里已有完整头部却被误判过长
    if head.len() > MAX_HEAD {
      return Err(Fail::of(Error::Head));
    }
    // 结尾可能只匹配到分隔符的一部分，回退 3 字节
    scan = head.len().saturating_sub(3);
    buf.clear();
    let BufResult(res, next) = io_wait(abort, dur, io.read(buf)).await?;
    buf = next;
    let n = res.map_err(|e| Fail::of(e.into()))?;
    if n == 0 {
      return Err(Fail::of(Error::Head));
    }
    head.extend_from_slice(&buf[..n]);
  };

  let (status, chunked) = parse_head(&head[..sep]).ok_or_else(|| Fail::of(Error::Head))?;
  // 头部与已读到的正文留在同一缓冲，直接从分隔符之后开始消费，免去一次拷贝
  // 头部阶段的读缓冲继续作为流式读缓冲，省一次分配
  let mut s = Stream::new(io, chunked, head, sep + 4, buf, abort.clone());

  if status != 200 {
    let msg = match s.raw(dur, MAX_ERR).await {
      Ok(msg) => msg,
      Err(e) if abort.is_aborted() => return Err(Fail::of(e)),
      Err(_) => Vec::new(),
    };
    return Err(Fail::of(Error::Status(
      status,
      String::from_utf8_lossy(&msg).into_owned(),
    )));
  }

  let mut reply = Acc::default();
  let mut started = false;
  let mut done = false;
  loop {
    let line = match s.line(dur).await {
      Ok(Some(line)) => line,
      Ok(None) => break,
      Err(err) => return fail(emit, &reply, err, started),
    };
    let Some(data) = line.strip_prefix(b"data:") else {
      continue;
    };
    let data = data.trim_ascii();
    if data.is_empty() {
      continue;
    }
    if data == DONE {
      done = true;
      break;
    }
    let chunk: Chunk<'_> = match sonic_rs::from_slice(data) {
      Ok(chunk) => chunk,
      Err(e) => {
        warn!("忽略无法解析的流片段: {e}");
        continue;
      }
    };
    if let Some(e) = chunk.error {
      return fail(emit, &reply, Error::Api(e.message.into_owned()), started);
    }
    if reply.id.is_empty()
      && let Some(id) = chunk.id
    {
      reply.id = Id::parse(&id);
    }
    if reply.model.is_empty()
      && let Some(model) = chunk.model
    {
      reply.model = model.into_owned().into();
    }
    if let Some(usage) = chunk.usage {
      reply.usage = usage;
    }
    if let Some(choice) = chunk.choices.and_then(|c| c.into_iter().next()) {
      if let Some(finish) = choice.finish_reason {
        reply.finish = Finish::parse(&finish);
      }
      let delta = choice.delta;
      if let Some(think) = delta.reasoning_content {
        let think = think.as_ref();
        if !think.is_empty() {
          started = true;
          reply.think.push_str(think);
          if emit.think(think) == Flow::Stop {
            return ok(emit, reply);
          }
        }
      }
      if let Some(text) = delta.content {
        let text = text.as_ref();
        if !text.is_empty() {
          started = true;
          reply.text.push_str(text);
          if emit.reply(text, &reply.think) == Flow::Stop {
            return ok(emit, reply);
          }
        }
      }
    }
  }
  if !done {
    // 部分服务端不发 [DONE] 直接关连接，有内容时按完成处理
    debug!("流结束但缺少 [DONE]");
  }
  if reply.text.is_empty() && reply.think.is_empty() {
    fail(emit, &reply, Error::Empty, started)
  } else {
    ok(emit, reply)
  }
}

/// 解析状态码与是否分块传输
fn parse_head(head: &[u8]) -> Option<(u16, bool)> {
  let text = str::from_utf8(head).ok()?;
  let mut line_it = text.split("\r\n");
  let status_line = line_it.next()?;
  if !status_line.starts_with("HTTP/") {
    return None;
  }
  let status = status_line.split_whitespace().nth(1)?.parse().ok()?;
  let chunked = line_it.any(|line| {
    let mut it = line.splitn(2, ':');
    it.next()
      .is_some_and(|k| k.eq_ignore_ascii_case("transfer-encoding"))
      && it.next().is_some_and(|v| {
        v.as_bytes()
          .windows(7)
          .any(|w| w.eq_ignore_ascii_case(b"chunked"))
      })
  });
  Some((status, chunked))
}

/// 流片段
///
/// 字符串用 [`Cow`]：无转义时借用响应缓冲，有 `\n`、`\"` 等转义时才分配，
/// 避免逐段分配，同时不会因转义而丢内容。
#[derive(Deserialize)]
struct Chunk<'a> {
  #[serde(default)]
  id: Option<Cow<'a, str>>,
  #[serde(default)]
  model: Option<Cow<'a, str>>,
  #[serde(default, borrow)]
  choices: Option<SmallVec<[Choice<'a>; 1]>>,
  #[serde(default, borrow)]
  error: Option<ErrMsg<'a>>,
  #[serde(default)]
  usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice<'a> {
  #[serde(default)]
  delta: Delta<'a>,
  #[serde(default)]
  finish_reason: Option<Cow<'a, str>>,
}

#[derive(Deserialize, Default)]
struct Delta<'a> {
  #[serde(default)]
  content: Option<Cow<'a, str>>,
  #[serde(default)]
  reasoning_content: Option<Cow<'a, str>>,
}

#[derive(Deserialize)]
struct ErrMsg<'a> {
  #[serde(default)]
  message: Cow<'a, str>,
}

/// 分块传输的解码状态
enum Part {
  /// 等待块长度行
  Size,
  /// 块数据，`left` 为剩余字节
  Data,
  /// 块尾部 CRLF
  Crlf,
  /// 已收到终止块
  Done,
}

/// 响应体流：分块解码 + SSE 行切分
///
/// 行完整落在当前载荷段时直接返回 `buf` 切片，零拷贝；
/// 只有跨分块边界的行才拼接进 `line`，每字节最多拷贝一次。
/// `buf` 中 `pos` 之前视为已消费，超过阈值按摊还 O(1) 压缩。
struct Stream<S> {
  io: S,
  abort: Abort,
  buf: Vec<u8>,
  pos: usize,
  line: Vec<u8>,
  /// 上一行取自 `line`，下次进入时清空
  dirty: bool,
  scratch: Vec<u8>,
  eof: bool,
  part: Option<Part>,
  left: usize,
}

impl<S: AsyncRead + Unpin> Stream<S> {
  /// `pos` 为缓冲中已属于头部的字节数，正文从这里开始
  fn new(io: S, chunked: bool, buf: Vec<u8>, pos: usize, scratch: Vec<u8>, abort: Abort) -> Self {
    Self {
      io,
      abort,
      buf,
      pos,
      line: Vec::new(),
      dirty: false,
      scratch,
      eof: false,
      part: chunked.then_some(Part::Size),
      left: 0,
    }
  }

  async fn pull(&mut self, dur: Duration) -> Result<()> {
    let mut buf = take(&mut self.scratch);
    buf.clear();
    let BufResult(res, buf) = io_wait(&self.abort, dur, self.io.read(buf)).await?;
    let n = res?;
    if n == 0 {
      self.eof = true;
    } else {
      self.buf.extend_from_slice(&buf[..n]);
    }
    self.scratch = buf;
    Ok(())
  }

  /// 当前载荷段结束位置，`pos == end` 表示需推进状态机
  fn end(&self) -> usize {
    match self.part {
      None => self.buf.len(),
      Some(Part::Data) if self.left > 0 => (self.pos + self.left).min(self.buf.len()),
      _ => self.pos,
    }
  }

  /// 推进到有载荷可读或流结束
  async fn fill(&mut self, dur: Duration) -> Result<()> {
    loop {
      self.trim();
      if self.pos < self.end() {
        return Ok(());
      }
      match self.part {
        Some(Part::Done) => return Ok(()),
        None => {
          if self.eof {
            return Ok(());
          }
          self.pull(dur).await?;
        }
        Some(Part::Data) => {
          if self.left > 0 {
            if self.eof {
              return Err(Error::Cut);
            }
            self.pull(dur).await?;
          } else {
            self.part = Some(Part::Crlf);
          }
        }
        Some(Part::Crlf) => {
          if self.buf.len() - self.pos < 2 {
            if self.eof {
              return Err(Error::Cut);
            }
            self.pull(dur).await?;
            continue;
          }
          if self.buf[self.pos..self.pos + 2] != *b"\r\n" {
            return Err(Error::Chunk);
          }
          self.pos += 2;
          self.part = Some(Part::Size);
        }
        Some(Part::Size) => {
          let Some(i) = memmem::find(&self.buf[self.pos..], b"\r\n") else {
            if self.buf.len() - self.pos > MAX_SIZE_LINE {
              return Err(Error::Chunk);
            }
            if self.eof {
              // 兼容不发终止块直接关连接的服务端
              if self.pos == self.buf.len() {
                self.part = Some(Part::Done);
                return Ok(());
              }
              return Err(Error::Cut);
            }
            self.pull(dur).await?;
            continue;
          };
          let size = str::from_utf8(&self.buf[self.pos..self.pos + i])
            .ok()
            .and_then(|s| usize::from_str_radix(s.split(';').next()?.trim(), 16).ok())
            .filter(|&size| size <= MAX_CHUNK)
            .ok_or(Error::Chunk)?;
          self.pos += i + 2;
          if size == 0 {
            self.part = Some(Part::Done);
            return Ok(());
          }
          self.left = size;
          self.part = Some(Part::Data);
        }
      }
    }
  }

  /// 消费 `n` 字节载荷
  fn consume(&mut self, n: usize) {
    if matches!(self.part, Some(Part::Data)) {
      self.left = self.left.saturating_sub(n);
    }
    self.pos += n;
  }

  /// 读少量原始字节，用于错误响应体
  async fn raw(&mut self, dur: Duration, cap: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(cap.min(CAP));
    while out.len() < cap {
      self.fill(dur).await?;
      let end = self.end();
      if self.pos == end {
        break;
      }
      let n = (end - self.pos).min(cap - out.len());
      out.extend_from_slice(&self.buf[self.pos..self.pos + n]);
      let step = end - self.pos;
      self.consume(step);
    }
    Ok(out)
  }

  /// 取下一行（不含换行与尾部 `\r`）
  async fn line(&mut self, dur: Duration) -> Result<Option<&[u8]>> {
    if self.abort.is_aborted() {
      return Err(Error::Aborted);
    }
    if self.dirty {
      self.line.clear();
      self.dirty = false;
    }
    loop {
      self.fill(dur).await?;
      let end = self.end();
      if self.pos == end {
        if self.line.is_empty() {
          return Ok(None);
        }
        self.dirty = true;
        return Ok(Some(strip(&self.line)));
      }
      let Some(i) = memchr::memchr(b'\n', &self.buf[self.pos..end]) else {
        let n = end - self.pos;
        if n > MAX_LINE - self.line.len() {
          return Err(Error::Line);
        }
        self.line.extend_from_slice(&self.buf[self.pos..end]);
        self.consume(n);
        continue;
      };
      if i > MAX_LINE - self.line.len() {
        return Err(Error::Line);
      }
      let start = self.pos;
      self.consume(i + 1);
      if self.line.is_empty() {
        return Ok(Some(strip(&self.buf[start..start + i])));
      }
      self.line.extend_from_slice(&self.buf[start..start + i]);
      self.dirty = true;
      return Ok(Some(strip(&self.line)));
    }
  }

  /// 丢弃已消费前缀，摊还 O(1)
  fn trim(&mut self) {
    if self.pos == self.buf.len() {
      self.buf.clear();
      self.pos = 0;
    } else if self.pos > COMPACT {
      self.buf.drain(..self.pos);
      self.pos = 0;
    }
  }
}

/// 去掉行尾 `\r`
fn strip(raw: &[u8]) -> &[u8] {
  raw.strip_suffix(b"\r").unwrap_or(raw)
}
