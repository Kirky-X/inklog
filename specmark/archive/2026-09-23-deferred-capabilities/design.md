# Design — deferred-capabilities

## Context

四项能力彼此独立但共享上一变更建立的地基：masking 管线（fancy-regex、键值双通道、深度上限）、subscriber 兜底缓冲（`fallback_pending` 双触发）、manager file 工厂与 ops hub、`derive_trace_ids_from_current` 的 dispatch downcast。全库 clippy 0 警告基线与 pre-commit（detect-secrets/typos/fmt/clippy 组合 feature）约束沿用。

## Decision

### D1 中文姓名键触发值掩码（C1）

- `SENSITIVE_FIELD_PATTERNS` 追加独立 `NAME_FIELD_PATTERNS` 静态组（与通用敏感键分离，语义不同：命中后不是整值 `***MASKED***` 而是形态化部分掩码）：`(?i)(full|real|legal|customer|owner|contact)[_-]?name`、`(?i)surname|family[_-]?name|given[_-]?name`、`姓名|真实姓名|客户姓名`。刻意不含裸 `name`/`user_name`（登录 ID 场景误伤）。
- `mask_value_depth` 对 Object 键命中 `is_name_field(k)` 且值为字符串、匹配 `^[\p{Han}]{2,4}$` 时 → `首字 + "**"`；其余键路径不变。`mask_hashmap` 同步（键名检测优先级：通用敏感键 > 姓名键 > 值模式）。
- fancy-regex 已支持 `\p{Han}`；姓名形态正则 `LazyLock` 预编译。

### D2 磁盘持久化 fallback 队列（C4）

- 新模块 `support/fallback_journal.rs`：`FallbackJournal::open(path, max_bytes)`（默认 `logs/fallback.journal`，10 MiB）；`spill(&LogRecord)` 追加 JSONL（复用 LogRecord serde），超限丢最旧（截断头部：重写文件）；`replay() -> Vec<LogRecord>` 原子读取后清空文件。
- 接线：subscriber 兜底缓冲 push 时被 LRU 淘汰的条目、以及缓冲满后新到的关键日志 → `journal.spill()`（journal 为 `Option<Arc<Mutex<FallbackJournal>>>`，未启用 None 零开销）；manager 构建期（启用了开关时）`replay()` 一次注入 async 通道并清空，重放的记录打 `replayed = true` 字段防循环。
- 配置：`InklogConfig.global.fallback_journal: bool`（默认 false）+ `global.fallback_journal_path`（默认 `logs/fallback.journal`）。
- 断电语义：spill 即 fsync？不做（性能）——journal 承诺"进程崩溃后可重放"（OS page cache 级），与 file sink 一致；文档明示。

### D3 CBFS 转正（C3）

- manager file sink 装配点：`is_simple_file_config = rotation_time 为 "daily"（默认值）且 !compress && !encrypt && !audit_chain_enabled`。简单配置 → `ChannelBufferedFileSink`（T007 已含掩码；其 `flush_interval_ms`/`channel_capacity`/`backpressure_strategy` 从 `performance` 配置映射）；高级配置 → 既有 `FileSink`。
- 公共行为差异文档化（docs/USER_GUIDE.md 简表）：CBFS 路径下轮转/加密/压缩/审计链/留存清理不可用，需要这些能力时显式配置任一高级项即自动回落 FileSink。
- `ChannelBufferedConfig` 映射：`base_config = file 配置`，`channel_capacity = performance.channel_capacity`，`flush_interval_ms`/`flush_batch_size` 取 file 配置对应值。

### D4 真 OTel 链路上下文（C2）

- 新 feature：`otel = ["dep:opentelemetry"]`，`opentelemetry = { version = "0.30", default-features = false, features = ["trace"], optional = true }`；docs.rs metadata 追加 `otel`。
- 提取函数（subscriber.rs，feature 门控）：
  1. `extract_trace_context`（tracing 路径）：`span.extensions().get::<opentelemetry::trace::SpanContext>()` 命中且 `is_valid()` → `trace_id = 32hex(sc.trace_id())`、`span_id = 16hex(sc.span_id())`，优先级最高；
  2. `derive_trace_ids_from_current`（log 门面路径）：复用既有 Registry downcast，同样读 extensions；
  3. 两条路径共用 `traceparent` 字段解析：事件/字段中显式 `traceparent` 字段（`00-<32hex>-<16hex>-<2hex>`，版本 `ff` 丢弃）→ 次优先级；
  4. 兜底：既有"根 span id 派生"。
- 版本矩阵：宿主 otel 与 inklog 依赖的 opentelemetry 未统一时 extensions downcast 落空 → 静默回退派生值（不报错、不 panic）；文档要求对齐 otel 主版本。
- 无 tracing-opentelemetry 依赖：只消费 extensions 中的类型。

## Alternatives Considered

- **中文姓名全掩码（`***MASKED***`）**：保留首字（张**）是主流惯例（等保/个保测评样例），信息量损失更小；且与 phone/email 的部分保留风格一致。
- **journal 用 bincode/二进制**：JSONL 人工可读、可 grep 审计，与 log 生态一致；体积不是瓶颈（10 MiB 上限）。
- **CBFS 内嵌 FileSink 组合**（通道灌 LogRecord 给内层 FileSink）：能力全量继承但改动面波及 CBFS 公共 API（当前通道载荷是渲染后 String），风险高于"简单配置转正"，留给后续变更。
- **otel 提取放 LogAdapter 单独实现**：与 tracing 路径重复实现优先级链，统一在 subscriber 侧共享函数（log 门面已 downcast Registry，可复用）。

## Consequences

- 正面：PII 矩阵补齐中文姓名；关键日志跨进程不丢；默认 file 写路径获得独立 flush 线程与背压策略；链路 ID 与 OTel 生态语义打通。
- 负面/技术债：feature 矩阵 +1（`otel` 的 docs.rs 组合需验证）；journal 重放记录带 `replayed` 字段（下游消费方需感知）；CBFS/FileSink 双路径能力矩阵需要用户认知成本（文档缓解）。
- 跟进项：CBFS 全能力化（组合方案）、otel 多版本兼容探测、journal 加密。
