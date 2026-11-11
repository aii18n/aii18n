// WhatsApp 官方模板支持语言列表
const ERR_PARSE = 2;

export default {
  file: 'whatsapp',
  title: 'WhatsApp 官方支持语言（模板语言代码）',
  url: 'https://developers.facebook.com/documentation/business-messaging/whatsapp/templates/supported-languages.md',
  base64: false,
  parse: (text) => {
    const code_li = [...text.matchAll(/\|\s*[^|\n]+\|\s*`([^`]+)`\s*\|/g)].map((m) => m[1]);
    if (!code_li.length) throw [ERR_PARSE, 'whatsapp', 'supported-languages'];
    return code_li;
  },
};
