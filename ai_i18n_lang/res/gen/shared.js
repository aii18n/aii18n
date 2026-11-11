// Rust 源码解析与渲染的公共工具
import { join } from 'node:path';

export const ROOT = join(import.meta.dirname, '..', '..'),
  ERR_PARSE = 2;

// 与并集语言不同码但语义相同的别名，避免误删
export const ALIAS = {
  no: 'nb', // 挪威语宏语言
  qu: 'quz', // 克丘亚语
  gom: 'kok', // 果阿孔卡尼语
};

export const fail = (code, ...data) => {
  throw [code, ...data];
};

export const baseOf = (code) => code.split('-')[0].toLowerCase();

export const readText = (file) => Bun.file(file).text();

// 解析 Code 枚举，保留每个变体的属性行
export const parseEnum = (text) => {
  const m = text.match(/pub enum Code \{([\s\S]*?)\n\}/);
  if (!m) fail(ERR_PARSE, 'code.rs', 'enum');
  const variant_li = [];
  let attr_li = [];
  for (const line of m[1].split('\n')) {
    if (line.trim().startsWith('#[')) {
      attr_li.push(line);
    } else if (line.trim()) {
      const vm = line.match(/^\s+([A-Z][A-Za-z0-9]*),$/);
      if (!vm) fail(ERR_PARSE, 'code.rs', line);
      variant_li.push({ name: vm[1], attr_li, line });
      attr_li = [];
    }
  }
  return variant_li;
};

export const tagsOf = (name, attr_li) => {
  const tags = [...attr_li.join(' ').matchAll(/serialize\s*=\s*"([^"]+)"/g)].map((m) => m[1]);
  return tags.length ? tags : [name.toLowerCase()];
};

export const coveredIn = (tags, base_set) =>
  tags.some((tag) => {
    const base = baseOf(tag);
    return base_set.has(base) || base_set.has(ALIAS[base]);
  });

// 取该变体所有标签（含别名）的系统来源并集
export const supportCount = (tags, base_os) => {
  const source_set = new Set();
  for (const tag of tags) {
    const base = baseOf(tag);
    for (const code of [base, ALIAS[base]]) {
      if (!code) continue;
      for (const os of base_os.get(code) ?? []) source_set.add(os);
    }
  }
  return source_set.size;
};

export const renderEnum = (text, variant_li) =>
  text.replace(
    /pub enum Code \{[\s\S]*?\n\}/,
    `pub enum Code {\n${variant_li.map(({ attr_li, line }) => [...attr_li, line].join('\n')).join('\n')}\n}`,
  );

const arrayRe = (name) => new RegExp(`pub const ${name}: \\[&str; \\d+\\] = \\[\\n([\\s\\S]*?)\\n\\];`);

export const parseArray = (text, name) => {
  const m = text.match(arrayRe(name));
  if (!m) fail(ERR_PARSE, name, 'array');
  return m[1].split('\n').map((line) => {
    const em = line.match(/^\s*"([^"]*)",$/);
    if (!em) fail(ERR_PARSE, name, line);
    return em[1];
  });
};

export const renderArray = (text, name, entry_li) =>
  text.replace(
    arrayRe(name),
    `pub const ${name}: [&str; ${entry_li.length}] = [\n${entry_li.map((e) => `  "${e}",`).join('\n')}\n];`,
  );
