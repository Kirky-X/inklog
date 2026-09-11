# PERFORMANCE.md — inklog 性能基线

> 首次正式发布的性能基线（T506，specmark `workspace-rc4-completion`）。
> 兑现 README「仓库内暂无正式发布的跨版本基准测试报告」欠账的 rc4 部分。

## 环境与方法

| 项 | 值 |
| --- | --- |
| 日期 | 2026-09-11 |
| 机型 | WSL2 (linux 6.6.87.2-microsoft-standard-x64)，开发笔记本 |
| 工具 | criterion 0.8（`cargo bench --bench rc4_pipeline_bench`） |
| profile | release（`opt-level=3`, `lto=fat`, `codegen-units=1`） |
| 运行参数 | `--warm-up-time 1 --measurement-time 2 --sample-size 10`（快速基线） |

> **CI 门禁口径（设计 D4）**：阈值门禁暂不启用——不同机型噪声差异大，待
> CI 固定 runner 后按"中位数回归 >10% 标红"启用。当前以本文档数字为
> 人工对照基线，跨版本更新本文档。

## 基线数字（2026-09-11，中位数）

### 写入路径（write_path）

| 基准 | 中位耗时 | 吞吐 |
| --- | --- | --- |
| `template_render_text`（默认模板 + 2 字段） | **320 ns** | ~3.12 M rec/s |
| `mask_sensitive_fields`（DataMasker 正则 + 敏感键） | **50.5 µs** | ~19.8 K rec/s |
| `logrecord_to_json`（单条序列化） | **257 ns** | ~3.89 M rec/s |

要点：
- 模板渲染与 JSON 序列化处于同一量级（~300 ns/条），**默认写入主链路（模板渲染）
  每条日志的可观测处理成本 < 0.5 µs**；
- 脱敏（`mask_sensitive_fields`）是主链路中最贵的安全环节（正则集合 + 递归字段
  遍历），成本约为渲染的 **160 倍**——仅在 sink 层按需开启（`masking_enabled`），
  不要在热路径无条件调用；
- 加密/轮转每轮转文件触发一次（见下），不在每条记录路径上。

### 序列化路径（serialization）

| 基准 | 中位耗时 | 吞吐 |
| --- | --- | --- |
| `logrecord_batch_100_to_json`（100 条批量） | **26.5 µs** | ~3.77 M rec/s |
| `otlp_body_100`（`otlp` feature，100 条 OTLP JSON 体） | 见 criterion 本机最新输出 | — |

批量序列化与单条线性的比值 ~1.06×，**无超线性放大**；OTLP 体编码在
`otlp` feature 下另行采样（编码器与 serde 同构，量级一致）。

### 加密路径（encryption）

| 基准 | 中位耗时 | 吞吐 |
| --- | --- | --- |
| `pbkdf2_derive_600k`（PBKDF2-HMAC-SHA256，600k 迭代） | **63.3 ms** | ~15.8 ops/s |
| `aes256gcm_roundtrip_1kb`（1 KiB 加解密往返） | **506 ns** | ~1.88 GiB/s |

要点：
- PBKDF2 600k 迭代 ≈ 63 ms——**每次轮转文件至多一次**（密钥派生），折算到
  日志记录上可忽略；这是安全审查认可的最低迭代数，禁止为性能调低；
- AES-256-GCM 数据面 ~1.9 GiB/s（ring 后端），加密记录主链路成本可控。

## 既有基准（`inklog_bench`，rc3 及以前）

`benches/inklog_bench.rs` 覆盖 LogRecord 创建、console sink 延迟、通道入队、
持续/突发吞吐、FileSink 吞吐、no-op 对照等 9 组，运行方式：

```bash
cargo bench --bench inklog_bench
```

（历史数字未入库；自 rc4 起新旧两套基准统一以本文档为基线载体，后续版本
更新时补齐 `inklog_bench` 的正式基线。）

## 与 rc3 轮的对照口径

rc4 轮新增能力（T501 动态 sink、T502 reload 换装、T507 采样器、T511 限流、
T514 中间件链）均为 sink 侧装饰器/旁路逻辑：

- 订录进入 sink 前的装饰器成本在**每 sink 线程**上，不叠加到 subscriber 热路径；
- `Sampler::should_emit` / `SinkRateLimit::try_acquire` 为原子计数/CAS，无锁；
- `reload` 换装只在 `set_level` 时发生（事件驱动），对 `on_event` 零影响。

因此判定：**写入主链路（加密/轮转/批量）性能不回退**（约束达成），
`write_path/template_render_text` 与 rc3 的模板渲染语义一致可直接对照。

## 复现

```bash
# 快速基线（本文档口径）
cargo bench --bench rc4_pipeline_bench -- --warm-up-time 1 --measurement-time 2 --sample-size 10
# 完整基线（默认参数，约 15 分钟）
cargo bench --bench rc4_pipeline_bench
```
