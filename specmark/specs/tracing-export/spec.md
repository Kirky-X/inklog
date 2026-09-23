# Spec — tracing-export

> Main spec for capability `tracing-export`.

## Requirements

### R-trace-001: otel feature 与 SpanContext 提取

新 feature `otel`；两条提取路径从 span extensions 读取 `opentelemetry::trace::SpanContext`。
**验收标准：**
- `otel = ["dep:opentelemetry"]`（0.30，default-features = false，features = ["trace"]），入 docs.rs metadata。
- tracing 路径：span extensions 中存在 `SpanContext` 且 `is_valid()` → `trace_id = 32 位小写 hex`、`span_id = 16 位小写 hex`，优先级最高。
- log 门面路径（`derive_trace_ids_from_current`）同等提取。
- 宿主 otel 版本未对齐（downcast 落空）时静默回退既有根 span 派生，不报错。

### R-trace-002: W3C traceparent 字段解析

事件/字段显式 `traceparent` 字段解析。
**验收标准：**
- 合法格式 `00-<32hex>-<16hex>-<2hex>` → trace_id/span_id 提取，两条路径共用同一解析函数。
- 版本段 `ff`、非 hex、段长不符 → 丢弃回退下一优先级。
- 优先级链：otel extension > traceparent 字段 > 根 span 派生。

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
- 不引入 tracing-opentelemetry 依赖（只消费 extensions 类型）。
- 无 otel feature / 无 extensions 时热路径零额外开销。
- 既有派生逻辑与 log 门面桥接行为（audit-hardening R-trace-001）不回归。

## Out of Scope

- 读 OpenTelemetry span extensions / W3C traceparent 提取（真 OTel 集成另立变更）。
- OTLP 批量/重试语义升级。
- 多版本 otel 兼容探测层。
- traceparent 注入（出站传播头生成）。
