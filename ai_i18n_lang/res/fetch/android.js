// Android AOSP 系统语言：languages_default.mk 中的 PRODUCT_LOCALES
export default {
  file: 'android',
  title: 'Android AOSP 官方系统语言（PRODUCT_LOCALES）',
  url: 'https://android.googlesource.com/platform/build/+/HEAD/target/product/languages_default.mk?format=TEXT',
  base64: true,
  parse: (text) =>
    [...text.matchAll(/^\s+([A-Za-z]{2,3}(?:_[A-Za-z]{2,4})+)\s*\\$/gm)].map((m) => m[1]),
};
