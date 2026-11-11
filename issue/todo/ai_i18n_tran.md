请基于 ai_api 实现 ai_i18n_tran

接口类似

async fn tran(from_lang:impl AsRef<str>,to_lang:impl AsRef<str>, txt: &[impl AsRef<str>])-> Result<Vec<String>>{
}

内部先给每一行标注上行号，然后生成提示词调用 ai_api（如果是空直接return）

要求ai翻译的时候，返回格式如下

```
行号:
{from_lang}:
{to_lang}:
---
行号:
{from_lang}:
{to_lang}:
```

然后按`\n---\n`split，然后解析, 如果有格式错误，或者有行号的原文和实际原文不匹配，用多轮对话提示ai修改

这里，修改只需要返回修订行的，不需要返回整体的，减少token

参考ai_api写example，演示翻译

代码规范见.agents/skills/rust_review/SKILL.md
