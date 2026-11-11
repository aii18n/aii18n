# ai_i18n_lang

## 特性

- `Code`：全部 93 种语言，`repr(u16)`，`as_str` / `AsRef<str>` / `Display` 由 strum 派生
- `NAME` / `EN` / `ZH`：语言自称 / 英文名 / 中文名
- `by_code`：解析 `zh-CN`、`zh-TW`、`en-US`、`fil-PH` 等代码，忽略大小写与 `-` / `_` 差异

## 用法

```rust
use ai_i18n_lang::{by_code, Code, NAME};

assert_eq!(by_code("zh-CN"), Some(Code::Zh));
assert_eq!(by_code("zh-TW"), Some(Code::ZhTW));
assert_eq!(by_code("en-US"), Some(Code::En));
assert_eq!(Code::Zh.as_str(), "zh");
assert_eq!(Code::ZhTW.to_string(), "zh-TW");
assert_eq!(NAME[Code::Zh as usize], "简体中文");
```

## 解析规则

1. 忽略大小写，`-` 与 `_` 等价
2. 按主子标签匹配（`en-US` → `en`）
3. `zh` 按区域 / 文字细分：`TW` / `HK` / `MO` / `Hant` → `ZhTW`，其余 → `Zh`

## 语言表

| # | Code | Endonym | English | 中文 |
|---:|---|---|---|---|
| 0 | `en` | English | English | 英语 |
| 1 | `zh` | 简体中文 | Simplified Chinese | 简体中文 |
| 2 | `fr` | français | French | 法语 |
| 3 | `de` | Deutsch | German | 德语 |
| 4 | `es` | español | Spanish | 西班牙语 |
| 5 | `ja` | 日本語 | Japanese | 日语 |
| 6 | `ar` | العربية | Arabic | 阿拉伯语 |
| 7 | `pt` | português | Portuguese | 葡萄牙语 |
| 8 | `ru` | русский | Russian | 俄语 |
| 9 | `it` | italiano | Italian | 意大利语 |
| 10 | `nl` | Nederlands | Dutch | 荷兰语 |
| 11 | `ko` | 한국어 | Korean | 韩语 |
| 12 | `tr` | Türkçe | Turkish | 土耳其语 |
| 13 | `id` | Bahasa Indonesia | Indonesian | 印尼语 |
| 14 | `zh-TW` | 正體中文 | Traditional Chinese | 正体中文 |
| 15 | `hi` | हिन्दी | Hindi | 印地语 |
| 16 | `pl` | polski | Polish | 波兰语 |
| 17 | `ta` | தமிழ் | Tamil | 泰米尔语 |
| 18 | `ga` | Gaeilge | Irish | 爱尔兰语 |
| 19 | `sv` | svenska | Swedish | 瑞典语 |
| 20 | `bn` | বাংলা | Bengali | 孟加拉语 |
| 21 | `he` | עברית | Hebrew | 希伯来语 |
| 22 | `th` | ไทย | Thai | 泰语 |
| 23 | `no` | norsk | Norwegian | 挪威语 |
| 24 | `vi` | Tiếng Việt | Vietnamese | 越南语 |
| 25 | `mr` | मराठी | Marathi | 马拉地语 |
| 26 | `ms` | Bahasa Melayu | Malay | 马来语 |
| 27 | `da` | dansk | Danish | 丹麦语 |
| 28 | `ro` | română | Romanian | 罗马尼亚语 |
| 29 | `ur` | اردو | Urdu | 乌尔都语 |
| 30 | `cs` | čeština | Czech | 捷克语 |
| 31 | `fa` | فارسی | Persian | 波斯语 |
| 32 | `te` | తెలుగు | Telugu | 泰卢固语 |
| 33 | `pa` | ਪੰਜਾਬੀ | Punjabi | 旁遮普语 |
| 34 | `kn` | ಕನ್ನಡ | Kannada | 卡纳达语 |
| 35 | `gu` | ગુજરાતી | Gujarati | 古吉拉特语 |
| 36 | `el` | Ελληνικά | Greek | 希腊语 |
| 37 | `fi` | suomi | Finnish | 芬兰语 |
| 38 | `ca` | català | Catalan | 加泰罗尼亚语 |
| 39 | `gd` | Gàidhlig | Scots Gaelic | 苏格兰盖尔语 |
| 40 | `kk` | қазақ тілі | Kazakh | 哈萨克语 |
| 41 | `tl` | Filipino | Filipino | 菲律宾语 |
| 42 | `sw` | Kiswahili | Swahili | 斯瓦希里语 |
| 43 | `ug` | ئۇيغۇرچە | Uyghur | 维吾尔语 |
| 44 | `hu` | magyar | Hungarian | 匈牙利语 |
| 45 | `uk` | українська | Ukrainian | 乌克兰语 |
| 46 | `uz` | oʻzbek tili | Uzbek | 乌兹别克语 |
| 47 | `ml` | മലയാളം | Malayalam | 马拉雅拉姆语 |
| 48 | `sk` | slovenčina | Slovak | 斯洛伐克语 |
| 49 | `af` | Afrikaans | Afrikaans | 南非语 |
| 50 | `hr` | hrvatski jezik | Croatian | 克罗地亚语 |
| 51 | `sr` | српски језик | Serbian | 塞尔维亚语 |
| 52 | `bg` | български език | Bulgarian | 保加利亚语 |
| 53 | `br` | brezhoneg | Breton | 布列塔尼语 |
| 54 | `am` | አማርኛ | Amharic | 阿姆哈拉语 |
| 55 | `cy` | Cymraeg | Welsh | 威尔士语 |
| 56 | `or` | ଓଡ଼ିଆ | Odia | 奥里亚语 |
| 57 | `si` | සිංහල | Sinhala | 僧伽罗语 |
| 58 | `ha` | Hausa | Hausa | 豪萨语 |
| 59 | `eu` | Euskara | Basque | 巴斯克语 |
| 60 | `szl` | ślōnskŏ gōdka | Silesian | 西里西亚语 |
| 61 | `lt` | lietuvių kalba | Lithuanian | 立陶宛语 |
| 62 | `be` | беларуская мова | Belarusian | 白俄罗斯语 |
| 63 | `zu` | isiZulu | Zulu | 祖鲁语 |
| 64 | `gl` | galego | Galician | 加利西亚语 |
| 65 | `my` | မြန်မာဘာသာ | Burmese | 缅甸语 |
| 66 | `sl` | slovenščina | Slovenian | 斯洛文尼亚语 |
| 67 | `az` | Azərbaycan dili | Azerbaijani | 阿塞拜疆语 |
| 68 | `as` | অসমীয়া | Assamese | 阿萨姆语 |
| 69 | `km` | ខ្មែរ | Khmer | 高棉语 |
| 70 | `ne` | नेपाली | Nepali | 尼泊尔语 |
| 71 | `ku` | Kurdî | Kurdish | 库尔德语 |
| 72 | `xh` | isiXhosa | Xhosa | 科萨语 |
| 73 | `lv` | latviešu valoda | Latvian | 拉脱维亚语 |
| 74 | `et` | eesti keel | Estonian | 爱沙尼亚语 |
| 75 | `tt` | татар теле | Tatar | 鞑靼语 |
| 76 | `is` | íslenska | Icelandic | 冰岛语 |
| 77 | `ka` | ქართული | Georgian | 格鲁吉亚语 |
| 78 | `bs` | bosanski jezik | Bosnian | 波斯尼亚语 |
| 79 | `sq` | Shqip | Albanian | 阿尔巴尼亚语 |
| 80 | `hy` | Հայերեն | Armenian | 亚美尼亚语 |
| 81 | `mn` | монгол хэл | Mongolian | 蒙古语 |
| 82 | `ky` | кыргыз тили | Kyrgyz | 吉尔吉斯语 |
| 83 | `mk` | македонски јазик | Macedonian | 马其顿语 |
| 84 | `lo` | ລາວ | Lao | 老挝语 |
| 85 | `tg` | тоҷикӣ | Tajik | 塔吉克语 |
| 86 | `rw` | Ikinyarwanda | Kinyarwanda | 卢旺达语 |
| 87 | `eo` | Esperanto | Esperanto | 世界语 |
| 88 | `fil` | Filipino | Filipino | 菲律宾语 |
| 89 | `nn` | norsk nynorsk | Norwegian Nynorsk | 新挪威语 |
| 90 | `ckb` | کوردی | Kurdish, Sorani | 库尔德语 |
| 91 | `wo` | Wolof | Wolof | 沃洛夫语 |
| 92 | `fy` | Frysk | Western Frisian | 西弗里斯语 |
