// TikTok 官方界面语言（首页 dntLanguageList）
const ERR_PARSE = 2;

export default {
  file: 'tiktok',
  title: 'TikTok 官方支持语言（dntLanguageList）',
  url: 'https://www.tiktok.com/',
  base64: false,
  parse: (text) => {
    const m = text.match(/"dntLanguageList":(\[[^\]]*\])/);
    if (!m) throw [ERR_PARSE, 'tiktok', 'dntLanguageList'];
    return JSON.parse(m[1]).map((item) => item.code);
  },
};
