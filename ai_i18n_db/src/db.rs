use std::{
  cmp::Ordering,
  fs::{File as StdFile, OpenOptions as StdOpenOptions, TryLockError},
  io, mem,
  path::{Path, PathBuf},
};

use async_lock::RwLock;
use compio::{
  buf::BufResult,
  fs::{File, OpenOptions, rename},
  io::AsyncWriteAtExt,
};
use gxhash::{HashMap, HashSet};

use crate::{Error, HEADER_PREFIX, LINE_PREFIX, Result, line_reader::LineReader, norm_path};

/// 写缓冲与读缓冲块大小。
const BUF_CAP: usize = 64 * 1024;

/// 只追加文本库。
///
/// `open` 会对 `<db>.lock` 加独占文件锁，禁止跨进程并发访问；
/// 进程内读写由异步读写锁协调。
pub struct Db {
  path: PathBuf,
  lock: RwLock<Writer>,
  _file_lock: StdFile,
}

#[derive(Default)]
struct Writer {
  file: Option<File>,
  end: u64,
}

impl Db {
  /// 打开数据库并获取独占文件锁；被其他实例或进程占用时返回 [`Error::Locked`]。
  ///
  /// 文件不存在时首次写入自动创建。
  pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
    let path = path.into();
    let mut name = path.as_os_str().to_os_string();
    name.push(".lock");
    let lock_path = PathBuf::from(name);
    let file_lock = StdOpenOptions::new()
      .create(true)
      .truncate(false)
      .read(true)
      .write(true)
      .open(&lock_path)?;
    match file_lock.try_lock() {
      Ok(()) => {}
      Err(TryLockError::WouldBlock) => return Err(Error::Locked(lock_path)),
      Err(TryLockError::Error(e)) => return Err(e.into()),
    }
    Ok(Self {
      path,
      lock: RwLock::new(Writer::default()),
      _file_lock: file_lock,
    })
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  /// 追加一条记录：同一 `(path, lang)` 后写覆盖先写。
  pub async fn set<S: AsRef<str>>(&self, path: &str, lang: &str, line_li: &[S]) -> Result<()> {
    let header = header(path, lang)?;
    let mut record = Vec::with_capacity(header.len() + 1);
    record.extend_from_slice(header.as_bytes());
    record.push(b'\n');
    for line in line_li {
      let line = line.as_ref().as_bytes();
      if line.iter().any(|&b| b == b'\n' || b == b'\r') {
        return Err(Error::BadLine);
      }
      record.push(LINE_PREFIX);
      record.extend_from_slice(line);
      record.push(b'\n');
    }
    let len = record.len() as u64;

    let mut w = self.lock.write().await;
    let pos = w.end;
    let file = self.ensure_writer(&mut w).await?;
    let BufResult(res, _) = file.write_all_at(record, pos).await;
    res?;
    w.end = pos + len;
    Ok(())
  }

  /// 读取 `(path, lang)` 的最新内容；不存在返回空 `Vec`。
  pub async fn get(&self, path: &str, lang: &str) -> Result<Vec<String>> {
    let header = header(path, lang)?;
    let needle = header.as_bytes();
    // 读锁：避免读到追加中的半条记录，也避让压实期间的 rename
    let _guard = self.lock.read().await;
    let file = match File::open(&self.path).await {
      Ok(f) => f,
      Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
      Err(e) => return Err(e.into()),
    };
    let mut reader = LineReader::new(file, 0);
    let mut out = Vec::new();
    let mut active = false;
    while let Some(line) = reader.next().await? {
      match line.first() {
        Some(&HEADER_PREFIX) => {
          active = trim_cr(line) == needle;
          if active {
            out.clear();
          }
        }
        Some(&LINE_PREFIX) if active => out.push(str::from_utf8(trim_cr(&line[1..]))?.to_owned()),
        _ => {}
      }
    }
    Ok(out)
  }

  /// 压实：只保留 `path_li` 中的路径，其余路径全部删除。
  ///
  /// - `path_li` 是白名单，白名单外的记录会被丢弃
  /// - `path_li` 为空：清空数据库
  /// - 压实部分按 `(path, lang)` 排序，只保留每个键的最后一条
  /// - 流式扫描 + 临时文件 + `mv` 覆盖
  pub async fn compact<S: AsRef<str>>(&self, path_li: &[S]) -> Result<()> {
    let keep = whitelist(path_li);
    let mut w = self.lock.write().await;
    // 先关旧句柄，Windows 下才能 rename 覆盖
    w.file = None;
    w.end = 0;
    let read = match File::open(&self.path).await {
      Ok(f) => f,
      Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
      Err(e) => return Err(e.into()),
    };

    // 第一遍：记录白名单内每个键最后一条的头部偏移，内存 O(键数)
    let mut latest: HashMap<String, u64> = HashMap::default();
    let mut reader = LineReader::new(read.clone(), 0);
    while let Some(line) = reader.next().await? {
      let Some((head, path)) = head_of(line) else {
        continue;
      };
      if keep.contains(path) {
        latest.insert(head.to_owned(), reader.line_off());
      }
    }
    let mut key_li: Vec<(String, u64)> = latest.into_iter().collect();
    key_li.sort_unstable_by(|a, b| cmp_head(&a.0, &b.0));

    let tmp = self.tmp_path();
    let tmp_file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open(&tmp)
      .await?;
    let mut out = BufFile::new(tmp_file);

    // 第二遍：按序重写白名单记录，内容流式搬运
    for (head, off) in key_li {
      out.push(head.as_bytes()).await?;
      out.push(b"\n").await?;
      let mut reader = LineReader::new(read.clone(), off);
      let mut head_done = false;
      while let Some(line) = reader.next().await? {
        match line.first() {
          Some(&HEADER_PREFIX) if !head_done => head_done = true,
          Some(&HEADER_PREFIX) => break,
          Some(&LINE_PREFIX) => out.push_line(line).await?,
          _ => {}
        }
      }
    }

    out.flush().await?;
    let file = out.into_file();
    file.sync_all().await?;
    drop(file);
    drop(read);
    rename(&tmp, &self.path).await?;
    Ok(())
  }

