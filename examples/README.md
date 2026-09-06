# inklog 示例索引

本目录是 workspace 内的独立子 crate `inklog-examples`，收录 inklog 的全部 **39 个可运行示例**，按主题分为 7 类。

- **Rust 版本要求**：1.97.1 及以上（与 workspace MSRV 一致）
- **运行方式**：在仓库根目录执行 `cargo run --package inklog-examples --example <名称>`
- **Feature 说明**：部分示例需要启用对应 feature 才能运行（数据库类需 `sqlite` / `postgres` / `mysql`，压缩与归档需 `compression` / `parquet`），命令中已逐一标注；其中 `compression` 与 `di_example` 在 `Cargo.toml` 中声明了 `required-features`，未启用时 cargo 会自动跳过编译

完整项目介绍见 [主 README](../README.md)，安全策略见 [SECURITY.md](../SECURITY.md)。

## 配置（config）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `config_file` | 配置文件加载示例：从 TOML 配置文件加载 `InklogConfig`（Layer 1 本地资源，临时目录自动清理） | `cargo run --package inklog-examples --example config_file` |
| `config_inspect` | 配置 inspect 示例：`InklogConfig::sinks_enabled()` 与 `LoggerManager::load()` 的使用 | `cargo run --package inklog-examples --example config_inspect` |
| `env_overrides` | 环境变量覆盖加载示例：`InklogConfig::load_with_env_overrides()` 的使用 | `cargo run --package inklog-examples --example env_overrides` |

## 核心（core）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `basic` | 基础用法示例：初始化、日志记录、验证和关闭全流程 | `cargo run --package inklog-examples --example basic` |
| `builder` | Builder 模式配置示例：`LoggerManager::builder()` 链式 API 的各种用法 | `cargo run --package inklog-examples --example builder` |
| `all_features` | 完整功能演示：综合演示 inklog 所有主要功能模块及组合使用 | `cargo run --package inklog-examples --example all_features` |
| `production` | 生产环境配置示例：开发/预发布/生产等不同环境的配置方式 | `cargo run --package inklog-examples --example production` |
| `template` | 日志模板示例：`LogTemplate` 用法、自定义占位符与不同格式效果 | `cargo run --package inklog-examples --example template` |
| `error_handling` | 错误处理示例（Layer 0 零依赖）：`InklogError` 与 `InklogResult` 的使用 | `cargo run --package inklog-examples --example error_handling` |
| `i18n` | 国际化 (i18n) 格式化示例：多语言（en-US / zh-CN）日志格式化 | `cargo run --package inklog-examples --example i18n` |

## Sink 与输出（sinks）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `console` | Console Sink 示例：控制台输出的三种配置方式（含颜色与 stderr 分流） | `cargo run --package inklog-examples --example console` |
| `file` | File Sink 示例：文件输出的三种核心功能 | `cargo run --package inklog-examples --example file` |
| `rotation` | 日志轮转示例（Layer 1 本地资源）：按大小、按时间等轮转策略 API | `cargo run --package inklog-examples --example rotation` |
| `compression` | Zstd 压缩/解压缩示例（Layer 1 本地资源）：日志压缩能力（需 `compression` feature） | `cargo run --package inklog-examples --features compression --example compression` |
| `ring_buffered_file` | ChannelBufferedFileSink 示例（Layer 1 本地资源）：基于 crossbeam channel 的高性能文件 sink | `cargo run --package inklog-examples --example ring_buffered_file` |
| `archive_format` | 归档格式示例（Layer 0 零依赖）：`ArchiveFormat` 枚举的使用 | `cargo run --package inklog-examples --example archive_format` |
| `parquet_archive` | Parquet 归档示例：`ParquetConfig` 与 `convert_logs_to_parquet()`（需数据库 feature 与 `parquet` feature） | `cargo run --package inklog-examples --features sqlite,parquet --example parquet_archive` |
| `partition_strategy` | 数据库分区策略示例：`PartitionStrategy`（Monthly / Yearly）的配置与使用 | `cargo run --package inklog-examples --example partition_strategy` |

