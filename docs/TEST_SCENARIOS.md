# 🧪 inklog 测试场景

本文档固化 inklog 的测试金字塔基线、测试目标注册表、E2E 场景定义、测试编写要点、组合矩阵与静态门槛。验证口径全部为 `cargo test`。

> 相关文档：[📖 用户指南](USER_GUIDE.md) · [🏗️ 架构设计](ARCHITECTURE.md) · [📊 性能基线](PERFORMANCE.md) · [🤝 贡献指南](CONTRIBUTING.md)

## 🗼 测试金字塔基线

| 层级 | 承载 | 数量基线 |
| --- | --- | --- |
| L1 lib 单元测试 | `src/**` 内 `#[cfg(test)]` | 1009 passed（manager/workers/sink/config/i18n 等模块自测） |
| L2 集成测试 | `tests/**`（4 个 `[[test]]` 显式注册 + 4 个顶层自动发现） | integration 95+1i / unit_tests 63 / combinations 20 / performance 11+3i / integration_tests 34+1i / docker 25 / cli_integration 9 |
| L3 E2E 场景 | `tests/e2e/e2e_advanced.rs`（目录承载，不裸放顶层） | 226 passed，按 mod 域隔离 |
| L4 容器级 | `tests/docker/`（main.rs 自动发现） | 25 passed（sqlite embedded 路径；pg/mysql 用例门控至对应驱动组） |

CI 主口径（`--features "sqlite http cli kit compression parquet fast-masking test-utils"`）全量结果：**lib 1009 / unit_tests 63 / integration 95+1i / combinations 20 / performance 11+3i / integration_tests 34+1i / docker 25 / cli_integration 9 / e2e_advanced 226，0 failed**。

> **口径说明**：上表为对应 feature 组合下 `cargo test` 的实际执行数；按测试函数静态统计的当前总规模口径见 [README](../README.md) 测试章节（v0.3.0-rc.3：src 内联 1,255 + tests 目录 528 = 1,783 个测试函数）。

## 🗂️ 测试目标注册与功能域落点

`[[test]]` 显式注册（深目录聚合的测试必须显式注册，否则 cargo 没有编译入口、不会执行）：

- **integration**（required-features = sqlite,http,cli,compression,parquet,test-utils）：additional/comprehensive（全 sink 组合与动态配置）/ batch（批量写）/ verification（file_sink：压缩+加密归档验证）/ log（log crate 原生 + tracing 双路径）/ config（env 覆盖）/ http（服务器生命周期）/ cli（inklog-cli 二进制）/ parquet（导出）/ compression_ratio / recovery（自动恢复）/ stability（long_running，manual）
- **combinations**（required-features = sqlite）：encryption_file（加密文件集成+密钥管理）/ multi_sink_fallback（SinkHealthMonitor 状态机+降级）/ complex_features（加密+压缩+数据库三特性叠加）
- **performance**：benchmark（单线程/多线程/延迟/批量/池化 ×7）/ large_volume（1GB 压缩轮转）/ long_running + resource_monitor（内存/CPU 采样，manual）
- **e2e_advanced**：见下节

顶层自动发现：**unit_tests**（mod unit：config/sink/cli/archive 分层单测）/ **integration_tests** / **cli_integration**（assert_cmd 驱动 inklog-cli）/ **docker**（database_lifecycle/database_sink/dbnexus_adapter）。

目录承载约定：e2e_* 文件必须位于 `tests/e2e/`；`required-features` 未满足时 cargo 会静默跳过目标（不报错不执行），因此 CI feature 组合必须覆盖各目标的 required-features（含 `test-utils`）。

## 🎬 E2E 场景定义（tests/e2e/e2e_advanced.rs，226 测试 × 15 mod 域）

| 域 | 场景要点 |
| --- | --- |
| log_level_e2e | FromStr 全变体回环/非法输入拒/short_str/Default=info |
| console_sink_e2e | stdout 缓冲写入/stderr 级别路由/掩码开启/shutdown 健康度 |
| rotation_strategy_e2e | 尺寸轮转触发/未触发/精确边界/尺寸串解析/next_path；时间轮转间隔/should_rotate；复合策略任一触发 |
| sink_registry_e2e | Sink 注册/查找/默认 sink 语义 |
| security_e2e | PathValidator 路径校验/LogSanitizer 清洗/CircuitBreaker 联动 |
| data_masker_e2e | DataMasker 手机号/邮箱/身份证/自定义模式/边界 |
| log_record_masking_e2e | LogRecord 级掩码管线（字段级生效） |
| inklog_error_e2e | InklogError 全变体 Display/分类 |
| log_template_e2e | LogTemplate 渲染/变量替换/缺省 |
| circuit_breaker_e2e | 熔断开/半开/关状态迁移 |
| object_pool_e2e | 对象池借还/耗尽/回收 |
| config_validation_e2e | Config 非法值校验矩阵 |
| sink_health_monitor_e2e | 健康度采集/不健康判定/恢复 |
| metrics_e2e | Metrics 计数/延迟记录/sink health |
| file_sink_e2e | FileSink 写入/轮转/flush 落盘 + tracing→FileSink、log→FileSink 多组件集成 |

