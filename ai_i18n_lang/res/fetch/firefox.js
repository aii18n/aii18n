// Firefox 官方本地化语言清单
const ERR_PARSE = 2;

const MAP = {
  'ja-JP-mac': 'ja-JP',
};

export default {
  file: 'firefox',
  title: 'Firefox 官方支持语言（l10n-changesets.json）',
  url: 'https://raw.githubusercontent.com/mozilla-firefox/firefox/main/browser/locales/l10n-changesets.json',
  base64: false,
  parse: (text) => {
    const data = JSON.parse(text);
    if (!data || Array.isArray(data)) throw [ERR_PARSE, 'firefox', 'l10n-changesets'];
    return Object.keys(data).map((code) => MAP[code] ?? code);
  },
};
