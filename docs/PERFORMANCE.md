# ⚡ inklog 性能基线

本文档记录 inklog 首份正式性能基线（2026-09-11，criterion 中位数）。基准覆盖写入、序列化与加密三条路径，数字与测试环境原样保留，跨版本更新时在本文档续记。

> 相关文档：[📖 用户指南](USER_GUIDE.md) · [🏗️ 架构设计](ARCHITECTURE.md) · [🧪 测试场景](TEST_SCENARIOS.md)

## 🖥️ 环境与方法

| 项 | 值 |
| --- | --- |
| 日期 | 2026-09-11 |
| 机型 | WSL2（linux 6.6.87.2-microsoft-standard-x64），开发笔记本 |
| 工具 | criterion 0.8（`cargo bench --bench rc4_pipeline_bench`） |
| profile | release（`opt-level=3`、`lto=fat`、`codegen-units=1`） |
| 运行参数 | `--warm-up-time 1 --measurement-time 2 --sample-size 10`（快速基线） |

> **CI 门禁口径**：CI `benchmark` job 为**手动开关**（`workflow_dispatch` 勾选 `run_bench`，默认关闭，不进 push/PR 门禁），阈值为宽松天花板（`write_path/template_render_text` 中位 ≤5 µs，≈16x 本地基线，只拦灾难性回退如热路径误接阻塞 I/O）。精确回归对照以本文档数字为人工载体；不同机型噪声差异大，待 CI 固定 runner 后再按「中位数回归 >10% 标红」收紧。

## 📊 基线数字（2026-09-11，中位数）

### 写入路径（write_path）

| 基准 | 中位耗时 | 吞吐 |
| --- | --- | --- |
| `template_render_text`（默认模板 + 2 字段） | **320 ns** | ~3.12 M rec/s |
| `mask_sensitive_fields`（DataMasker 正则 + 敏感键） | **50.5 µs** | ~19.8 K rec/s |
| `logrecord_to_json`（单条序列化） | **257 ns** | ~3.89 M rec/s |

要点：

- 模板渲染与 JSON 序列化处于同一量级（~300 ns/条），默认写入主链路（模板渲染）每条日志的可观测处理成本 **< 0.5 µs**；
- 脱敏（`mask_sensitive_fields`）是主链路中最贵的安全环节（正则集合 + 递归字段遍历），成本约为渲染的 **160 倍**。建议仅在 sink 层按需开启（`masking_enabled`），不要在热路径无条件调用；
- 加密与轮转按轮转文件触发一次（见下文加密路径），不在每条记录的写入路径上。

### 序列化路径（serialization）

| 基准 | 中位耗时 | 吞吐 |
| --- | --- | --- |
| `logrecord_batch_100_to_json`（100 条批量） | **26.5 µs** | ~3.77 M rec/s |
| `otlp_body_100`（`otlp` feature，100 条 OTLP JSON 体） | 见 criterion 本机最新输出 | — |

批量序列化与单条线性的比值约 1.06x，无超线性放大；OTLP 体编码在 `otlp` feature 下另行采样（编码器与 serde 同构，量级一致）。

### 加密路径（encryption）

| 基准 | 中位耗时 | 吞吐 |
| --- | --- | --- |
| `pbkdf2_derive_600k`（PBKDF2-HMAC-SHA256，600k 迭代） | **63.3 ms** | ~15.8 ops/s |
| `aes256gcm_roundtrip_1kb`（1 KiB 加解密往返） | **506 ns** | ~1.88 GiB/s |

要点：

- PBKDF2 600k 迭代约 63 ms，**每次轮转文件至多一次**（密钥派生），折算到单条日志上可忽略；600,000 是 OWASP 对 PBKDF2-HMAC-SHA256 的推荐值，也是安全审查认可的最低迭代数，禁止为性能调低；
- AES-256-GCM 数据面约 1.9 GiB/s（ring 后端），加密记录的主链路成本可控。

## 📈 既有基准（inklog_bench）

`benches/inklog_bench.rs` 覆盖 LogRecord 创建、console sink 延迟、通道入队、持续/突发吞吐、FileSink 吞吐、no-op 对照、模板渲染、掩码、背压、并发、对象池、零分配等组（parquet 转换按 database feature 门控），运行方式：

```bash
cargo bench --bench inklog_bench
```

其中 `sampling_stress_overhead` 组锁定限流压力路径的采样决策每记录开销：`RateLimiter::new(0)` 使每条记录都进入压力分支，对比 `no_policy_builtin_fallback`（内置兜底：非关键级别丢弃）与 `policy_per_level_and_prefix`（per_level 采样率 + target 前缀规则的 `SamplingPolicy` 决策）两态，即采样策略的每记录增量代价。