## 数据库（database）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `database` | Database Sink 示例：SQLite 内存数据库连接、批量写入与查询演示（需 `sqlite` feature） | `cargo run --package inklog-examples --features sqlite --example database` |
| `database_pg_mysql` | PostgreSQL/MySQL 数据库驱动示例：`DatabaseDriver` 枚举、连接配置与驱动对比 | `cargo run --package inklog-examples --example database_pg_mysql` |
| `di_example` | DI（依赖注入）模式示例：使用真实适配器和 Mock 实现创建 `LoggerManager`（需 `sqlite` feature） | `cargo run --package inklog-examples --features sqlite --example di_example` |

## 基础设施（infra）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `channel_strategy` | 自适应 Channel 策略示例：`ChannelStrategy` 枚举与 `PerformanceConfig` 自适应阈值参数 | `cargo run --package inklog-examples --example channel_strategy` |
| `circuit_breaker` | 断路器示例（Layer 2 外部服务）：Closed → Open → HalfOpen → Closed 完整状态机 | `cargo run --package inklog-examples --example circuit_breaker` |
| `fallback` | Sink 降级机制示例：故障检测和自动降级策略 | `cargo run --package inklog-examples --example fallback` |
| `log_adapter` | `log` crate 适配器示例（Layer 0 零依赖）：`LogAdapter` 将标准 `log` 宏日志桥接到 inklog | `cargo run --package inklog-examples --example log_adapter` |
| `log_level` | LogLevel 类型示例：解析、比较与 Display | `cargo run --package inklog-examples --example log_level` |
| `metrics` | 健康监控与指标收集示例（Layer 2 外部服务）：指标采集、Sink 健康状态与 Prometheus 格式导出 | `cargo run --package inklog-examples --example metrics` |
| `object_pool` | 对象池示例（Layer 0 零依赖）：`ObjectPool` async API 与全局线程本地池便捷函数 | `cargo run --package inklog-examples --example object_pool` |
| `output_format` | 输出格式示例（Layer 0 零依赖）：`OutputFormat` 枚举的使用 | `cargo run --package inklog-examples --example output_format` |
| `performance` | 性能测试示例：吞吐量、延迟和并发能力 | `cargo run --package inklog-examples --example performance` |
| `rate_limiter` | 速率限制器示例（Layer 0 零依赖）：`RateLimiter` 令牌桶算法 | `cargo run --package inklog-examples --example rate_limiter` |
| `runtime_ops` | 运行时操作示例：`LoggerManager` 的运行时监控与运维 API | `cargo run --package inklog-examples --example runtime_ops` |

## 网络（network）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `http` | HTTP 健康检查和指标端点示例：启动 HTTP 服务器并访问健康检查与 Prometheus 指标端点 | `cargo run --package inklog-examples --example http` |
| `http_auth` | HTTP 认证与 IP 白名单示例：`HttpAuthConfig` 与 `HttpServerConfig.ip_whitelist` 配置 | `cargo run --package inklog-examples --example http_auth` |
| `tls_config` | TLS 配置示例：`TlsConfig` 证书与密钥路径等配置 | `cargo run --package inklog-examples --example tls_config` |

## 安全（security）

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `encryption` | 日志加密示例：使用加密功能保护日志数据 | `cargo run --package inklog-examples --example encryption` |
| `log_sanitizer` | 日志内容净化示例（Layer 0 零依赖）：敏感数据脱敏、严格转义、JSON 安全转义 | `cargo run --package inklog-examples --example log_sanitizer` |
| `masking` | 数据脱敏示例：使用 `DataMasker` 对敏感信息进行脱敏处理 | `cargo run --package inklog-examples --example masking` |
| `path_validator` | 路径验证器示例（Layer 0 零依赖）：路径遍历检测、危险组件与符号链接检测 | `cargo run --package inklog-examples --example path_validator` |

## 许可证

MIT + Commons Clause 许可证 — 商业使用需单独授权，参见项目根目录 [LICENSE](../LICENSE) 文件。
