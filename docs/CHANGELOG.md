# 更新日志

本项目的所有显著变更将记录在此文件中。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<details open>
<summary>📑 目录</summary>

- [Unreleased](#unreleased)
- [0.3.0-rc.6](#030-rc6---2026-09-28)
- [0.3.0-rc.5](#030-rc5---2026-09-21)
- [0.3.0-rc.4](#030-rc4---2026-09-14)
- [0.3.0-rc.2](#030-rc2---2026-09-03)
- [0.2.0](#020---2026-08-05)
- [0.1.12](#0112---2026-07-22)
- [0.1.11](#0111---2026-07-17)
- [0.1.7](#017---2026-07-13)
- [0.1.6](#016---2026-07-12)
- [0.1.5](#015---2026-07-11)
- [0.1.2](#012---2026-07-05)
- [0.1.1](#011---2026-06-29)
- [0.1.0](#010---2026-01-18)
- [0.0.0](#000---2025-12-30)

</details>

## [Unreleased]

### Added

- **per-target 分级限流**：`rate_limit.rules`（target 前缀 → 该组每秒令牌数，最长前缀优先、ASCII 大小写不敏感）配置按前缀分组的配额限流；命中组由组桶独立裁决（放行后不再进入 `performance.rate_limit` 全局限流），未命中 target 维持既有全局路径；组预算耗尽与全局限流拒绝共用同一关键级别救援（ERROR/FATAL 按 1-in-N 采样保留），非关键级别计为 `logs_dropped`；加载期校验（前缀非空、速率 ≥ 1），未配置时整体不接线、默认全局行为不变；查找路径为无分配线性扫描（基准 `target_rate_limiter_lookup`：最长前缀命中 ≈6ns、全表未命中 ≈4ns、含组桶裁决 ≈41ns）
- **可配采样策略**：`InklogConfig.sampling`（`SamplingConfig`：`per_level` 每级别采样率 + `per_target_prefix` 按 target 前缀规则，最长前缀优先、大小写不敏感）配置限流压力下的保留规则，加载期校验（级别合法、采样率 ≥ 1、前缀非空）；规则未命中或未配置时回退内置兜底（非关键级别丢弃、ERROR/FATAL 保留 1/100），行为与既有语义一致
- **采样细分指标**：新增 `inklog_sampled_out_total`（`Metrics::sampled_out()`），仅统计采样决策淘汰的记录（Subscriber 压力路径采样与 `SamplingSink` 淘汰），限流非采样丢弃与通道满丢弃不计入；`MetricsSnapshot` 同步携带 `sampled_out`

## [0.3.0-rc.6] - 2026-09-28

### Added

- **全出口 PII 掩码覆盖**：net 转发、OTLP 导出、`ChannelBufferedFileSink` 三个出口此前绕过 DataMasker（仅上游弱版 LogSanitizer 兜底），现统一接入 21 条规则掩码（`masking_enabled` 门控，默认开启）；DB sink 改键值遍历式掩码——`password` 等敏感键整值替换，不再依赖值正则的偶然命中
- **Sink masker 注入点**：console/file/net/otlp/ChannelBufferedFileSink 新增 `with_masker()`，支持经 `DataMasker::builder()` 注入自定义规则（`fast-masking` feature 下 literal 规则自动走 Aho-Corasick 加速路径，接线从死代码转为可用）
- **脱敏正则修复**：phone/id_card/bank_card/ssn 等数字类规则以负向断言替代 `\b`（CJK 汉字紧贴数字不再漏报）；身份证支持小写 `x` 校验位；bank_card 收紧为 16–19 位（13 位毫秒时间戳不再误伤）；passport 排除混合 hex 形态（git 短 SHA 不再误掩）；脱敏递归深度 16 层上限（深嵌套 JSON 防栈溢出）、单条输入 1 MiB 上限；JSON Number/Bool 形态的敏感值参与掩码
- **ERROR/FATAL 兜底运行期补发**：async 通道打满时进入兜底缓冲的关键日志，在通道恢复半满时立即补发，并有 60s 周期任务兜底——不再依赖进程退出时的 Drop drain
- **崩溃一致性**：`FileSinkConfig.fsync`（默认 false）每批写盘后 `sync_all`；空闲期批量缓冲由轮转定时线程代为落盘（不再依赖"下一条写入"触发）
- **文件权限加固**：FileSink 活动日志、轮转加密产物（`.enc`）、压缩产物（`.zst`/`.gz`）以 0600 创建；新建日志目录 0700（已存在目录不回改）
- **留存合规**：年龄清理与大小清理拆为独立阶段（`max_total_size` 不可解析时过期清理仍执行）；`encrypt=true` 且 `retention_days < 180` 输出等保 2.0 留存提示
- **审计链接入轮转管线**：`audit_chain_enabled = true` 后每次轮转向 `<stem>.chain.jsonl` 追加 `{path, sha256, timestamp}` 条目；新增 `inklog-cli verify-chain` 子命令（链完整退出码 0、篡改退出码 2，密钥读 `INKLOG_AUDIT_KEY`）
- **log 门面补齐**：桥接当前 tracing span 的 trace_id/span_id（与 tracing 路径共享派生逻辑）；per-target 级别过滤（共享 manager 指令集，`RUST_LOG`/`target_levels` 对 log 来源生效）
- **OTLP 导出补链路字段**：编码输出 `traceId`/`spanId`/`severityNumber`（标准映射 DEBUG=5/INFO=9/WARN=13/ERROR=17/FATAL=21）；https endpoint 经 rustls 传输（`net-sink` feature，CA PEM 或显式跳过校验二选一；feature 未启用时构造期报错）
- **net 重连指数退避**：connect 失败按 `min(2^n × 100ms, 30s)` 退避，窗口内 write 直接入缓冲不再发起连接，成功清零
- **解压炸弹防护**：查询与内存解压路径施加 1 GiB 输出上限（流式读取，超限报 i18n 错误）
- **内部故障 ops 事件**：轮转 rename 失败、轮转产物压缩/加密失败、sink 降级与恢复路径自动发布 `sink_degraded`/`sink_recovered` 事件（全局 hub 复用 manager ops 通道）
- **服务身份静态字段注入**：`GlobalConfig` 新增 `service_name`/`service_instance`/`service_env`/`service_version` 与 `static_fields`（附加键值）；配置后由 subscriber 在记录入通道前注入每条日志的 `fields`（键名同名，全部出口一致，事件显式同名字段优先），未配置时热路径零开销；支持 TOML（`[global]`）、环境变量（`INKLOG_GLOBAL_SERVICE_*`、`INKLOG_GLOBAL_STATIC_FIELDS=k=v,...`，沿用既有前缀语义）与 confers/DI 键路径（`global.service_*`）；三条配置链统一身份校验：TOML 由 `InklogConfig::validate` 硬拒绝、env 覆盖点对空白值/控制字符告警忽略、DI 装配链拒绝构建；adapter 键路径对未设置的身份字段透传 None（不产生空串注入）
- **deferred-capabilities 四项遗留能力**：中文姓名掩码（`NAME_FIELD_PATTERNS` 姓名族键词 + 2-4 汉字值形态，保留首字脱敏）；磁盘持久化 fallback 队列 `FallbackJournal`（JSONL 10MiB 上限丢最旧、启动重放防循环、`global.fallback_journal` 默认关）；ChannelBufferedFileSink 转正为主 file sink 路径（简单配置走 CBFS，高级配置回落 FileSink，USER_GUIDE 双路径能力矩阵）；真 OTel 链路上下文（新 `otel` feature，opentelemetry 0.30，span extensions → W3C traceparent → 根 span 派生三级提取，版本未对齐静默回退）
- **masking API 扩展**：detect-only 检测面（`MaskMatch`/`detect`/`has_match` 与 pattern 访问器）；`MaskRuleRegistry` 全量快照 `rules()` 与构造器 `from_rules()`；行级 KV 脱敏 `DataMasker::mask_kv_lines`

### Changed

- **脱敏规则引擎切换 fancy-regex**：正则环视（lookbehind/lookahead）是 CJK 友好边界的前提，`regex` crate 不支持；新增 `fancy-regex` 依赖（规则热路径行为不变，编译期 expect 守卫）
- **FileSink 写路径**：文件句柄改 64 KiB `BufWriter`（每批一次 flush 摊销逐条 syscall）；`inner` 状态升级 `Arc<RwLock<>>` 供定时线程执行空闲 flush；批末 flush 失败整批回填重试（at-least-once）
- **对象池闭环**：worker 消费端 `Arc::try_unwrap` 成功即归还 `LogRecord`（池此前只取不还，零分配目标落空）
- **`set_level` 双门面同步**：运行时热调级别时同步 `log::set_max_level`（此前 log 门面被旧级别拦截）
- **行为变更：console sink `enabled=false` 不再输出**：`ConsoleSink::write` 此前不检查 `enabled` 字段，显式禁用组合仍全量写 stdout/stderr；现禁用态在 write 处丢弃记录（默认组合 `enabled=true` 行为逐字节不变）。该语义落实使性能基准等声明禁用 console 的场景不再向 stdout 泄漏海量输出
- **CI 数据库后端矩阵补齐 duckdb**：docker 数据库流水线分组矩阵扩为 sqlite/postgres/mysql/duckdb 四后端（互斥 feature 逐一验证）；duckdb 为 embedded 后端不起 compose 服务，经 `duckdb:///` 文件库直跑集成测试；`tests/docker` 测试目标 crate cfg 同步纳入 duckdb（此前该 feature 下整个目标被 cfg 掉，矩阵项只会空跑）。随真跑暴露并修复 duckdb 链路两处缺陷：`DbNexusAdapter` 建表此前走 SeaORM 通道（DuckDB 连接直接报错），现按驱动分派至 `execute_duckdb_raw`；DuckDB 建表 DDL 去除自增 id 列（duckdb-rs 绑定对 AUTOINCREMENT/IDENTITY 约束报 "Constraint not implemented"，行标识由隐式 rowid 提供）；评审收尾：DuckDB DDL 分派显式限定 CREATE TABLE 模板（`execute_duckdb_raw` 的 DdlGuard 仅对可识别 DDL 关键字设防，其余语句会以 admin 绕过 guard 策略），并清理 docker 测试遗留的死辅助 DDL 代码
- **i18n feature 化**：icu/fluent 依赖树可裁剪（不启用 i18n feature 的消费方不再承担其编译成本），default 行为零变化
- **console 着色构造期缓存**：着色判定构造期缓存与禁用分支零克隆
- **error sink 延迟创建**：error sink 延迟到首次 error 写入时才创建文件
- **ChannelBufferedFileSink 落盘格式**：缓冲落盘记录间补换行分隔

---

## [0.3.0-rc.5] - 2026-09-21

### 新增

- **通用业务指标 registry**：Prometheus 文本导出（收编并行会话改动）

### 变更

- **i18n 整改**：统一错误与消息文案管理并清理零引用死键
- **特性守卫**：补四数据库后端互斥守卫，引入 database/zstd 谓词收敛
- **依赖与发布**：跨仓 path 依赖改走 crates.io；rustls 升 0.23.45；为 path-only 依赖补全 version 字段；release 工作流 publish 步骤幂等容错
- **工程加固**：detect-secrets 基线、pre-commit 门禁、typos 白名单、CI examples serde 依赖修复

### 修复

- **logger-install**：依赖注入构建路径补装全局 tracing/log 前端
- **dbnexus 依赖特性**：补 failover/replica-routing

---

## [0.3.0-rc.4] - 2026-09-14

> 注：0.3.0-rc.3 未单独发布（无 tag、未上 crates.io），本节内容含原 rc.3 开发批次，随 0.3.0-rc.4 一并发布。

### Added

- **可配置日志门面**：新增 `init_inklog_logger()` 与 `init_inklog_logger_with_config(config)` 便捷初始化函数，进程级单例语义（重复初始化返回明确错误）
- **Per-crate target 级别预设**：`InklogConfig` 新增 `target_levels: HashMap<String, String>` 字段，按 crate target 设置独立日志级别，合并到 EnvFilter（优先级低于 RUST_LOG、高于全局默认）
- **dbnexus 池指标接入**：`DbNexusAdapter` 新增 `pool_status()` 透传连接池状态快照；`Metrics` 新增 `record_pool_metrics(total, active, idle)` 方法及对应 Prometheus 导出（`inklog_db_pool_total/active/idle`）
- **动态 Sink 注册**：`LoggerBuilder::add_sink(Arc<dyn AsyncSink>)` + 通用 SinkWorker——第三方 Sink 实现 `LogSink` 即可零核心改动接入，每 sink 独立通道互不抢占；`AsyncSink` 为 `LogSink` 子 trait（blanket impl，trait 上转型直通）
- **运行时级别热调**：`LoggerManager::set_level(target, level)` 经 `tracing_subscriber::reload` 换装 EnvFilter，进程内即时生效；指令集支持全局/per-target upsert，RUST_LOG 附加指令跨重建保留
- **dbnexus AuditStorage 适配器（`dbnexus-audit` feature）**：`InklogAuditStorage` 实现 dbnexus 审计存储端口——审计事件 → 结构化日志（`audit::<entity_type>`）→ inklog DB 落库
- **追踪 ID 关联**：`LogRecord` 新增 `trace_id`/`span_id`（从当前 tracing span 提取；无 OTel 时沿 parent 链以根 span id 派生 32/16 位 hex），模板新增 `{trace_id}`/`{span_id}` 占位符，serde 兼容旧数据
- **日志查询 CLI**：`inklog-cli query` 按时间/级别/关键词检索本地日志（含 `ENCLOG1` v1/v2 解密与 zstd/gzip 解包，目录递归），`--json` 输出，退出码 0/2/1
- **性能基线门禁**：新增 `rc4_pipeline_bench`（写入/序列化/加密路径）与 `docs/PERFORMANCE.md` 首份正式基线
- **日志采样器**：`Sampler`（级别阈值 + N 取 1 + 关键词白名单豁免）与 `SamplingSink` 装饰器
- **confers 配置集成（`config-confers` feature）**：`InklogConfig` 经 confers 加载 + `ConfersConfigWatcher` watch 热更新级别/轮转参数（非法 TOML 保持旧配置）
- **KMS 密钥提供者（`kms` feature）**：`KeyProvider` 端口 + `EnvKeyProvider`/`ConfersKeyProvider`（confers AsyncKeyProvider 适配）+ Vault transit MVP（mock server 测试）
- **按目标限流端口**：`SinkRateLimit`（对象安全）+ `NoOpRateLimit` 默认 + `TokenBucketRateLimit` 基线 + `RateLimitedSink` 装饰器——供 limiteron 上层实现
- **内部审计事件流 + 归档防篡改链**：`publish_ops_event` 广播 ops 事件到全部 sink 通道；`ArchiveChain` 归档 HMAC-SHA256 链（随机链首盐，防篡改/删除/重排/伪造）
- **网络转发 Sink（`net-sink` feature）**：`TcpSink`（可 TLS，rustls 客户端）+ `UdpSink`（NDJSON），断线缓冲 + 半开探测 + 自动重连按序补发
- **直方图/中间件/参数化批量/OTLP**：Prometheus 原生 `inklog_write_latency_us` histogram 导出；`MiddlewareChain`/`MiddlewareSink`（filter/transform 组合）；DuckDB 批量插入参数化（预编译语句绑定，消除转义拼接）；`otlp` feature OTLP/HTTP JSON 导出 MVP（mock collector 测试）

### Changed

- **kit feature 显式包含 `trait-kit/observer`**：消除编译定时炸弹（下游项目不再依赖传递依赖意外启用 observer）
- **oxcache 死使能清理**：移除零消费的 `macros`/`serialization`/`metrics`/`batch` feature，仅保留 `memory`
- **dbnexus 死使能清理**：移除 `failover`/`replica-routing` feature（`with_full_config` 硬编码 `None`，无行为）
- 依赖升级：dbnexus → 0.6.0-rc.3、oxcache → 0.5.0-rc.4、trait-kit → 0.5.0-rc.3
- 新增依赖：confers 0.6.0-rc.3（optional，`config-confers`/`kms` feature）+ `[patch.crates-io]` 本地路径

---

## [0.3.0-rc.2] - 2026-09-03

### ⚠️ BREAKING CHANGES

- **版本号升级**: 0.2.0 → 0.3.0-rc.2，反映 trait-kit 0.5.0-rc.2 集成与 i18n 重构

### Changed

- **依赖升级**: trait-kit 0.4 → 0.5.0-rc.2
- **S3 归档功能移除**: 文档中清理 S3 归档相关描述（0.1.1 后已统一走本地/对象存储抽象层）

### Documentation

- 同步 docs/ 下版本号至 0.3.0-rc.2
- 同步 src/lib.rs html_root_url 至 0.3.0-rc.2
- 同步 ARCHITECTURE.md 文档版本至 3.0
- uat.md: MSRV 1.70 → 1.94

## [0.2.0] - 2026-08-05

### ⚠️ BREAKING CHANGES

- **版本号升级**: 0.1.12 → 0.2.0，反映项目成熟度和 API 稳定性提升
- **依赖升级**: dbnexus 0.4 → 0.5, sea-orm 保持 2.0, oxcache 0.3 → 0.4, thiserror 1.x → 2.0, axum 0.6 → 0.8, trait-kit 0.3 → 0.4

### Added

- **compression feature**: 可选 ZSTD 压缩支持（避免 zstd-sys 符号冲突）
- **parquet feature**: Parquet/Arrow 导出支持（数据库 Sink 归档）
- **fast-masking feature**: Aho-Corasick 多模式加速脱敏
- **i18n 核心模块**: fluent-bundle + ICU 国际化支持（始终编译，非 feature flag）
- **通道缓冲文件 Sink**: ChannelBufferedFileSink 与背压策略
- **断路器保护**: CircuitBreaker 机制保护 Sink 故障恢复
- **环形缓冲**: RingBufferedFileSink 高吞吐场景

### Changed

- **模块重构**: `infrastructure` 模块重命名为 `integrations`
- **公共 API 统一**: 所有集成类型通过根级别 re-export 访问
- **edition 升级**: Rust edition 2024, MSRV 1.94
- **依赖特性化**: 所有依赖显式声明 `default-features = false`

### Removed

- **不存在的 feature 清理**: 文档中移除 `test-local`、`debug`、`metrics`、`i18n` 等从未定义的 feature flag
- **压缩算法简化**: 文档中移除 Brotli/LZ4 引用（实际仅支持 ZSTD 和 GZIP）

### Fixed

- 文档版本号、功能标志表、模块路径与实际代码同步
- README.md 许可证信息修正为 MIT（与 Cargo.toml 一致）
- README_EN.md Rust 版本徽章修正为 1.94+

## [0.1.12] - 2026-07-22

### 测试

- 新增 `tests/e2e_advanced.rs`（226 个测试）：覆盖 CORE/ROT/PIPE/SEC/REL/PERF/OBS/CFG/compression/i18n/Integration 共 19 个模块的边界与异常场景

### 维护

- 移除未使用依赖：parking_lot（examples）
- 更新 sea-orm 到 2.0 稳定版

## [0.1.11] - 2026-07-17

### 安全修复

- **[vuln-0001]** 对 `table_name` 做 SQL 注入防护清洗
- **[vuln-0002]** `FileSink` 集成 `PathValidator` 防路径穿越；收紧敏感组件黑名单（新增 `.bashrc`/`.profile`/`.zshrc`/`.netrc`/`id_rsa`/`id_ed25519` 等），禁止落到用户主目录与密钥文件
- **[vuln-0003]** HTTP auth token 改为启动期缓存，获取失败 fail-closed（不再降级为无认证）

### 修复

- `compress_file`/`encrypt_file` 参数改 `&Path` 修 clippy `ptr_arg`
- 已安装的全局 logger/subscriber 不再重复 warn

### 重构

- examples/tests/benches：扩展 L1 重导出隔离 + 补 `tracing`/`chrono`/`tokio`/`serde` re-export

## [0.1.7] - 2026-07-13

### ⚠️ BREAKING CHANGES（仅影响启用 db 后端 feature 的用户）

- dbnexus 0.3 → 0.4（pre-1.0 minor bump，Cargo 视为不兼容）；启用 `sqlite`/`postgres`/`mysql`/`duckdb`/`kit` feature 的用户需同步升级
- trait-kit 0.2 → 0.3（同上，仅影响 `kit` feature 用户）

### Dependencies

- dbnexus 0.3 → 0.4（对齐下游 sdforge/limiteron 依赖链）
- trait-kit 0.2 → 0.3（dbnexus 0.4 依赖 trait-kit 0.3）
- oxcache 0.3.4 → 0.3.8

### Fixed

- `src/error.rs`: Moved `InklogResult<T>` type alias from after `#[cfg(test)] mod tests` to before it, fixing clippy `items_after_test_module` lint
- `Cargo.toml`: MSRV updated from `1.85` to `1.94` to match actual requirement (sea-orm 2.0.0-rc + sqlx 0.9)

## [0.1.6] - 2026-07-12

### ⚠️ BREAKING CHANGES

- `error` module moved from `src/domain/types/error.rs` to `src/error.rs`, import path `crate::domain::types::error::` → `crate::error::`
- Added `InklogResult<T>` type alias

### Changed

- 文件级 import 扁平化为 mod 级 import
- `pub mod` 收敛为 `mod`（lib.rs 收敛）
- `mod.rs` 拆分为 `mod.rs` + `<module>.rs`（impl 提取）
- 导入路径统一为最浅 re-export 路径
- README 翻译为中文，新增 README_EN 英文版，徽章下添加语言切换链接
- AGENTS.md 添加到 .gitignore（禁止 AI 文件入库）

## [0.1.5] - 2026-07-11

### Changed

- 无代码变更，版本号对齐 workspace 同步升级

### Changed

- **edition 升级**: 从 edition 2021 升级至 edition 2024
- **MSRV 声明**: `rust-version = "1.85"` 显式声明最低支持 Rust 版本
- **MIT license 统一**: 许可证从 `"MIT OR Apache-2.0"` 统一为 `"MIT"`，移除 Apache-2.0 选项

### Security

- **edition 2024 unsafe 要求**: `std::env::set_var` / `std::env::remove_var` 调用包裹 `unsafe` 块（edition 2024 将其标记为 unsafe 操作）

## [0.1.2] - 2026-07-05

### BREAKING CHANGES

- **LogSink trait async 化**: `LogSink` trait 的 `write`/`flush`/`shutdown` 方法签名改为 `async`
  - `FileSink`、`ConsoleSink`、`RingBufferedFileSink`、`DatabaseSink` 全部迁移至 async
  - `DatabaseSink` 移除 `block_in_place`，使用原生 async
  - `LoggerManager` 中所有 sink 调用更新为 `.await`
  - 自定义 Sink 实现需要相应改为 async

- **ObjectPool API 重构**:
  - `ObjectPool::new`/`with_config`/`get`/`put` 改为 `async` 并返回 `Result<_, InklogError>`
  - 移除内部 `SHARED_RUNTIME`（内部 tokio runtime）
  - ThreadLocal 池（`get_log_record`/`put_log_record`/`get_string_buffer`/`put_string_buffer`）保留为同步便捷函数

- **Cache trait 错误处理显性化**: `Cache` trait 的 `get`/`delete`/`exists` 方法返回 `Result<_, InklogError>`（此前为 `Option`/`bool`，错误被 `tracing::warn!` 静默吞掉）
  - 移除 `OxCacheAdapter::default()`（原本通过 `.expect()` 触发 panic）
  - `ObjectPool::with_config` 不再静默回退到 `Cache::default()`

- **dbnexus feature 拆分**: `dbnexus` feature 拆分为四个独立 feature：`sqlite`、`postgres`、`mysql`、`duckdb`（`duckdb` 仅用于 `--all-features` 测试场景，DatabaseSink 不直接支持 duckdb 驱动）

### Changed

#### 依赖重构

- `oxcache` 升级 0.2.0 → 0.3.3，启用 features `memory`/`serialization`/`tracing`/`macros`
- `dbnexus` 升级 0.2.0 → 0.3.1，新增 `duckdb` feature 用于 `--all-features` 测试场景（dbnexus 0.3.1 规则 2 例外：sqlite+duckdb+postgres+mysql 全部 4 个后端启用时允许共存）
- 移除直接的 `moka` 和 `dashmap` 依赖（现仅通过 oxcache 间接依赖）
- `sea-orm` TLS 后端从 `runtime-tokio-native-tls` 切换至 `runtime-tokio-rustls`

#### 代码清理

- 移除源码注释中所有 `confers` 引用
- 清理 `object_pool.rs` 中的死代码（1507 → ~650 行）

### Removed

- `SHARED_RUNTIME`（内部 tokio runtime）
- `ObjectPoolBuilder`、`PoolMetrics` 类型
- `ObjectPool` 死代码方法：`with_capacity`/`remove`/`contains`/`capacity`/`metrics`/`clear`/`execute_async`
- `LOG_RECORD_POOL`/`STRING_POOL` 静态变量
- `OxCacheAdapter::default()` 实现
- 直接的 `moka` 和 `dashmap` 依赖

### Security

- 在 `deny.toml` 忽略列表中添加 `RUSTSEC-2026-0173`（proc-macro-error2 unmaintained，transitive via dbnexus-macros/sea-bae，无安全升级路径）

## [0.1.1] - 2026-06-29

### BREAKING
- 完全移除 S3/AWS 代码和依赖（aws feature、aws-sdk-s3、aws-config 等）
- 移除 ArchiveConfig、S3Error、ArchiveError 类型
- 移除 archive 模块和 S3 归档服务
- **MSRV 提升**: Rust 1.75+ → 1.85+（因 rand 0.10 要求 MSRV 1.85）
- **rand 0.9 → 0.10**: `RngCore` trait 重命名为 `Rng`，`Rng` 重命名为 `RngExt`，`OsRng` 重命名为 `SysRng`
- **aes-gcm 0.10 → 0.11**: `Nonce::from_slice` 已废弃，改用 `Nonce::from`；`cipher.encrypt/decrypt` 接受 `&Nonce` 而非 `Nonce`

### Added
- 新增 10 个示例：object_pool、path_validator、log_sanitizer、log_adapter、compression、rotation、ring_buffered_file、config_file、metrics、circuit_breaker
- 测试覆盖率达 90.12%（3291/3652 行）
- 新增 Docker 测试基础设施（SQLite/PostgreSQL/MySQL）
- 新增 CI 流水线（test-docker.yml）
- Cargo.toml 新增 `rust-version = "1.85"` 显式声明 MSRV

### Changed
- **依赖升级**:
  - `toml` 0.9 → 1.1（major 版本升级）
  - `validator` 0.19 → 0.20
  - `aes-gcm` 0.10 → 0.11（破坏性 API 变更已适配）
  - `rand` 0.9 → 0.10（破坏性 API 变更已适配，MSRV 1.85+）
  - `parquet` 57.3 → 59.0（major 版本升级）
  - `arrow-array` 57.3 → 59.0
  - `arrow-schema` 57.3 → 59.0
  - `cron` 0.15 → 0.17
  - `sha2` 0.10 → 0.11
  - `pbkdf2` 0.12 → 0.13
- CI MSRV 矩阵从 1.70.0 提升至 1.85.0
- README/README_zh Rust 版本徽章从 1.75+ 更新为 1.85+
- docs/CONTRIBUTING.md MSRV 与版本引用同步更新
- examples/Cargo.toml 移除 s3_archive [[bin]] 定义
- lib.rs 移除 "S3 归档" 描述

### Fixed
- 修复 examples/Cargo.toml s3_archive 残留导致编译失败
- 修复 lib.rs S3 归档描述残留
- 适配 rand 0.10 破坏性 API：`RngCore` → `Rng`，`Rng` → `RngExt`（src/support/io/sink/encryption.rs、src/support/io/sink/file.rs、src/cli/decrypt.rs、benches/inklog_bench.rs）
- 适配 aes-gcm 0.11 破坏性 API：`Nonce::from_slice` → `Nonce::from`，`cipher.encrypt/decrypt` 接受引用（src/support/io/sink/file.rs、src/cli/decrypt.rs、tests/cli_integration.rs）

## [0.1.0] - 2026-01-18

### Added

#### 核心功能
- **LoggerManager**: 异步日志管理器，支持多种初始化方式
  - `LoggerManager::new()` 默认初始化
  - `LoggerManager::builder()` 构建器模式
  - `LoggerManager::with_config()` 自定义配置

- **多输出目标支持**: 基于trait的可扩展sink架构
  - ConsoleSink: 控制台输出，支持彩色显示
  - FileSink: 文件输出，支持轮转和压缩
  - DatabaseSink: 数据库输出，支持批量写入

- **配置系统**: 完整的TOML配置支持
  - 全局配置、性能配置、HTTP服务器配置
  - 环境变量覆盖
  - 配置验证和错误处理

- **性能优化**: 基于crossbeam-channel的异步架构
  - 有界通道，支持背压控制
  - 多线程工作池
  - 内存池优化

- **监控和指标**: 内置健康检查和性能指标
  - HTTP健康检查端点
  - Prometheus兼容指标
  - 实时状态监控

#### 功能特性
- **日志轮转**: 基于大小和时间的自动轮转
- **数据掩码**: 敏感信息自动掩码功能
- **S3归档**: AWS S3云存储归档（可选功能）
- **CLI工具**: 配置生成、验证、日志解密命令行工具

#### 技术栈
- **异步运行时**: tokio 1.32+
- **日志框架**: tracing 0.1
- **序列化**: serde 1.0
- **并发**: crossbeam-channel 0.5
- **HTTP服务**: axum 0.6（可选）

### 兼容性
- **Rust**: 1.70+
- **平台**: Linux, macOS, Windows
- **数据库**: SQLite, PostgreSQL, MySQL（通过SeaORM）
- **云存储**: AWS S3兼容存储

### 示例用法

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;

    tracing::info!("Hello, inklog!");
    Ok(())
}
```

---

## [0.0.0] - 2025-12-30

### Added

- 初始项目结构
- 基础Cargo配置
- CI/CD工作流

<!-- Links -->
[Unreleased]: https://github.com/Kirky-X/inklog/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Kirky-X/inklog/compare/v0.1.12...v0.2.0
[0.1.12]: https://github.com/Kirky-X/inklog/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/Kirky-X/inklog/compare/v0.1.7...v0.1.11
[0.1.7]: https://github.com/Kirky-X/inklog/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/Kirky-X/inklog/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/Kirky-X/inklog/compare/v0.1.2...v0.1.5
[0.1.2]: https://github.com/Kirky-X/inklog/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Kirky-X/inklog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Kirky-X/inklog/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Kirky-X/inklog/releases/tag/v0.0.0
