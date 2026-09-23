# Spec — masking

> Delta spec for change `audit-hardening`. 覆盖此变更引入/修改的脱敏管线需求。

## Requirements

### R-masking-001: 数字类 PII 规则的边界与覆盖修复
内置正则在 CJK 上下文与小写变体下不漏报、对时间戳与十六进制串不误伤。
**验收标准：**
- `mask("电话13812345678")` 输出不含明文手机号（汉字紧贴数字场景）。
- `mask("身份证110101199001011234x")` 小写 x 结尾被掩码。
- 13 位毫秒时间戳（如 `1695000000000`）不被 bank_card 规则掩码；16–19 位卡号仍被掩码。
- git 短 SHA（9 位全 hex，如 `e1a2b3c4d`）不被 passport 规则掩码；`E12345678` 仍被掩码。
- 16/17 号段、国际 E.164、SSN 规则行为不回归（既有测试全绿）。

### R-masking-002: 递归与输入上限
掩码/清洗递归有深度上限，超大输入跳过而非阻塞或崩溃。
**验收标准：**
- `mask_value`/`mask_hashmap`/`sanitize_field_value` 在嵌套 ≥16 层时该子树替换为 `***TRUNCATED***`，函数正常返回（1000 层嵌套输入不栈溢出）。
- `mask()` 对长度 > 1 MiB 的字符串输入原样返回并产生一条 warn 日志。

### R-masking-003: 非字符串 JSON 值掩码
`Value::Number`/`Value::Bool` 的字段值经字符串化后参与掩码正则，命中即替换为掩码字符串值；`Value::Null` 保持跳过。
**验收标准：**
- 字段 `{"phone": 13812345678}` 掩码后 phone 值为掩码字符串。
- 既有嵌套 Object/Array 掩码测试不回归。

### R-masking-004: DB sink 键值掩码
database sink 对 fields 采用键名检测参与的 `mask_hashmap` 路径，敏感键整值替换。
**验收标准：**
- fields 含 `password` 键（值 ≥16 字符）时，落库 JSON 中该值为 `***MASKED***`，而非依赖值正则的偶然命中。
- 非敏感键的普通值内容不被破坏。

### R-masking-005: 三出口 sink 掩码覆盖
net、otlp、ChannelBufferedFileSink 三个出口在写出前执行与 console/file 同源的 DataMasker，受 `masking_enabled` 门控。
**验收标准：**
- 经 net sink 发出的 JSON、OTLP 编码产物、ChannelBufferedFileSink 写出文件均不含未掩码手机号（`masking_enabled=true` 默认）。
- `masking_enabled=false` 时三出口行为与现状一致（不掩码）。

### R-masking-006: masker 注入点与 AC 接线
sink 公开 `with_masker` 注入点；fast-masking feature 下 builder 对 literal 自定义规则自动构建 AcMasker。
**验收标准：**
- 注入含自定义 literal 规则的 masker 后，sink 输出体现该规则。
- feature=fast-masking 时 literal 规则经 AC 单趟替换（行为等价测试通过），文档明示内置规则仍走正则。

## Constraints

- 掩码正则全部 `LazyLock` 预编译，热路径零重复编译（既有约束）。
- 深度上限常量与幂等短路（`***REDACTED`/`***MASKED` 标记跳过）行为保留。
- 新增错误/警告消息走 fluent i18n 键（en/zh 成对）。

## Out of Scope

- 中文姓名值模式识别（无上下文不可判定，仅键名检测可另行扩展）。
- 规则热更新（运行期 reload）。
- ChannelBufferedFileSink 替换为默认 file sink。
