# ai_i18n_conf

Config loader for `ai_i18n`. Reads a [NestedText](https://nestedtext.org) file and deserializes it into `Conf` via serde.

## Install

```toml
[dependencies]
ai_i18n_conf = "0.1"
```

## Config path

1. `AII18N_CONF_PATH`: full path to the config file
2. Fallback: `<system config dir>/aii18n.nt` (e.g. `~/.config/aii18n.nt` on Linux/macOS)

## Config file

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

Each entry in `api` maps to `Api` (defined in `ai_api`):

| Field   | Type       | Description                          |
| ------- | ---------- | ------------------------------------ |
| `base`  | `String`   | API endpoint                         |
| `token` | `Vec<String>` | Tokens, tried in order            |
| `model` | `String`   | Model name                           |
| `think` | `Think`    | Thinking level, `medium` by default  |

Note: NestedText has no inline mapping for list items, so `- base: ...` is **invalid**. The `-` must be alone on its line, with the dict fields indented below.

## Usage

```rust,no_run
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

Parse a string or a specific file:

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

Any `Deserialize` type works with `from_str` / `load_from`, so you can use your own structs instead of `Conf`.

## API

| Function | Description |
| -------- | ----------- |
| `conf() -> &'static Path` | Resolved config file path |
| `from_str(text) -> Result<Option<T>>` | Parse text; `None` for an empty document |
| `load_from(path) -> Result<Option<T>>` | Parse a file; `None` if missing or empty |
| `load() -> Result<Option<Conf>>` | Parse the resolved config file |

## Errors

`Error` has two variants:

- `Error::Io`: reading the file failed (except `NotFound`, which yields `Ok(None)`)
- `Error::Parse`: NestedText syntax or type error, with line and column info