## 📌 测试编写要点

以下为测试编写与断言必须遵循的真实运行时行为（测试基础设施修复过程中固化，均为库的当前语义）：

1. **shutdown 使用 2 秒超时发送**：worker 运行于 `spawn_blocking`，其内部写入依赖调用方 runtime 的驱动线程推进；`shutdown` 对停止信号采用 `send_timeout(2s)`，超时后继续轮询 worker handles，保证永不挂死。测试中的 shutdown 断言需允许驱动线程参与。
2. **LoggerSubscriber 需手动实现 Clone**：字段含 `AtomicU64`（std 不派生 Clone）；测试 harness 在子线程按 thread-local 语义安装 subscriber（`tracing::with_default` 不跨线程继承）。
3. **runtime-in-runtime 规避**：默认 `DbNexusAdapter` 的创建位于 `build_detached`（async 上下文），不在 `start_workers`（可能运行于 runtime 线程，`Handle::current().block_on` 会 panic）中创建。
4. **全局 subscriber 单次安装语义**：`set_global_default` 进程级一次不可更换；多用例并发下统一口径为 `build_detached` + 线程级 `set_default` + `tracing::!`（`additional_tests.rs` 为范本）。
5. **FileSink 弱密钥校验**：加密密钥要求 base64 解码恰 32 字节且 Shannon 熵 ≥ 4.0（低熵串如 32 个重复数字会被拒绝且不产出 `.enc` 文件）。测试统一使用高熵密钥。
6. **加密仅作用于轮转归档**：活跃日志文件保持明文是设计行为（`rotate_inner` 先 compress 得 `.zst`、后台 encrypt 得 `.enc`）；comprehensive 断言据此核正（活跃文件含明文，归档不含）。
7. **LoggerBuilder 延迟校验**：非法 level 等配置问题在 `build()` 时统一返回 `ConfigError`（builder 返回 `Result`）。
8. **Adaptive 通道扩缩容存在瞬态**：扩容后排空等待（`shrink_wait_seconds`）即收缩；轮询断言需 20ms 级峰值采样，观察到扩容即 break，断言 `peak_capacity ≥ initial`。
9. **log 宏入口是进程级全局 logger**（`LogLogger::install` 首次绑定不可更换）：并发测试无法定向绑定，log 宏路径只做 smoke 验证，真实落盘断言走 tracing 路径。

## 🧮 组合矩阵

| 组合 | 覆盖 | 结果 |
| --- | --- | --- |
| sqlite+http+cli+kit+compression+parquet+fast-masking+test-utils（CI 主口径） | 全 9 目标 | 1472 passed / 0 failed / 5 ignored |
| 单独 performance 复跑 ×2 | performance 目标稳定复验 | 11+3i ×2 全绿 |
| integration 串行（--test-threads=1） | shutdown 回归验证 | 95+1i 全绿（31.81s） |
| 默认 features（无 db） | lib/e2e_advanced/performance 可编译 | 编译+运行通过 |
| 驱动互斥 | sqlite/postgres/mysql/duckdb 互斥（dbnexus 禁混合） | 不适用 --all-features |

## 🚧 静态门槛

| 门槛 | 命令口径 | 结果 |
| --- | --- | --- |
| fmt | `cargo fmt --all -- --check` | 净 |
| clippy | `cargo clippy --all-targets --features "sqlite http cli kit compression parquet fast-masking test-utils" -- -D warnings` | 零告警 |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "sqlite http cli kit compression parquet fast-masking"` | 零告警 |
| deny | `cargo deny check` | 4 项 ok（licenses 经 clarify 绑定 LICENSE hash：inklog/oxcache/oxcache_macros/trait-kit/dbnexus） |
| audit | `cargo audit` | rc=0（514 crate 无命中） |
| MSRV | rust-version = 1.97.1（workspace 统一，CI dtolnay/rust-toolchain@1.97.1） | 一致 |
