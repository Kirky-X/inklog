# Spec — sinks

> Main spec for capability `sinks`.

## Requirements

### R-sink-001: 简单配置默认 CBFS

manager file 工厂在"简单配置"下构建 `ChannelBufferedFileSink`。
**验收标准：**
- 简单配置判定：`rotation_time == "daily"`（默认值）且 `!compress` 且 `!encrypt` 且 `!audit_chain_enabled`。
- 简单配置下构建的 file sink 写入→落盘可用（含掩码语义，`masking_enabled` 门控与既有一致）。
- 任一高级项启用（非默认 `rotation_time` / `compress` / `encrypt` / `audit_chain_enabled`）→ 回落 `FileSink`，高级能力行为不回归。

### R-sink-002: 参数映射

CBFS 参数从既有配置映射。
**验收标准：**
- `channel_capacity` 取 `performance.channel_capacity`；`flush_batch_size` 取 `file.batch_size`；`flush_interval_ms` 取 `file.flush_interval_ms`；`base_config` 承载 file 配置（路径/格式/掩码开关）。
- 背压策略默认 `Block`（async 路径退避重试语义）。

### R-sink-003: 能力矩阵文档化

docs/USER_GUIDE.md 增补双路径能力矩阵。
**验收标准：**
- 明示 CBFS 路径下轮转/加密/压缩/审计链/留存清理不可用，需要时如何触发 FileSink 回落。

## Constraints

- 现有 FileSink 行为（高级配置路径）零回归。
- CBFS 掩码（T007 of audit-hardening）语义保持。

## Out of Scope

- CBFS 内嵌 FileSink 的全能力组合方案（另立变更）。
