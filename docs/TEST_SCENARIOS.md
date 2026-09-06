# inklog 测试场景固化（TEST_SCENARIOS）

> 阶段 2 验收产物。记录测试金字塔基线、9 个测试目标的功能域落点、E2E 场景定义、
> 本轮真实行为核正发现、组合矩阵与静态门槛。验证口径全部为 `cargo test`。

## §1 测试金字塔基线

| 层级 | 承载 | 数量基线 |
| --- | --- | --- |
| L1 lib 单元测试 | `src/**` 内 `#[cfg(test)]` | 1009 passed（manager/workers/sink/config/i18n 等模块自测） |
| L2 集成测试 | `tests/**`（4 个 `[[test]]` 显式注册 + 4 个顶层自动发现） | integration 95+1i / unit_tests 63 / combinations 20 / performance 11+3i / integration_tests 34+1i / docker 25 / cli_integration 9 |
| L3 E2E 场景 | `tests/e2e/e2e_advanced.rs`（目录承载，不裸放顶层） | 226 passed，按 mod 域隔离 |
| L4 容器级 | `tests/docker/`（main.rs 自动发现） | 25 passed（sqlite embedded 路径；pg/mysql 用例门控至对应驱动组） |

CI 主口径（`--features "sqlite http cli kit compression parquet fast-masking test-utils"`）全量结果：
**lib 1009 / unit_tests 63 / integration 95+1i / combinations 20 / performance 11+3i / integration_tests 34+1i / docker 25 / cli_integration 9 / e2e_advanced 226 — 0 failed**。

## §2 测试目标注册与功能域落点

- `[[test]]` 显式注册（此前深目录聚合无编译入口，从未被 cargo test 运行）：
  - **integration**（required-features = sqlite,http,cli,compression,parquet,test-utils）：additional/comprehensive（全 sink 组合与动态配置）/ batch（批量写）/ verification（file_sink：压缩+加密归档验证）/ log（log crate 原生 + tracing 双路径）/ config（env 覆盖）/ http（服务器生命周期）/ cli（inklog-cli 二进制）/ parquet（导出）/ compression_ratio / recovery（自动恢复）/ stability（long_running，manual）
  - **combinations**（required-features = sqlite）：encryption_file（加密文件集成+密钥管理）/ multi_sink_fallback（SinkHealthMonitor 状态机+降级）/ complex_features（加密+压缩+数据库三特性叠加）
  - **performance**：benchmark（单线程/多线程/延迟/批量/池化 ×7）/ large_volume（1GB 压缩轮转）/ long_running + resource_monitor（内存/CPU 采样，manual）
  - **e2e_advanced**：见 §3
- 顶层自动发现：**unit_tests**（mod unit：config/sink/cli/archive 分层单测）/ **integration_tests** / **cli_integration**（assert_cmd 驱动 inklog-cli）/ **docker**（database_lifecycle/database_sink/dbnexus_adapter）
- e2e 目录承载：e2e_* 文件必须位于 `tests/e2e/`（本轮从顶层迁入并显式注册）

## §3 E2E 场景定义（tests/e2e/e2e_advanced.rs，226 测试 ×15 mod 域）

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

## §4 真实行为核正与发现（阶段 2）

1. **shutdown 死锁（库缺陷修复，manager.rs send_timeout）**：worker（spawn_blocking）内
   `block_on(sink.write)` 依赖调用方 runtime 的驱动线程推进（sqlx pool 任务 spawn 在创建时
   runtime 上）；drop→shutdown 路径下驱动线程自身阻塞在 `shutdown_txs.send`，write 永不完成
   → worker 不回循环消费信号 → channel 满 → send 永久阻塞，死锁环（gdb 现场：测试线程卡
   crossbeam array::send，db worker 卡 CachedParkThread::park）。修复为 `send_timeout(2s)`：
   超时后继续 handles 轮询，驱动恢复后 worker 自行消费信号正常退出。integration 复验
   ×5（含串行 --test-threads=1，此前必卡）全绿。
2. **LoggerSubscriber 补手动 Clone（库改动）**：字段含 AtomicU64（std 无 derive Clone），
   手写 impl（error_sample_counter 以 Relaxed load 快照）。用途：测试 harness 子线程按
   thread-local 语义安装 subscriber（tracing with_default 不跨线程继承）。
3. **runtime-in-runtime 修复（库改动）**：默认 DbNexusAdapter 创建从 start_workers（可能运行
   于 runtime 线程，Handle::current().block_on 必 panic）上移至 build_detached（async 上下文）。
