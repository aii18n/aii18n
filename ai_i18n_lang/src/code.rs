use core::fmt;

/// 六大系统官方支持语言的并集，u16 作为判别式
#[repr(u16)]
#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  Hash,
  strum::IntoStaticStr,
  strum::EnumString,
  strum::VariantArray,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Code {
  En,
  Zh,
  Fr,
  De,
  Es,
  Ja,
  Ar,
  Pt,
  Ru,
  It,
  Nl,
  Ko,
  Tr,
  Id,
  #[strum(serialize = "zh-TW")]
  ZhTW,
  Hi,
  Pl,
  Ta,
  Ga,
  Sv,
  Bn,
  He,
  Th,
  No,
  Vi,
  Mr,
  Ms,
  Da,
  Ro,
  Ur,
  Cs,
  Fa,
  Te,
  Pa,
  Kn,
  Gu,
  El,
  Fi,
  Ca,
  Gd,
  Kk,
  Tl,
  Sw,
  Ug,
  Hu,
  Uk,
  Uz,
  Ml,
  Sk,
  Af,
  Hr,
  Sr,
  Bg,
  Br,
  Am,
  Cy,
  Or,
  Si,
  Ha,
  Eu,
  Szl,
  Lt,
  Be,
  Zu,
  Gl,
  My,
  Sl,
  Az,
  As,
  Km,
  Ne,
  Ku,
  Xh,
  Lv,
  Et,
  Tt,
  Is,
  Ka,
  Bs,
  Sq,
  Hy,
  Mn,
  Ky,
  Mk,
  Lo,
  Tg,
  Rw,
  Eo,
  Fil,
  Nn,
  Ckb,
  Wo,
  Fy,
}

impl Code {
  /// Google Translate 语言代码，如 `zh-TW`
  pub fn as_str(self) -> &'static str {
    self.into()
  }
}

impl AsRef<str> for Code {
  fn as_ref(&self) -> &str {
    (*self).as_str()
  }
}

impl fmt::Display for Code {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str((*self).as_str())
  }
}
