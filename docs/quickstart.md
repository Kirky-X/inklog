# 🚀 inklog 快速开始指南

## 简介

inklog 是一个企业级 Rust 日志基础设施，提供高性能、高可靠、可扩展的日志记录能力。

## 特性

- **零丢失**: 有界 Channel + 背压阻塞 + 优雅关闭
- **高性能**: 异步架构，Console 延迟 <50μs
- **多输出**: Console、File、Database、S3 归档
- **安全**: AES-256-GCM 加密、敏感信息过滤
- **可观测**: HTTP 监控端点、Prometheus 指标

## 快速集成

### 1. 添加依赖

```toml
[dependencies]
inklog = "0.1"
```

### 2. 最简使用

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 零配置，直接使用
    let _logger = LoggerManager::new().await?;
    
    tracing::info!("Hello, inklog!");
    Ok(())
}
```

### 3. 使用配置文件

需要配置文件功能，启用 `confers` 特性：

```toml
[dependencies]
inklog = { version = "0.1", features = ["confers"] }
```

创建 `inklog.toml` 配置文件：

```toml
[global]
level = "info"
format = "{timestamp} [{level}] {target} - {message}"

[console_sink]
enabled = true
colored = true

[file_sink]
enabled = true
path = "logs/app.log"
max_size = "100MB"
rotation_time = "daily"
keep_files = 30
compress = true
encrypt = true
encryption_key_env = "LOG_ENCRYPTION_KEY"

[performance]
channel_capacity = 10000
worker_threads = 3
```

加载配置：

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::from_file("inklog.toml").await?;
    Ok(())
}
```

### 4. Builder 模式

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::builder()
        .level("debug")
        .enable_console(true)
        .enable_file("logs/app.log")
        .file_max_size("100MB")
        .file_compress(true)
        .file_encrypt(true, "LOG_ENCRYPTION_KEY")
        .channel_capacity(5000)
        .build()
        .await?;
    
    tracing::info!("Logger configured with Builder pattern");
    Ok(())
}
```

## 高级配置

### 环境变量配置

```bash
export INKLOG_GLOBAL_LEVEL=debug
export INKLOG_CONSOLE_SINK_ENABLED=true
export INKLOG_FILE_SINK_ENABLED=true
export INKLOG_FILE_SINK_PATH=/var/log/myapp/app.log
export INKLOG_DATABASE_SINK_ENABLED=true
export INKLOG_DATABASE_SINK_URL=postgres://localhost/logs
export LOG_ENCRYPTION_KEY="your-base64-key"
```

### 数据库配置

```toml
[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/logs"
batch_size = 100
archive_to_s3 = true
archive_after_days = 30

[parquet_config]
compression_level = 3
encoding = "PLAIN"
max_row_group_size = 10000
```

### S3 归档配置

```toml
[archive]
enabled = true
bucket = "my-logs-archive"
region = "us-east-1"
archive_interval_days = 7
schedule_expression = "0 2 * * *"  # 每天凌晨2点
local_retention_days = 30
compression = "zstd"
storage_class = "standard_ia"
```

### HTTP 监控

```toml
[http_server]
enabled = true
host = "0.0.0.0"
port = 8080
health_path = "/health"
metrics_path = "/metrics"
```

访问健康检查：`http://localhost:8080/health`
访问指标端点：`http://localhost:8080/metrics`

## 常用日志宏

```rust
use tracing::{info, warn, error, debug, trace};

// 普通日志
info!("User {} logged in", user_id);
warn!("Rate limit approaching");
error!("Connection failed: {}", err);

// 结构化日志
info!(user_id = 123, action = "login", "User logged in");
debug!(target = "database", "Query executed in {}ms", duration);

// 条件日志
if enabled {
    info!("Feature enabled");
}
```

## 性能基准

| 指标 | 目标值 | 实测值 |
|------|--------|--------|
| Console 延迟 | <50μs | ~1μs |
| 吞吐量 | 500条/秒 | ~3.6M ops/s |
| 内存占用 | <30MB | ~15MB |

## 下一步

- [配置参考手册](config-reference.md) - 完整配置选项
- [故障排查指南](troubleshooting.md) - 常见问题解决
- [示例代码](../examples/) - 完整使用示例
