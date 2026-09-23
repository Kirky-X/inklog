# Spec — tracing-export

> Delta spec for change `audit-hardening`. 覆盖此变更引入/修改的门面与导出链路需求。

## Requirements

### R-trace-001: log 门面 trace 上下文桥接
`log` 门面记录在调用点 `tracing::Span::current()` 非空时携带 trace_id/span_id（与 tracing 路径共享同一派生函数）。
**验收标准：**
- 在 `info_span` 内调用 `log::info!`，落盘记录的 trace_id/span_id 非空且与 span 派生值一致。
- 无 span 上下文时 trace_id/span_id 为 None（现状保持）。

### R-trace-002: log 门面 per-target 过滤
`LogAdapter` 持有 EnvFilter 快照（init 与 set_level 时同步），`enabled()` 先按 target 指令判断。
**验收标准：**
- `target_levels` 含 `myapp=warn` 时，`log::info!(target: "myapp", …)` 被丢弃、`log::error!(target: "myapp", …)` 通过。
- 未匹配 target 指令的记录回落到全局 max_level 判断。

### R-trace-003: set_level 级别同步
`manager.set_level` 热调后 log 门面的 `log::max_level()` 与新级别一致。
**验收标准：**
- init（level=info）后 `set_level(debug)`，`log::debug!` 记录可经 LogAdapter 落盘。
- 双门面在 init 与每次热调后的过滤边界一致（同一级别语义）。

### R-trace-004: OTLP 链路字段与严重级映射
OTLP 编码输出包含 `traceId`（32 hex）/`spanId`（16 hex）/`severityNumber`，字段值来自 LogRecord 顶层字段。
**验收标准：**
- 带 trace 上下文的记录编码 JSON 含三字段，宽度与 hex 格式正确。
- severityNumber 映射：TRACE=5、DEBUG=5、INFO=9、WARN=13、ERROR=17、FATAL=21。
- 无上下文时三字段缺省（不输出空串字段）。

### R-trace-005: OTLP 传输加密选项
https scheme 的 OTLP endpoint 在 `net-sink` feature 启用时经 rustls 传输；feature 未启用时构造期返回明确错误。
**验收标准：**
- https URL + feature 未启用：构造返回错误，错误文案含 net-sink feature 提示。
- http URL：不依赖 net-sink，行为与现状一致。

## Constraints

- tracing 路径调用点捕获后跨 channel 不丢失的行为不回归。
- 双门面无互桥、无双写的设计不回归（不引入 tracing→log 或 log→tracing 转发）。
- OTLP 手写传输不新增第三方依赖（rustls 复用 net-sink 已有依赖）。

## Out of Scope

- 读 OpenTelemetry span extensions / W3C traceparent 提取（真 OTel 集成另立变更）。
- OTLP 批量/重试语义升级。
