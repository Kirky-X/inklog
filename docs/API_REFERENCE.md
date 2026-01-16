<div align="center">

# 📚 API 参考

Inklog 的完整 API 文档

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🏗️ 架构](ARCHITECTURE.md) • [📋 配置参考](config-reference.md)

---

</div>

## 目录

- [LoggerManager](#loggermanager)
- [InklogConfig](#inklogconfig)
- [错误处理](#错误处理)
- [指标和健康监控](#指标和健康监控)
- [常用模式](#常用模式)

---

## LoggerManager

日志记录系统的主要入口点。

```rust
pub struct LoggerManager {
    // 私有字段
}
```

### 构造函数

```rust
impl LoggerManager {
    /// 使用默认配置创建新的日志管理器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use inklog::LoggerManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let _logger = LoggerManager::new().await?;
    ///     log::info!("应用启动");
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> Result<Self, InklogError>

    /// 使用自定义配置创建日志管理器
    ///
    /// # 参数
    ///
    /// * `config` - 日志配置实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use inklog::{LoggerManager, InklogConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = InklogConfig::default();
    ///     let _logger = LoggerManager::with_config(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_config(config: InklogConfig) -> Result<Self, InklogError>

    /// 使用构建器模式创建日志管理器
    pub fn builder() -> LoggerBuilder

    /// 构建分离的日志记录器（不设置全局订阅者）
    pub async fn build_detached(config: InklogConfig) -> Result<(Self, Subscriber, EnvFilter), InklogError>
}
```

### 方法

```rust
impl LoggerManager {
    /// 获取当前健康状态
    ///
    /// # 返回
    ///
    /// `HealthStatus` 结构体，包含各 sink 的健康状态
    ///
    /// # 示例
    ///
    /// ```rust
    /// use inklog::LoggerManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let logger = LoggerManager::new().await?;
    ///     let health = logger.get_health_status();
    ///     println!("Status: {:?}", health.status);
    ///     Ok(())
    /// }
    /// ```
    pub fn get_health_status(&self) -> HealthStatus

    /// 获取指标快照
    ///
    /// # 返回
    ///
    /// `MetricsSnapshot` 结构体，包含当前指标数据
    pub fn get_metrics(&self) -> MetricsSnapshot

    /// 启动 HTTP 服务器（如果启用了 http 功能）
    #[cfg(feature = "http")]
    pub async fn start_http_server(&self, config: &HttpServerConfig) -> Result<(), InklogError>

    /// 启动归档服务（如果启用了 aws 功能）
    #[cfg(feature = "aws")]
    pub async fn start_archive_service(&self) -> Result<(), InklogError>

    /// 停止归档服务
    #[cfg(feature = "aws")]
    pub async fn stop_archive_service(&self) -> Result<(), InklogError>

    /// 触发手动归档
    #[cfg(feature = "aws")]
    pub async fn trigger_archive(&self) -> Result<(), InklogError>
}
```

---

## InklogConfig

根配置结构，包含所有配置选项。

```rust
pub struct InklogConfig {
    pub global: GlobalConfig,
    pub console_sink: Option<ConsoleSinkConfig>,
    pub file_sink: Option<FileSinkConfig>,
    pub database_sink: Option<DatabaseSinkConfig>,
    pub s3_archive: Option<S3ArchiveConfig>,
    pub performance: PerformanceConfig,
    pub http_server: Option<HttpServerConfig>,
}
```

### 默认配置

```rust
impl Default for InklogConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            console_sink: Some(ConsoleSinkConfig::default()),
            file_sink: None,
            database_sink: None,
            s3_archive: None,
            performance: PerformanceConfig::default(),
            http_server: None,
        }
    }
}
```

### 配置方法

```rust
impl InklogConfig {
    /// 从 TOML 文件加载配置
    #[cfg(feature = "confers")]
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, InklogError>

    /// 从默认位置加载配置
    #[cfg(feature = "confers")]
    pub fn load() -> Result<Self, InklogError>

    /// 加载并监控配置文件变化
    #[cfg(feature = "confers")]
    pub fn load_with_watch() -> Result<(Self, PathBuf, tokio::sync::mpsc::Receiver<PathBuf>), InklogError>

    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), InklogError>

    /// 应用环境变量覆盖
    pub fn apply_env_overrides(&mut self)

    /// 获取已启用的 sink 列表（用于审计日志，不包含敏感信息）
    pub fn sinks_enabled(&self) -> Vec<&'static str>
}
```

### 全局配置

```rust
pub struct GlobalConfig {
    pub level: String,              // 默认: "info"
    pub format: String,             // 默认: "{timestamp} [{level}] {target} - {message}"
    pub masking_enabled: bool,      // 默认: true
}
```

### 控制台配置

```rust
pub struct ConsoleSinkConfig {
    pub enabled: bool,              // 默认: true
    pub colored: bool,              // 默认: true
    pub stderr_levels: Vec<String>, // 默认: ["error", "warn"]
}
```

### 文件配置

```rust
pub struct FileSinkConfig {
    pub enabled: bool,
    pub path: PathBuf,                       // 默认: "logs/app.log"
    pub max_size: String,                    // 默认: "100MB"
    pub rotation_time: String,               // 默认: "daily"
    pub keep_files: u32,                     // 默认: 30
    pub compress: bool,                      // 默认: true
    pub compression_level: i32,              // 默认: 3
    pub encrypt: bool,                       // 默认: false
    pub encryption_key_env: Option<String>,
    pub retention_days: u32,                 // 默认: 30
    pub max_total_size: String,              // 默认: "1GB"
    pub cleanup_interval_minutes: u64,       // 默认: 60
}
```

### 数据库配置

```rust
pub struct DatabaseSinkConfig {
    pub enabled: bool,
    pub driver: DatabaseDriver,              // 默认: PostgreSQL
    pub url: String,
    pub pool_size: u32,                      // 默认: 10
    pub batch_size: usize,                   // 默认: 100
    pub flush_interval_ms: u64,              // 默认: 500
    pub archive_to_s3: bool,
    pub archive_after_days: u32,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub table_name: String,                  // 默认: "logs"
    pub archive_format: String,              // 默认: "json"
    pub parquet_config: ParquetConfig,
}

