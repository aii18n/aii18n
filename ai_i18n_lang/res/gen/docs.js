// 读取 src 中的枚举与名称表，生成 readme 的中英文语言表
import { join } from 'node:path';
import { shortName } from '../short_name.js';
import { ERR_PARSE, ROOT, fail, parseArray, parseEnum, readText, tagsOf } from './shared.js';

const ARRAY_FILES = [
  ['en.rs', 'EN'],
  ['zh.rs', 'ZH'],
  ['name_li.rs', 'NAME'],
];

const TABLE_HEAD = ['| # | Code | Endonym | English | 中文 |', '|---:|---|---|---|---|'];

const cell = (value) => String(value).replaceAll('|', '\\|');

const tableText = (title, entry_li) =>
  [
    title,
    '',
    ...TABLE_HEAD,
    ...entry_li.map(
      ({ code, name, en, zh }, i) =>
        `| ${i} | \`${code}\` | ${cell(name)} | ${cell(en)} | ${cell(zh)} |`,
    ),
    '',
  ].join('\n');

const DOCS = [
  ['zh.md', /全部 \d+ 种语言/, '## 语言表', (n) => `全部 ${n} 种语言`],
  ['en.md', /all \d+ languages/, '## Language table', (n) => `all ${n} languages`],
];

export const genDocs = async () => {
  const code_text = await readText(join(ROOT, 'src', 'code.rs')),
    variant_li = parseEnum(code_text),
    array_li = await Promise.all(
      ARRAY_FILES.map(async ([file, name]) => [
        name,
        parseArray(await readText(join(ROOT, 'src', 'name', file)), name),
      ]),
    ),
    arrays = Object.fromEntries(array_li);

  for (const [name, entry_li] of array_li) {
    if (entry_li.length !== variant_li.length)
      fail(ERR_PARSE, name, `数组 ${entry_li.length} ≠ 变体 ${variant_li.length}`);
  }

  const entry_li = variant_li.map((variant, i) => ({
    code: tagsOf(variant.name, variant.attr_li)[0],
    name: shortName(arrays.NAME[i]),
    en: shortName(arrays.EN[i]),
    zh: shortName(arrays.ZH[i]),
  }));

  for (const [file, count_re, table_title, count_text] of DOCS) {
    const doc_file = join(ROOT, 'readme', file),
      text = await readText(doc_file),
      table_re = new RegExp(`${table_title}[\\s\\S]*?(?=\\n## |$)`);
    if (!count_re.test(text)) fail(ERR_PARSE, `readme/${file}`, 'count');
    if (!table_re.test(text)) fail(ERR_PARSE, `readme/${file}`, table_title);
    const new_text = text
      .replace(count_re, count_text(entry_li.length))
      .replace(table_re, tableText(table_title, entry_li));
    await Bun.write(doc_file, new_text);
  }
  return entry_li.length;
};
