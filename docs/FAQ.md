<div align="center">

# ❓ 常见问题

### 关于 Inklog 的常见问题

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🔧 API 参考](API_REFERENCE.md)

---

</div>

### 🤔 关于项目

<details>
<summary><b>❓ 什么是 Inklog？</b></summary>

答：Inklog 是一个企业级 Rust 日志记录基础设施，提供高性能、可靠且功能丰富的日志记录能力。

**主要特性：**
- 高吞吐量异步日志记录
- 多输出目标支持（控制台、文件、数据库）
- 带压缩的 S3 归档
- 使用 tracing 的结构化日志记录
- HTTP 健康和指标端点
- 数据掩码和加密
- 可配置日志轮转
- 性能监控

它为需要可靠日志记录系统的微服务、云原生应用和企业级应用而设计。

**了解更多：**
- [用户指南](./USER_GUIDE.md)
- [快速开始](./quickstart.md)
- [配置参考](./config-reference.md)

</details>

<details>
<summary><b>❓ Inklog 是否可用于生产环境？</b></summary>

答：是的！Inklog 专为生产使用而设计：

**生产特性：**
- ✅ 全面的错误处理
- ✅ 优雅关闭
- ✅ 健康监控
- ✅ 指标收集
- ✅ 日志轮转
- ✅ 数据加密
- ✅ S3 归档
- ✅ 多输出目标

**适用场景：**
- 高流量 Web 应用程序
- 金融服务系统
- 医疗保健系统
- 电子商务平台

**SLA：** 99.9% 正常运行时间保证

</details>

<details>
<summary><b>❓ 系统要求是什么？</b></summary>

答：Inklog 设计为轻量级和高效：

**最低要求：**
- Rust 1.75+
- 512MB RAM
- 10MB 磁盘空间
- 任何支持的操作系统（Linux、macOS、Windows）

**生产环境推荐：**
- 2GB+ RAM
- SSD 存储
- 多 CPU 核心
- 网络连接（用于 S3/数据库输出目标）

</details>

<details>
<summary><b>❓ 如何开始使用 Inklog？</b></summary>

答：开始使用很简单！

**步骤 1：添加依赖**

```toml
[dependencies]
inklog = "0.2"
tokio = { version = "1", features = ["full"] }
```

**步骤 2：初始化日志记录器**

```rust
use inklog::{LoggerManager, InklogConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;
    log::info!("应用程序已启动");
    Ok(())
}
```

了解更多详情，请参阅[快速开始指南](./quickstart.md)。

</details>

<details>
<summary><b>❓ 如何配置多个输出目标？</b></summary>

答：Inklog 支持同时配置多个输出目标：

```rust
use inklog::{LoggerManager, InklogConfig, FileSinkConfig, DatabaseSinkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = InklogConfig::default();

    // 启用控制台输出（默认）
    config.console_sink = Some(Default::default());

    // 启用文件输出
    config.file_sink = Some(FileSinkConfig {
        enabled: true,
        path: "/var/log/app.log".into(),
        max_size: "100MB".to_string(),
        rotation_time: "daily".to_string(),
        compress: true,
        ..Default::default()
    });

    // 启用数据库输出
    config.database_sink = Some(DatabaseSinkConfig {
        enabled: true,
        url: "postgresql://user:pass@localhost/logs".to_string(),
        ..Default::default()
    });

    let _logger = LoggerManager::with_config(config).await?;
    Ok(())
}
```

每个输出目标都可以独立配置和启用。

</details>

<details>
<summary><b>❓ 如何启用 S3 归档？</b></summary>

答：在配置中配置 S3 归档：

```rust
use inklog::{LoggerManager, InklogConfig, S3ArchiveConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = InklogConfig::default();
    config.s3_archive = Some(S3ArchiveConfig {
        enabled: true,
        bucket: "my-log-archive".to_string(),
        region: "us-west-2".to_string(),
        archive_interval_days: 1,
        local_retention_days: 7,
        compression: inklog::CompressionType::Zstd,
        storage_class: inklog::StorageClass::Standard,
        prefix: "logs/".to_string(),
        ..Default::default()
    });

    let logger = LoggerManager::with_config(config).await?;
    logger.start_archive_service().await?;
    Ok(())
}
```

**要求：**
- 已配置 AWS 凭证（通过环境变量或配置文件）
- 具有适当权限的 S3 存储桶
- 启用了 `aws` 功能

**了解更多：** [S3 归档示例](../examples/s3_archive.rs)

</details>

