# 📘 inklog API 参考

本文档提供 inklog v0.3.0-rc.3 公共 API 的逐项参考：核心类型、配置结构体、错误类型、健康监控、Sink 家族、依赖注入 trait 与测试 Mock。使用教程见 [📖 用户指南](USER_GUIDE.md)，内部设计见 [🏗️ 架构设计](ARCHITECTURE.md)。

<details open>
<summary>📑 目录</summary>

- [📭 概述](#-概述)
- [🚀 顶层便捷函数](#-顶层便捷函数)
- [🧩 核心类型](#-核心类型)
- [⚙️ 配置结构体](#️-配置结构体)
- [⚠️ 错误类型](#️-错误类型)
- [🩺 健康监控类型](#-健康监控类型)
- [🧾 日志记录类型](#-日志记录类型)
- [🔌 Sink 类型](#-sink-类型)
- [🧪 依赖注入类型](#-依赖注入类型)
- [🎭 Mock 实现（测试用）](#-mock-实现测试用)
- [🏭 适配器实现](#-适配器实现)
- [💡 示例参考](#-示例参考)

</details>

---

## 📭 概述

### 公共 API 总览

| 类别 | 类型 | 描述 |
|------|------|------|
| 核心 | `LoggerManager` | 核心日志管理器，协调日志收集与路由 |
| 核心 | `LoggerBuilder` | 流式构建器，用于创建管理器 |
| 核心 | `LoggerDependencies` | 依赖注入容器 |
| 核心 | `InklogContainer` / `InklogContainerBuilder` | DI 容器 |
| 配置 | `InklogConfig` 及子配置 | 根配置与各 Sink 配置 |
| 错误 | `InklogError` / `InklogResult` | 错误枚举与结果别名 |
| 健康 | `HealthStatus` / `Metrics` / `SinkHealthMonitor` | 健康状态与指标 |
| Sink | `LogSink` / `AsyncSink` 及内置实现 | 输出目标抽象与实现 |
| 记录 | `LogRecord` | 日志记录结构 |
| 注入 | `Cache` / `Config` / `Database` trait | 基础设施抽象 |
| 适配 | `OxCacheAdapter` / `InklogConfigAdapter` / `DbNexusAdapter` | 生产环境适配器 |
| 测试 | `MockCache` / `MockConfig` / `MockDatabaseAdapter` | 测试 Mock（`test-utils` feature） |

### 导入公共 API

```rust
use inklog::{
    // 核心类型
    LoggerManager,
    LoggerBuilder,
    LoggerDependencies,
    InklogConfig,
    LogRecord,
    LogLevel,

    // 配置类型
    config::{
        GlobalConfig,
        ConsoleSinkConfig,
        FileSinkConfig,
        DatabaseSinkConfig,
        DatabaseDriver,
        HttpServerConfig,
        PerformanceConfig,
        ParquetConfig,
    },

    // 错误类型
    InklogError,
    InklogResult,

    // 健康监控
    HealthStatus,
    Metrics,
    SinkStatus,
    SinkHealth,

    // Sink 抽象
    LogSink,
    AsyncSink,
};
```

## 🚀 顶层便捷函数

### `init_inklog_logger`

以默认配置初始化并安装全局日志前端，进程级单例语义（重复初始化返回明确错误）。

**签名**

```rust
pub async fn init_inklog_logger() -> Result<(), InklogError>
```

**示例**

```rust
inklog::init_inklog_logger().await?;
tracing::info!("Hello, inklog!");
```

### `init_inklog_logger_with_config`

以自定义配置初始化，语义与 [`init_inklog_logger`](#init_inklog_logger) 一致。

**签名**

```rust
pub async fn init_inklog_logger_with_config(config: InklogConfig) -> Result<(), InklogError>
```

## 🧩 核心类型

### LoggerManager

核心日志管理器，协调日志收集、脱敏、分发与路由到各个 Sink，并负责安装全局 tracing subscriber 与 `log` crate 前端。

#### 方法

##### `new`

创建带有默认配置的 `LoggerManager` 并安装全局前端。

```rust
pub async fn new() -> Result<Self, InklogError>
```

##### `with_config`

使用给定配置创建 `LoggerManager` 并安装全局前端。

```rust
pub async fn with_config(config: InklogConfig) -> Result<Self, InklogError>
```

**示例**

```rust
use inklog::{FileSinkConfig, InklogConfig};

let config = InklogConfig {
    file_sink: Some(FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        ..Default::default()
    }),
    ..Default::default()
};

let logger = LoggerManager::with_config(config).await?;
```

##### `with_dependencies`

使用依赖注入创建 `LoggerManager`。

```rust
pub async fn with_dependencies(deps: LoggerDependencies) -> Result<Self, InklogError>
```

##### `builder`

创建 `LoggerBuilder` 实例。

```rust
pub fn builder() -> LoggerBuilder
```

##### `from_file`

从指定路径加载 TOML 配置并初始化。

```rust
pub async fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, InklogError>
```

##### `load`

从默认位置加载配置文件并初始化。查找优先级（从高到低）：

1. `$INKLOG_CONFIG_PATH` 指定的路径；
2. `./inklog_config.toml`（当前目录）；
3. `~/.config/inklog/config.toml`（用户配置目录）；
4. 系统配置路径（Unix: `/etc/inklog/config.toml`）。

```rust
pub async fn load() -> Result<Self, InklogError>
```

##### `build_detached`

构建 `LoggerManager` 但不安装全局订阅者，返回管理器、订阅者与 EnvFilter 三元组，供测试/基准自行装配（如线程级 `set_default`）。

```rust
pub async fn build_detached(
    config: InklogConfig,
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    database: Option<Arc<dyn Database>>,
) -> Result<(Self, LoggerSubscriber, tracing_subscriber::filter::EnvFilter), InklogError>
```

另有 `build_detached_with_sinks` 变体，可同时注册动态第三方 Sink。

##### `set_level`

运行时级别热调：经 `tracing_subscriber::reload` 换装 EnvFilter，进程内即时生效；支持全局与 per-target upsert，`RUST_LOG` 附加指令跨重建保留。

```rust
pub fn set_level(&self, target: Option<&str>, level: &str) -> Result<(), InklogError>
```

| 参数 | 描述 |
|------|------|
| `target` | `None` 调整全局级别；`Some("hyper")` 调整指定 target |
| `level` | 目标级别（`trace` / `debug` / `info` / `warn` / `error`） |

##### `get_health_status`

获取当前健康状态快照。

```rust
pub fn get_health_status(&self) -> HealthStatus
```

##### `recover_sink`

向指定 Sink 发送恢复指令（重新初始化并重置断路器）。

```rust
pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError>
```

##### `trigger_recovery_for_unhealthy_sinks`

恢复所有不健康的 Sink，返回已恢复的 Sink 名称列表。

```rust
pub fn trigger_recovery_for_unhealthy_sinks(&self) -> Result<Vec<String>, InklogError>
```

##### `effective_channel_capacity` / `channel_len`

查询有效通道容量与当前积压条数。

```rust
pub fn effective_channel_capacity(&self) -> usize
pub fn channel_len(&self) -> usize
```

##### `publish_ops_event`

广播内部运维事件（ops event）到全部 Sink 通道。

```rust
pub fn publish_ops_event(&self, kind: &str, sink: Option<&str>, detail: serde_json::Value)
```

##### `current_level_filter_string`

返回当前生效的 EnvFilter 指令串。

```rust
pub fn current_level_filter_string(&self) -> String
```

##### `cache` / `database`

访问注入的缓存 / 数据库实现（未注入返回 `None`）。

```rust
pub fn cache(&self) -> Option<Arc<dyn Cache>>
pub fn database(&self) -> Option<Arc<dyn Database>>
```

##### `shutdown`

优雅关闭：对停止信号采用 2 秒超时发送（`send_timeout`），超时后继续轮询 worker handles，保证永不挂死；等待通道中剩余日志排空后关闭全部 Sink。

```rust
pub fn shutdown(&self) -> Result<(), InklogError>
```

### LoggerBuilder

流式构建器，用于创建 `LoggerManager`。非法参数延迟到 `build()` 统一报 `ConfigError`（builder 返回 `Result`）。

#### 方法

##### 基础配置

| 方法 | 签名 | 描述 |
|------|------|------|
| `new` | `pub fn new() -> Self` | 创建构建器 |
| `level` | `pub fn level(self, level: impl Into<String>) -> Self` | 全局日志级别 |
| `format` | `pub fn format(self, format: impl Into<String>) -> Self` | 格式模板 |
| `console` | `pub fn console(self, enabled: bool) -> Self` | 启用/禁用控制台 Sink |
| `file` | `pub fn file(self, path: impl Into<PathBuf>) -> Self` | 启用文件 Sink 并设置路径 |
| `database` | `pub fn database(self, url: impl Into<String>) -> Self` | 启用数据库 Sink 并设置 URL |
| `channel_capacity` | `pub fn channel_capacity(self, capacity: usize) -> Self` | 通道容量 |
| `worker_threads` | `pub fn worker_threads(self, threads: usize) -> Self` | 工作线程数 |

##### 数据库细化（需数据库后端 feature）

| 方法 | 签名 | 描述 |
|------|------|------|
| `with_driver` | `pub fn with_driver(self, driver: DatabaseDriver) -> Self` | 数据库驱动 |
| `with_pool_size` | `pub fn with_pool_size(self, pool_size: u32) -> Self` | 连接池大小 |
| `with_batch_size` | `pub fn with_batch_size(self, batch_size: usize) -> Self` | 批量大小 |
| `with_flush_interval_ms` | `pub fn with_flush_interval_ms(self, flush_interval_ms: u64) -> Self` | 刷新间隔 |
| `with_table_name` | `pub fn with_table_name(self, table_name: impl Into<String>) -> Self` | 日志表名 |
| `with_admin_role` | `pub fn with_admin_role(self, admin_role: impl Into<String>) -> Self` | 管理角色名 |

##### 文件细化

| 方法 | 签名 | 描述 |
|------|------|------|
| `file_max_size` | `pub fn file_max_size(self, max_size: impl Into<String>) -> Self` | 单文件最大大小（如 `"50MB"`） |
| `file_compress` | `pub fn file_compress(self, compress: bool) -> Self` | 是否压缩轮转文件 |
| `file_rotation_time` | `pub fn file_rotation_time(self, rotation: impl Into<String>) -> Self` | 时间轮转策略 |
| `file_keep_files` | `pub fn file_keep_files(self, keep: u32) -> Self` | 保留轮转文件数 |

##### 控制台细化

| 方法 | 签名 | 描述 |
|------|------|------|
| `console_colored` | `pub fn console_colored(self, colored: bool) -> Self` | 是否彩色输出 |
| `console_stderr_levels` | `pub fn console_stderr_levels(self, levels: &[&str]) -> Self` | 输出到 stderr 的级别（默认 `["error", "warn"]`） |

##### HTTP 服务器细化（需 `http` feature）

| 方法 | 签名 | 描述 |
|------|------|------|
| `enable_http_server` | `pub fn enable_http_server(self, enabled: bool) -> Self` | 启用 HTTP 端点 |
| `http_host` / `http_port` | `pub fn http_host(self, host: impl Into<String>) -> Self` / `pub fn http_port(self, port: u16) -> Self` | 监听地址与端口 |
| `http_metrics_path` / `http_health_path` | `pub fn http_metrics_path(self, path: impl Into<String>) -> Self` 等 | 端点路径 |
| `http_error_mode` | `pub fn http_error_mode(self, mode: impl Into<String>) -> Self` | 错误模式（`"warn"` / `"strict"`） |

##### 依赖注入与动态 Sink

| 方法 | 签名 | 描述 |
|------|------|------|
| `cache` | `pub fn cache(self, cache: Arc<dyn Cache>) -> Self` | 注入自定义 Cache |
| `config` | `pub fn config(self, config: Arc<dyn Config>) -> Self` | 注入自定义 Config |
| `with_database` | `pub fn with_database(self, database: Arc<dyn Database>) -> Self` | 注入自定义 Database（需数据库后端 feature） |
| `add_sink` | `pub fn add_sink(self, sink: Arc<dyn AsyncSink>) -> Self` | 注册动态第三方 Sink（每 Sink 独立通道） |

##### `build`

构建并返回 `LoggerManager`（安装全局前端）。

```rust
pub async fn build(self) -> Result<LoggerManager, InklogError>
```

**示例**

```rust
use inklog::LoggerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = LoggerBuilder::new()
        .level("debug")
        .file("logs/app.log")
        .channel_capacity(20000)
        .worker_threads(4)
        .build()
        .await?;
    Ok(())
}
```

## ⚙️ 配置结构体

### InklogConfig

根配置结构，支持 TOML 文件与 `INKLOG_*` 环境变量覆盖。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InklogConfig {
    pub global: GlobalConfig,
    pub console_sink: Option<ConsoleSinkConfig>,   // 默认 Some(default())
    pub file_sink: Option<FileSinkConfig>,
    pub database_sink: Option<DatabaseSinkConfig>,
    pub performance: PerformanceConfig,
    pub http_server: Option<HttpServerConfig>,
    pub target_levels: HashMap<String, String>,    // per-target 级别预设
}
```

> `target_levels` 的条目合并进 EnvFilter：优先级低于 `RUST_LOG`、高于全局默认级别。

#### 方法

| 方法 | 签名 | 描述 |
|------|------|------|
| `validate` | `pub fn validate(&self) -> Result<(), InklogError>` | 校验配置（级别合法性、路径非空等） |
| `sinks_enabled` | `pub fn sinks_enabled(&self) -> Vec<&'static str>` | 查询已启用的 Sink 名称 |

### GlobalConfig

```rust
pub struct GlobalConfig {
    pub level: String,                    // 默认 "info"
    pub format: String,                   // 默认 "{timestamp} [{level}] {target} - {message}"
    pub masking_enabled: bool,            // 默认 true
    pub auto_fallback: bool,              // 默认 true
    pub fallback_initial_delay_ms: u64,   // 默认 1000
    pub fallback_max_delay_ms: u64,       // 默认 60000
    pub fallback_max_retries: u32,        // 默认 10
    pub output_format: OutputFormat,      // 默认 Text
}
```

`OutputFormat` 枚举：`Text`（模板渲染）、`Json`（NDJSON，自动禁用彩色）。

### ConsoleSinkConfig

```rust
pub struct ConsoleSinkConfig {
    pub enabled: bool,               // 默认 true
    pub colored: bool,               // 默认 true（NO_COLOR / TERM=dumb 时自动禁用）
    pub stderr_levels: Vec<String>,  // 默认 ["error", "warn"]
    pub masking_enabled: bool,       // 默认 true
    pub output_format: OutputFormat, // 默认 Text
}
```

### FileSinkConfig

```rust
pub struct FileSinkConfig {
    pub enabled: bool,                        // 默认 true
    pub path: PathBuf,                        // 默认 "logs/app.log"
    pub max_size: String,                     // 默认 "100MB"
    pub rotation_time: String,                // 默认 "daily"（hourly / daily / weekly）
    pub keep_files: u32,                      // 默认 30
    pub compress: bool,                       // 默认 true
    pub compression_level: i32,               // 默认 3（0-22）
    pub encrypt: bool,                        // 默认 false
    pub encryption_key_env: Option<String>,   // 默认 None
    pub retention_days: u32,                  // 默认 30
    pub max_total_size: String,               // 默认 "1GB"
    pub cleanup_interval_minutes: u64,        // 默认 60
    pub batch_size: usize,                    // 默认 100
    pub flush_interval_ms: u64,               // 默认 100
    pub masking_enabled: bool,                // 默认 true
    pub output_format: OutputFormat,          // 默认 Text
}
```

> 压缩需 `compression`（Zstd）或 `gzip`（flate2）feature；未启用任一压缩 feature 时轮转文件保持未压缩。加密与压缩仅作用于轮转归档（先压缩后加密），活跃文件保持明文。

### DatabaseSinkConfig

```rust
pub struct DatabaseSinkConfig {
    pub name: String,                     // 默认 "default"
    pub enabled: bool,                    // 默认 false
    pub driver: DatabaseDriver,           // 默认 SQLite
    pub url: String,                      // 默认 "sqlite::memory:"
    pub pool_size: u32,                   // 默认 10（SQLite 自动设为 1）
    pub batch_size: usize,                // 默认 100
    pub flush_interval_ms: u64,           // 默认 500
    pub partition: PartitionStrategy,     // 默认 Monthly
    pub table_name: String,               // 默认 "logs"
    pub archive_format: ArchiveFormat,    // 默认 Json
    pub parquet_config: ParquetConfig,    // 默认 default()
    pub permissions_path: Option<String>, // 默认 None（RBAC 权限配置）
    pub admin_role: String,               // 默认 "admin"
}
```

#### DatabaseDriver

```rust
#[derive(Default)]
pub enum DatabaseDriver {
    PostgreSQL, // serde: "postgres"
    MySQL,      // serde: "mysql"
    #[default]
    SQLite,     // serde: "sqlite"
    DuckDB,     // serde: "duckdb"
}
```

实现 `FromStr` 与 `Display`（`"postgres"` / `"mysql"` / `"sqlite"` / `"duckdb"`，解析忽略大小写）；合法值外的输入返回本地化（i18n）错误消息。

#### PartitionStrategy

| 变体 | serde 值 | 描述 |
|------|----------|------|
| `Monthly` | `"monthly"` | 按月分区（默认） |
| `Yearly` | `"yearly"` | 按年分区 |

#### ArchiveFormat

| 变体 | 描述 |
|------|------|
| `Json` | JSON 归档（默认） |
| `Parquet` | Parquet/Arrow 归档（需 `parquet` feature） |
| `Csv` | CSV 归档 |

#### ParquetConfig

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `compression_level` | `i32` | `3` | Zstd 压缩级别（0-22） |
| `encoding` | `String` | `"PLAIN"` | 编码方式：`PLAIN`、`DICTIONARY`、`RLE` |
| `max_row_group_size` | `usize` | `10000` | Row Group 大小（行数） |
| `max_page_size` | `usize` | `1048576` | 页面大小（字节） |
| `include_fields` | `Vec<String>` | `[]` | 包含字段列表（空 = 全部） |

可用字段：`id`、`timestamp`、`level`、`target`、`message`、`fields`、`file`、`line`、`thread_id`。

### HttpServerConfig

HTTP 端点配置（需 `http` feature）。

```rust
pub struct HttpServerConfig {
    pub enabled: bool,                    // 默认 false
    pub host: String,                     // 默认 "127.0.0.1"
    pub port: u16,                        // 默认 9090
    pub metrics_path: String,             // 默认 "/metrics"
    pub health_path: String,              // 默认 "/health"
    pub error_mode: HttpErrorMode,        // 默认 Strict
    pub auth: Option<HttpAuthConfig>,     // 默认 None
    pub ip_whitelist: Option<Vec<String>>,// 默认 None
    pub tls: Option<TlsConfig>,           // 默认 None
}
```

| 关联类型 | 字段/变体 | 描述 |
|----------|-----------|------|
| `HttpErrorMode` | `Strict` / `Warn` | 启动失败时返回错误 / 记录警告并继续 |
| `HttpAuthConfig` | `enabled: bool`、`token_env: String` | 认证 token 环境变量，启动期缓存、获取失败 fail-closed |
| `TlsConfig` | `cert_path: String`、`key_path: String` | TLS 证书与私钥路径（rustls） |

### PerformanceConfig

```rust
pub struct PerformanceConfig {
    pub channel_capacity: usize,        // 默认 10000
    pub worker_threads: usize,          // 默认 3
    pub channel_strategy: ChannelStrategy, // 默认 Fixed
    pub expand_threshold_percent: u8,   // 默认 80
    pub shrink_threshold_percent: u8,   // 默认 20
    pub shrink_wait_seconds: u64,       // 默认 30
    pub min_capacity: usize,            // 默认 1000
    pub max_capacity: usize,            // 默认 50000
    pub rate_limit: Option<u64>,        // 默认 None（条/秒上限）
}
```

`ChannelStrategy` 枚举：`Fixed`（固定容量）、`Adaptive`（按水位在 `min_capacity` 与 `max_capacity` 之间扩缩容）。

## ⚠️ 错误类型

### InklogError

```rust
#[derive(Error, Debug)]
pub enum InklogError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Database error: {message}")]
    DatabaseError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("Cache error: {0}")]
    CacheError(String),
    #[error("Encryption error: {message}")]
    EncryptionError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("Shutdown error: {0}")]
    Shutdown(String),
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("Compression error: {0}")]
    CompressionError(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    #[error("HTTP server error: {0}")]
    HttpServerError(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}
```

| 变体 | 描述 |
|------|------|
| `ConfigError` | 配置相关错误（级别非法、路径为空、builder 校验失败等） |
| `IoError` | I/O 操作错误（从 `std::io::Error` 自动转换） |
| `SerializationError` | 序列化错误（从 `serde_json::Error` 自动转换） |
| `DatabaseError` | 数据库操作错误（带可选底层 source） |
| `CacheError` | 缓存操作错误 |
| `EncryptionError` | 加密/解密错误（带可选底层 source） |
| `Shutdown` | 关闭过程中的错误 |
| `ChannelError` | 通道通信错误 |
| `CompressionError` | 压缩错误 |
| `RuntimeError` | 运行时错误 |
| `HttpServerError` | HTTP 服务器错误 |
| `Unknown` | 未知错误 |

错误消息经 Fluent + ICU 按系统 locale 渲染（zh-CN / en），可用 `INKLOG_LOCALE` 固定语言。

### InklogResult

```rust
pub type InklogResult<T> = Result<T, InklogError>;
```

## 🩺 健康监控类型

### HealthStatus

```rust
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub overall_status: SinkStatus,
    pub sinks: HashMap<String, SinkHealth>,
    pub channel_usage: f64,           // 0.0 - 1.0
    pub uptime_seconds: u64,
    pub metrics: MetricsSnapshot,
    pub pool_stats: Option<PoolStats>,
    pub encryption_key_valid: bool,
}
```

### SinkStatus

```rust
#[derive(Debug, Serialize, Clone, PartialEq, Default)]
pub enum SinkStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { error: String },
    #[default]
    NotStarted,
}
```

方法：

```rust
pub fn is_operational(&self) -> bool  // Healthy 或 Degraded 视为可操作
```

### SinkHealth

```rust
#[derive(Debug, Serialize, Clone)]
pub struct SinkHealth {
    pub status: SinkStatus,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
}
```

### Metrics

指标收集器（内部为原子计数与直方图，无锁读取）。

#### 方法

| 方法 | 签名 | 描述 |
|------|------|------|
| `new` | `pub fn new() -> Self` | 创建实例 |
| `logs_written` / `inc_logs_written` | `pub fn logs_written(&self) -> u64` 等 | 写入总数读取 / 递增 |
| `logs_dropped` / `inc_logs_dropped` | 同上 | 丢弃总数 |
| `channel_blocked` / `inc_channel_blocked` | 同上 | 通道阻塞次数 |
| `sink_errors` / `inc_sink_error` | 同上 | Sink 错误总数 |
| `active_workers` | `pub fn active_workers(&self) -> i64` | 活跃工作线程数 |
| `record_latency` | `pub fn record_latency(&self, duration: Duration)` | 记录写入延迟（进入直方图） |
| `record_pool_metrics` | `pub fn record_pool_metrics(&self, total: u64, active: u64, idle: u64)` | 记录连接池指标 |
| `update_sink_health` | `pub fn update_sink_health(&self, name: &str, healthy: bool, error: Option<String>)` | 更新 Sink 健康状态 |
| `sink_started` / `sink_degraded` | `pub fn sink_started(&self, name: &str)` 等 | Sink 生命周期事件 |
| `get_status` | `pub fn get_status(&self, channel_len: usize, channel_cap: usize) -> HealthStatus` | 汇总健康快照 |
| `export_prometheus` | `pub fn export_prometheus(&self) -> String` | 导出 Prometheus 文本格式 |

### MetricsSnapshot

```rust
pub struct MetricsSnapshot {
    pub logs_written: u64,
    pub logs_dropped: u64,
    pub channel_blocked: u64,
    pub sink_errors: u64,
    pub db_batch_size: i64,
    pub db_batch_records_total: u64,
    pub avg_latency_us: u64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
    pub latency_distribution: Vec<u64>,
    pub active_workers: i64,
    pub pool_hit_rate: f64,
}
```

### PoolStats

数据库连接池状态快照（`DbNexusAdapter::pool_status()` 透传）。

### Prometheus 指标

| 指标名 | 类型 | 描述 |
|--------|------|------|
| `inklog_logs_written_total` | counter | 成功写入总数 |
| `inklog_logs_dropped_total` | counter | 丢弃总数 |
| `inklog_sink_errors_total` | counter | Sink 错误总数 |
| `inklog_channel_blocked_total` | counter | 通道阻塞次数 |
| `inklog_write_latency_us` | histogram | 写入延迟（微秒，含 bucket/sum/count） |
| `inklog_sink_healthy` | gauge | 各 Sink 健康状态 |
| `inklog_uptime_seconds` | gauge | 运行时长 |
| `inklog_db_batch_size` | gauge | 数据库批量大小 |
| `inklog_db_batch_records_total` | counter | 数据库批量写入记录总数 |
| `inklog_db_pool_total` / `inklog_db_pool_active` / `inklog_db_pool_idle` | gauge | 连接池总量/活跃/空闲 |

## 🧾 日志记录类型

### LogRecord

```rust
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub level: String,               // "trace"/"debug"/"info"/"warn"/"error"
    pub target: String,              // 模块路径
    pub message: String,
    pub fields: HashMap<String, Value>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub thread_id: String,
    pub trace_id: Option<String>,    // 从当前 tracing span 提取
    pub span_id: Option<String>,     // 无 OTel 时沿 parent 链派生 16 位 hex
}
```

#### `new`

```rust
pub fn new(level: tracing::Level, target: String, message: String) -> Self
```

**示例**

```rust
use inklog::LogRecord;

let record = LogRecord::new(
    tracing::Level::INFO,
    "my_app::auth".to_string(),
    "User logged in".to_string(),
);
```

### LogLevel

`LogLevel` 枚举实现 `FromStr` 与 `Display`，用于级别解析与比较（非法输入返回 `LogLevelParseError`）。

## 🔌 Sink 类型

### LogSink trait

所有输出目标的统一抽象，方法均为 `&self`（内部可变性由实现方使用 Mutex/RwLock/原子类型保证）：

```rust
#[async_trait]
pub trait LogSink: Send + Sync {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;
    async fn flush(&self) -> Result<(), InklogError>;
    fn is_healthy(&self) -> bool { true }  // 默认恒健康，建议覆写
    async fn shutdown(&self) -> Result<(), InklogError>;
}
```

### AsyncSink trait

`LogSink` 的标记子 trait（blanket impl：所有 `LogSink` 自动实现），用于 `LoggerBuilder::add_sink` 的动态 Sink 注册与 trait 上转型：

```rust
pub trait AsyncSink: LogSink {}
impl<T: LogSink + ?Sized> AsyncSink for T {}
```

### 内置 Sink 一览

| Sink | 模块 | feature | 描述 |
|------|------|---------|------|
| `ConsoleSink` | `support::io::sink::console` | 无 | ANSI 彩色、stderr 级别路由、NO_COLOR 支持 |
| `FileSink` | `support::io::sink::file` | 无（压缩/加密按 feature） | 轮转、压缩、AES-256-GCM 加密、断路器、磁盘空间管理 |
| `DatabaseSink` | `support::io::sink::database` | `sqlite`/`postgres`/`mysql`/`duckdb` 之一 | 批量落库、分区表、断路器 |
| `ChannelBufferedFileSink` | `support::io::sink::ring_buffered_file` | 无 | 通道缓冲高吞吐文件 Sink |
| `TcpSink` / `UdpSink` | `support::io::sink::net` | `net-sink` | TCP（可 TLS，rustls）+ UDP（NDJSON），断线缓冲与自动重连 |
| `OtlpSink` | `support::io::sink::otlp` | `otlp` | OTLP/HTTP JSON 日志导出 |
| `SamplingSink` | `support::io::sink::sampling` | 无 | 采样装饰器（级别阈值 + N 取 1 + 关键词白名单豁免） |
| `RateLimitedSink` | `support::io::sink::rate_limit` | 无 | 令牌桶限流装饰器 |
| `MiddlewareSink` | `support::io::sink::middleware` | 无 | 中间件链装饰器（filter / transform 组合） |

### 关联类型

| 类型 | 描述 |
|------|------|
| `CircuitBreaker` / `CircuitBreakerConfig` / `CircuitState` | 断路器（默认失败阈值 5 次、冷却 30 秒，半开动态批大小减半） |
| `SinkRegistry` | Sink 注册表与查找 |
| `RotationStrategy` / `SizeBasedRotation` / `TimeBasedRotation` / `CompositeRotation` | 轮转策略组合 |
| `Rotatable` / `RotationContext` / `RotationResult` | 轮转抽象 |
| `CompressionStrategy` / `NoCompression` / `ZstdCompression` / `GzipCompression` | 压缩后端抽象 |
| `SinkFactory` / `FileSinkFactory` / `SinkMetadata` / `SinkWriteOutcome` | Sink 工厂与元数据 |
| `Sampler` | 采样器（`should_emit(&LogRecord) -> bool`） |
| `SinkRateLimit` / `NoOpRateLimit` / `TokenBucketRateLimit` | 限流端口与实现（对象安全，供上层实现注入） |
| `MiddlewareChain` / `MiddlewareVerdict` / `RecordMiddleware` / `EnrichMiddleware` / `LevelFilterMiddleware` | 中间件组合子 |

## 🧪 依赖注入类型

### LoggerDependencies

```rust
pub struct LoggerDependencies {
    pub cache: Option<Arc<dyn Cache>>,
    pub config: Option<Arc<dyn Config>>,
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    pub database: Option<Arc<dyn Database>>,
}
```

未注入的依赖使用默认实现（`OxCacheAdapter` / `InklogConfigAdapter` / 无数据库）。

### Cache trait

```rust
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, InklogError>;
    async fn set(&self, key: &str, value: String) -> Result<(), InklogError>;
    async fn delete(&self, key: &str) -> Result<bool, InklogError>;
    async fn exists(&self, key: &str) -> Result<bool, InklogError>;
}
```

### Config trait

```rust
pub trait Config: Send + Sync {
    fn get_string(&self, key: &str) -> Option<String>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_bool(&self, key: &str) -> Option<bool>;
    fn get_float(&self, key: &str) -> Option<f64>;
}
```

支持的配置键路径（点分层级）：`global.level`、`global.format`、`global.masking_enabled`、`global.auto_fallback`、`global.fallback_initial_delay_ms`、`global.fallback_max_delay_ms`、`global.fallback_max_retries`、`global.output_format`、`file_sink.*`（enabled/path/max_size/rotation_time/keep_files/compress/compression_level/encrypt/encryption_key_env/retention_days/max_total_size/cleanup_interval_minutes/batch_size/flush_interval_ms/masking_enabled/output_format）、`console_sink.*`（enabled/colored/stderr_levels/masking_enabled）。

### Database trait

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError>;
    async fn is_healthy(&self) -> bool;
}
```

## 🎭 Mock 实现（测试用）

三个 Mock 由 `test-utils` feature 门控（src 内联测试经 `cfg(test)` 直接可见，外部消费者需显式启用）：

```toml
[dev-dependencies]
inklog = { version = "0.3.0-rc.3", features = ["test-utils"] }
```

### MockCache

基于 `RwLock<HashMap>` 的内存 Cache。

| 方法 | 描述 |
|------|------|
| `new()` | 创建空 MockCache |
| `with_delay(ms: u64)` | 创建带延迟模拟的 MockCache（测试超时场景） |
| Cache trait 四方法 | `get` / `set` / `delete` / `exists`，返回 `Result` |

### MockConfig

基于 `RwLock<HashMap>` 的内存 Config。

| 方法 | 描述 |
|------|------|
| `new()` | 创建空 MockConfig |
| `with_value(key: &str, value: &str)` | 链式添加配置值 |
| `set(key: &str, value: &str)` | 运行时修改配置值 |
| Config trait 四方法 | `get_string` / `get_int` / `get_bool` / `get_float` |

### MockDatabaseAdapter

基于 `RwLock<Vec<LogRecord>>` 的内存 Database。

| 方法 | 描述 |
|------|------|
| `new()` | 创建健康的 MockDatabaseAdapter |
| `set_healthy(healthy: bool)` | 设置健康状态（测试降级场景） |
| `record_count()` / `stored_count()` | 已存储记录数 |
| `get_records()` | 获取全部记录（测试验证） |
| `clear()` | 清空记录（测试隔离） |
| Database trait 两方法 | `insert_batch` / `is_healthy` |

## 🏭 适配器实现

| 适配器 | 实现的 trait | 依赖 | 特性 |
|--------|--------------|------|------|
| `OxCacheAdapter` | `Cache` | oxcache（内建） | 高性能内存缓存，支持 TTL 与容量（`builder()` / `ttl()` / `capacity()`，`new()` 返回 `Result`） |
| `InklogConfigAdapter` | `Config` | 无（默认可用） | 基于 `InklogConfig` 的链式键路径访问 |
| `DbNexusAdapter` | `Database` | `sqlite`/`postgres`/`mysql`/`duckdb` 之一 | 连接池管理（dbnexus），`pool_status() -> PoolStatus` 透传池快照 |
| `InklogModule` | trait-kit 模块 | `kit` + 数据库后端 | trait-kit 生命周期与可观测集成 |
| `InklogAuditStorage` | dbnexus AuditStorage 端口 | `dbnexus-audit` | 审计事件经 inklog DB Sink 落库（路径 `inklog::integrations::InklogAuditStorage`） |

## 💡 示例参考

`examples/` crate 提供 7 类共 39 个可运行示例（完整清单见 [README](../README.md#-示例)），运行方式：

```bash
cargo run --package inklog-examples --example <名称>
```

与本文档对应的精选示例：

| 示例 | 对应 API |
|------|----------|
| `basic` | `LoggerManager::new` / `shutdown` / `get_health_status` |
| `builder` | `LoggerBuilder` 全套方法 |
| `config_file` | `InklogConfig` TOML 加载（`from_file` / `load`） |
| `env_overrides` | `INKLOG_*` 环境变量覆盖 |
| `rotation` | `FileSinkConfig` 轮转字段 |
| `ring_buffered_file` | `ChannelBufferedFileSink` |
| `metrics` | `Metrics` / `HealthStatus` / `export_prometheus` |
| `circuit_breaker` | `CircuitBreaker` / `recover_sink` |
| `di_example` | `LoggerDependencies` 注入（需 `sqlite`） |
| `runtime_ops` | `set_level` / `publish_ops_event` 等运行时 API |

---

**[⬆ 返回顶部](#-inklog-api-参考)**
