# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 语言设置

始终使用中文回复用户。

## 项目概述

inklog 是一个企业级 Rust 日志基础设施，提供高性能、高可靠、可扩展的日志记录能力。

**核心特性**：
- 零丢失：基于有界 Channel + 背压阻塞机制
- 高性能：500条/秒吞吐，微秒级延迟
- 多 Sink 支持：Console、File、Database
- 企业特性：文件轮转、Zstd 压缩、AES-256-GCM 加密、S3 归档
- 可观测性：Prometheus 指标 + HTTP 健康检查端点

## 常用命令

```bash
# 构建项目
cargo build                    # 默认构建（包含 aws、http、cli 特性）
cargo build --release          # Release 构建

# 运行测试
cargo test                     # 运行所有测试
cargo test --lib               # 仅运行库测试
cargo test --test integration  # 运行特定集成测试
cargo test -- --test-threads=1 # 单线程顺序执行（串行测试）

# 性能基准测试
cargo bench                    # 运行 Criterion 基准测试

# 代码质量检查
cargo clippy                   # 运行 clippy linter
cargo clippy -- -D warnings    # 将警告视为错误
cargo fmt                      # 格式化代码

# 生成文档
cargo doc --no-deps            # 生成 API 文档

# 运行 CLI 工具
cargo run --bin inklog-cli -- --help
```

## 项目架构

### 模块结构

```
src/
├── lib.rs              # 库入口，导出公共 API
├── manager.rs          # LoggerManager：日志管理器核心
├── config.rs           # 配置系统：InklogConfig 及解析
├── log_record.rs       # LogRecord：日志记录数据结构
├── subscriber.rs       # Tracing Subscriber：拦截日志事件
├── template.rs         # 日志格式化模板
├── masking.rs          # 敏感信息脱敏
├── metrics.rs          # 指标收集与健康检查
├── pool.rs             # 对象池优化
├── error.rs            # 错误类型定义
├── sink/
│   ├── mod.rs          # LogSink trait 定义
│   ├── console.rs      # Console Sink：彩色控制台输出
│   ├── file.rs         # File Sink：文件写入、轮转、压缩、加密
│   └── database.rs     # Database Sink：批量写入数据库
├── archive/
│   ├── mod.rs
│   └── service.rs      # S3 归档服务
└── cli/
    ├── mod.rs
    ├── decrypt.rs      # 加密日志解密工具
    ├── generate.rs     # 密钥生成工具
    └── validate.rs     # 配置验证工具
```

### 核心架构

```
[应用代码]
    ↓ tracing 宏 (info!, error! 等)
[LoggerSubscriber]
    ├── Fast Path → Console Sink（同步，<50μs）
    └── Slow Path → crossbeam-channel（有界队列，10,000 容量）
            ↓
    [Worker Thread Pool]
        ├── Thread 1: File Worker → File Sink（轮转、压缩、加密）
        ├── Thread 2: DB Worker → Database Sink（批量写入、S3 归档）
        └── Thread 3: Health Check（故障检测、自动恢复）
```

### 数据流

1. **日志拦截**：Subscriber 接收 tracing Event，转换为 LogRecord
2. **双路径分发**：
   - Console：同步写入，不阻塞主线程
   - Async：发送到 Channel，由 Worker 处理
3. **Worker 处理**：
   - File Worker：检查轮转条件，异步压缩加密
   - DB Worker：批量缓冲区管理，超时 flush
   - Health Check：监控 Sink 健康状态，触发自动恢复

## 配置系统

支持双重初始化方式：

```rust
// 方式1：零依赖直接初始化
let logger = LoggerManager::new().await?;
let logger = LoggerManager::builder()
    .level("debug")
    .enable_console(true)
    .enable_file("logs/app.log")
    .build()
    .await?;

// 方式2：配置文件初始化（需 confers 特性）
let logger = LoggerManager::from_file("config.toml").await?;
let logger = LoggerManager::load().await?; // 自动搜索配置
```

**Cargo 特性**：
- `default`：aws、http、cli
- `confers`：启用 TOML 配置文件支持
- `http`：启用 HTTP 指标/健康端点
- `aws`：启用 S3 归档功能
- `cli`：启用 CLI 工具

## 关键技术决策

| 决策 | 选择 | 原因 |
|------|------|------|
| Channel 库 | crossbeam-channel v0.5 | 性能最优、独立运行、不依赖 tokio runtime |
| 数据库 ORM | Sea-ORM | 跨数据库支持（SQLite/PostgreSQL/MySQL） |
| 压缩算法 | Zstd | 高压缩比、异步处理 |
| 加密算法 | AES-256-GCM | AEAD 认证加密、硬件加速支持 |
| 并行处理 | Rayon | 数据级并行、压缩加密任务 |

## 错误处理规范

- 库代码返回 `Result<T, E>`，不使用 `panic!`
- 使用 `thiserror` 定义自定义错误类型
- 应用代码使用 `anyhow` 提供上下文

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InklogError {
    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Channel error: {0}")]
    ChannelError(String),
}
```

## 代码质量要求

- **零警告**：所有 `cargo clippy` 警告必须修复或显式允许
- **命名规范**：snake_case 函数、PascalCase 类型、SCREAMING_SNAKE_CASE 常量
- **错误处理**：使用 `?` 运算符，避免 `unwrap()`
- **文档注释**：为公共 API 添加 `///` 格式文档

## Git 工作流

**分支命名**：`feature/TICKET-ID-描述`、`bugfix/TICKET-ID-描述`

**提交信息格式**：
```
<type>(<scope>): <subject>

<body>

<footer>
```

类型：`feat`、`fix`、`docs`、`style`、`refactor`、`test`

## Sink 降级策略

| 故障场景 | 降级策略 |
|----------|----------|
| DB 连接失败 | 降级到 FileSink（写入 db_fallback.log） |
| 磁盘满 | 降级到 Console Sink |
| S3 不可达 | 本地保留归档文件，网络恢复后重试 |
| 加密密钥错误 | 降级为明文写入 + 警告日志 |
| Channel 满 | 阻塞发送端（背压） |

## 加密文件格式

```
┌─────────────────────────────────────────┐
│ Magic Header (8 bytes)  - "ENCLOG1\0"   │
├─────────────────────────────────────────┤
│ Version (2 bytes)       - 0x0001        │
├─────────────────────────────────────────┤
│ Algorithm ID (2 bytes)  - 0x0001 (AES)  │
├─────────────────────────────────────────┤
│ Nonce (12 bytes)        - 随机/文件唯一  │
├─────────────────────────────────────────┤
│ Encrypted Data (可变)    - AES-GCM 密文 │
├─────────────────────────────────────────┤
│ Auth Tag (16 bytes)     - GCM 认证标签  │
└─────────────────────────────────────────┘
```

密钥从环境变量 `LOG_ENCRYPTION_KEY` 读取（Base64 编码的 32 字节）。
