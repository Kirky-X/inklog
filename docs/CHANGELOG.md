# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security Fixes

- **恒定时间比较**: 在 `src/manager.rs` 中使用 `subtle::ConstantTimeEq` 实现时序安全的 bearer token 比较，防止时序攻击
- **拒绝弱密钥**: 在 `src/sink/file.rs` 中实现基于 Shannon 熵（阈值 4.0）的密钥强度验证，低熵密钥将被拒绝并返回错误
- **随机盐值 PBKDF2**: 在 `src/sink/encryption.rs` 中使用 `rand::rng().fill_bytes()` 生成 16 字节随机盐值替代硬编码盐值

### Code Quality Improvements

- **LogLevel 枚举**: 新增 `src/log_level.rs` 定义 `LogLevel` 枚举类型（Trace/Debug/Info/Warn/Error/Fatal），替代字符串字面量
- **apply_env! 宏**: 在 `src/config.rs` 中定义 `apply_env!` 宏消除配置解析重复代码，支持嵌套字段路径和环境变量覆盖
- **词边界正则**: 在 `src/masking.rs` 中使用 `\b` 词边界正则替代简单子串匹配，避免 "cakey"/"polygon" 等字段误判
- **模板转义修复**: 在 `src/template.rs` 中支持 `{{` → `{` 和 `}}` → `}` 转义序列

### Bug Fixes

- **集成测试密钥**: 修复集成测试中弱熵密钥导致加密文件未生成的问题
- **confers 库**: 修复 `ConfigError::LockPoisoned` 枚举变体在 match 中缺失的问题
- **chrono Datelike**: 修复 `src/archive/mod.rs` 中 `Datelike` trait 未导入导致编译错误
- **LogSink trait 导入**: 修复集成测试中 `LogSink` trait 未导入导致的编译错误
- **apply_env! 宏**: 修复宏无法匹配 `self.global.level` 等嵌套路径的问题

### Dependencies

- 添加 `subtle` crate 用于恒定时间比较

### Added
#### 依赖注入架构 (v0.2.0)

完整的依赖注入支持，实现控制反转 (IoC) 设计模式：

- **Trait 定义**: 基础设施层抽象接口
  - `Cache` trait: 缓存操作接口 (`src/infrastructure/cache.rs`)
  - `Config` trait: 配置访问接口 (`src/infrastructure/config.rs`)
  - `Database` trait: 数据库操作接口 (`src/infrastructure/database.rs`)

- **适配器实现**: 连接外部依赖
  - `OxCacheAdapter`: 连接 oxcache 缓存服务
  - `ConfersAdapter`: 连接 confers 配置系统
  - `DbNexusAdapter`: 连接 dbnexus 数据库连接池 (需要 `dbnexus` feature)

- **Mock 实现**: 测试替身
  - `MockCache`: 内存缓存模拟
  - `MockConfig`: 内存配置模拟
  - `MockDatabaseAdapter`: 内存数据库模拟

- **依赖注入 API**:
  - `LoggerDependencies`: 依赖集合结构体
  - `LoggerManager::with_dependencies()`: 依赖注入构造方法
  - `LoggerBuilder::cache()`: 设置缓存依赖
  - `LoggerBuilder::config()`: 设置配置依赖
  - `LoggerBuilder::with_database()`: 设置数据库依赖

- **应用容器**:
  - `InklogContainer`: 应用级依赖管理容器
  - `InklogContainerBuilder`: 容器构建器
  - 实例共享：多个 Logger 共享同一依赖实例

- **示例代码**: `examples/src/bin/di_example.rs`

### BREAKING CHANGES

- **LogSink trait**: 方法签名从 `&mut self` 改为 `&self`
  - `write()`, `flush()`, `shutdown()` 等方法现在使用 `&self`
  - 自定义 Sink 实现需要使用内部可变性（RwLock/Mutex）
  - 所有内置 Sink 实现已更新
- 移除 `aws` feature 和所有 S3/archive 相关代码
- 移除 `aws-sdk-s3`、`aws-config`、`aws-types`、`aws-credential-types` 依赖
- 从 `default` feature 中移除 `aws`
- 移除 `InklogConfig.s3_archive` 配置字段
- 移除 `InklogError::S3Error` 和 `InklogError::ArchiveError` 错误变体
- 移除 `src/integrations/storage/archive/` 整个目录
- 移除 `examples/src/bin/s3_archive.rs` 示例
- 移除 `DatabaseSinkConfig` 中的 `archive_to_s3`、`archive_after_days`、`s3_bucket`、`s3_region` 字段

### Migration

- 使用 `default` feature 的用户不再获得 S3 归档能力
- 使用 `aws` feature 的用户需要移除该 feature 引用
- 配置文件中的 `[s3_archive]` 部分需要移除
- `DatabaseSinkConfig` 中的 S3 相关字段需要移除

### Changed

- 所有 Sink 实现使用内部可变性模式
- `FileSink`: 使用 `RwLock` 保护内部状态
- `ConsoleSink`: 使用 `RwLock` 保护内部状态
- `DatabaseSink`: 使用 `RwLock` 保护内部状态
- `AsyncFileSink`: 使用 `RwLock` 保护内部状态
- `RingBufferedFile`: 使用 `RwLock` 保护内部状态

#### 计划中的功能 (未来版本)
- 性能基准测试工具
- 更多压缩算法支持
- 分布式追踪集成

### Changed
#### 改进计划
- 配置热重载优化
- 更详细的错误上下文
- 代码质量改进
  - 测试代码中 `unwrap()` → `expect()` 替换 (~52处)
  - 示例代码中 `unwrap()` → `expect()` 替换
  - 修复 clippy 警告

#### 配置系统增强
- S3 加密算法环境变量支持 (`INKLOG_S3_ENCRYPTION_ALGORITHM`)
- S3 KMS 密钥 ID 环境变量支持 (`INKLOG_S3_ENCRYPTION_KMS_KEY_ID`)
- INKLOG_ARCHIVE_FORMAT 环境变量支持

### Fixed
- 配置系统环境变量处理优化
- HTTP 监控端点实现完成
- 归档服务 Parquet 导出配置传递修复
- `src/archive/mod.rs` 中时间戳转换 unwrap 修复
- 文件锁竞争问题修复
- 异步上下文日志丢失问题修复
- 数据库连接池泄漏修复
- S3 分片上传重试逻辑修复

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
[Unreleased]: https://github.com/kirkyx/inklog/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kirkyx/inklog/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/kirkyx/inklog/releases/tag/v0.0.0
