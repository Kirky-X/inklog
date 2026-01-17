# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### AWS S3 归档功能
- S3 云存储集成
- 分片上传支持（>5MB 文件）
- 多种存储类别支持（Standard, Glacier, etc.）
- SSE-AES256 和 SSE-KMS 加密支持

#### HTTP 健康监控端点
- `/metrics` - Prometheus 指标端点
- `/health` - 健康检查端点
- 实时 sink 状态监控
- 延迟直方图指标

#### CLI 工具支持
- `decrypt` - 解密加密日志文件
- `generate` - 生成配置模板
- `validate` - 验证配置文件

#### 数据脱敏功能
- 敏感字段自动检测（password, token, api_key 等）
- 模式匹配脱敏（邮箱、电话、身份证等）
- JSON 结构递归脱敏

#### 数据库增强
- Parquet 格式导出
- 分区表管理
- MySQL 支持
- Parquet 字段过滤配置

### Changed

#### 代码质量改进
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

## [0.1.0] - 2026-01-01

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
