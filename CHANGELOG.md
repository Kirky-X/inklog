# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### ⚠️ BREAKING CHANGES

- `error` module moved from `src/domain/types/error.rs` to `src/error.rs`, import path `crate::domain::types::error::` → `crate::error::`
- Added `InklogResult<T>` type alias

## [0.1.5] - 2026-07-11

### Changed

- 无代码变更，版本号对齐 workspace 同步升级

### Changed（Phase 6 前置）

- **edition 升级**: 从 edition 2021 升级至 edition 2024
- **MSRV 声明**: `rust-version = "1.85"` 显式声明最低支持 Rust 版本
- **MIT license 统一**: 许可证从 `"MIT OR Apache-2.0"` 统一为 `"MIT"`，移除 Apache-2.0 选项

### Security

- **edition 2024 unsafe 要求**: `std::env::set_var` / `std::env::remove_var` 调用包裹 `unsafe` 块（edition 2024 将其标记为 unsafe 操作）

## [0.1.2] - 2026-07-05

### BREAKING CHANGES

- **LogSink trait async 化**: `LogSink` trait 的 `write`/`flush`/`shutdown` 方法签名改为 `async`
  - `FileSink`、`ConsoleSink`、`RingBufferedFileSink`、`DatabaseSink` 全部迁移至 async
  - `DatabaseSink` 移除 `block_in_place`，使用原生 async
  - `LoggerManager` 中所有 sink 调用更新为 `.await`
  - 自定义 Sink 实现需要相应改为 async

- **ObjectPool API 重构**:
  - `ObjectPool::new`/`with_config`/`get`/`put` 改为 `async` 并返回 `Result<_, InklogError>`
  - 移除内部 `SHARED_RUNTIME`（内部 tokio runtime）
  - ThreadLocal 池（`get_log_record`/`put_log_record`/`get_string_buffer`/`put_string_buffer`）保留为同步便捷函数

- **Cache trait 错误处理显性化**: `Cache` trait 的 `get`/`delete`/`exists` 方法返回 `Result<_, InklogError>`（此前为 `Option`/`bool`，错误被 `tracing::warn!` 静默吞掉）
  - 移除 `OxCacheAdapter::default()`（原本通过 `.expect()` 触发 panic）
  - `ObjectPool::with_config` 不再静默回退到 `Cache::default()`

- **dbnexus feature 拆分**: `dbnexus` feature 拆分为四个独立 feature：`sqlite`、`postgres`、`mysql`、`duckdb`（`duckdb` 仅用于 `--all-features` 测试场景，DatabaseSink 不直接支持 duckdb 驱动）

### Changed

#### 依赖重构

- `oxcache` 升级 0.2.0 → 0.3.3，启用 features `memory`/`serialization`/`tracing`/`macros`
- `dbnexus` 升级 0.2.0 → 0.3.1，新增 `duckdb` feature 用于 `--all-features` 测试场景（dbnexus 0.3.1 规则 2 例外：sqlite+duckdb+postgres+mysql 全部 4 个后端启用时允许共存）
- 移除直接的 `moka` 和 `dashmap` 依赖（现仅通过 oxcache 间接依赖）
- `sea-orm` TLS 后端从 `runtime-tokio-native-tls` 切换至 `runtime-tokio-rustls`

#### 代码清理

- 移除源码注释中所有 `confers` 引用
- 清理 `object_pool.rs` 中的死代码（1507 → ~650 行）

### Removed

- `SHARED_RUNTIME`（内部 tokio runtime）
- `ObjectPoolBuilder`、`PoolMetrics` 类型
- `ObjectPool` 死代码方法：`with_capacity`/`remove`/`contains`/`capacity`/`metrics`/`clear`/`execute_async`
- `LOG_RECORD_POOL`/`STRING_POOL` 静态变量
- `OxCacheAdapter::default()` 实现
- 直接的 `moka` 和 `dashmap` 依赖

### Security

- 在 `deny.toml` 忽略列表中添加 `RUSTSEC-2026-0173`（proc-macro-error2 unmaintained，transitive via dbnexus-macros/sea-bae，无安全升级路径）

## [0.1.1] - 2026-06-29

### BREAKING
- 完全移除 S3/AWS 代码和依赖（aws feature、aws-sdk-s3、aws-config 等）
- 移除 ArchiveConfig、S3Error、ArchiveError 类型
- 移除 archive 模块和 S3 归档服务
- **MSRV 提升**: Rust 1.75+ → 1.85+（因 rand 0.10 要求 MSRV 1.85）
- **rand 0.9 → 0.10**: `RngCore` trait 重命名为 `Rng`，`Rng` 重命名为 `RngExt`，`OsRng` 重命名为 `SysRng`
- **aes-gcm 0.10 → 0.11**: `Nonce::from_slice` 已废弃，改用 `Nonce::from`；`cipher.encrypt/decrypt` 接受 `&Nonce` 而非 `Nonce`

### Added
- 新增 10 个示例：object_pool、path_validator、log_sanitizer、log_adapter、compression、rotation、ring_buffered_file、config_file、metrics、circuit_breaker
- 测试覆盖率达 90.12%（3291/3652 行）
- 新增 Docker 测试基础设施（SQLite/PostgreSQL/MySQL）
- 新增 CI 流水线（test-docker.yml）
- Cargo.toml 新增 `rust-version = "1.85"` 显式声明 MSRV

### Changed
- **依赖升级**:
  - `toml` 0.9 → 1.1（major 版本升级）
  - `validator` 0.19 → 0.20
  - `aes-gcm` 0.10 → 0.11（破坏性 API 变更已适配）
  - `rand` 0.9 → 0.10（破坏性 API 变更已适配，MSRV 1.85+）
  - `parquet` 57.3 → 59.0（major 版本升级）
  - `arrow-array` 57.3 → 59.0
  - `arrow-schema` 57.3 → 59.0
  - `cron` 0.15 → 0.17
  - `sha2` 0.10 → 0.11
  - `pbkdf2` 0.12 → 0.13
- CI MSRV 矩阵从 1.70.0 提升至 1.85.0
- README/README_zh Rust 版本徽章从 1.75+ 更新为 1.85+
- docs/CONTRIBUTING.md MSRV 与版本引用同步更新
- examples/Cargo.toml 移除 s3_archive [[bin]] 定义
- lib.rs 移除 "S3 归档" 描述

### Fixed
- 修复 examples/Cargo.toml s3_archive 残留导致编译失败
- 修复 lib.rs S3 归档描述残留
- 适配 rand 0.10 破坏性 API：`RngCore` → `Rng`，`Rng` → `RngExt`（src/support/io/sink/encryption.rs、src/support/io/sink/file.rs、src/cli/decrypt.rs、benches/inklog_bench.rs）
- 适配 aes-gcm 0.11 破坏性 API：`Nonce::from_slice` → `Nonce::from`，`cipher.encrypt/decrypt` 接受引用（src/support/io/sink/file.rs、src/cli/decrypt.rs、tests/cli_integration.rs）

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
[Unreleased]: https://github.com/Kirky-X/inklog/compare/v0.1.5...HEAD
[0.1.5]: https://github.com/Kirky-X/inklog/compare/v0.1.2...v0.1.5
[0.1.2]: https://github.com/Kirky-X/inklog/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Kirky-X/inklog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Kirky-X/inklog/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Kirky-X/inklog/releases/tag/v0.0.0