pub enum DatabaseDriver {
    PostgreSQL,
    MySQL,
    SQLite,
}
```

### 性能配置

```rust
pub struct PerformanceConfig {
    pub channel_capacity: usize,   // 默认: 10000
    pub worker_threads: usize,     // 默认: 3
}
```

### HTTP 服务器配置

```rust
pub struct HttpServerConfig {
    pub enabled: bool,             // 默认: false
    pub host: String,              // 默认: "127.0.0.1"
    pub port: u16,                 // 默认: 9090
    pub metrics_path: String,      // 默认: "/metrics"
    pub health_path: String,       // 默认: "/health"
    pub error_mode: HttpErrorMode, // 默认: Panic
}

pub enum HttpErrorMode {
    Panic,   // 启动失败时 panic
    Warn,    // 启动失败时记录警告
    Strict,  // 启动失败时返回错误
}
```

---

## 错误处理

### InklogError

```rust
#[derive(Error, Debug)]
pub enum InklogError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Logger error: {0}")]
    LoggerError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Health check failed: {0}")]
    HealthCheckError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
```

### 错误处理模式

```rust
use inklog::{LoggerManager, InklogError};

#[tokio::main]
async fn main() {
    match LoggerManager::new().await {
        Ok(logger) => {
            log::info!("Logger initialized successfully");
            // 使用 logger
        }
        Err(InklogError::ConfigError(msg)) => {
            eprintln!("Configuration error: {}", msg);
            std::process::exit(1);
        }
        Err(InklogError::IoError(msg)) => {
            eprintln!("IO error: {}", msg);
            // 处理文件/目录权限问题
        }
        Err(InklogError::DatabaseError(msg)) => {
            eprintln!("Database error: {}", msg);
            // 检查数据库连接配置
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        }
    }
}
```

---

## 指标和健康监控

### Metrics

```rust
pub struct Metrics {
    // 内部指标状态
}

impl Metrics {
    /// 创建新的指标实例
    pub fn new() -> Self

    /// 记录日志写入
    pub fn record_write(&self, sink: &str)

    /// 记录错误
    pub fn record_error(&self, sink: &str)

    /// 记录处理延迟（微秒）
    pub fn record_latency(&self, latency_us: u64)

    /// 获取当前指标快照
    pub fn snapshot(&self) -> MetricsSnapshot
}

