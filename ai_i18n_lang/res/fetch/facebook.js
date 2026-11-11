// Facebook 官方 locale 列表（wix/facebook-locales 镜像）
const ERR_PARSE = 2;

// 非自然语言 / 彩蛋 locale
const JOKE = new Set(['en_PI', 'en_UD', 'fb_LT', 'tl_ST']);

// 非标准 locale 统一到 BCP-47
const MAP = {
  ar_AR: 'ar',
  es_LA: 'es-419',
  cb_IQ: 'ckb',
  ck_US: 'chr',
  cx_PH: 'ceb',
  gx_GR: 'grc',
  ja_KS: 'ja',
  sz_PL: 'szl',
  sy_SY: 'syr',
  tl_PH: 'fil',
  tz_MA: 'tzm',
  zz_TR: 'zza',
};

export default {
  file: 'facebook',
  title: 'Facebook 官方支持语言（facebook-locales）',
  url: 'https://raw.githubusercontent.com/wix/facebook-locales/master/src/Facebook.js',
  base64: false,
  parse: (text) => {
    const m = text.match(/export const supportedLocales = \[([\s\S]*?)\r?\n\]/);
    if (!m) throw [ERR_PARSE, 'facebook', 'supportedLocales'];
    return [...m[1].matchAll(/'([^']+)'/g)]
      .map((x) => x[1])
      .filter((x) => !JOKE.has(x))
      .map((x) => MAP[x] ?? x);
  },
};