  async fn ensure_writer<'a>(&self, w: &'a mut Writer) -> Result<&'a mut File> {
    if w.file.is_none() {
      let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&self.path)
        .await?;
      w.end = file.metadata().await?.len();
      w.file = Some(file);
    }
    w.file
      .as_mut()
      .ok_or_else(|| io::Error::other("writer 未初始化").into())
  }

  fn tmp_path(&self) -> PathBuf {
    let mut name = self.path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
  }
}

/// 带位置的写缓冲，减少小写入系统调用。
struct BufFile {
  file: File,
  pos: u64,
  buf: Vec<u8>,
}

impl BufFile {
  fn new(file: File) -> Self {
    Self {
      file,
      pos: 0,
      buf: Vec::with_capacity(BUF_CAP),
    }
  }

  async fn push(&mut self, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() >= BUF_CAP {
      self.flush().await?;
      let len = bytes.len() as u64;
      let BufResult(res, _) = self.file.write_all_at(bytes.to_vec(), self.pos).await;
      res?;
      self.pos += len;
      return Ok(());
    }
    if self.buf.len() + bytes.len() > self.buf.capacity() {
      self.flush().await?;
    }
    self.buf.extend_from_slice(bytes);
    Ok(())
  }

  async fn push_line(&mut self, line: &[u8]) -> io::Result<()> {
    self.push(trim_cr(line)).await?;
    self.push(b"\n").await
  }

  async fn flush(&mut self) -> io::Result<()> {
    if self.buf.is_empty() {
      return Ok(());
    }
    let len = self.buf.len() as u64;
    let buf = mem::take(&mut self.buf);
    let BufResult(res, mut buf) = self.file.write_all_at(buf, self.pos).await;
    buf.clear();
    self.buf = buf;
    self.pos += len;
    res
  }

  fn into_file(self) -> File {
    self.file
  }
}

/// 生成记录头（不含换行）。
fn header(path: &str, lang: &str) -> Result<String> {
  let path = norm_path(path);
  if path.is_empty() {
    return Err(Error::EmptyPath);
  }
  if path.as_bytes().iter().any(|&b| b == b'\n' || b == b'\r') {
    return Err(Error::BadPath(path.into_owned()));
  }
  if lang.is_empty() {
    return Err(Error::EmptyLang);
  }
  if lang
    .as_bytes()
    .iter()
    .any(|&b| matches!(b, b'/' | b'\n' | b'\r'))
  {
    return Err(Error::BadLang(lang.to_owned()));
  }
  let mut head = String::with_capacity(2 + path.len() + lang.len());
  head.push(HEADER_PREFIX as char);
  head.push_str(&path);
  head.push('/');
  head.push_str(lang);
  Ok(head)
}

/// 解析记录头，返回 `(完整头部, path)`；非法头返回 `None`。
fn head_of(line: &[u8]) -> Option<(&str, &str)> {
  let line = trim_cr(line);
  let head = str::from_utf8(line).ok()?;
  head.strip_prefix(HEADER_PREFIX as char)?;
  let slash = head.rfind('/')?;
  // path 与 lang 都必须非空
  (slash > 1 && slash + 1 < head.len()).then(|| (head, &head[1..slash]))
}

/// 按 `(path, lang)` 比较两个记录头。
fn cmp_head(a: &str, b: &str) -> Ordering {
  let (a_path, a_lang) = split_head(a);
  let (b_path, b_lang) = split_head(b);
  a_path.cmp(b_path).then_with(|| a_lang.cmp(b_lang))
}

fn split_head(head: &str) -> (&str, &str) {
  match head.rfind('/') {
    Some(i) => (&head[1..i], &head[i + 1..]),
    None => (head, ""),
  }
}

fn whitelist<S: AsRef<str>>(path_li: &[S]) -> HashSet<String> {
  path_li
    .iter()
    .map(|p| norm_path(p.as_ref()).into_owned())
    .collect()
}

/// 去掉行尾 `\r`（容忍 CRLF 手工编辑）。
fn trim_cr(line: &[u8]) -> &[u8] {
  match line.last() {
    Some(b'\r') => &line[..line.len() - 1],
    _ => line,
  }
}
