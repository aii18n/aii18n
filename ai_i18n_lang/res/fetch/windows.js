// Windows 11 官方语言包与语言界面包（LP / LIP）
const stripTags = (html) =>
  html
    .replace(/<[^>]*>/g, ' ')
    .replaceAll('&nbsp;', ' ')
    .replaceAll('&amp;', '&')
    .replaceAll('&ndash;', '-')
    .replaceAll('&mdash;', '-')
    .replace(/\s+/g, ' ')
    .trim();

export default {
  file: 'windows',
  title: 'Windows 11 官方语言包与语言界面包（LP / LIP）',
  url: 'https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/available-language-packs-for-windows?view=windows-11',
  base64: false,
  parse: (text) =>
    [...text.matchAll(/<tr>([\s\S]*?)<\/tr>/g)]
      .map((m) => [...m[1].matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)].map((x) => stripTags(x[1])))
      .filter(
        (cells) => cells.length >= 2 && !/No longer used|Not available in Windows/i.test(cells[1]),
      )
      .map((cells) => cells[1].match(/^[A-Za-z]{2,3}(?:[-_][A-Za-z0-9]+)*/)?.[0])
      .filter(Boolean),
};