`target_rate_limiter_lookup` 组锁定 per-target 分级限流的查找路径开销：`lookup_longest_prefix_hit`（最长前缀命中）与 `lookup_no_rule_fallthrough`（未命中全表扫完）为纯前缀查找基线（实测 ≈6ns / ≈4ns，远低于 100ns 量级），`evaluate_governed_pass` 为含组桶锁与令牌扣减的端到端裁决（实测 ≈41ns，单线程无竞争地板值；组桶为组内共享单锁，多生产者场景含锁竞争）。2026-10-01 同环境复测值（≈8ns / ≈4ns / ≈39ns）见下方正式基线表。

### inklog_bench 正式基线（2026-10-01，criterion 中位数）

同环境快速口径：`--warm-up-time 1 --measurement-time 3 --sample-size 10`（`parquet_conversion` 组按 database feature 门控，本次未启用，不入基线；`otlp_body_100` 属 rc4 组亦未启用）。数字取 criterion `estimates.json` 中位点估计。

| 组 / 基准 | 中位耗时 | 说明 |
| --- | --- | --- |
| `create_log_record` | 93 ns | LogRecord 构造（含 thread_id 格式化） |
| `template_rendering/render_simple_template` | 242 ns | 简单模板渲染 |
| `template_rendering/render_complex_template` | 268 ns | 复杂模板渲染 |
| `masking/mask_email` | 99 ns | 单条 email 规则命中 |
| `masking/mask_phone` | 852 ns | 单条 phone 规则命中 |
| `masking/mask_data_masher` | 44.1 µs | DataMasker 全规则集遍历 |
| `console_sink_latency/console_sync_latency` | 390 µs | console sink 同步写延迟（含 TTY 检测与格式化；本机为管道输出非真实 TTY） |
| `channel_enqueue_latency/async_channel_enqueue` | 2.94 µs | 异步通道入队（有界 crossbeam channel） |
| `file_sink/async_file_log` | 3.01 µs | FileSink 异步写入（通道分发路径） |
| `noop_sink/async_noop_log` | 3.88 µs | no-op 对照（管道固定开销上界） |
| `throughput_sustained/sustained_5_logs_per_sec` | 190.8 ms | 100 条按 5 rec/s 节拍持续写入（含固定等待，非单条开销） |
| `throughput_burst/burst_500_logs_per_sec` | 3.27 µs | 500 条突发入队（≈6.5 ns/条） |
| `backpressure/backpressure_100_capacity` | 73.0 µs | 容量 100 通道背压触发 |
| `backpressure/backpressure_burst_10k` | 65.9 ms | 10k 条突发全链路 |
| `concurrency/concurrent_4_threads` | 7.25 ms | 4 线程并发写 |
| `concurrency/concurrent_async_tasks` | 1.02 ms | tokio 任务并发写 |
| `memory_usage/steady_state_memory` | 4.83 µs | 稳态内存巡检路径 |
| `object_pool/pool_get_put_log_record` | 72 ns | 对象池 LogRecord 取还 |
| `object_pool/pool_get_put_string_buffer` | 3.8 ns | 对象池 String 缓冲取还 |
| `object_pool/pool_hit_rate_after_warmup` | 57 ns | 预热后池命中 |
| `object_pool/pool_vs_allocation_log_record` | 156 ns | 池化 vs 直接分配（LogRecord） |
| `object_pool/pool_vs_allocation_string` | ≈0（ps 级） | 池化 vs 直接分配（String，差异低于计时分辨率） |
| `zero_allocation/hot_path_with_pool` | 150 ns | 热路径（对象池启用） |
| `zero_allocation/hot_path_without_pool` | 96 ns | 热路径（对象池停用） |
| `zero_allocation/string_pool_reuse` | 4.1 ns | 字符串池复用 |
| `sampling_stress_overhead/no_policy_builtin_fallback` | 264 ns | 压力分支内置兜底决策 |
| `sampling_stress_overhead/policy_per_level_and_prefix` | 573 ns | 压力分支采样策略决策（增量 ≈309 ns/条） |
| `target_rate_limiter_lookup/lookup_longest_prefix_hit` | 7.6 ns | 最长前缀命中 |
| `target_rate_limiter_lookup/lookup_no_rule_fallthrough` | 4.2 ns | 未命中全表扫完 |
| `target_rate_limiter_lookup/evaluate_governed_pass` | 39 ns | 组桶裁决端到端 |

