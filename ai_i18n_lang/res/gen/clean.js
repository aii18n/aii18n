// 清理 src 中不在六系统并集内的语言
import { join } from 'node:path';
import {
  ERR_PARSE,
  ROOT,
  fail,
  parseArray,
  parseEnum,
  readText,
  renderArray,
  renderEnum,
  supportCount,
  tagsOf,
} from './shared.js';

const ARRAY_FILES = [
  ['en.rs', 'EN'],
  ['zh.rs', 'ZH'],
  ['name_li.rs', 'NAME'],
];

const cleanTests = (text, removed_li) =>
  removed_li.reduce(
    (acc, name) => acc.split('\n').filter((line) => !line.includes(`Code::${name}`)).join('\n'),
    text,
  );

// 支持率低于该比例的语言删除
const DEL_RATIO = 0.2;

export const clean = async (base_os, total) => {
  const code_file = join(ROOT, 'src', 'code.rs'),
    code_text = await readText(code_file),
    variant_li = parseEnum(code_text);

  for (const variant of variant_li)
    variant.keep =
      supportCount(tagsOf(variant.name, variant.attr_li), base_os) / total >= DEL_RATIO;
  const kept_idx = variant_li.flatMap((v, i) => (v.keep ? [i] : [])),
    kept_li = variant_li.filter((v) => v.keep),
    removed_li = variant_li.filter((v) => !v.keep).map((v) => v.name);

  await Bun.write(code_file, renderEnum(code_text, kept_li));

  for (const [file, name] of ARRAY_FILES) {
    const path = join(ROOT, 'src', 'name', file),
      text = await readText(path),
      entry_li = parseArray(text, name);
    if (entry_li.length !== variant_li.length)
      fail(ERR_PARSE, file, `数组 ${entry_li.length} ≠ 变体 ${variant_li.length}`);
    await Bun.write(path, renderArray(text, name, kept_idx.map((i) => entry_li[i])));
  }

  const test_file = join(ROOT, 'tests', 'code.rs'),
    test_text = await readText(test_file),
    new_test = cleanTests(test_text, removed_li);
  if (new_test !== test_text) await Bun.write(test_file, new_test);

  return { kept_count: kept_li.length, removed_li };
};
