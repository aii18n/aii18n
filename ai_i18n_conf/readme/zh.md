# ai_i18n_conf

`ai_i18n` 的配置加载器。读取 [NestedText](https://nestedtext.org) 文件，通过 serde 反序列化为 `Conf`。

## 安装

```toml
[dependencies]
ai_i18n_conf = "0.1"
```

## 配置路径

1. 环境变量 `AII18N_CONF_PATH`：直接指定配置文件完整路径
2. 未设置时回退到 `<系统配置目录>/aii18n.nt`（如 `~/.config/aii18n.nt`）

## 配置文件

```nt
# ai_i18n API 配置
api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
      - sk-yyy
    model: deepseek-v4.1-flash
    think: medium
```

`api` 的每一项对应 `Api`（结构体定义在 `ai_api`）：

| 字段    | 类型          | 说明             |
| ------- | ------------- | ---------------- |
| `base`  | `String`      | 接口地址         |
| `token` | `Vec<String>` | 可用 token，按顺序尝试 |
| `model` | `String`      | 模型名           |
| `think` | `Think`       | 思考强度，缺省 `medium` |

注意：NestedText 不支持列表项内联映射，`- base: ...` 是**非法**写法。`-` 必须独占一行，字典字段缩进在其下方。

## 使用

```rust
use ai_i18n_conf::{Conf, Error};

fn main() -> Result<(), Error> {
  if let Some(conf: Conf) = ai_i18n_conf::load()? {
    for api in conf.api {
      println!("{} {} {}", api.base, api.model, api.token.len());
    }
  }
  Ok(())
}
```

解析字符串或指定文件：

```rust
use std::path::Path;

use ai_i18n_conf::{Conf, Error};

fn demo() -> Result<(), Error> {
  let text = r#"api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
    model: deepseek-v4.1-flash
"#;
  let from_text: Option<Conf> = ai_i18n_conf::from_str(text)?;
  let from_file: Option<Conf> = ai_i18n_conf::load_from(Path::new("/path/to/aii18n.nt"))?;
  println!("{from_text:?} {from_file:?}");
  Ok(())
}
```

也可以用自己的结构体，`from_str` / `load_from` 对任意 `Deserialize` 类型泛型。

## 接口

| 函数 | 说明 |
| ---- | ---- |
| `conf() -> &'static Path` | 解析后的配置文件路径 |
| `from_str(text) -> Result<Option<T>>` | 解析文本，空文档返回 `None` |
| `load_from(path) -> Result<Option<T>>` | 解析文件，文件不存在或为空返回 `None` |
| `load() -> Result<Option<Conf>>` | 解析当前配置文件 |

## 错误

`Error` 有两个变体：

- `Error::Io`：读取文件失败（`NotFound` 除外，此时返回 `Ok(None)`）
- `Error::Parse`：NestedText 语法或类型错误，带行列信息
