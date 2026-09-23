# Spec — tracing-export

> Delta spec for change `deferred-capabilities`. 覆盖真 OpenTelemetry 链路上下文与 W3C traceparent 需求。

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

## Constraints

- 不引入 tracing-opentelemetry 依赖（只消费 extensions 类型）。
- 无 otel feature / 无 extensions 时热路径零额外开销。
- 既有派生逻辑与 log 门面桥接行为（audit-hardening R-trace-001）不回归。

## Out of Scope

- 多版本 otel 兼容探测层。
- traceparent 注入（出站传播头生成）。
