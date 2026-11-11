// HarmonyOS / OpenHarmony 系统资源包的语言目录
const NON_LANG = new Set([
  'base',
  '2in1',
  '2in1-dark',
  'car',
  'dark',
  'phone',
  'tablet',
  'tv',
  'tv-dark',
  'wearable',
]);

export default {
  file: 'harmonyos',
  title: 'HarmonyOS 官方系统语言（OpenHarmony system_resources）',
  url: 'https://github.com/openharmony/global_system_resources/tree/master/systemres/main/resources',
  base64: false,
  parse: (text) =>
    [
      ...text.matchAll(/"path":"systemres\/main\/resources\/([^"/]+)","contentType":"directory"/g),
    ]
      .map((m) => m[1])
      .filter(
        (name) =>
          !NON_LANG.has(name) &&
          !name.endsWith('-wearable') &&
          !name.endsWith('-dark') &&
          name !== 'zz_ZX',
      ),
};
