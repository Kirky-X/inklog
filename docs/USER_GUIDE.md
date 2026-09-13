# 📖 inklog 用户指南

本指南介绍 inklog（企业级 Rust 日志基础设施）的完整使用方式：安装、配置详解、常用示例、高级主题与故障排除。API 细节请配合 [📘 API 参考](API_REFERENCE.md) 阅读。

> 相关文档：[📘 API 参考](API_REFERENCE.md) · [🏗️ 架构设计](ARCHITECTURE.md) · [⚡ 性能基线](PERFORMANCE.md) · [🔒 安全文档](SECURITY.md)

<details open>
<summary>📑 目录</summary>

- [🧭 项目概述](#-项目概述)
- [✨ 核心特性](#-核心特性)
- [📦 安装](#-安装)
- [🚀 快速开始](#-快速开始)
- [⚙️ 配置详解](#️-配置详解)
- [💡 常用示例](#-常用示例)
- [🛠️ 高级主题](#️-高级主题)
- [🌍 环境变量配置](#-环境变量配置)
- [✅ 最佳实践](#-最佳实践)
- [🔧 故障排除](#-故障排除)
- [❓ 常见问题](#-常见问题)
- [🔗 相关资源](#-相关资源)

</details>

---

## 🧭 项目概述

inklog 是为 Rust 生产环境设计的日志基础设施库：应用代码继续使用 `log` / `tracing` 标准宏，由 inklog 接管订阅、脱敏、分发与落盘。

### 设计理念

| 理念 | 说明 |
|------|------|
| **高性能** | Crossbeam 有界通道 + 专用工作线程池，发送端非阻塞、队列满时背压 |
| **安全优先** | AES-256-GCM 加密、PII 数据脱敏、密钥内存清零 |
| **灵活配置** | 多目标输出（控制台、文件、数据库、网络、OTLP） |
| **可观测性** | 健康监控、Prometheus 指标导出、延迟直方图 |
| **生产就绪** | 自动轮转、断路器、故障降级与自动恢复、优雅关闭 |

### 核心概念

初始化（`LoggerManager` 安装全局 tracing subscriber 与 `log` crate 前端、进程级单例语义）、记录（标准宏无侵入）、Sink（输出目标抽象与内置实现）、配置（TOML + 环境变量优先级）与关闭（`shutdown()` 排空通道）五大核心概念，逐项说明见 [README · 快速开始](../README.md#-快速开始)。

## ✨ 核心特性

### 核心功能（始终可用）

| 功能 | 描述 |
|------|------|
| **异步管线** | Crossbeam 有界通道 + 专用工作线程池，背压控制 |
| **多目标输出** | 同时输出到控制台、文件、数据库等多个目标 |
| **结构化日志** | tracing 生态集成，`trace_id` / `span_id` 追踪关联 |
| **模板格式化** | `{timestamp}` `{level}` `{message}` `{trace_id}` 等占位符模板 |
| **文件轮转** | 基于大小和时间的自动轮转 |
| **数据脱敏** | 敏感字段名检测 + 正则规则库（PII 覆盖） |
| **运行时热调** | `set_level` 即时调整全局与 per-target 日志级别 |
| **健康监控** | Sink 状态、通道水位与指标追踪 |
| **动态 Sink** | `LoggerBuilder::add_sink` 注册第三方 Sink，每 Sink 独立通道 |
| **i18n** | 错误消息经 Fluent + ICU 按系统 locale 渲染 |

### 可选功能（feature 门控）

| 功能 | 描述 |
|------|------|
| **数据库 Sink** | PostgreSQL、MySQL、SQLite、DuckDB（经 dbnexus，批量落库、分区表） |
| **压缩** | Zstd（`compression`）/ Gzip（`gzip`）压缩轮转文件 |
| **加密** | AES-256-GCM 轮转归档加密 |
| **Parquet 导出** | 分析就绪的列式归档格式 |
| **HTTP 端点** | Axum 健康检查与 Prometheus 指标端点 |
| **网络转发** | TCP（可 TLS）+ UDP Sink，断线缓冲与自动重连 |
| **OTLP 导出** | OTLP/HTTP JSON 日志导出 |
| **CLI 工具** | `inklog-cli`：decrypt / generate / validate / query |
| **采样与限流** | 采样器（N 取 1）、令牌桶限流、中间件链装饰器 |
| **归档防篡改** | HMAC-SHA256 归档链，防删除、重排与伪造 |

## 📦 安装

将以下内容添加到 `Cargo.toml`（`default = []`，默认仅启用核心能力）：

```toml
[dependencies]
inklog = "0.3.0-rc.3"
```

### 启用可选 feature

```toml
# HTTP 端点
inklog = { version = "0.3.0-rc.3", features = ["http"] }

# CLI 工具
inklog = { version = "0.3.0-rc.3", features = ["cli"] }

# 数据库支持（四选一，互斥）
inklog = { version = "0.3.0-rc.3", features = ["sqlite"] }
inklog = { version = "0.3.0-rc.3", features = ["postgres"] }
inklog = { version = "0.3.0-rc.3", features = ["mysql"] }
inklog = { version = "0.3.0-rc.3", features = ["duckdb"] }

# 压缩与性能
inklog = { version = "0.3.0-rc.3", features = ["compression", "gzip", "parquet", "fast-masking"] }

# 集成与扩展
inklog = { version = "0.3.0-rc.3", features = ["net-sink", "otlp", "kms", "config-confers", "dbnexus-audit", "kit"] }
```

完整 feature 清单与说明见 [README](../README.md#-特性标志)。

> ⚠️ **数据库后端互斥**：`sqlite` / `postgres` / `mysql` / `duckdb` 不可同时启用，不适用 `--all-features`；互斥原因与按后端分组的启用方式见 [README · 特性标志](../README.md#-特性标志)。

> 注：TOML 配置文件加载（`LoggerManager::from_file` / `load`）为内建能力，无需额外 feature。

## 🚀 快速开始

### 基础日志记录

最简单的使用方式，使用默认配置初始化日志系统：

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 默认配置初始化并安装全局 subscriber（Console Sink，级别 info）
    let logger = LoggerManager::new().await?;

    tracing::trace!("这是一条追踪消息");
    tracing::debug!("这是一条调试消息");
    tracing::info!("这是一条信息消息");
    tracing::info!(user_id = 42, action = "login", "结构化字段示例");
    tracing::warn!("这是一条警告消息");
    tracing::error!("这是一条错误消息");

    // 退出前排空通道并关闭全部 Sink
    logger.shutdown()?;
    std::mem::forget(logger); // 已显式关闭，阻止 Drop 重复关闭
    Ok(())
}
```

也可使用 `log` crate 标准宏（经内置 `LogLogger` 桥接）：

```rust
log::info!("应用已启动");
log::error!("发生错误: {}", "详情");
```

进程级单例初始化（重复初始化返回明确错误）：

```rust
inklog::init_inklog_logger().await?;
```

### 文件日志

配置文件日志输出，支持自动轮转和压缩：

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 确保日志目录存在
    std::fs::create_dir_all("logs")?;

    // 配置文件 Sink
    let file_config = FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        max_size: "10MB".into(),       // 文件达到 10MB 时轮转
        rotation_time: "daily".into(), // 每天轮转
        keep_files: 7,                 // 保留 7 个轮转文件
        compress: true,                // 压缩轮转文件（Zstd，需 compression feature；未启用时回退 gzip）
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

将日志输出到数据库，支持批量写入（需任一数据库后端 feature）：

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
        ..Default::default()
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

## ⚙️ 配置详解

### 基础配置结构

`InklogConfig` 是根配置结构，包含所有子配置：

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "info".into(),
        format: "{timestamp} [{level}] {target} - {message}".to_string(),
        masking_enabled: true,
        ..Default::default()
    },
    console_sink: Some(inklog::config::ConsoleSinkConfig::default()),
    file_sink: None,
    database_sink: None,
    performance: inklog::config::PerformanceConfig::default(),
    http_server: None,
    ..Default::default()
};
```

#### InklogConfig 字段

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `global` | `GlobalConfig` | `default()` | 全局配置 |
| `console_sink` | `Option<ConsoleSinkConfig>` | `Some(default())` | 控制台 Sink 配置 |
| `file_sink` | `Option<FileSinkConfig>` | `None` | 文件 Sink 配置 |
| `database_sink` | `Option<DatabaseSinkConfig>` | `None` | 数据库 Sink 配置 |
| `performance` | `PerformanceConfig` | `default()` | 性能配置 |
| `http_server` | `Option<HttpServerConfig>` | `None` | HTTP 服务器配置 |
| `target_levels` | `HashMap<String, String>` | `{}` | per-target 级别预设（如 `{"hyper" = "warn"}`） |

#### 全局配置（GlobalConfig）

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `level` | `String` | `"info"` | 日志级别：`trace`、`debug`、`info`、`warn`、`error`、`fatal` |
| `format` | `String` | `"{timestamp} [{level}] {target} - {message}"` | 日志格式模板 |
| `masking_enabled` | `bool` | `true` | 是否启用数据脱敏 |
| `auto_fallback` | `bool` | `true` | Sink 失败时是否自动降级 |
| `fallback_initial_delay_ms` | `u64` | `1000` | 首次重试前等待时间（毫秒） |
| `fallback_max_delay_ms` | `u64` | `60000` | 重试延迟上限（毫秒） |
| `fallback_max_retries` | `u32` | `10` | 最大重试次数 |
| `output_format` | `OutputFormat` | `Text` | 输出格式：`Text`（模板）或 `Json`（NDJSON） |

#### 模板变量

| 变量 | 描述 |
|------|------|
| `{timestamp}` | ISO 8601 时间戳 |
| `{level}` | 日志级别 |
| `{target}` | 日志目标（模块/文件） |
| `{message}` | 日志消息 |
| `{fields}` | 附加结构化字段（JSON） |
| `{file}` | 源文件名 |
| `{line}` | 源代码行号 |
| `{thread_id}` | 线程 ID |
| `{trace_id}` | 追踪 ID（从当前 tracing span 提取） |
| `{span_id}` | Span ID |

### 文件输出配置

#### FileSinkConfig 字段

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `enabled` | `bool` | `true` | 是否启用文件 Sink |
| `path` | `PathBuf` | `"logs/app.log"` | 日志文件路径 |
| `max_size` | `String` | `"100MB"` | 触发轮转的最大文件大小（如 `"10MB"`、`"500KB"`） |
| `rotation_time` | `String` | `"daily"` | 时间轮转策略：`"hourly"`、`"daily"`、`"weekly"` |
| `keep_files` | `u32` | `30` | 保留的轮转文件数量 |
| `compress` | `bool` | `true` | 是否压缩轮转文件 |
| `compression_level` | `i32` | `3` | 压缩级别（0-22，数值越高压缩率越高） |
| `encrypt` | `bool` | `false` | 是否加密轮转归档 |
| `encryption_key_env` | `Option<String>` | `None` | 加密密钥的环境变量名 |
| `retention_days` | `u32` | `30` | 日志保留天数 |
| `max_total_size` | `String` | `"1GB"` | 日志目录最大总大小 |
| `cleanup_interval_minutes` | `u64` | `60` | 清理旧日志的间隔（分钟） |
| `batch_size` | `usize` | `100` | 写入前缓冲的日志条数 |
| `flush_interval_ms` | `u64` | `100` | 最大刷新间隔（毫秒） |
| `masking_enabled` | `bool` | `true` | 是否启用文件输出脱敏 |
| `output_format` | `OutputFormat` | `Text` | 输出格式：`Text` 或 `Json` |

#### 文件轮转示例

```rust
let file_config = FileSinkConfig {
    enabled: true,
    path: "logs/app.log".into(),
    max_size: "50MB".into(),        // 达到 50MB 时轮转
    rotation_time: "daily".into(),  // 每天轮转
    keep_files: 14,                 // 保留 14 个轮转文件
    compress: true,                 // 压缩旧日志
    ..Default::default()
};
```

#### 轮转文件命名

轮转触发时，当前文件重命名为带时间戳的归档（时间戳格式 `%Y%m%d_%H%M%S`）：

```text
logs/
├── app.log                      # 当前活动文件（明文）
├── app_20260913_143022.log      # 已轮转
├── app_20260912_080000.log.zst  # 已压缩（compression / gzip feature）
└── app_20260911_080000.log.zst.enc  # 已压缩并加密（encrypt = true）
```

> 加密与压缩仅作用于**轮转归档**，活跃文件保持明文；处理顺序为先压缩（`.zst`）后加密（`.enc`）。解密使用 `inklog-cli decrypt`，详见 [🔒 安全文档](SECURITY.md)。

### 数据库输出配置

#### DatabaseSinkConfig 字段

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `name` | `String` | `"default"` | Sink 名称 |
| `enabled` | `bool` | `false` | 是否启用数据库 Sink |
| `driver` | `DatabaseDriver` | `SQLite` | 数据库驱动 |
| `url` | `String` | `"sqlite::memory:"` | 数据库连接 URL |
| `pool_size` | `u32` | `10` | 连接池大小（SQLite 自动设为 1） |
| `batch_size` | `usize` | `100` | 批量写入条数 |
| `flush_interval_ms` | `u64` | `500` | 刷新间隔（毫秒） |
| `partition` | `PartitionStrategy` | `Monthly` | 分区策略：`Monthly`、`Yearly` |
| `table_name` | `String` | `"logs"` | 日志表名 |
| `archive_format` | `ArchiveFormat` | `Json` | 归档导出格式：`Json`、`Parquet`、`Csv` |
| `parquet_config` | `ParquetConfig` | `default()` | Parquet 导出配置（`parquet` feature） |
| `permissions_path` | `Option<String>` | `None` | RBAC 权限配置文件路径 |
| `admin_role` | `String` | `"admin"` | DDL 与写操作的管理角色名 |

> `DatabaseSinkConfig` 无需 feature 即可配置，但实际落库需要 `sqlite` / `postgres` / `mysql` / `duckdb` 之一。

#### 数据库驱动类型

| 驱动 | 字符串表示 | URL 示例 |
|------|------------|----------|
| `PostgreSQL` | `"postgres"` | `postgres://user:pass@localhost/logs` |
| `MySQL` | `"mysql"` | `mysql://user:pass@localhost/logs` |
| `SQLite` | `"sqlite"` | `sqlite://logs/app.db` |
| `DuckDB` | `"duckdb"` | DuckDB 数据库文件路径 |

### 数据脱敏配置

inklog 支持自动脱敏敏感个人信息（PII）：内置 21 条正则规则（信用卡号带 Luhn 校验）+ 敏感字段名检测，按优先级顺序应用。

#### 脱敏效果示例

| 数据类型 | 脱敏前 | 脱敏后 |
|---------|---------|---------|
| 邮箱地址 | `user@example.com` | `***@***.***` |
| 中国手机号 | `13812345678` | `***-****-****` |
| 信用卡号（Luhn 校验通过） | `4111111111111111` | `****-****-****-1111` |
| 身份证号 | `110101199001011234` | `******1234` |

#### 启用数据脱敏

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "info".into(),
        format: "{timestamp} {level} {message}".to_string(),
        masking_enabled: true,  // 启用数据脱敏（默认开启）
        ..Default::default()
    },
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;

// 敏感数据将自动脱敏
log::info!("用户邮箱: user@example.com");
// 输出: 用户邮箱: ***@***.***
```

#### 自定义脱敏规则

通过 `MaskRuleBuilder` / `DataMaskerBuilder` / `MaskRuleRegistry` 自定义规则，完整 API 见 [📘 API 参考](API_REFERENCE.md)：

```rust
use inklog::{MaskRule, MaskRuleBuilder};

let custom_rule = MaskRule::builder("employee_id")
    .pattern(r"\bEMP-\d{6}\b")
    .replacement("EMP-***")
    .priority(30)
    .build()
    .expect("Invalid pattern");
```

### HTTP 服务器配置

#### HttpServerConfig 字段

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `enabled` | `bool` | `false` | 是否启用 HTTP 服务器（需 `http` feature） |
| `host` | `String` | `"127.0.0.1"` | 监听主机地址 |
| `port` | `u16` | `9090` | 监听端口 |
| `metrics_path` | `String` | `"/metrics"` | Prometheus 指标端点路径 |
| `health_path` | `String` | `"/health"` | 健康检查端点路径 |
| `error_mode` | `HttpErrorMode` | `Strict` | 启动失败处理模式：`Strict` 返回错误、`Warn` 记录警告并继续 |
| `auth` | `Option<HttpAuthConfig>` | `None` | 认证配置（token 环境变量，启动期缓存、失败 fail-closed） |
| `ip_whitelist` | `Option<Vec<String>>` | `None` | IP 白名单 |
| `tls` | `Option<TlsConfig>` | `None` | TLS 配置（`cert_path` / `key_path`） |

#### HTTP 服务器示例

```rust
use inklog::config::{HttpErrorMode, HttpServerConfig};
use inklog::{InklogConfig, LoggerManager};

let config = InklogConfig {
    http_server: Some(HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 8080,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
        ..Default::default()
    }),
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;

// 健康检查端点: http://0.0.0.0:8080/health
// 指标端点: http://0.0.0.0:8080/metrics
```

#### 健康检查端点（/health）

返回 JSON 格式的健康状态（结构对应 `HealthStatus` / `MetricsSnapshot`）：

```json
{
  "overall_status": "Healthy",
  "sinks": {
    "console": {
      "status": "Healthy",
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

#### Prometheus 指标端点（/metrics）

返回 Prometheus 格式的指标（节选）：

```text
# HELP inklog_logs_written_total Total logs successfully written
# TYPE inklog_logs_written_total counter
inklog_logs_written_total 1000

# HELP inklog_sink_errors_total Total sink errors
# TYPE inklog_sink_errors_total counter
inklog_sink_errors_total 0

# HELP inklog_write_latency_us Log write latency in microseconds
# TYPE inklog_write_latency_us histogram
inklog_write_latency_us_bucket{le="50"} 950
```

### 性能配置

#### PerformanceConfig 字段

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `channel_capacity` | `usize` | `10000` | 日志通道容量 |
| `worker_threads` | `usize` | `3` | 工作线程数 |
| `channel_strategy` | `ChannelStrategy` | `Fixed` | 通道容量策略：`Fixed`、`Adaptive`（按水位扩缩容） |
| `expand_threshold_percent` | `u8` | `80` | 扩容触发水位（%） |
| `shrink_threshold_percent` | `u8` | `20` | 缩容触发水位（%） |
| `shrink_wait_seconds` | `u64` | `30` | 缩容前等待时间（秒） |
| `min_capacity` | `usize` | `1000` | 最小通道容量 |
| `max_capacity` | `usize` | `50000` | 最大通道容量 |
| `rate_limit` | `Option<u64>` | `None` | 全局日志速率上限（条/秒） |

#### 性能调优示例

```rust
use inklog::{InklogConfig, config::PerformanceConfig};

let config = InklogConfig {
    performance: PerformanceConfig {
        channel_capacity: 20000,  // 增加通道容量
        worker_threads: 4,        // 增加工作线程
        ..Default::default()
    },
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;
```

调优经验与正式基准数字见 [⚡ 性能基线](PERFORMANCE.md)。

## 💡 常用示例

### 多级别与结构化字段

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;

    // 记录不同级别的日志
    tracing::info!("信息消息：应用已启动");
    tracing::warn!("警告消息：配置文件使用默认值");
    tracing::error!("错误消息：无法连接到数据库");

    // 使用 target 指定日志来源
    tracing::info!(target: "auth", "用户登录");
    tracing::info!(target: "database", "查询执行");

    Ok(())
}
```

### 文件日志与轮转

文件 Sink 的完整初始化示例（含 `FileSinkConfig` 配置与优雅关闭）见[快速开始 · 文件日志](#文件日志)；`FileSinkConfig` 全部轮转字段与归档命名规则见[文件输出配置](#文件输出配置)。

### 加密日志

使用 AES-256-GCM 加密轮转归档（需先设置密钥环境变量）：

```bash
# 生成 32 字节高熵密钥（Base64 编码）
export INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)
```

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("logs")?;

    let file_config = FileSinkConfig {
        enabled: true,
        path: "logs/secure.log".into(),
        max_size: "10MB".into(),
        rotation_time: "daily".into(),
        keep_files: 5,
        encrypt: true,  // 启用加密（作用于轮转归档）
        encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("敏感操作：支付处理");

    Ok(())
}
```

> 密钥要求：Base64 解码后恰 32 字节，且 Shannon 熵 ≥ 4.0（低熵密钥会在启动时报错拒绝）。密钥轮换与解密流程见 [🔒 安全文档](SECURITY.md)。

### 自定义格式化

```rust
use inklog::{InklogConfig, config::GlobalConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InklogConfig {
        global: GlobalConfig {
            level: "debug".into(),
            // 自定义格式：含文件位置信息
            format: "[{timestamp}] [{level:>5}] {target} - {message} | {file}:{line}".to_string(),
            masking_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("使用自定义格式的日志");

    Ok(())
}
```

### 运行示例（cargo run --example）

`examples/` crate 提供 7 类共 39 个可运行示例，完整清单见 [README](../README.md#-示例)。运行方式：

```bash
cargo run --package inklog-examples --example <名称>
```

精选示例：

| 示例 | 主题 | 特性要求 |
|------|------|----------|
| `basic` | 初始化、级别、结构化字段、优雅关闭 | 无 |
| `builder` | Builder 模式配置 | 无 |
| `rotation` | 基于大小和时间的文件轮转 | 无 |
| `ring_buffered_file` | 通道缓冲文件 Sink（高吞吐） | 无 |
| `masking` | 数据脱敏 | 无 |
| `encryption` | 日志加密 | 无 |
| `circuit_breaker` | Sink 断路器与故障恢复 | 无 |
| `database` | Database Sink（SQLite 内存库） | `sqlite` |
| `compression` | Zstd 压缩与解压缩 | `compression` |
| `metrics` | 健康监控与指标收集 | 无 |

> 数据库相关示例需启用对应 feature（如 `sqlite`），运行前请参考 `examples/Cargo.toml` 中的 required-features 配置。

## 🛠️ 高级主题

### 运行时级别热调

不重启进程即可调整全局或 per-target 日志级别（经 `tracing_subscriber::reload` 换装 EnvFilter，`RUST_LOG` 附加指令跨重建保留）：

```rust
use inklog::LoggerManager;

let manager = LoggerManager::new().await?;

// 调整全局级别
manager.set_level(None, "debug")?;

// 调整指定 target 的级别（如降噪第三方库）
manager.set_level(Some("hyper"), "warn")?;

// 查看当前生效的过滤器
println!("{}", manager.current_level_filter_string());
```

### 动态 Sink 注册

第三方 Sink 实现 `LogSink` trait 后，经 `LoggerBuilder::add_sink` 零核心改动接入，每个动态 Sink 拥有独立通道、互不抢占：

```rust
use inklog::{AsyncSink, LoggerBuilder};
use std::sync::Arc;

let logger = LoggerBuilder::new()
    .level("info")
    .add_sink(Arc::new(MyCustomSink::new()))  // 自定义 Sink（impl LogSink）
    .build()
    .await?;
```

自定义 Sink 的完整实现步骤见 [🏗️ 架构设计](ARCHITECTURE.md)「扩展点」章节。

### 多目标输出

```rust
use inklog::{DatabaseSinkConfig, FileSinkConfig, InklogConfig};

let config = InklogConfig {
    console_sink: Some(inklog::config::ConsoleSinkConfig {
        enabled: true,
        colored: true,
        ..Default::default()
    }),
    file_sink: Some(FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        ..Default::default()
    }),
    database_sink: Some(DatabaseSinkConfig {
        enabled: true,
        url: "sqlite::memory:".to_string(),
        ..Default::default()
    }),
    ..Default::default()
};
```

### 批量写入

数据库 Sink 缓冲日志，满足任一条件即批量落库：达到 `batch_size` 条数，或经过 `flush_interval_ms` 时间：

```rust
let database_sink = DatabaseSinkConfig {
    enabled: true,
    batch_size: 500,        // 每 500 条写入
    flush_interval_ms: 2000, // 或每 2 秒写入
    ..Default::default()
};
```

### Sink 故障恢复

inklog 提供断路器、三级降级（DB → File → Console）与自动恢复；健康检查线程每 10 秒巡检，连续失败超阈值且冷却期已过时自动重建 Sink。

```rust
// 手动恢复特定 Sink
logger.recover_sink("file")?;

// 恢复所有不健康的 Sink
let recovered = logger.trigger_recovery_for_unhealthy_sinks()?;
println!("已恢复的 Sink: {:?}", recovered);

// 查看健康状态
let health = logger.get_health_status();
println!("整体状态: {:?}", health.overall_status);
println!("Channel 使用率: {:.2}%", health.channel_usage * 100.0);
```

故障处理与恢复的完整流程见 [🏗️ 架构设计](ARCHITECTURE.md)「错误处理流程」章节。

### 使用 Mock 实现进行测试

inklog 提供 `MockCache` / `MockConfig` / `MockDatabaseAdapter` 三个 Mock 实现，用于单元测试与集成测试，无需启动真实基础设施服务。

> Mock 类型由 `test-utils` feature 门控（v0.3.0-rc.2 起，默认公共 API 不含 Mock），外部测试消费者需显式启用：

```toml
[dev-dependencies]
inklog = { version = "0.3.0-rc.3", features = ["test-utils"] }
```

#### 依赖注入测试模式

```rust
use inklog::{LoggerDependencies, LoggerManager, MockCache, MockConfig, MockDatabaseAdapter};
use std::sync::Arc;

#[tokio::test]
async fn test_with_mocks() -> Result<(), Box<dyn std::error::Error>> {
    let database = Arc::new(MockDatabaseAdapter::new());

    let deps = LoggerDependencies {
        cache: Some(Arc::new(MockCache::new())),
        config: Some(Arc::new(MockConfig::new().with_value("level", "debug"))),
        database: Some(database.clone()),
    };

    let logger = LoggerManager::with_dependencies(deps).await?;

    log::info!("Test message");

    // 验证日志被写入 Mock 数据库
    assert_eq!(database.record_count(), 1);
    Ok(())
}
```

#### MockCache 使用示例

```rust
use inklog::MockCache;

#[tokio::test]
async fn test_cache_operations() -> Result<(), inklog::InklogError> {
    let cache = MockCache::new();

    cache.set("key1", "value1".to_string()).await?;
    assert_eq!(cache.get("key1").await?, Some("value1".to_string()));

    assert!(cache.exists("key1").await?);
    assert!(!cache.exists("nonexistent").await?);

    assert!(cache.delete("key1").await?);
    assert_eq!(cache.get("key1").await?, None);

    // 延迟模拟（测试超时场景）
    let slow_cache = MockCache::with_delay(100); // 100ms 延迟
    Ok(())
}
```

#### MockConfig 使用示例

```rust
use inklog::MockConfig;

#[test]
fn test_config_operations() {
    let config = MockConfig::new()
        .with_value("level", "debug")
        .with_value("port", "8080");

    assert_eq!(config.get_string("level"), Some("debug".to_string()));
    assert_eq!(config.get_int("port"), Some(8080));

    // 运行时修改配置（测试动态配置场景）
    config.set("level", "error");
    assert_eq!(config.get_string("level"), Some("error".to_string()));
}
```

#### MockDatabaseAdapter 使用示例

```rust
use inklog::{LogRecord, MockDatabaseAdapter};

#[tokio::test]
async fn test_database_operations() {
    let db = MockDatabaseAdapter::new();

    let records = vec![
        LogRecord::new(tracing::Level::INFO, "app".to_string(), "Test message 1".to_string()),
        LogRecord::new(tracing::Level::ERROR, "app".to_string(), "Test message 2".to_string()),
    ];

    // 批量插入
    let count = db.insert_batch(&records).await.unwrap();
    assert_eq!(count, 2);

    // 健康状态控制（测试降级场景）
    db.set_healthy(false);
    assert!(!db.is_healthy().await);

    // 清空记录（测试隔离）
    db.clear();
    assert_eq!(db.record_count(), 0);
}
```

#### 测试隔离最佳实践

```rust
use inklog::{LoggerDependencies, LoggerManager, MockCache, MockConfig, MockDatabaseAdapter};
use std::sync::Arc;

struct TestContext {
    database: Arc<MockDatabaseAdapter>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            database: Arc::new(MockDatabaseAdapter::new()),
        }
    }

    async fn create_logger(&self) -> Result<LoggerManager, inklog::InklogError> {
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: Some(Arc::new(MockConfig::new())),
            database: Some(self.database.clone()),
        };
        LoggerManager::with_dependencies(deps).await
    }

    fn reset(&self) {
        self.database.clear(); // 清空数据库
    }
}

#[tokio::test]
async fn test_isolated() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TestContext::new();
    let _logger = ctx.create_logger().await?;

    // 执行测试...

    ctx.reset(); // 重置状态，不影响其他测试
    Ok(())
}
```

## 🌍 环境变量配置

inklog 支持通过 `INKLOG_*` 环境变量覆盖配置（优先级高于配置文件）。

### 全局配置

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_GLOBAL_LEVEL` | 日志级别 | `INKLOG_GLOBAL_LEVEL=debug` |
| `INKLOG_GLOBAL_FORMAT` | 日志格式 | `INKLOG_GLOBAL_FORMAT="{timestamp} {message}"` |
| `INKLOG_GLOBAL_MASKING_ENABLED` | 启用数据脱敏 | `INKLOG_GLOBAL_MASKING_ENABLED=true` |
| `INKLOG_GLOBAL_AUTO_FALLBACK` | 启用自动降级 | `INKLOG_GLOBAL_AUTO_FALLBACK=true` |
| `INKLOG_LOCALE` | 消息本地化语言（zh-CN / en） | `INKLOG_LOCALE=en` |

### 文件 Sink

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_FILE_SINK_ENABLED` | 启用文件 Sink | `INKLOG_FILE_SINK_ENABLED=true` |
| `INKLOG_FILE_SINK_PATH` | 日志文件路径 | `INKLOG_FILE_SINK_PATH=logs/app.log` |
| `INKLOG_FILE_SINK_MAX_SIZE` | 最大文件大小 | `INKLOG_FILE_SINK_MAX_SIZE=100MB` |

### 数据库 Sink

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_DATABASE_SINK_URL` | 数据库连接 URL | `INKLOG_DATABASE_SINK_URL=postgres://localhost/logs` |
| `INKLOG_DATABASE_SINK_POOL_SIZE` | 连接池大小 | `INKLOG_DATABASE_SINK_POOL_SIZE=10` |
| `INKLOG_DATABASE_SINK_BATCH_SIZE` | 批量大小 | `INKLOG_DATABASE_SINK_BATCH_SIZE=100` |
| `INKLOG_DATABASE_SINK_FLUSH_INTERVAL_MS` | 刷新间隔（毫秒） | `INKLOG_DATABASE_SINK_FLUSH_INTERVAL_MS=1000` |
| `INKLOG_DATABASE_SINK_TABLE_NAME` | 日志表名 | `INKLOG_DATABASE_SINK_TABLE_NAME=logs` |

### HTTP 服务器

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_HTTP_SERVER_ENABLED` | 启用 HTTP 服务器 | `INKLOG_HTTP_SERVER_ENABLED=true` |
| `INKLOG_HTTP_SERVER_HOST` | 监听主机 | `INKLOG_HTTP_SERVER_HOST=0.0.0.0` |
| `INKLOG_HTTP_SERVER_PORT` | 监听端口 | `INKLOG_HTTP_SERVER_PORT=8080` |
| `INKLOG_HTTP_SERVER_METRICS_PATH` | 指标端点路径 | `INKLOG_HTTP_SERVER_METRICS_PATH=/metrics` |
| `INKLOG_HTTP_SERVER_HEALTH_PATH` | 健康检查路径 | `INKLOG_HTTP_SERVER_HEALTH_PATH=/health` |
| `INKLOG_HTTP_SERVER_ERROR_MODE` | 错误处理模式 | `INKLOG_HTTP_SERVER_ERROR_MODE=strict` |

### 性能配置

| 环境变量 | 描述 | 示例 |
|----------|------|--------|
| `INKLOG_PERFORMANCE_CHANNEL_CAPACITY` | 通道容量 | `INKLOG_PERFORMANCE_CHANNEL_CAPACITY=10000` |
| `INKLOG_PERFORMANCE_WORKER_THREADS` | 工作线程数 | `INKLOG_PERFORMANCE_WORKER_THREADS=3` |

### 其他

| 环境变量 | 描述 |
|----------|------|
| `INKLOG_CONFIG_PATH` | 配置文件路径（`LoggerManager::load()` 查找优先级最高） |
| `INKLOG_ENCRYPTION_KEY` | 加密密钥（Base64 或原始 32 字节） |
| `INKLOG_DECRYPT_KEY` | CLI 解密密钥（`inklog-cli decrypt --key-env` 的默认变量名） |

## ✅ 最佳实践

### 日志级别选择

| 级别 | 使用场景 | 示例 |
|------|----------|--------|
| `trace` | 详细执行追踪 | 函数入口/出口、变量值 |
| `debug` | 调试信息 | 中间变量、逻辑分支 |
| `info` | 常规信息 | 应用启动、用户操作 |
| `warn` | 警告情况 | 使用默认值、降级功能 |
| `error` | 错误情况 | 异常、失败操作 |

### 格式化建议

| 环境 | 推荐格式 |
|------|----------|
| 生产环境 | `{timestamp} [{level}] {target} - {message}` |
| 开发环境 | `[{timestamp}] [{level}] {target} - {message} \| {file}:{line}` |
| 结构化采集 | `output_format = "Json"`（NDJSON） |

### 文件轮转策略

| 场景 | 推荐配置 |
|------|----------|
| 高频日志 | `max_size = "50MB"`、`rotation_time = "hourly"`、`keep_files = 24` |
| 常规日志 | `max_size = "100MB"`、`rotation_time = "daily"`、`keep_files = 30` |
| 低频日志 | `max_size = "500MB"`、`rotation_time = "weekly"`、`keep_files = 12` |

### 性能优化

- **高吞吐场景**：`channel_capacity = 50000`、`worker_threads = 8`，可关闭轮转压缩（`compress = false`）；
- **平衡配置（推荐）**：`channel_capacity = 10000`、`worker_threads = 4`、`compress = true`；
- **低延迟场景**：减小 `channel_capacity`（如 1000），缩短队列驻留时间；
- **脱敏按需开启**：正则脱敏是主链路中最贵的安全环节（约为模板渲染的 160 倍），仅在需要的 Sink 开启，量化依据见 [⚡ 性能基线](PERFORMANCE.md)。

### 安全最佳实践

1. **不要在代码中硬编码密钥**，从环境变量或 KMS 注入（`kms` feature）；
2. **启用数据脱敏**（`masking_enabled = true`，默认开启）；
3. **启用轮转归档加密**（`encrypt = true` + `encryption_key_env`）；
4. **限制日志文件权限**（inklog 对加密文件自动设置 `0600`，目录建议 `0700`）。

完整安全设计见 [🔒 安全文档](SECURITY.md)。

## 🔧 故障排除

### 日志未输出

**问题**：日志未输出到任何地方。

**排查**：

1. 日志级别设置过高（如全局 `error` 却在打 `info`）；
2. 对应 Sink 未启用（如 `file_sink: None`）；
3. 全局 subscriber 被宿主应用抢先安装（inklog 检测到后降级跳过，日志流向已安装的 subscriber）。

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "debug".into(),  // 降低级别
        ..Default::default()
    },
    console_sink: Some(inklog::config::ConsoleSinkConfig {
        enabled: true,  // 确保控制台 Sink 启用
        ..Default::default()
    }),
    ..Default::default()
};
```

### 文件轮转不生效

**问题**：日志文件未轮转。

**排查**：

1. `max_size` 设置过大，未达到阈值；
2. `rotation_time` 间隔过长；
3. 磁盘空间不足触发保护逻辑。

```rust
let file_config = inklog::FileSinkConfig {
    enabled: true,
    path: "logs/app.log".into(),
    max_size: "10MB".into(),        // 降低阈值
    rotation_time: "hourly".into(), // 缩短间隔
    keep_files: 10,
    ..Default::default()
};
```

### 数据库连接失败

**问题**：数据库 Sink 无法连接。

**排查**：

1. 数据库 URL 错误（先用本地 SQLite 验证：`sqlite://logs/app.db`）；
2. 数据库服务未启动或网络不通；
3. 认证失败；
4. 同时启用了多个数据库后端 feature（互斥，编译/运行行为异常）。

```rust
let database_sink = inklog::DatabaseSinkConfig {
    enabled: true,
    url: "sqlite://logs/app.db".to_string(), // 使用本地数据库排查
    pool_size: 5,
    batch_size: 10,
    flush_interval_ms: 1000,
    ..Default::default()
};
```

### 加密日志无法读取

**问题**：加密归档解密失败。

**排查**：

1. 密钥丢失或不匹配（v2 格式从文件头读取盐做确定性派生，密钥必须与加密时一致）；
2. 密钥格式错误（需 Base64 解码恰 32 字节或原始 32 字节）；
3. v1 时代的「密码模式」文件因未存盐无法恢复，请迁移到 v2。

```bash
# 使用 CLI 工具解密
inklog-cli decrypt \
  --input logs/app_20260913_143022.log.zst.enc \
  --output decrypted/ \
  --key-env INKLOG_DECRYPT_KEY
```

## ❓ 常见问题

### inklog 是否兼容标准 `log` crate？

兼容。inklog 内置 `LogLogger` 桥接，`log::info!`、`log::error!` 等标准宏与 `tracing` 宏均可使用。

### 如何同时输出到文件和控制台？

`console_sink` 默认启用，再配置 `file_sink` 即可，示例见[多目标输出](#多目标输出)。

### 如何禁用数据脱敏？

```rust
let config = InklogConfig {
    global: inklog::config::GlobalConfig {
        masking_enabled: false, // 禁用脱敏
        ..Default::default()
    },
    ..Default::default()
};
```

### 加密和压缩可以同时使用吗？

可以。轮转归档的处理顺序为**先压缩（`.zst`）后加密（`.enc`）**，两者作用于轮转文件；活跃日志文件保持明文（设计使然，保证实时写入性能）。

### 如何更改日志格式？

```rust
let config = InklogConfig {
    global: inklog::config::GlobalConfig {
        format: "[{timestamp}] {level}: {message}".to_string(),
        ..Default::default()
    },
    ..Default::default()
};
```

可用模板变量见「模板变量」表（含 `{trace_id}` / `{span_id}`）。

### 如何在运行时调整日志级别而无需重启？

使用 `LoggerManager::set_level`（见「运行时级别热调」章节），全局与 per-target 级别均即时生效。

### 如何检索本地日志文件？

使用 `inklog-cli query`（`cli` feature），支持按时间、级别、关键词检索，自动处理解密与解包：

```bash
inklog-cli query --path logs/ --level error --since 2026-09-13T00:00:00Z --grep "payment"
```

## 🔗 相关资源

### 项目文档

| 文档 | 说明 |
|------|------|
| [📘 API 参考](API_REFERENCE.md) | 核心类型、配置结构体与 trait 逐项说明 |
| [🏗️ 架构设计](ARCHITECTURE.md) | 分层设计、数据流与并发模型 |
| [⚡ 性能基线](PERFORMANCE.md) | criterion 基准环境与基线数字 |
| [🧪 测试场景](TEST_SCENARIOS.md) | 测试金字塔与 E2E 场景定义 |
| [🔒 安全文档](SECURITY.md) | 安全设计、漏洞报告与合规说明 |
| [📋 更新日志](CHANGELOG.md) | 版本变更记录 |
| [📦 在线 API 文档](https://docs.rs/inklog) | docs.rs 自动生成的最新文档 |

### 生态系统

- [tracing](https://github.com/tokio-rs/tracing)：结构化日志与 Subscriber 生态
- [Tokio](https://tokio.rs/)：异步运行时
- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam)：高性能有界通道
- [Axum](https://github.com/tokio-rs/axum)：HTTP 端点

### 社区支持

- [GitHub Issues](https://github.com/Kirky-X/inklog/issues)：报告 bug 和问题
- [GitHub Discussions](https://github.com/Kirky-X/inklog/discussions)：提问和分享想法

---

**[⬆ 返回顶部](#-inklog-用户指南)**