4. **with_config 全局单次语义**：set_global_default 进程级一次不可更换，多用例并发下后装者
   日志流向首个 logger——测试统一口径为 build_detached + 线程级 set_default + tracing::!
   （additional_tests.rs 范本）；performance/integration/comprehensive 全部随之改造。
5. **FileSink 弱密钥校验**：get_encryption_key 要求 base64 解码恰 32 字节且 Shannon 熵 ≥4.0
   （"12345678901234567890123456789012" 熵 3.31 被拒且后台 error! 无 logger 被吞 → .enc 不产出）。
   测试密钥换高熵 "YVozeFc4dksybVE3dE41clU5eUI0Y0U2ZkgxZ0owZEw="（熵 5.0）。
6. **encrypt 仅作用于轮转归档**：活跃文件明文是设计（file.rs rotate_inner：compress→.zst[.enc]，
   encrypt→.enc 后台加密）。comprehensive 断言按此核正（活跃文件含明文 + 归档不含）。
7. **LoggerBuilder validation_errors 延迟校验**：非法 level 在 build() 时统一报 ConfigError
   （builder 返回 Result），test_manager_invalid_level 断言 is_err 核正。
8. **Adaptive 扩缩容瞬态**：扩容后 channel 排空 shrink_wait（1s）即收缩——轮询验证需 20ms
   峰值采样 + 观察到扩容即 break，断言 peak_capacity ≥ initial。
9. **log 宏唯一入口是进程级全局 logger**（LogLogger::install 首次绑定不可更换）：cargo test
   并发下无法定向绑定，test_log_to_file 拆为 log 宏 smoke（不验证内容）+ tracing 路径真实落盘
   断言；6 个 smoke 用例 assert!(true) 改为显式 shutdown（并修正 build()/new() 返回 Result
   的 unwrap 缺失——原代码绑定 Result 从未展开，恒真断言掩盖）。
10. **combinations 三文件修复**：encryption_file 裁撤已移除的 Encryptor/EncryptionKey 原语用例
    （现公开面为密钥派生 + FileSink 加密落盘）；multi_sink_fallback 按 SinkHealthMonitor 真实
    API 重写（FallbackState 已重构为 enum，FallbackConfig.auto_fallback→enabled）；complex
    unsafe 段删除。
11. **clippy 静态门槛清理 ×19**：module_inception ×6（测试文件内嵌同名 mod 去 _test 后缀；
    stability/verification 目录承载文件改名 long_running.rs/file_sink.rs 消除同名嵌套）+
    approx_constant（3.14→std::f64::consts::PI）+ 恒真断言 ×6 + is_multiple_of ×3 +
    trim→split_whitespace ×2 + 冗余 struct update ×1。
12. **CI test-utils 缺口**：integration 目标 required-features 含 test-utils，CI 口径缺它时
    cargo 静默跳过该目标（不报错不执行）——ci.yml 的 test/clippy 双 job 补齐并注释。

## §5 组合矩阵

| 组合 | 覆盖 | 结果 |
| --- | --- | --- |
| sqlite+http+cli+kit+compression+parquet+fast-masking+test-utils（CI 主口径） | 全 9 目标 | 1472 passed / 0 failed / 5 ignored |
| 单独 performance 复跑 ×2 | performance 目标稳定复验 | 11+3i ×2 全绿 |
| integration 串行（--test-threads=1） | 死锁修复回归（此前必卡） | 95+1i 全绿（31.81s） |
| 默认 features（无 db） | lib/e2e_advanced/performance 可编译 | 编译+运行通过 |
| 驱动互斥 | sqlite/postgres/mysql/duckdb 互斥（dbnexus 禁混合） | 不适用 --all-features |

## §6 静态门槛

| 门槛 | 命令口径 | 结果 |
| --- | --- | --- |
| fmt | `cargo fmt --all -- --check` | 净 |
| clippy | `cargo clippy --all-targets --features "sqlite http cli kit compression parquet fast-masking test-utils" -- -D warnings` | 零告警 |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "sqlite http cli kit compression parquet fast-masking"` | 零告警 |
| deny | `cargo deny check` | 4 项 ok（licenses 经 clarify 绑定 LICENSE hash：inklog/oxcache/oxcache_macros/trait-kit/dbnexus） |
| audit | `cargo audit` | rc=0（514 crate 无命中） |
| MSRV | rust-version = 1.97.1（workspace 统一，CI dtolnay/rust-toolchain@1.97.1） | 一致 |
