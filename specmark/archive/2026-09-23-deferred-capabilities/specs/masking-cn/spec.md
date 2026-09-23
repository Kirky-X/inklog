# Spec — masking-cn

> Delta spec for change `deferred-capabilities`. 覆盖中文姓名键触发值掩码需求。

## Requirements

### R-maskcn-001: 姓名键词表
`NAME_FIELD_PATTERNS` 覆盖姓名族键且不误伤通用键。
**验收标准：**
- 命中：`full_name` / `real_name` / `legal_name` / `customer_name` / `owner_name` / `contact_name` / `surname` / `family_name` / `given_name` / `姓名` / `真实姓名` / `客户姓名`（大小写不敏感，`_`/`-` 分隔均可）。
- 不命中：`name`、`user_name`、`password`、`file_name`（裸 name 与登录 ID 场景明确排除）。

### R-maskcn-002: 键触发值形态掩码
姓名键 + 值为 2–4 个汉字（`\p{Han}`）→ 替换为"保留首字 + `**`"。
**验收标准：**
- `{"real_name": "张三丰"}` → `张**`；`{"姓名": "欧阳文长"}` → `欧**`。
- 值为非纯汉字（`John Smith`、`张三123`、4 字以上）→ 不动。
- 通用敏感键优先级不变：`{"real_name": "<password-like>"}` 场景行为不回归。
- 掩码结果不被后续值模式规则二次处理（幂等标记契约保持）。

## Constraints

- 姓名形态正则 `LazyLock` 预编译，fancy-regex `\p{Han}` 语法。
- 递归深度上限与 1 MiB 输入上限语义不变。
- 默认开启（随 `pii_masking_enabled`/`masking_enabled` 门控），无独立开关。

## Out of Scope

- 裸中文文本（无键上下文）的姓名识别。
- 非中文姓名（拼音/多段西名）。
