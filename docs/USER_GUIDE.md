# Inklog 用户指南

本文档提供了 Inklog 企业级 Rust 日志基础设施的完整使用指南。

---

## 目录

- [项目概述](#项目概述)
- [核心特性](#核心特性)
- [安装](#安装)
- [快速开始](#快速开始)
  - [基础日志记录](#基础日志记录)
  - [文件日志](#文件日志)
  - [数据库日志](#数据库日志)
- [配置详解](#配置详解)
  - [基础配置结构](#基础配置结构)
  - [文件输出配置](#文件输出配置)
  - [数据库输出配置](#数据库输出配置)
  - [数据脱敏配置](#数据脱敏配置)
  - [HTTP 服务器配置](#http-服务器配置)
  - [性能配置](#性能配置)
- [常用示例](#常用示例)
  - [基本日志记录](#基本日志记录)
  - [文件日志与轮转](#文件日志与轮转)
  - [加密日志](#加密日志)
  - [数据库日志](#数据库日志)
  - [自定义格式化](#自定义格式化)
  - [HTTP 健康检查](#http-健康检查)
- [高级主题](#高级主题)
  - [异步性能优化](#异步性能优化)
  - [多目标输出](#多目标输出)
  - [批量写入](#批量写入)
  - [Sink 故障恢复](#sink-故障恢复)
- [环境变量配置](#环境变量配置)
- [最佳实践](#最佳实践)
- [故障排除](#故障排除)
  - [日志未输出](#日志未输出)
  - [文件轮转问题](#文件轮转问题)
  - [数据库连接失败](#数据库连接失败)
  - [加密日志无法读取](#加密日志无法读取)
- [常见问题](#常见问题)
- [相关资源](#相关资源)

---

## 项目概述

Inklog 是一个为 Rust 企业级应用设计的高性能、安全、功能丰富的日志基础设施库。

### 设计理念

- **高性能**：基于 Tokio 的异步 I/O，支持批量写入和压缩
- **安全优先**：AES-256-GCM 加密、数据脱敏、安全密钥管理
- **灵活配置**：多目标输出（控制台、文件、数据库）
- **可观测性**：健康监控、Prometheus 指标导出
- **生产就绪**：自动轮转、故障恢复、优雅关闭

### 架构概览

```
应用代码 (log! 宏)
         ↓
Inklog API 层 (LoggerManager)
         ↓
Sink 抽象层 (Console, File, Database)
         ↓
核心处理层 (格式化、脱敏、加密、压缩)
         ↓
存储与外部服务 (文件系统、数据库)
```

---

## 核心特性

### 核心功能（始终可用）

| 功能 | 描述 |
|------|------|
| **异步 I/O** | Tokio 驱动的非阻塞日志记录 |
| **多目标输出** | 同时输出到控制台、文件、数据库等多个目标 |
| **结构化日志** | tracing 生态系统集成 |
| **自定义格式** | 基于模板的日志格式化 |
| **文件轮转** | 基于大小和时间的自动轮转 |
| **数据脱敏** | 基于 PII 模式的正则表达式脱敏 |
| **健康监控** | Sink 状态和指标追踪 |
| **命令行工具** | decrypt、generate、validate 命令 |

### 企业功能

| 功能 | 描述 |
|------|------|
| **压缩** | ZSTD、GZIP、Brotli、LZ4 支持 |
| **加密** | AES-256-GCM 文件加密 |
| **数据库 Sink** | PostgreSQL、MySQL、SQLite (Sea-ORM) |
| **Parquet 导出** | 分析就绪的日志格式 |
| **HTTP 端点** | Axum 健康检查服务器 |

---

## 安装

### 使用 Cargo

将以下内容添加到 `Cargo.toml` 文件：

```toml
[dependencies]
inklog = "0.1"
```

### 功能标志

默认包含 `http`、`cli` 功能：

```toml
inklog = { version = "0.1", features = ["default"] }
```

### 可选功能

```toml
# HTTP 服务器
inklog = { version = "0.1", features = ["http"] }

# 命令行工具
inklog = { version = "0.1", features = ["cli"] }

# 外部配置支持
inklog = { version = "0.1", features = ["confers"] }

# 完整功能
inklog = { version = "0.1", features = ["http", "cli", "confers"] }
```

---

## 快速开始

### 基础日志记录

最简单的使用方式，使用默认配置初始化日志系统：

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用默认配置初始化日志系统
    let _logger = LoggerManager::new().await?;

    // 记录不同级别的日志消息
    log::trace!("这是一条追踪消息");
    log::debug!("这是一条调试消息");
    log::info!("这是一条信息消息");
    log::warn!("这是一条警告消息");
    log::error!("这是一条错误消息");

    Ok(())
}
```

### 文件日志

配置文件日志输出，支持自动轮转和压缩：

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_path: PathBuf = "logs/app.log".into();

    // 确保日志目录存在
    std::fs::create_dir_all("logs")?;

    // 配置文件 Sink
    let file_config = FileSinkConfig {
        enabled: true,
        path: log_path,
        max_size: "10MB".into(),       // 文件达到 10MB 时轮转
        rotation_time: "daily".into(), // 每天轮转
        keep_files: 7,                 // 保留 7 个轮转文件
        compress: true,                // 使用 ZSTD 压缩轮转文件
        encrypt: false,                // 不加密
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("应用已启动");
    log::error!("发生错误: {}", "详情");

    Ok(())
}
```

### 数据库日志

将日志输出到数据库，支持批量写入：

```rust
use inklog::config::DatabaseDriver;
use inklog::{DatabaseSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_sink = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: "sqlite://logs/app.db".to_string(),
        pool_size: 5,
        batch_size: 100,           // 每批写入 100 条日志
        flush_interval_ms: 1000,   // 每秒刷新一次
        table_name: "logs".to_string(),
        archive_format: "json".to_string(),
        parquet_config: inklog::config::ParquetConfig::default(),
    };

    let config = InklogConfig {
        database_sink: Some(database_sink),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("日志已写入数据库");

    Ok(())
}
```

---

## 配置详解

### 基础配置结构

`InklogConfig` 是根配置结构，包含所有子配置：

```rust
use inklog::InklogConfig;

let config = InklogConfig {
    global: inklog::config::GlobalConfig {
        level: "info".into(),
        format: "{timestamp} [{level}] {target} - {message}".to_string(),
        masking_enabled: true,
    },
    console_sink: Some(inklog::config::ConsoleSinkConfig::default()),
    file_sink: None,
    database_sink: None,
    performance: inklog::config::PerformanceConfig::default(),
    http_server: None,
};
```

#### 全局配置（GlobalConfig）

| 字段 | 类型 | 默认值 | 描述 |
|------|------|----------|------|
| `level` | `String` | `"info"` | 日志级别：`trace`、`debug`、`info`、`warn`、`error` |
| `format` | `String` | `"{timestamp} [{level}] {target} - {message}"` | 日志格式模板 |
| `masking_enabled` | `bool` | `true` | 是否启用数据脱敏 |

**可用的格式变量：**
- `{timestamp}` - 时间戳
- `{level}` - 日志级别
- `{target}` - 日志目标（模块/文件）
- `{message}` - 日志消息
- `{file}` - 源文件名
- `{line}` - 源代码行号
- `{thread_id}` - 线程 ID

---

### 文件输出配置

#### FileSinkConfig 字段说明

| 字段 | 类型 | 默认值 | 描述 |
|------|------|----------|------|
| `enabled` | `bool` | `true` | 是否启用文件 Sink |
| `path` | `PathBuf` | `"logs/app.log"` | 日志文件路径 |
| `max_size` | `String` | `"100MB"` | 触发轮转的最大文件大小（如：`"10MB"`、`"500KB"`） |
| `rotation_time` | `String` | `"daily"` | 时间轮转策略：`"hourly"`、`"daily"`、`"weekly"` |
| `keep_files` | `u32` | `30` | 保留的轮转文件数量 |
| `compress` | `bool` | `true` | 是否压缩轮转文件 |
| `compression_level` | `i32` | `3` | 压缩级别（0-22，数值越高压缩率越高） |
| `encrypt` | `bool` | `false` | 是否加密日志文件 |
| `encryption_key_env` | `Option<String>` | `None` | 加密密钥的环境变量名 |
| `retention_days` | `u32` | `30` | 日志保留天数 |
| `max_total_size` | `String` | `"1GB"` | 日志目录最大总大小 |
| `cleanup_interval_minutes` | `u64` | `60` | 清理旧日志的间隔（分钟） |

#### 文件轮转示例

```rust
let file_config = FileSinkConfig {
    enabled: true,
    path: "logs/app.log".into(),
    max_size: "50MB".into(),        // 达到 50MB 时轮转
    rotation_time: "daily".into(),   // 每天轮转
    keep_files: 14,                  // 保留 14 天的日志
    compress: true,                   // 压缩旧日志
    ..Default::default()
};
```

#### 轮转文件命名

轮转后的文件将自动重命名，格式为：

```
app.log           # 当前日志文件
app.log.1        # 第 1 个轮转文件
app.log.2        # 第 2 个轮转文件
...
app.log.gz        # 压缩后的轮转文件（如果启用压缩）
```

---

### 数据库输出配置

#### DatabaseSinkConfig 字段说明

| 字段 | 类型 | 默认值 | 描述 |
|------|------|----------|------|
| `enabled` | `bool` | `false` | 是否启用数据库 Sink |
| `driver` | `DatabaseDriver` | `PostgreSQL` | 数据库驱动：`PostgreSQL`、`MySQL`、`SQLite` |
| `url` | `String` | `"postgres://localhost/logs"` | 数据库连接 URL |
| `pool_size` | `u32` | `10` | 连接池大小 |
| `batch_size` | `usize` | `100` | 批量写入的日志数量 |
| `flush_interval_ms` | `u64` | `500` | 刷新间隔（毫秒） |
| `table_name` | `String` | `"logs"` | 日志表名 |
| `archive_format` | `String` | `"json"` | 归档格式：`"json"` 或 `"parquet"` |
| `parquet_config` | `ParquetConfig` | `default()` | Parquet 导出配置 |

#### 数据库驱动类型

| 驱动 | 字符串表示 | URL 示例 |
|------|------------|----------|
| `PostgreSQL` | `"postgres"` | `postgres://user:pass@localhost/logs` |
| `MySQL` | `"mysql"` | `mysql://user:pass@localhost/logs` |
| `SQLite` | `"sqlite"` | `sqlite://logs/app.db` |

#### 数据库日志示例

```rust
use inklog::config::DatabaseDriver;
use inklog::{DatabaseSinkConfig, InklogConfig, LoggerManager};

let database_sink = DatabaseSinkConfig {
    enabled: true,
    driver: DatabaseDriver::PostgreSQL,
    url: "postgresql://user:password@localhost:5432/logs".to_string(),
    pool_size: 10,
    batch_size: 100,
    flush_interval_ms: 1000,
    ..Default::default()
};

let config = InklogConfig {
    database_sink: Some(database_sink),
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;
```

---

### 数据脱敏配置

Inklog 支持自动脱敏敏感个人信息（PII）数据。

#### 脱敏模式

| 数据类型 | 脱敏前 | 脱敏后 |
|---------|---------|---------|
| 邮箱地址 | `user@example.com` | `***@***.***` |
| 身份证号 | `110101199001011234` | `**************1234` |
| 信用卡号 | `4111111111111111` | `************1111` |

#### 启用数据脱敏

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "info".into(),
        format: "{timestamp} {level} {message}".to_string(),
        masking_enabled: true,  // 启用数据脱敏
        ..Default::default()
    },
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;

// 敏感数据将自动脱敏
log::info!("用户邮箱: user@example.com");
// 输出: 用户邮箱: ***@***.***
```

#### 自定义脱敏模式

如需自定义脱敏模式，请参考 `src/masking.rs` 模块的实现，并扩展正则表达式模式。

---

### HTTP 服务器配置

#### HttpServerConfig 字段说明

| 字段 | 类型 | 默认值 | 描述 |
|------|------|----------|------|
| `enabled` | `bool` | `false` | 是否启用 HTTP 服务器 |
| `host` | `String` | `"127.0.0.1"` | 监听主机地址 |
| `port` | `u16` | `9090` | 监听端口 |
| `metrics_path` | `String` | `"/metrics"` | Prometheus 指标端点路径 |
| `health_path` | `String` | `"/health"` | 健康检查端点路径 |
| `error_mode` | `HttpErrorMode` | `Panic` | 启动失败时的错误处理模式 |

#### 错误处理模式（HttpErrorMode）

| 模式 | 描述 |
|------|------|
| `Panic` | 启动失败时 panic（默认，向后兼容） |
| `Warn` | 启动失败时记录警告，系统继续运行 |
| `Strict` | 启动失败时返回错误，阻止系统启动 |

#### HTTP 服务器示例

```rust
use inklog::{InklogConfig, LoggerManager};

let config = InklogConfig {
    http_server: Some(inklog::config::HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 8080,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: inklog::config::HttpErrorMode::Panic,
    }),
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;

// 健康检查端点: http://0.0.0.0:8080/health
// 指标端点: http://0.0.0.0:8080/metrics
```

#### 端点说明

**健康检查端点（/health）**
返回 JSON 格式的健康状态：

```json
{
  "overall_status": {
    "Healthy": null
  },
  "sinks": {
    "console": {
      "status": {
        "Healthy": null
      },
      "last_error": null,
      "consecutive_failures": 0
    },
    "file": {
      "status": {
        "Healthy": null
      },
      "last_error": null,
      "consecutive_failures": 0
    }
  },
  "channel_usage": 0.1,
  "uptime_seconds": 1234,
  "metrics": {
    "logs_written": 1000,
    "logs_dropped": 0,
    "channel_blocked": 0,
    "sink_errors": 0,
    "avg_latency_us": 150,
    "latency_distribution": [10, 50, 100, 500, 2000],
    "active_workers": 3
  }
}
```

**Prometheus 指标端点（/metrics）**

返回 Prometheus 格式的指标：

```
# HELP inklog_logs_written_total Total logs successfully written
# TYPE inklog_logs_written_total counter
inklog_logs_written_total 1000

# HELP inklog_sink_errors_total Total sink errors
# TYPE inklog_sink_errors_total counter
inklog_sink_errors_total 0

# HELP inklog_avg_latency_us Average log processing latency in microseconds
# TYPE inklog_avg_latency_us gauge
inklog_avg_latency_us 150
```

---

### 性能配置

#### PerformanceConfig 字段说明

| 字段 | 类型 | 默认值 | 描述 |
|------|------|----------|------|
| `channel_capacity` | `usize` | `10000` | 日志通道容量 |
| `worker_threads` | `usize` | `3` | 工作线程数 |

#### 性能调优示例

```rust
use inklog::{InklogConfig, config::PerformanceConfig};

let config = InklogConfig {
    performance: PerformanceConfig {
        channel_capacity: 20000,  // 增加通道容量
        worker_threads: 4,          // 增加工作线程
    },
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;
```

#### 性能优化建议

**高吞吐量场景**
```toml
[performance]
channel_capacity = 50000   # 增加通道容量
worker_threads = 8          # 增加工作线程
```

**低延迟场景**
```toml
[performance]
channel_capacity = 1000    # 减少通道容量，更快刷新
worker_threads = 2          # 减少线程，降低上下文切换
```

---

## 常用示例

### 基本日志记录

完整示例展示如何记录不同级别的日志：

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;

    // 记录不同级别的日志
    log::trace!("追踪信息：函数入口参数 = {}", args);
    log::debug!("调试信息：变量值 = {}", variable);
    log::info!("信息消息：应用已启动");
    log::warn!("警告消息：配置文件使用默认值");
    log::error!("错误消息：无法连接到数据库");

    // 使用 target 指定日志来源
    log::info!(target: "auth", "用户登录: {}", username);
    log::info!(target: "database", "查询执行: {}", query);

    Ok(())
}
```

### 文件日志与轮转

配置文件日志，包含轮转和压缩：

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("logs")?;

    let file_config = FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        max_size: "10MB".into(),
        rotation_time: "daily".into(),
        keep_files: 7,
        compress: true,
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // 模拟日志生成
    for i in 0..1000 {
        log::info!("日志消息 #{}", i);
    }

    Ok(())
}
```

### 加密日志

使用 AES-256-GCM 加密日志文件：

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 生成 32 字节的加密密钥（Base64 编码）
    let encryption_key = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=";

    // 通过环境变量设置密钥
    std::env::set_var("INKLOG_ENCRYPTION_KEY", encryption_key);

    std::fs::create_dir_all("logs")?;

    let file_config = FileSinkConfig {
        enabled: true,
        path: "logs/encrypted.log.enc".into(),
        max_size: "10MB".into(),
        rotation_time: "daily".into(),
        keep_files: 5,
        compress: false,           // 加密日志不压缩
        encrypt: true,             // 启用加密
        encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // 记录敏感数据
    log::info!("用户密码已加密");
    log::warn!("敏感操作：支付处理");
    log::error!("安全事件：未授权访问尝试");

    // 清理环境变量
    std::env::remove_var("INKLOG_ENCRYPTION_KEY");

    Ok(())
}
```

### 数据库日志

将日志写入 SQLite 数据库：

```rust
use inklog::config::DatabaseDriver;
use inklog::{DatabaseSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("logs")?;

    let database_sink = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: "sqlite://logs/app.db".to_string(),
        pool_size: 5,
        batch_size: 100,
        flush_interval_ms: 1000,
        table_name: "logs".to_string(),
        ..Default::default()
    };

    let config = InklogConfig {
        database_sink: Some(database_sink),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    // 记录日志
    for i in 0..50 {
        log::info!(target: "app", "数据库日志 #{}", i);
    }

    Ok(())
}
```

### 自定义格式化

创建自定义日志格式：

```rust
use inklog::{InklogConfig, config::GlobalConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 自定义格式字符串
    let format_string = "[{timestamp}] [{level:>5}] {target} - {message} | {file}:{line}";

    let config = InklogConfig {
        global: GlobalConfig {
            level: "debug".into(),
            format: format_string.to_string(),
            masking_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("使用自定义格式的日志");
    log::debug!("包含文件位置信息");

    Ok(())
}
```

### HTTP 健康检查

集成 HTTP 健康检查端点：

```rust
use axum::{routing::get, Json, Router};
use inklog::LoggerManager;
use std::sync::Arc;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Arc::new(LoggerManager::new().await?);

    let app = Router::new()
        .route(
            "/health",
            get({
                let logger = logger.clone();
                || async move { Json(logger.get_health_status()) }
            }),
        )
        .route(
            "/metrics",
            get({
                let logger = logger.clone();
                || async move { logger.get_health_status().metrics.export_prometheus() }
            }),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1]), 3000);
    println!("健康检查服务启动: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::Server::bind(&listener.local_addr()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### 可运行示例（cargo run --example）

除上述内联示例外，`examples/` crate 还提供了 10 个聚焦特定主题的可运行示例。在 `examples/` 目录下执行 `cargo run --example <名称>`，或在工作区根目录使用 `cargo run --package inklog-examples --example <名称>` 运行。

| 示例 | 主题 | 运行命令 |
|------|------|----------|
| `object_pool` | 对象池复用，减少高频路径上的内存分配 | `cargo run --example object_pool` |
| `path_validator` | 文件 Sink 路径校验，防止越权写入 | `cargo run --example path_validator` |
| `log_sanitizer` | 日志输入净化，防止日志注入与控制字符污染 | `cargo run --example log_sanitizer` |
| `log_adapter` | `log` 与 `tracing` 生态桥接适配器 | `cargo run --example log_adapter` |
| `compression` | 文件 Sink 压缩（ZSTD/GZIP/Brotli/LZ4）对比 | `cargo run --example compression` |
| `rotation` | 基于大小和时间的文件轮转策略 | `cargo run --example rotation` |
| `ring_buffered_file` | 环形缓冲文件 Sink，适用于高吞吐场景 | `cargo run --example ring_buffered_file` |
| `config_file` | TOML 配置文件加载（需启用 `confers` feature） | `cargo run --example config_file` |
| `metrics` | 健康指标采集与 Prometheus 格式导出 | `cargo run --example metrics` |
| `circuit_breaker` | Sink 断路器与故障自动恢复 | `cargo run --example circuit_breaker` |

> 提示：部分示例（如 `config_file`）需要启用对应 feature。运行前请参考 `examples/Cargo.toml` 中的 feature 配置。

---

## 高级主题

### 异步性能优化

Inklog 基于 Tokio 异步运行时构建，提供高性能日志记录。

#### 批量写入

数据库 Sink 自动批量写入日志，减少数据库操作次数：

```rust
let database_sink = DatabaseSinkConfig {
    enabled: true,
    batch_size: 1000,          // 增大批量大小
    flush_interval_ms: 5000,    // 延长刷新间隔
    ..Default::default()
};
```

#### 通道容量调整

调整日志通道容量以适应不同的日志负载：

```rust
use inklog::config::PerformanceConfig;

let performance = PerformanceConfig {
    channel_capacity: 50000,  // 高负载场景
    worker_threads: 8,
};
```

### 多目标输出

同时输出到多个目标：

```rust
let config = InklogConfig {
    console_sink: Some(inklog::config::ConsoleSinkConfig {
        enabled: true,
        colored: true,
        ..Default::default()
    }),
    file_sink: Some(inklog::FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        ..Default::default()
    }),
    database_sink: Some(inklog::DatabaseSinkConfig {
        enabled: true,
        url: "sqlite://logs/app.db".to_string(),
        ..Default::default()
    }),
    ..Default::default()
};
```

### 批量写入

数据库 Sink 支持批量写入，提高性能：

```rust
// 日志消息被缓冲，直到满足以下条件之一：
// 1. 达到 batch_size 条数
// 2. 经过 flush_interval_ms 时间

let database_sink = DatabaseSinkConfig {
    enabled: true,
    batch_size: 500,          // 每 500 条写入
    flush_interval_ms: 2000,    // 或每 2 秒写入
    ..Default::default()
};
```

### Sink 故障恢复

Inklog 提供自动和手动故障恢复机制。

#### 自动恢复

当 Sink 连续失败超过阈值时，自动尝试恢复：

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = LoggerManager::new().await?;

    // Sink 会自动恢复
    log::info!("Sink 故障恢复已自动启用");

    Ok(())
}
```

#### 手动恢复

手动触发 Sink 恢复：

```rust
// 恢复特定 Sink
manager.recover_sink("file")?;

// 恢复所有不健康的 Sink
let recovered = manager.trigger_recovery_for_unhealthy_sinks()?;
log::info!("已恢复的 Sink: {:?}", recovered);
```

### 使用 Mock 实现进行测试

Inklog 提供了完整的 Mock 实现，用于单元测试和集成测试，无需启动真实的基础设施服务。

#### 依赖注入测试模式

```rust
use inklog::{LoggerManager, LoggerDependencies};
use inklog::infrastructure::{MockCache, MockConfig, MockDatabaseAdapter};
use std::sync::Arc;

#[tokio::test]
async fn test_with_mocks() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Mock 依赖
    let cache = Arc::new(MockCache::new());
    let config = Arc::new(
        MockConfig::new()
            .with_value("level", "debug")
            .with_value("file_sink.enabled", "true")
    );
    let database = Arc::new(MockDatabaseAdapter::new());

    // 注入依赖
    let deps = LoggerDependencies {
        cache: Some(cache.clone()),
        config: Some(config.clone()),
        database: Some(database.clone()),
    };

    let logger = LoggerManager::with_dependencies(deps).await?;

    // 使用 logger 进行测试...
    log::info!("Test message");

    // 验证日志被写入 Mock 数据库
    let records = database.get_records();
    assert_eq!(records.len(), 1);

    Ok(())
}
```

#### MockCache 使用示例

```rust
use inklog::infrastructure::MockCache;

#[tokio::test]
async fn test_cache_operations() {
    let cache = MockCache::new();

    // 设置缓存
    cache.set("key1", "value1".to_string()).await;
    cache.set("key2", "value2".to_string()).await;

    // 获取缓存
    assert_eq!(cache.get("key1").await, Some("value1".to_string()));

    // 检查存在性
    assert!(cache.exists("key1").await);
    assert!(!cache.exists("nonexistent").await);

    // 删除缓存
    assert!(cache.delete("key1").await);
    assert_eq!(cache.get("key1").await, None);

    // 延迟模拟（测试超时场景）
    let slow_cache = MockCache::with_delay(100); // 100ms 延迟
}
```

#### MockConfig 使用示例

```rust
use inklog::infrastructure::MockConfig;

#[test]
fn test_config_operations() {
    let config = MockConfig::new()
        .with_value("level", "debug")
        .with_value("port", "8080")
        .with_value("enabled", "true")
        .with_value("ratio", "3.14");

    // 获取各种类型的值
    assert_eq!(config.get_string("level"), Some("debug".to_string()));
    assert_eq!(config.get_int("port"), Some(8080));
    assert_eq!(config.get_bool("enabled"), Some(true));
    assert_eq!(config.get_float("ratio"), Some(3.14));

    // 获取不存在的键
    assert_eq!(config.get_string("nonexistent"), None);

    // 运行时修改配置（测试动态配置场景）
    config.set("level", "error");
    assert_eq!(config.get_string("level"), Some("error".to_string()));
}
```

#### MockDatabaseAdapter 使用示例

```rust
use inklog::infrastructure::MockDatabaseAdapter;
use inklog::LogRecord;
use chrono::Utc;

#[tokio::test]
async fn test_database_operations() {
    let db = MockDatabaseAdapter::new();

    // 创建测试日志记录
    let records = vec![
        LogRecord::new("info".to_string(), "Test message 1".to_string()),
        LogRecord::new("error".to_string(), "Test message 2".to_string()),
    ];

    // 批量插入
    let count = db.insert_batch(&records).await.unwrap();
    assert_eq!(count, 2);

    // 验证记录
    let stored = db.get_records();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].message, "Test message 1");

    // 健康状态控制（测试降级场景）
    db.set_healthy(false);
    assert!(!db.is_healthy().await);

    // 恢复健康
    db.set_healthy(true);
    assert!(db.is_healthy().await);

    // 清空记录（测试隔离）
    db.clear();
    assert_eq!(db.get_records().len(), 0);
}
```

#### 混合模式（部分注入）

```rust
use inklog::{LoggerManager, LoggerDependencies};
use inklog::infrastructure::MockDatabaseAdapter;
use std::sync::Arc;

#[tokio::test]
async fn test_mixed_mode() {
    // 只注入数据库，其他使用默认实现
    let deps = LoggerDependencies {
        cache: None,              // 使用默认 OxCacheAdapter
        config: None,             // 使用默认配置
        database: Some(Arc::new(MockDatabaseAdapter::new())),
    };

    let logger = LoggerManager::with_dependencies(deps).await?;
    // 日志会写入 Mock 数据库，但缓存和配置使用生产实现
}
```

#### 测试隔离最佳实践

```rust
use inklog::infrastructure::{MockCache, MockConfig, MockDatabaseAdapter};

struct TestContext {
    cache: Arc<MockCache>,
    config: Arc<MockConfig>,
    database: Arc<MockDatabaseAdapter>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            cache: Arc::new(MockCache::new()),
            config: Arc::new(MockConfig::new()),
            database: Arc::new(MockDatabaseAdapter::new()),
        }
    }

    async fn create_logger(&self) -> Result<LoggerManager, InklogError> {
        let deps = LoggerDependencies {
            cache: Some(self.cache.clone()),
            config: Some(self.config.clone()),
            database: Some(self.database.clone()),
        };
        LoggerManager::with_dependencies(deps).await
    }

    fn reset(&self) {
        self.cache.delete_all().await;  // 清空缓存
        self.database.clear();           // 清空数据库
    }
}

#[tokio::test]
async fn test_isolated() {
    let ctx = TestContext::new();
    let logger = ctx.create_logger().await?;

    // 执行测试...

    ctx.reset();  // 重置状态，不影响其他测试
}
```

---

## 环境变量配置

Inklog 支持通过环境变量覆盖配置。

### 全局配置变量

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_LEVEL` | 日志级别 | `INKLOG_LEVEL=debug` |
| `INKLOG_FORMAT` | 日志格式 | `INKLOG_FORMAT="{timestamp} {message}"` |
| `INKLOG_MASKING_ENABLED` | 启用数据脱敏 | `INKLOG_MASKING_ENABLED=true` |

### 控制台 Sink 配置变量

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_CONSOLE_ENABLED` | 启用控制台 Sink | `INKLOG_CONSOLE_ENABLED=true` |
| `INKLOG_CONSOLE_COLORED` | 启用彩色输出 | `INKLOG_CONSOLE_COLORED=true` |
| `INKLOG_CONSOLE_STDERR_LEVELS` | 输出到 stderr 的日志级别 | `INKLOG_CONSOLE_STDERR_LEVELS=error,warn` |

### 文件 Sink 配置变量

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_FILE_ENABLED` | 启用文件 Sink | `INKLOG_FILE_ENABLED=true` |
| `INKLOG_FILE_PATH` | 日志文件路径 | `INKLOG_FILE_PATH=logs/app.log` |
| `INKLOG_FILE_MAX_SIZE` | 最大文件大小 | `INKLOG_FILE_MAX_SIZE=100MB` |
| `INKLOG_FILE_ROTATION_TIME` | 轮转时间策略 | `INKLOG_FILE_ROTATION_TIME=daily` |
| `INKLOG_FILE_KEEP_FILES` | 保留文件数 | `INKLOG_FILE_KEEP_FILES=30` |
| `INKLOG_FILE_COMPRESS` | 启用压缩 | `INKLOG_FILE_COMPRESS=true` |
| `INKLOG_FILE_COMPRESSION_LEVEL` | 压缩级别 | `INKLOG_FILE_COMPRESSION_LEVEL=5` |
| `INKLOG_FILE_ENCRYPT` | 启用加密 | `INKLOG_FILE_ENCRYPT=true` |
| `INKLOG_FILE_ENCRYPTION_KEY_ENV` | 加密密钥环境变量名 | `INKLOG_FILE_ENCRYPTION_KEY_ENV=LOG_KEY` |
| `INKLOG_FILE_RETENTION_DAYS` | 保留天数 | `INKLOG_FILE_RETENTION_DAYS=30` |
| `INKLOG_FILE_MAX_TOTAL_SIZE` | 最大总大小 | `INKLOG_FILE_MAX_TOTAL_SIZE=1GB` |
| `INKLOG_FILE_CLEANUP_INTERVAL_MINUTES` | 清理间隔 | `INKLOG_FILE_CLEANUP_INTERVAL_MINUTES=60` |

### 数据库 Sink 配置变量

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_DB_ENABLED` | 启用数据库 Sink | `INKLOG_DB_ENABLED=true` |
| `INKLOG_DB_DRIVER` | 数据库驱动 | `INKLOG_DB_DRIVER=postgres` |
| `INKLOG_DB_URL` | 数据库连接 URL | `INKLOG_DB_URL=postgres://localhost/logs` |
| `INKLOG_DB_POOL_SIZE` | 连接池大小 | `INKLOG_DB_POOL_SIZE=10` |
| `INKLOG_DB_TABLE_NAME` | 日志表名 | `INKLOG_DB_TABLE_NAME=logs` |
| `INKLOG_DB_BATCH_SIZE` | 批量大小 | `INKLOG_DB_BATCH_SIZE=100` |
| `INKLOG_DB_FLUSH_INTERVAL_MS` | 刷新间隔 | `INKLOG_DB_FLUSH_INTERVAL_MS=1000` |
| `INKLOG_DB_PARQUET_COMPRESSION_LEVEL` | Parquet 压缩级别 | `INKLOG_DB_PARQUET_COMPRESSION_LEVEL=3` |
| `INKLOG_DB_PARQUET_ENCODING` | Parquet 编码方式 | `INKLOG_DB_PARQUET_ENCODING=PLAIN` |
| `INKLOG_DB_PARQUET_MAX_ROW_GROUP_SIZE` | Parquet Row Group 大小 | `INKLOG_DB_PARQUET_MAX_ROW_GROUP_SIZE=10000` |
| `INKLOG_DB_PARQUET_MAX_PAGE_SIZE` | Parquet 页面大小 | `INKLOG_DB_PARQUET_MAX_PAGE_SIZE=1048576` |

### HTTP 服务器配置变量

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_HTTP_ENABLED` | 启用 HTTP 服务器 | `INKLOG_HTTP_ENABLED=true` |
| `INKLOG_HTTP_HOST` | 监听主机 | `INKLOG_HTTP_HOST=0.0.0.0` |
| `INKLOG_HTTP_PORT` | 监听端口 | `INKLOG_HTTP_PORT=8080` |
| `INKLOG_HTTP_METRICS_PATH` | 指标端点路径 | `INKLOG_HTTP_METRICS_PATH=/metrics` |
| `INKLOG_HTTP_HEALTH_PATH` | 健康检查路径 | `INKLOG_HTTP_HEALTH_PATH=/health` |
| `INKLOG_HTTP_ERROR_MODE` | 错误处理模式 | `INKLOG_HTTP_ERROR_MODE=panic` |

### 性能配置变量

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_CHANNEL_CAPACITY` | 通道容量 | `INKLOG_CHANNEL_CAPACITY=10000` |
| `INKLOG_WORKER_THREADS` | 工作线程数 | `INKLOG_WORKER_THREADS=3` |

---

## 最佳实践

### 日志级别选择

| 级别 | 使用场景 | 示例 |
|------|----------|--------|
| `trace` | 详细的执行追踪 | 函数入口/出口、变量值 |
| `debug` | 调试信息 | 中间变量、逻辑分支 |
| `info` | 常规信息 | 应用启动、用户操作 |
| `warn` | 警告情况 | 使用默认值、降级功能 |
| `error` | 错误情况 | 异常、失败操作 |

### 格式化建议

**生产环境**
```rust
format = "{timestamp} [{level}] {target} - {message}"
```

**开发环境**
```rust
format = "[{timestamp}] [{level}] {target} - {message} | {file}:{line}"
```

**调试环境**
```rust
format = "[{timestamp}] [{level}] {target} | {file}:{line} | {message}"
```

### 文件轮转策略

**高频日志场景**
```rust
max_size = "50MB"
rotation_time = "hourly"
keep_files = 24  // 保留 1 天
```

**常规日志场景**
```rust
max_size = "100MB"
rotation_time = "daily"
keep_files = 30  // 保留 30 天
```

**低频日志场景**
```rust
max_size = "500MB"
rotation_time = "weekly"
keep_files = 12  // 保留 12 周
```

### 性能优化

**高吞吐量配置**
```toml
[performance]
channel_capacity = 50000
worker_threads = 8

[file]
compress = false  // 禁用压缩提高速度
```

**平衡配置（推荐）**
```toml
[performance]
channel_capacity = 10000
worker_threads = 4

[file]
compress = true  // 启用压缩节省空间
```

### 安全最佳实践

1. **不要在代码中硬编码密钥**
   ```rust
   // 错误
   let key = "my-secret-key";

   // 正确
   std::env::set_var("INKLOG_ENCRYPTION_KEY", key);
   ```

2. **使用数据脱敏保护敏感信息**
   ```rust
   masking_enabled = true
   ```

3. **启用文件加密**
   ```rust
   encrypt = true
   encryption_key_env = Some("INKLOG_ENCRYPTION_KEY".into())
   ```

4. **限制日志文件权限**
   ```bash
   chmod 600 logs/*.log
   ```

---

## 故障排除

### 日志未输出

**问题**：日志未输出到任何地方。

**可能原因**：
1. 日志级别设置过高
2. Sink 未启用
3. 日志被过滤

**解决方案**：
```rust
// 检查日志级别
let config = InklogConfig {
    global: GlobalConfig {
        level: "debug".into(),  // 降低级别
        ..Default::default()
    },
    ..Default::default()
};

// 启用控制台输出
let config = InklogConfig {
    console_sink: Some(ConsoleSinkConfig {
        enabled: true,
        ..Default::default()
    }),
    ..Default::default()
};
```

### 文件轮转问题

**问题**：日志文件未轮转。

**可能原因**：
1. `max_size` 设置过大
2. `rotation_time` 设置过长
3. 磁盘空间不足

**解决方案**：
```rust
let file_config = FileSinkConfig {
    enabled: true,
    path: "logs/app.log".into(),
    max_size: "10MB".into(),        // 降低阈值
    rotation_time: "hourly".into(),  // 缩短间隔
    keep_files: 10,
    ..Default::default()
};
```

### 数据库连接失败

**问题**：数据库 Sink 无法连接。

**可能原因**：
1. 数据库 URL 错误
2. 数据库服务未启动
3. 网络问题
4. 认证失败

**解决方案**：
```rust
// 检查连接 URL
let database_sink = DatabaseSinkConfig {
    enabled: true,
    url: "sqlite://logs/app.db".to_string(),  // 使用本地数据库测试
    pool_size: 5,
    batch_size: 10,
    flush_interval_ms: 1000,
    ..Default::default()
};

// 验证数据库可用性
// 对于 SQLite：检查文件是否存在
// 对于 PostgreSQL/MySQL：检查网络连接
```

### 加密日志无法读取

**问题**：加密日志无法读取。

**可能原因**：
1. 加密密钥丢失
2. 密钥不匹配
3. 加密算法不一致

**解决方案**：
```rust
// 1. 备份密钥
let key = std::env::var("INKLOG_ENCRYPTION_KEY")?;
// 保存到安全位置

// 2. 使用 CLI 工具解密
// cargo run --example decrypt --file logs/encrypted.log.enc --key $KEY

// 3. 验证密钥
// 确保密钥长度为 32 字节（Base64 编码）
```

---

## 常见问题

### Inklog 是否兼容标准 `log` crate？

是的，Inklog 完全兼容 Rust 标准的 `log` crate。你可以使用 `log!`、`info!`、`error!` 等宏。

### 如何同时输出到文件和控制台？

```rust
let config = InklogConfig {
    console_sink: Some(ConsoleSinkConfig {
        enabled: true,
        ..Default::default()
    }),
    file_sink: Some(FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        ..Default::default()
    }),
    ..Default::default()
};
```

### 如何禁用数据脱敏？

```rust
let config = InklogConfig {
    global: GlobalConfig {
        masking_enabled: false,  // 禁用脱敏
        ..Default::default()
    },
    ..Default::default()
};
```

### 加密和压缩可以同时使用吗？

不建议同时使用。加密后的数据难以有效压缩。如果需要两者，先压缩再加密。

### 如何更改日志格式？

```rust
let config = InklogConfig {
    global: GlobalConfig {
        format: "[{timestamp}] {level}: {message}".to_string(),
        ..Default::default()
    },
    ..Default::default()
};
```

### 如何查看 Sink 健康状态？

```rust
let health = logger.get_health_status();
println!("整体状态: {:?}", health.overall_status);
println!("Channel 使用率: {:.2}%", health.channel_usage * 100.0);
```

---

## 相关资源

### 官方文档

- **API 参考**：[https://docs.rs/inklog](https://docs.rs/inklog)
- **示例代码**：[examples/](./examples/)
- **架构文档**：[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- **安全文档**：[docs/SECURITY.md](./docs/SECURITY.md)
- **贡献指南**：[docs/CONTRIBUTING.md](./docs/CONTRIBUTING.md)

### 生态系统

- **tracing**：[https://github.com/tokio-rs/tracing](https://github.com/tokio-rs/tracing)
- **Tokio**：[https://tokio.rs/](https://tokio.rs/)
- **Sea-ORM**：[https://www.sea-ql.org/SeaORM/](https://www.sea-ql.org/SeaORM/)
- **Axum**：[https://github.com/tokio-rs/axum](https://github.com/tokio-rs/axum)

### 社区支持

- **GitHub Issues**：[https://github.com/Kirky-X/inklog/issues](https://github.com/Kirky-X/inklog/issues)
- **GitHub Discussions**：[https://github.com/Kirky-X/inklog/discussions](https://github.com/Kirky-X/inklog/discussions)

---

**[返回顶部](#inklog-用户指南)**