要点：

- 采样策略与 per-target 限流查找均为旁路开销（ns 级），不在 Sink 写路径上叠加；
- 突发吞吐 ≈6.5 ns/条（`throughput_burst`，500 条摊薄），背压在容量边界显性转价（`backpressure_100_capacity` 73 µs）；
- 对象池取还本身为 ns 级（LogRecord 72 ns / String 3.8 ns）；`zero_allocation` 热路径对比显示此尺度下启用池无优势（150 ns vs 96 ns，池维护开销可见）——池的价值定位在更大对象与高分配频率场景，热路径默认未池化。

## 🔍 与 0.3.0-rc.3 新增能力的对照

0.3.0-rc.3 新增能力（动态 Sink 注册、运行时级别热调、采样器、令牌桶限流、中间件链）均为 Sink 侧装饰器或旁路逻辑：

- 记录进入 Sink 前的装饰器成本落在**每 Sink 线程**上，不叠加到 subscriber 热路径；
- `Sampler::should_emit` 与 `SinkRateLimit::try_acquire` 为原子计数/CAS，无锁；
- `reload` 换装只在调用 `set_level` 时发生（事件驱动），对 `on_event` 零影响。

结论：写入主链路（加密/轮转/批量）性能不回退；`write_path/template_render_text` 与 0.3.0-rc.3 之前的模板渲染语义一致，可直接对照。

## 🔁 复现

```bash
# 快速基线（本文档口径）
cargo bench --bench rc4_pipeline_bench -- --warm-up-time 1 --measurement-time 2 --sample-size 10
cargo bench --bench inklog_bench -- --warm-up-time 1 --measurement-time 3 --sample-size 10

# 完整基线（默认参数，约 15 分钟）
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
```

### rc4_pipeline_bench 续记（2026-10-01，同环境同口径）

与 2026-09-11 基线同机复测（快速口径），全组量级一致、无回退：

| 基准 | 2026-09-11 | 2026-10-01 |
| --- | --- | --- |
| `write_path/template_render_text` | 320 ns | 311 ns |
| `write_path/mask_sensitive_fields` | 50.5 µs | 38.4 µs |
| `serialization/logrecord_to_json` | 257 ns | 259 ns |
| `serialization/logrecord_batch_100_to_json` | 26.5 µs | 26.1 µs |
| `encryption/pbkdf2_derive_600k` | 63.3 ms | 46.4 ms |
| `encryption/aes256gcm_roundtrip_1kb` | 506 ns（≈1.9 GiB/s） | 496 ns（≈1.9 GiB/s） |

（`otlp_body_100` 按 `otlp` feature 门控，本次未启用，不入本表。）

### secret_scan_bench 基线（2026-10-04，criterion 中位数，`--features secret-scan`）

出站脱敏门热路径（`benches/secret_scan_bench.rs`，WSL2 x64 / 16 线程，与其他基线同机）。全部 8 项同会话一次跑出的中位数；连续两次全量运行观测到约 ±15% 的整体漂移（机器噪声，非代码回归），绝对值请以复现为准：

| 基准 | 中位数 | 说明 |
| --- | --- | --- |
| `registry_scan_hit` | 16.2 µs | 9 模式扫描命中行（sk- + email 正文） |
| `registry_scan_clean` | 14.2 µs | 同上，干净行——扫描本身主导，与命中行差值小 |
| `gate_mask_value_shapes` | 14.7 µs | 门替换（值形态 9 模式） |
| `gate_mask_clean` | 14.2 µs | 门替换干净行（无命中零中间分配路径） |
| `gate_mask_with_entropy` | 14.7 µs | 门替换 + 熵扫描 |
| `entropy_scan_only` | 544 ns | 熵扫描 token 密集行（固定计数数组快路径；前一轮实测 464 ns） |
| `mask_fast_with_gate` | 42.2 µs | 全出站路径（门 + fast-masking + 规则集） |
| `mask_baseline_without_gate` | 31.2 µs | 无门基线；门净成本 ≈ 上行差值 |

要点：门的每行固定成本由 9 条 fancy-regex 逐模式扫描主导（clean ≈ hit），与内容是否命中几乎无关；分配侧优化（无命中零重组、计数只收非零命中、熵扫描固定数组）体现在分配次数而非常规时间项。`registry_scan_hit` 曾观测到单轮采样双峰（std_dev ≈ 29 µs），复现时建议以多次全量运行的中位区间为准。
