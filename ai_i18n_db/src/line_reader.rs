use std::{io, mem};

use compio::{buf::BufResult, fs::File, io::AsyncReadAt};
use memchr::memchr;

/// 单次读取块大小，决定扫描时的常量内存占用。
const BUF_CAP: usize = 64 * 1024;

/// 流式按行读取器：内存 O(`BUF_CAP`)，超长行才写入 `spill`。
///
/// 返回的行不含 `\n`，文件末尾没有 `\n` 的最后一行也会返回。
pub(crate) struct LineReader {
  file: File,
  buf: Vec<u8>,
  spill: Vec<u8>,
  start: usize,
  base: u64,
  eof: bool,
  line_off: u64,
}

impl LineReader {
  pub(crate) fn new(file: File, pos: u64) -> Self {
    Self {
      file,
      buf: Vec::with_capacity(BUF_CAP),
      spill: Vec::new(),
      start: 0,
      base: pos,
      eof: false,
      line_off: pos,
    }
  }

  /// 当前行的文件起始偏移，在 `next` 返回后有效。
  pub(crate) fn line_off(&self) -> u64 {
    self.line_off
  }

  /// 填充缓冲区，返回是否读到新数据。
  ///
  /// 调用前缓冲区必然已全部消费（`start == buf.len()`）。
  async fn fill(&mut self) -> io::Result<bool> {
    if self.eof {
      return Ok(false);
    }
    debug_assert_eq!(self.start, self.buf.len());
    self.base += self.buf.len() as u64;
    self.buf.clear();
    self.start = 0;
    let buf = mem::take(&mut self.buf);
    let BufResult(res, buf) = self.file.read_at(buf, self.base).await;
    self.buf = buf;
    match res {
      Ok(0) => {
        self.eof = true;
        Ok(false)
      }
      Ok(_) => Ok(true),
      Err(e) => Err(e),
    }
  }

  /// 读取下一行；EOF 返回 `None`。
  pub(crate) async fn next(&mut self) -> io::Result<Option<&[u8]>> {
    self.spill.clear();
    self.line_off = self.base + self.start as u64;
    let mut spanning = false;
    loop {
      if self.start == self.buf.len() && !self.fill().await? {
        return Ok(if spanning { Some(&self.spill) } else { None });
      }
      let start = self.start;
      match memchr(b'\n', &self.buf[start..]) {
        Some(i) => {
          let end = start + i;
          self.start = end + 1;
          if spanning {
            self.spill.extend_from_slice(&self.buf[start..end]);
            return Ok(Some(&self.spill));
          }
          return Ok(Some(&self.buf[start..end]));
        }
        None => {
          spanning = true;
          self.spill.extend_from_slice(&self.buf[start..]);
          self.start = self.buf.len();
        }
      }
    }
  }
}
