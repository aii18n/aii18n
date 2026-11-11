// 向 src 追加被 3 个及以上系统支持、但尚不存在的语言
import { join } from 'node:path';
import { shortName } from '../short_name.js';
import {
  ALIAS,
  ERR_PARSE,
  ROOT,
  baseOf,
  fail,
  parseArray,
  parseEnum,
  readText,
  renderArray,
  renderEnum,
  tagsOf,
} from './shared.js';

const ARRAY_FILES = [
  ['en.rs', 'EN'],
  ['zh.rs', 'ZH'],
  ['name_li.rs', 'NAME'],
];

// 支持率高于该比例的语言加入
const ADD_RATIO = 0.3;

const zh_names = new Intl.DisplayNames(['zh-CN'], { type: 'language' }),
  en_names = new Intl.DisplayNames(['en'], { type: 'language' });

const nameOf = (dn, code) => {
  const name = dn.of(code);
  return name && name !== code ? name : code;
};

// NAME 缺少自称时以英文名兜底
const NATIVE = {
  ckb: 'کوردی',
  fy: 'Frysk',
  nn: 'norsk nynorsk',
};

const extraOf = (name, added_li) => {
  if (name === 'ZH') return added_li.map((a) => a.entry.zh);
  if (name === 'NAME') return added_li.map((a) => a.entry.native);
  return added_li.map((a) => a.entry.en);
};

export const add = async (union_base, base_os, total) => {
  const code_file = join(ROOT, 'src', 'code.rs'),
    code_text = await readText(code_file),
    variant_li = parseEnum(code_text),
    covered = new Set();

  for (const variant of variant_li) {
    for (const tag of tagsOf(variant.name, variant.attr_li)) {
      const base = baseOf(tag);
      covered.add(base);
      if (ALIAS[base]) covered.add(ALIAS[base]);
    }
  }

  const missing_li = [...union_base].filter((b) => !covered.has(b)).sort(),
    add_base_li = missing_li.filter(
      (base) => (base_os.get(base)?.size ?? 0) / total > ADD_RATIO,
    ),
    existing = new Set(variant_li.map((v) => v.name)),
    added_li = add_base_li.map((base) => {
      const name = base[0].toUpperCase() + base.slice(1);
      if (existing.has(name)) fail(ERR_PARSE, 'code.rs', name);
      existing.add(name);
      return {
        code: base,
        variant: { name, attr_li: [], line: `  ${name},` },
        entry: {
          zh: shortName(nameOf(zh_names, base)),
          en: shortName(nameOf(en_names, base)),
          native: NATIVE[base] ?? nameOf(en_names, base),
        },
      };
    });

  if (added_li.length) {
    await Bun.write(code_file, renderEnum(code_text, [...variant_li, ...added_li.map((a) => a.variant)]));
    for (const [file, name] of ARRAY_FILES) {
      const path = join(ROOT, 'src', 'name', file),
        text = await readText(path),
        entry_li = parseArray(text, name);
      if (entry_li.length !== variant_li.length)
        fail(ERR_PARSE, file, `数组 ${entry_li.length} ≠ 变体 ${variant_li.length}`);
      await Bun.write(path, renderArray(text, name, [...entry_li, ...extraOf(name, added_li)]));
    }
  }

  return {
    added_li: added_li.map(({ code, variant }) => ({ code, name: variant.name })),
    missing_li: missing_li.filter((b) => !add_base_li.includes(b)),
  };
};
