// LibreOffice 官方本地化语言列表
const ERR_PARSE = 2;

const parseLangList = (text) => {
  const code_li = [];
  let in_list = false;
  for (const line of text.split('\n')) {
    if (!in_list) {
      if (!line.includes('completelangiso=')) continue;
      in_list = true;
    }
    const clean = line.replace(/#.*/, '').trim(),
      cont = clean.endsWith('\\'),
      body = clean.replace(/\\$/, '').replace(/^completelangiso\s*=\s*/, '');
    code_li.push(...body.split(/\s+/).filter(Boolean));
    if (!cont) break;
  }
  if (!code_li.length) throw [ERR_PARSE, 'libreoffice', 'completelangiso'];
  return code_li.filter((code) => code !== 'qtz');
};

export default {
  file: 'libreoffice',
  title: 'LibreOffice 官方支持语言（completelangiso）',
  url: 'https://raw.githubusercontent.com/LibreOffice/core/master/solenv/inc/langlist.mk',
  base64: false,
  parse: parseLangList,
};
