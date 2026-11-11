//! 生成无需翻译的 Unicode 码点区间
//!
//! 标点、符号（含表情）、组合符、格式符合并成连续区间；ASCII 单独压成
//! 两个 u64 位图，其余码点保留有序区间供二分。

use std::{env, fmt::Write, fs, path::PathBuf};

use unicode_general_category::{GeneralCategory as G, get_general_category};

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  let mut mask = [0u64; 2];
  for cp in 0..128u32 {
    if hit(cp) {
      mask[(cp >> 6) as usize] |= 1 << (cp & 63);
    }
  }

  let mut code = String::new();
  let _ = writeln!(
    code,
    "/// ASCII 无需翻译码点位图\npub(crate) const ASCII_NOISE: [u64; 2] = [0x{:016x}, 0x{:016x}];",
    mask[0], mask[1]
  );
  code.push_str(
    "/// 非 ASCII 无需翻译码点闭区间，由 build.rs 生成\npub(crate) const NOISE: &[(u32, u32)] = &[\n",
  );
  let mut begin = None;
  for cp in 128..=char::MAX as u32 {
    match (hit(cp), begin) {
      (true, None) => begin = Some(cp),
      (false, Some(start)) => {
        let _ = writeln!(code, "  ({start}, {}),", cp - 1);
        begin = None;
      }
      _ => {}
    }
  }
  if let Some(start) = begin {
    let _ = writeln!(code, "  ({start}, {}),", char::MAX as u32);
  }
  code.push_str("];\n");

  let out = PathBuf::from(env::var("OUT_DIR").expect("缺少 OUT_DIR")).join("noise.rs");
  // 内容未变就不落盘，避免无意义的增量重编译
  if fs::read_to_string(&out).ok().as_deref() != Some(code.as_str()) {
    fs::write(out, code).expect("写入 noise.rs 失败");
  }
}

/// 码点是否属于标点、符号、组合符、格式符、空白或控制符
fn hit(cp: u32) -> bool {
  char::from_u32(cp).is_some_and(|c| no_tran(get_general_category(c)))
}

/// 单个字符是否无需翻译
fn no_tran(c: G) -> bool {
  matches!(
    c,
    G::ConnectorPunctuation
      | G::DashPunctuation
      | G::OpenPunctuation
      | G::ClosePunctuation
      | G::InitialPunctuation
      | G::FinalPunctuation
      | G::OtherPunctuation
      | G::MathSymbol
      | G::CurrencySymbol
      | G::ModifierSymbol
      | G::OtherSymbol
      | G::NonspacingMark
      | G::SpacingMark
      | G::EnclosingMark
      | G::Format
      | G::SpaceSeparator
      | G::Control
  )
}
