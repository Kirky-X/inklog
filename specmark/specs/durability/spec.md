# Spec — durability

> Main spec for capability `durability`.

## Requirements

### R-dur-001: FallbackJournal 模块

`support/fallback_journal.rs` 提供 JSONL 日志的溢出落盘与重放。
**验收标准：**
- `FallbackJournal::open(path, max_bytes)` 创建/打开 journal（默认路径 `logs/fallback.journal`，默认上限 10 MiB）。
- `spill(&LogRecord)` 追加一行 JSON；`replay()` 返回全部记录并清空文件（原子重放）。
- 超过 `max_bytes` 时丢最旧（截断头部保留最新），截断后仍可继续 spill。
- 损坏行（非 JSON/半行）跳过并计数，不 panic、不中断重放。
- 逐字段往返一致（timestamp/level/target/message/fields/trace_id/span_id）。

### R-dur-002: subscriber 溢出接线

兜底缓冲溢出与 LRU 淘汰的 ERROR/FATAL 记录写入 journal。
**验收标准：**
- 兜底缓冲（容量 100）打满后新进的关键日志与被淘汰的最旧条目落 journal（未启用开关时 journal 为 None，行为与现状一致、零开销）。
- 开关关闭时全链路无文件 IO。

### R-dur-003: 启动重放

manager 构建期（开关启用且 journal 非空）重放一次。
**验收标准：**
- 重放记录注入 async 通道抵达 file sink，字段完整并携带 `replayed = true` 字段（防重放循环）。
- 重放后 journal 文件清空；同一次进程生命周期不二次重放。

## Constraints

- journal 落盘不 fsync（承诺"进程崩溃后可重放"，断电窗口与 file sink 一致，文档明示）。
- spill 路径不得反压主日志链路（写失败静默计数，不 panic）。
- 配置：`global.fallback_journal: bool`（默认 false）、`global.fallback_journal_path`（默认 `logs/fallback.journal`）。

## Out of Scope

- journal 内容加密（可后续叠加）与跨机聚合。
- 非 ERROR/FATAL 级别的持久化兜底。
