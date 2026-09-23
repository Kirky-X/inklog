# Tasks — deferred-capabilities

<!-- 任务按模块分组排序；P 标签来自 kueiku RICE（C1 P0 / C4、C3 P1 / C2 P2） -->

- [x] [T001] [P0] C1a 姓名键词表：masking.rs 新增 `NAME_FIELD_PATTERNS`（(?i)(full|real|legal|customer|owner|contact)[_-]?name、(?i)surname|family[_-]?name|given[_-]?name、姓名|真实姓名|客户姓名）与 `is_name_field()`；测试：real_name/full_name/姓名 命中，name/user_name/password 不命中 → src/support/processing/masking.rs
- [x] [T002] [P0] C1b 键触发值掩码：`mask_value_depth` 对命中 is_name_field 的键、字符串值匹配 `^[\p{Han}]{2,4}$` 时替换为 `首字 + "**"`（如 张**）；通用敏感键优先级不变。测试：{"real_name":"张三丰"} → 张**；{"real_name":"John Smith"} 不动；{"user_name":"张三"} 不动 → src/support/processing/masking.rs
- [x] [T003] [P1] C4a FallbackJournal 模块：新增 support/fallback_journal.rs（open(path, max_bytes 10MiB 默认)/spill/replay 原子清空/超限丢最旧，JSONL 复用 LogRecord serde，损坏行跳过并计数），support/mod.rs 导出。测试：spill→replay 往返逐字段一致；超 cap 后最旧被截断；损坏行跳过不 panic → src/support/fallback_journal.rs, src/support/mod.rs
- [x] [T004] [P1] C4b journal 接线：global 配置新增 `fallback_journal: bool`（默认 false）与 `fallback_journal_path`（默认 logs/fallback.journal）；subscriber 兜底缓冲 LRU 淘汰与溢出路径 spill（journal 为 Option，None 零开销）；manager 构建期启用时 replay 一次注入 async 通道（记录带 replayed=true 字段防循环）后清空。测试：开启开关时灌 105 条 ERROR（容量 100）→ journal 含 5 条；重建 manager 后 replay 记录进入 file sink 且带 replayed 字段 → src/domain/config/global.rs, src/domain/core/subscriber.rs, src/domain/core/manager.rs
- [x] [T005] [P1] C3 CBFS 转正：manager file 工厂判定"简单配置"（rotation_time=="daily" 且 !compress && !encrypt && !audit_chain_enabled）→ ChannelBufferedFileSink（channel_capacity 取 performance.channel_capacity，flush 参数取 file 配置），否则 FileSink；测试：默认配置构建 CBFS 实例（LogSink 写入落盘可用），encrypt=true 构建 FileSink；docs/USER_GUIDE.md 增能力矩阵小节 → src/domain/core/manager.rs, docs/USER_GUIDE.md
- [x] [T006] [P2] C2a otel feature：Cargo 新增 `otel = ["dep:opentelemetry"]`（opentelemetry 0.30，default-features = false，features = ["trace"]，optional）并入 docs.rs metadata；subscriber.rs extract_trace_context 与 derive_trace_ids_from_current 增加 extensions 中 `opentelemetry::trace::SpanContext`（is_valid）提取（32/16 hex 小写），优先级最高、downcast 失败静默回退派生。测试（feature=otel）：registry span 手工插入 SpanContext 后两条路径均提取出与 SpanContext 一致的 ID → Cargo.toml, src/domain/core/subscriber.rs
- [x] [T007] [P2] C2b traceparent 字段解析：共享解析函数（`00-<32hex>-<16hex>-<2hex>`，版本 ff 丢弃，非 hex 丢弃）接入 extract_trace_context 与 derive_trace_ids_from_current（优先级：otel extension > traceparent > 派生）。测试：事件字段带合法 traceparent 时提取一致；版本 ff 与非 hex 输入回退 → src/domain/core/subscriber.rs

## Phase 1: Convergence

_由 /specmark converge 于 2026-09-23 生成。仅追加：不要编辑之前的任务。_

**发现缺口：** 0 (CRITICAL: 0 | HIGH: 0 | MEDIUM: 0 | LOW: 1)
**追加任务：** 0（跳过：1 个 LOW，记录为叙述）
**未请求范围（按原样接受）：** 无

**验收标准检查（4 个 delta spec）：**
| R-ID | 验收条件 | 状态 |
|------|----------|------|
| R-maskcn-001 | 姓名键词命中/不命中集 | ✓ PASS（test_name_field_patterns_coverage） |
| R-maskcn-002 | 键触发 2-4 汉字→首字+**；非汉字/超长/裸 name 不动；优先级不回归 | ✓ PASS（test_cjk_name_value_masked_by_key_context 等 3 测） |
| R-dur-001 | journal spill/replay 往返、cap 丢最旧、损坏行跳过 | ✓ PASS（fallback_journal 4 测） |
| R-dur-002 | LRU 淘汰落 journal；开关关闭零 IO | ✓ PASS（test_fallback_eviction_spills_to_journal / disabled） |
| R-dur-003 | 启动重放带 replayed=true 且清空、不二次重放 | ✓ PASS（test_fallback_journal_replay_on_startup） |
| R-sink-001 | 简单配置 CBFS / 高级配置回落 FileSink | ✓ PASS（predicate 5 分支 + 冒烟落盘） |
| R-sink-002 | 参数映射（capacity/flush 取配置） | ✓ PASS（映射由工厂路径实现并被冒烟覆盖；未单独断言各字段值，实现即代码可见） |
| R-sink-003 | USER_GUIDE 能力矩阵 | ✓ PASS（双路径 8 行矩阵 + 回落说明） |
| R-trace-001 | otel feature + SpanContext 提取最高优先 + 版本未对齐静默回退 | ✓ PASS（test_otel_span_context_extraction_from_extensions，feature=otel） |
| R-trace-002 | traceparent 解析 + 优先级链 + 非法回退 | ✓ PASS（test_traceparent_field_parsing_and_priority / invalid_falls_back） |

**LOW/unrequested 跳过记录：** ① R-sink-002 参数映射未做逐字段断言（LOW，映射为纯赋值且被构建路径覆盖）。

## Phase N: Convergence