pub struct MetricsSnapshot {
    pub total_records: u64,
    pub total_errors: u64,
    pub avg_latency_us: f64,
    pub sink_status: HashMap<String, bool>,
}
```

### HealthStatus

```rust
pub struct HealthStatus {
    pub status: HealthCheckStatus,
    pub sinks: HashMap<String, SinkHealth>,
    pub uptime_seconds: u64,
}

pub enum HealthCheckStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct SinkHealth {
    pub is_healthy: bool,
    pub error_count: u64,
    pub last_error: Option<String>,
}
```

### Prometheus 指标端点

当启用 HTTP 服务器时，可以访问以下指标：

```bash
# 获取所有指标
curl http://127.0.0.1:9090/metrics

# 获取特定指标
curl http://127.0.0.1:9090/metrics | grep inklog_records_total
```

示例指标：

```
# HELP inklog_records_total Total number of log records
# TYPE inklog_records_total counter
inklog_records_total{sink="console"} 1234
inklog_records_total{sink="file"} 5678

# HELP inklog_errors_total Total number of errors
# TYPE inklog_errors_total counter
inklog_errors_total{sink="database"} 5

# HELP inklog_latency_us Log processing latency in microseconds
# TYPE inklog_latency_us histogram
inklog_latency_us_bucket{le="100"} 1000
inklog_latency_us_bucket{le="500"} 1500
inklog_latency_us_bucket{le="1000"} 1600
```

---

## 常用模式

### 模式 1: 基本日志记录

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用默认配置初始化
    let _logger = LoggerManager::new().await?;

    // 开始日志记录
    log::trace!("Trace level message");
    log::debug!("Debug level message");
    log::info!("Info level message");
    log::warn!("Warning message");
    log::error!("Error message");

    Ok(())
}
```

### 模式 2: 自定义配置

```rust
use inklog::{LoggerManager, InklogConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = InklogConfig::default();

    // 修改全局配置
    config.global.level = "debug".to_string();
    config.global.masking_enabled = true;

    // 启用文件日志
    config.file_sink = Some(inklog::FileSinkConfig {
        enabled: true,
        path: std::path::PathBuf::from("logs/app.log"),
        max_size: "100MB".to_string(),
        rotation_time: "daily".to_string(),
        compress: true,
        ..Default::default()
    });

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("使用自定义配置的日志消息");

    Ok(())
}
```

### 模式 3: 高级配置

```rust
use inklog::{LoggerManager, InklogConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InklogConfig {
        global: inklog::GlobalConfig {
            level: "info".to_string(),
            format: "{timestamp} [{level}] {target} - {message}".to_string(),
            masking_enabled: true,
        },
        console_sink: Some(inklog::ConsoleSinkConfig {
            enabled: true,
            colored: true,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
        }),
        file_sink: Some(inklog::FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/var/log/myapp/app.log"),
            max_size: "100MB".to_string(),
            rotation_time: "daily".to_string(),
            keep_files: 30,
            compress: true,
            encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".to_string()),
            ..Default::default()
        }),
        performance: inklog::PerformanceConfig {
            channel_capacity: 10000,
            worker_threads: 4,
        },
        http_server: Some(inklog::HttpServerConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            port: 9090,
            ..Default::default()
        }),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("高级配置示例");

    Ok(())
}
```

### 模式 4: 结构化日志

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;

    // 简单结构化日志
    log::info!(user_id = 42, action = "login", "用户登录");

    // 多字段结构化日志
    log::info!(
        event = "request_complete",
        method = "GET",
        path = "/api/users",
        status = 200,
        duration_ms = 42,
        "HTTP 请求完成"
    );

    // 使用 ?
    let user_id = 123;
    log::info!(user_id, "User action completed");

    Ok(())
}
```

### 模式 5: 健康检查

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = LoggerManager::new().await?;

    // 获取健康状态
    let health = logger.get_health_status();
    println!("Status: {:?}", health.status);

    // 检查各 sink 状态
    for (sink, status) in &health.sinks {
        println!("{}: {}", sink, if status.is_healthy { "healthy" } else { "unhealthy" });
    }

    // 获取指标
    let metrics = logger.get_metrics();
    println!("Total records: {}", metrics.total_records);
    println!("Total errors: {}", metrics.total_errors);

    Ok(())
}
```

---

<div align="center">

**[📖 用户指南](USER_GUIDE.md)** • **[🏗️ 架构](ARCHITECTURE.md)** • **[📋 配置参考](config-reference.md)** • **[🏠 首页](../README.md)**

</div>
