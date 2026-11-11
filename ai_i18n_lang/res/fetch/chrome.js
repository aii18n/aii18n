// Chrome 界面语言：locales.gni 中非 Android / ChromeOS / iOS 的 platform_pak_locales
const ERR_PARSE = 2;

const fail = (...data) => {
  throw [ERR_PARSE, ...data];
};

const gniArr = (text, name) => {
  const m = text.match(new RegExp(`\\b${name}\\s*=\\s*\\[([\\s\\S]*?)\\]`));
  if (!m) fail('locales.gni', name);
  return [...m[1].replaceAll(/#.*$/gm, '').matchAll(/"([^"]+)"/g)].map((x) => x[1]);
};

export default {
  file: 'chrome',
  title: 'Chrome 官方界面语言（platform_pak_locales）',
  url: 'https://chromium.googlesource.com/chromium/src/build/config/+/refs/heads/main/locales.gni?format=TEXT',
  base64: true,
  parse: (text) => {
    const pseudo = new Set(gniArr(text, 'pseudolocales')),
      ext = new Set(gniArr(text, 'extended_locales'));
    return gniArr(text, 'all_chrome_locales').filter((x) => !pseudo.has(x) && !ext.has(x));
  },
};