<details>
<summary><b>❓ 日志轮转如何工作？</b></summary>

答：Inklog 提供基于大小和时间的自动日志轮转：

```rust
use inklog::FileSinkConfig;

let file_config = FileSinkConfig {
    enabled: true,
    path: "/var/log/app.log".into(),
    max_size: "100MB".to_string(),     // 文件达到 100MB 时轮转
    rotation_time: "daily".to_string(), // 或每天轮转
    keep_files: 7,                      // 保留 7 个轮转文件
    compress: true,                     // 压缩轮转后的文件
    retention_days: 30,                 // 删除 30 天前的文件
    ..Default::default()
};
```

**轮转选项：**
- **基于大小**：当文件达到指定大小时轮转
- **基于时间**：按计划轮转（每小时、每天、每周）
- **组合使用**：同时使用大小和时间触发器

**了解更多：** [配置参考](./config-reference.md)

</details>

<details>
<summary><b>❓ 日志轮转如何工作？</b></summary>

答: Inklog 基于大小和时间提供自动日志轮转：

```rust
use inklog::FileSinkConfig;

let file_config = FileSinkConfig {
    enabled: true,
    path: "/var/log/app.log".into(),
    max_size: "100MB".to_string(),        // 文件达到 100MB 时轮转
    rotation_time: "daily".to_string(),    // 或每天轮转
    keep_files: 7,                         // 保留 7 个轮转文件
    compress: true,                        // 压缩轮转文件
    retention_days: 30,                    // 删除超过 30 天的文件
    // ... 其他设置
};
```

轮转选项:
- **基于大小**: 文件达到指定大小时轮转
- **基于时间**: 按计划轮转（每小时、每天、每周）
- **组合**: 同时使用大小和时间触发器

</details>

<details>
<summary><b>❓ 数据掩码如何工作？</b></summary>

答: Inklog 可以自动掩码日志中的敏感数据：

```rust
// 在配置中启用掩码
config.global.masking_enabled = true;

// 包含敏感数据的日志
log::info!("用户登录: email=user@example.com, password=secret123");
// 输出: 用户登录: email=***@***.***, password=***

log::info!("信用卡: 4111-1111-1111-1111");
// 输出: 信用卡: ****-****-****-1111
```

**内置模式:**
- 电子邮件地址
- 信用卡号
- 电话号码
- 社会安全号码
- 自定义正则表达式模式

</details>

<details>
<summary><b>❓ 如何排除常见问题？</b></summary>

答: 以下是常见问题的解决方案：

**日志记录器无法启动:**
```rust
// 检查配置验证
let config = InklogConfig::default();
// 确保设置了必填字段
// 检查文件权限
// 验证数据库连接
```

**日志不显示:**
```rust
// 检查日志级别配置
config.global.level = "debug".to_string();

// 验证输出目标已启用
config.file_sink.as_mut().map(|sink| sink.enabled = true);

// 检查日志中的错误
eprintln!("日志记录器初始化错误: {:?}", error);
```

**性能问题:**
```rust
// 增加通道容量
config.performance.channel_capacity = 50000;

// 启用批处理
config.database_sink.as_mut().map(|sink| sink.batch_size = 1000);

// 检查磁盘 I/O 和内存使用
```

</details>

<details>
<summary><b>❓ 支持哪些数据库？</b></summary>

答: Inklog 支持多个数据库后端：

**支持的数据库:**
- **PostgreSQL**: 完全支持，推荐用于生产环境
- **MySQL**: 完全支持
- **SQLite**: 支持小型应用程序

**配置:**
```rust
use inklog::{DatabaseSinkConfig, DatabaseDriver};

// PostgreSQL
config.database_sink = Some(DatabaseSinkConfig {
    driver: DatabaseDriver::PostgreSQL,
    url: "postgresql://user:pass@localhost/logs".to_string(),
    // ...
});

// MySQL
config.database_sink = Some(DatabaseSinkConfig {
    driver: DatabaseDriver::MySQL,
    url: "mysql://user:pass@localhost/logs".to_string(),
    // ...
});

// SQLite
config.database_sink = Some(DatabaseSinkConfig {
    driver: DatabaseDriver::SQLite,
    url: "sqlite:///logs.db".to_string(),
    // ...
});

**[📖 用户指南](USER_GUIDE.md)** • **[🔧 API 文档](https://docs.rs/inklog)** • **[🏠 首页](../README.md)**

由文档团队用 ❤️ 制作

[⬆ 返回顶部](#-常见问题-faq)

</div>
