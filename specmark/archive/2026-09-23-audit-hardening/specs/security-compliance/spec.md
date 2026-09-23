# Spec — security-compliance

> Delta spec for change `audit-hardening`. 覆盖此变更引入/修改的安全与合规需求。

## Requirements

### R-sec-001: 日志与产物文件权限
日志文件、轮转加密产物（.enc）、压缩产物（.zst/.gz）以 0o600 创建；新建日志目录以 0o700 设置权限（unix）。
**验收标准：**
- unix 下活动日志文件、加密产物、压缩产物的 mode 均为 0o600（测试以 PermissionsExt 断言）。
- 已存在文件被打开时不回改权限（不破坏用户显式设置）。
- O_NOFOLLOW 语义保留。

### R-sec-002: 年龄清理独立执行
`perform_cleanup` 中年龄清理不再依赖 `max_total_size` 可解析才执行：大小清理与年龄清理为互不遮蔽的两个阶段。
**验收标准：**
- `max_total_size` 为空/不可解析时，超过 `retention_days` 的轮转文件仍被删除。
- `max_total_size` 超限时大小清理执行，年龄清理结果不与其冲突（双重保护语义保留）。
- 清理范围仍限定于当前日志集前缀文件（既有安全测试不回归）。

### R-sec-003: 等保留存提示
`FileSinkConfig::validate` 在 `encrypt=true` 且 `retention_days<180` 时发出 warn（等保 2.0 六个月留存指引），不阻断配置。
**验收标准：**
- 触发条件产生一条含 180/等保语义的 warn 日志。
- `retention_days=180` 或 `encrypt=false` 不产生该 warn。

## Constraints

- 默认值变更最小化：`retention_days` 默认值保持 30（库默认不改，等保场景经配置达成——由 validate 提示引导），避免静默改变既有部署的磁盘行为。
- i18n：新增 warn 走 fluent 键（en/zh 成对）。

## Out of Scope

- 审计日志 WORM/远端不可变存储。
- 日志内容级加密（活动文件明文、轮转后加密的既有语义不变）。
