<!-- domain: code -->
# deferred-capabilities

## Motivation

上一变更 `audit-hardening`（2026-09-23 归档）将四项能力显式列为 Non-Goal：中文姓名值模式识别、真实 OpenTelemetry 层链路上下文集成、ChannelBufferedFileSink 转正为默认 file sink、磁盘持久化 fallback 队列。四项均为审计确认的真实缺口（中文姓名完全不脱敏、trace_id 为派生值非 OTel 语义、默认 file sink 串行化且无独立 flush 线程、ERROR/FATAL 兜底缓冲上限 100 条且断电即失），用户要求以**单个变更**统一解决。kueiku RICE 排序：C1(10) > C4(6) ≈ C3(6) > C2(4)。

## Scope

1. **C1 中文姓名掩码**（masking.rs）：敏感键词表补充姓名族键（`real_name`/`full_name`/`legal_name`/`customer_name`/`owner_name`/`contact_name`/`surname`/`family_name`/`given_name`/`姓名`/`真实姓名`/`客户姓名`，刻意排除裸 `name`/`user_name`——登录 ID 误伤面过大）；键触发 + 值形态（2–4 个 `\p{Han}` 字符）命中时替换为"保留首字 + `**`"（如 `张**`）。
2. **C4 磁盘持久化 fallback 队列**（新增 `support/fallback_journal.rs` + subscriber/manager 接线）：ERROR/FATAL 兜底缓冲溢出与 LRU 淘汰的记录落盘 JSONL journal（容量上限默认 10 MiB）；进程启动时重放一次后清空；`global.fallback_journal` 开关（默认 false）。
3. **C3 CBFS 转正**（manager.rs file 工厂）：file 配置未启用轮转时间变体/压缩/加密/审计链（"简单配置"）时，默认 file sink 用 `ChannelBufferedFileSink`（已具备掩码、独立 flush 线程、可配背压策略）；启用高级能力的配置仍走 FileSink，能力矩阵文档化。
4. **C2 真 OTel 链路上下文**（新 feature `otel`，依赖 `opentelemetry`）：`extract_trace_context` 与 `derive_trace_ids_from_current` 从 span extensions 读取 `opentelemetry::trace::SpanContext`（优先级最高，宿主 otel 版本需与 inklog 对齐，未对齐时静默回退既有派生）；事件字段显式 `traceparent`（W3C `00-{tid32}-{sid16}-{flags}`）解析次之。

## Non-Goals

- **裸中文文本的姓名识别**（无键上下文扫描正文找姓名）：无字典不可判定、误伤面不可控；仅键触发 + 值形态。
- **tracing-opentelemetry 依赖引入**：只读 extensions 中的 `SpanContext` 类型，不需要 layer 本体；与宿主 otel 版本矩阵对齐问题以"静默回退"消解，不做多版本兼容层。
- **journal 的跨机聚合与加密**：journal 是本地兜底缓冲，落盘内容与 file sink 同级保护（后续可叠加 `encrypt`），不引入远端复制。
- **CBFS 具备全部 FileSink 高级能力**：高级能力场景继续走 FileSink，本变更只做"简单配置默认 CBFS"的转正与文档矩阵。
