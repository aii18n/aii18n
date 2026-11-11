#!/usr/bin/env -S bun
// 生成六系统语言并集，并依次执行清理与添加
// 用法: bun res/gen.js
import { join } from 'node:path';
import { add } from './gen/add.js';
import { clean } from './gen/clean.js';
import { genDocs } from './gen/docs.js';
import { baseOf, fail } from './gen/shared.js';

// 参与合并的系统文件（不含 union.yml 自身）
const OS_FILES = [
  'ubuntu',
  'android',
  'chrome',
  'windows',
  'ios',
  'harmonyos',
  'facebook',
  'firefox',
  'tiktok',
  'libreoffice',
  'whatsapp',
];

const ERR_READ = 1;

const q = (value) => JSON.stringify(value);

const loadYaml = async (file) => Bun.YAML.parse(await Bun.file(file).text());

// 各系统语言并集，按 BCP-47 完整标签去重
const mergeUnion = async () => {
  const entry_li = await Promise.all(
    OS_FILES.map(async (os) => {
      const file = join(import.meta.dirname, 'support', `${os}.yml`),
        data = await loadYaml(file);
      if (!Array.isArray(data)) fail(ERR_READ, file);
      return data.map((entry) => ({ ...entry, os_li: [os] }));
    }),
  );
  const map = new Map();
  for (const entry of entry_li.flat()) {
    const old = map.get(entry.code);
    if (old) old.os_li.push(...entry.os_li);
    else map.set(entry.code, entry);
  }
  return [...map.values()].sort((a, b) => a.code.localeCompare(b.code, 'en'));
};

const unionYaml = (entry_li, base_li) =>
  [
    '# 六大系统官方支持语言并集',
    `# 来源: res/support/{${OS_FILES.join(',')}}.yml`,
    `# 共 ${entry_li.length} 个 BCP-47 标签，${base_li.length} 种基础语言`,
    '',
    ...entry_li.flatMap(({ code, zh, en }) => [
      `- code: ${q(code)}`,
      `  zh: ${q(zh)}`,
      `  en: ${q(en)}`,
    ]),
    '',
  ].join('\n');

const main = async () => {
  const union_li = await mergeUnion(),
    union_base = new Set(union_li.map((e) => baseOf(e.code))),
    base_os = new Map();

  for (const { code, os_li } of union_li) {
    const base = baseOf(code);
    if (!base_os.has(base)) base_os.set(base, new Set());
    for (const os of os_li) base_os.get(base).add(os);
  }

  await Bun.write(
    join(import.meta.dirname, 'support', 'union.yml'),
    unionYaml(union_li, [...union_base].sort()),
  );

  const { kept_count, removed_li } = await clean(base_os, OS_FILES.length),
    { added_li, missing_li } = await add(union_base, base_os, OS_FILES.length),
    name_of_base = new Map(union_li.map((e) => [baseOf(e.code), e]));
  const doc_count = await genDocs();

  console.log(`并集: ${union_li.length} 个标签 / ${union_base.size} 种语言`);
  console.log(
    `变体: ${kept_count + added_li.length}（清理 ${removed_li.length}，新增 ${added_li.length}）`,
  );
  console.log(`文档: readme/{zh,en}.md（${doc_count} 种语言）`);
  if (added_li.length) console.log(`新增: ${added_li.map((a) => `${a.name}(${a.code})`).join(' ')}`);
  console.log(`移除: ${removed_li.join(' ')}`);
  const missing_map = new Map();
  for (const base of missing_li) {
    const { zh } = name_of_base.get(base),
      os_set = missing_map.get(zh) ?? new Set();
    for (const os of base_os.get(base)) os_set.add(os);
    missing_map.set(zh, os_set);
  }
  const missing_out_li = [...missing_map]
    .map(([zh, os_set]) => ({ zh, os: [...os_set].sort() }))
    .sort((a, b) => a.os.length - b.os.length || a.zh.localeCompare(b.zh, 'zh-CN'));

  console.log(`# 缺失语言 (${missing_out_li.length})`);
  for (const { zh, os } of missing_out_li) {
    console.log(`- zh: ${q(zh)}`);
    console.log(`  os: ${q(os)}`);
  }
};

await main();
