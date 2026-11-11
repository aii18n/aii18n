// Ubuntu 官方语言包（Packages 索引，非网页）
export default {
  file: 'ubuntu',
  title: 'Ubuntu Linux 官方语言包（noble-updates Packages）',
  url: 'https://archive.ubuntu.com/ubuntu/dists/noble-updates/main/binary-amd64/Packages.gz',
  base64: false,
  gzip: true,
  parse: (text) =>
    [...text.matchAll(/^Package: (language-pack-[a-z0-9-]+)$/gm)]
      .map((m) => m[1].slice('language-pack-'.length))
      .filter((x) => !x.endsWith('-base') && !x.startsWith('gnome-') && !x.startsWith('kde-')),
};
