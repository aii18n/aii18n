// iOS 技术规格中的 Language support 段落
const ERR_PARSE = 2;

const fail = (...data) => {
  throw [ERR_PARSE, ...data];
};

const stripTags = (html) =>
  html
    .replace(/<[^>]*>/g, ' ')
    .replaceAll('&nbsp;', ' ')
    .replaceAll('&amp;', '&')
    .replaceAll('&ndash;', '-')
    .replaceAll('&mdash;', '-')
    .replace(/\s+/g, ' ')
    .trim();

const LANG = {
  English: 'en',
  Chinese: 'zh',
  French: 'fr',
  German: 'de',
  Italian: 'it',
  Japanese: 'ja',
  Korean: 'ko',
  Spanish: 'es',
  Arabic: 'ar',
  Bulgarian: 'bg',
  Catalan: 'ca',
  Croatian: 'hr',
  Czech: 'cs',
  Danish: 'da',
  Dutch: 'nl',
  Finnish: 'fi',
  Greek: 'el',
  Hebrew: 'he',
  Hindi: 'hi',
  Hungarian: 'hu',
  Indonesian: 'id',
  Kazakh: 'kk',
  Malay: 'ms',
  Norwegian: 'nb',
  Polish: 'pl',
  Portuguese: 'pt',
  Romanian: 'ro',
  Russian: 'ru',
  Slovak: 'sk',
  Swedish: 'sv',
  Thai: 'th',
  Turkish: 'tr',
  Ukrainian: 'uk',
  Vietnamese: 'vi',
};

const VARIANT = {
  'English|Australia': 'en-AU',
  'English|UK': 'en-GB',
  'English|US': 'en-US',
  'English|U.S.': 'en-US',
  'Chinese|Simplified': 'zh-Hans',
  'Chinese|Traditional': 'zh-Hant',
  'Chinese|Traditional - Hong Kong': 'zh-Hant-HK',
  'French|Canada': 'fr-CA',
  'French|France': 'fr-FR',
  'Spanish|Latin America': 'es-419',
  'Spanish|Spain': 'es-ES',
  'Portuguese|Brazil': 'pt-BR',
  'Portuguese|Portugal': 'pt-PT',
};

// 按括号外的逗号切分
const splitTop = (text) => {
  const out = [];
  let buf = '',
    depth = 0;
  for (const ch of text) {
    if (ch === '(') depth++;
    else if (ch === ')') depth--;
    if (ch === ',' && depth === 0) {
      out.push(buf.trim());
      buf = '';
    } else buf += ch;
  }
  if (buf.trim()) out.push(buf.trim());
  return out;
};

const iosToken = (token) => {
  const m = token.match(/^([^(]+?)\s*\(([^)]+)\)$/);
  if (!m) {
    const code = LANG[token];
    if (!code) fail('ios', token);
    return [code];
  }
  const [, langRaw, variantRaw] = m,
    lang = langRaw.trim();
  return splitTop(variantRaw.replace(/[\u2013\u2014]/g, '-')).map((variant) => {
    const code = VARIANT[`${lang}|${variant}`];
    if (!code) fail('ios', `${lang} (${variant})`);
    return code;
  });
};

export default {
  file: 'ios',
  title: 'iOS 官方系统语言支持',
  url: 'https://support.apple.com/en-us/125089',
  base64: false,
  parse: (text) => {
    const m = text.match(/<b>\s*Language support\s*<\/b><\/p><p[^>]*>([\s\S]*?)<\/p>/);
    if (!m) fail('ios', 'Language support');
    return splitTop(stripTags(m[1])).flatMap(iosToken);
  },
};
