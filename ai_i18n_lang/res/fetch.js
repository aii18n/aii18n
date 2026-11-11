#!/usr/bin/env -S bun
// 扫描 res/fetch/*.js 抓取各系统官方语言，统一输出为 res/support/<系统>.yml
// 每个脚本默认导出: { file, title, url, base64?, gzip?, parse(text) => string[] }
// 用法: bun res/fetch.js
import { mkdir, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { shortName } from './short_name.js';

const SOURCE_DIR = join(import.meta.dirname, 'fetch');

// 伪语言（仅供测试，非自然语言）
const PSEUDO = new Set(['ar-XB', 'en-XA']);

// 错误码
const ERR_FETCH = 1,
  ERR_PARSE = 2,
  ERR_SOURCE = 3;

const fail = (code, ...data) => {
  throw [code, ...data];
};

// 统一为 BCP-47 标签，处理 gettext 与 AOSP 的下划线、@ 修饰符
const canon = (raw) => {
  const fixed = raw
    .trim()
    .replaceAll('_', '-')
    .replace(/@(?:latin|latn)$/i, '-Latn')
    .replace(/@(?:arab)$/i, '-Arab');
  try {
    return Intl.getCanonicalLocales(fixed)[0] ?? fixed;
  } catch {
    return fixed;
  }
};

// Unicode 名称缺失时的中文兜底
const FALLBACK = {
  bqi: ['巴赫蒂亚里语', 'Bakhtiari'],
  cak: ['卡克奇克尔语', 'Kaqchikel'],
  grc: ['古希腊语', 'Ancient Greek'],
  ltg: ['拉特加莱语', 'Latgalian'],
  meh: ['努尤米斯特克语', 'Nuyoo Mixtec'],
  son: ['桑海语', 'Songhay'],
  syr: ['叙利亚语', 'Syriac'],
  trs: ['特里基语', 'Triqui'],
  zza: ['扎扎其语', 'Zazaki'],
};

const zh_names = new Intl.DisplayNames(['zh-CN'], { type: 'language' }),
  en_names = new Intl.DisplayNames(['en'], { type: 'language' });

// idx: 0 中文名, 1 英文名
const nameOf = (dn, code, idx) => {
  const base = code.split('-')[0];
  let name;
  try {
    name = dn.of(code);
  } catch {
    name = null;
  }
  if (name && name !== code) return name;
  return FALLBACK[base]?.[idx] ?? (idx === 1 ? code : base);
};

const finalize = (code_li) =>
  [...new Set(code_li.map(canon).filter((c) => !PSEUDO.has(c)))]
    .sort((a, b) => a.localeCompare(b, 'en'))
    .map((code) => ({
      code,
      zh: shortName(nameOf(zh_names, code, 0)),
      en: shortName(nameOf(en_names, code, 1)),
    }));

const q = (value) => JSON.stringify(value);

const toYaml = (title, url, entries) =>
  [
    `# ${title}`,
    `# 官方源: ${url}`,
    `# 共 ${entries.length} 种语言，由 res/fetch.js 生成`,
    '',
    ...entries.flatMap(({ code, zh, en }) => [
      `- code: ${q(code)}`,
      `  zh: ${q(zh)}`,
      `  en: ${q(en)}`,
    ]),
    '',
  ].join('\n');

const loadModule = async (file) => {
  const mod = await import(pathToFileURL(join(SOURCE_DIR, file)).href),
    source = mod.default;
  if (!source?.file || !source?.url || !source?.parse) fail(ERR_SOURCE, file);
  return source;
};

const load = async ({ file, title, url, base64, gzip, parse }) => {
  const res = await fetch(url);
  if (!res.ok) fail(ERR_FETCH, url, res.status);
  let text;
  if (gzip) {
    text = Buffer.from(Bun.gunzipSync(new Uint8Array(await res.arrayBuffer()))).toString('utf8');
  } else {
    const raw = await res.text();
    text = base64 ? Buffer.from(raw, 'base64').toString('utf8') : raw;
  }
  const entries = finalize(parse(text));
  if (!entries.length) fail(ERR_PARSE, url, 'empty');
  return [file, title, url, entries];
};

const main = async () => {
  const dir = join(import.meta.dirname, 'support');
  await mkdir(dir, { recursive: true });
  const file_li = (await readdir(SOURCE_DIR)).filter((file) => file.endsWith('.js')).sort(),
    source_li = await Promise.all(file_li.map(loadModule)),
    result_li = await Promise.all(source_li.map(load));
  await Promise.all(
    result_li.map(([file, title, url, entries]) =>
      Bun.write(join(dir, `${file}.yml`), toYaml(title, url, entries)),
    ),
  );
  for (const [file, , , entries] of result_li) console.log(`${file}: ${entries.length}`);
};

await main();
