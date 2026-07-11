# Agents Guide

## Overview

inklog 是 Rust 企业级结构化日志库，基于 Tokio 异步运行时，提供多输出目标（Console/File/Database）、日志轮转、压缩（ZSTD/GZIP/Brotli/LZ4）、AES-256-GCM 加密、数据掩码、健康监控等企业级功能。

- **版本**: 0.1.4
- **edition**: 2024
- **MSRV**: Rust 1.85+
- **许可证**: MIT
- **仓库**: https://github.com/Kirky-X/inklog

## Project Structure

```
inklog/
- src/
  - lib.rs                    # 公共 API 入口
  - log_level.rs              # 日志级别定义
  - validation/               # 输入验证 (path, sanitize)
  - domain/                   # 领域层
    - core/                   # 核心日志管理
      - manager.rs            # LoggerManager (~1113 行)
      - container.rs          # InklogContainer DI 容器
      - subscriber.rs         # tracing subscriber
      - mod.rs
    - config/                 # 配置系统
      - config.rs             # InklogConfig 等配置结构体 (~989 行)
      - mod.rs
    - types/                  # 类型定义
      - error.rs              # InklogError 错误类型
      - log_record.rs         # LogRecord 日志记录
      - mod.rs
    - db_provider.rs          # 数据库提供者 trait
    - mod.rs
  - support/                  # 支撑层
    - io/                     # I/O 层
      - sink/                 # 输出 Sink
        - mod.rs              # LogSink trait 定义
        - console.rs          # ConsoleSink 控制台输出
        - file.rs             # FileSink 文件输出 (~1513 行，最复杂)
        - database/           # DatabaseSink 数据库输出
          - mod.rs
        - ring_buffered_file.rs # RingBufferedFileSink
        - compression.rs      # 压缩 (ZSTD/GZIP/Brotli/LZ4)
        - encryption.rs       # AES-256-GCM 加密
        - rotation.rs         # 日志轮转
        - circuit_breaker.rs  # 断路器保护
        - entity.rs           # Sink 实体
        - registry.rs         # Sink 注册表
      - log_adapter.rs        # log → tracing 适配器
      - mod.rs
    - processing/             # 处理层
      - masking.rs            # PII 数据掩码
      - template.rs          # 日志模板
      - object_pool.rs        # 对象池 (~650 行)
      - mod.rs
    - observability/          # 可观测性
      - metrics.rs            # 健康监控和指标 (~1475 行)
      - mod.rs
    - mod.rs
  - integrations/             # 集成层
    - kit/                    # DI Kit 集成
      - module.rs             # InklogModule
      - mod.rs
    - infra/                  # 基础设施适配器
      - database.rs           # 数据库适配器
      - cache.rs              # 缓存适配器 (OxCacheAdapter)
      - config.rs             # 配置适配器
      - mod.rs
    - dbnexus_adapter.rs      # DbNexus 日志适配器
    - mod.rs
  - cli/                      # 命令行工具
    - mod.rs                  # CLI 入口 (inklog-cli)
    - decrypt.rs              # 解密命令
    - generate.rs             # 配置生成命令
    - validate.rs             # 验证命令
  - i18n/                     # 国际化 (可选, feature = "i18n")
    - mod.rs
- tests/                      # 测试套件
  - unit/                     # 单元测试
  - integration/              # 集成测试
  - combinations/             # 组合测试
  - performance/              # 性能测试
- examples/                   # 示例 crate (workspace member)
- docs/                       # 项目文档
- scripts/                    # 脚本 (pre-commit 等)
- benches/                    # 基准测试
- Cargo.toml                  # 项目配置
- deny.toml                   # cargo-deny 配置
- rustfmt.toml                # 代码格式化配置
- .clippy.toml               # Clippy 配置
```

## Where to Look

| 任务 | 位置 | 说明 |
|------|----------|-------|
| 初始化日志 | `src/domain/core/manager.rs` | `LoggerManager::new()` / `builder()` / `with_config()` |
| 配置系统 | `src/domain/config/config.rs` | `InklogConfig`, `FileSinkConfig`, `DatabaseSinkConfig` 等 |
| 核心日志 | `src/domain/core/` | LoggerManager, LoggerBuilder, InklogContainer |
| 输出 Sink | `src/support/io/sink/` | ConsoleSink, FileSink, DatabaseSink, RingBufferedFileSink |
| 文件输出 | `src/support/io/sink/file.rs` | 轮转、压缩、加密 (~1513 行，最复杂模块) |
| 数据库输出 | `src/support/io/sink/database/mod.rs` | Sea-ORM 批量写入 |
| 处理器 | `src/support/processing/` | masking, template, object_pool |
| CLI | `src/cli/` | decrypt, generate, validate 命令 |
| 可观测性 | `src/support/observability/metrics.rs` | 健康监控、Prometheus 指标 |
| 数据掩码 | `src/support/processing/masking.rs` | PII 正则脱敏 |
| 加密 | `src/support/io/sink/encryption.rs` | AES-256-GCM |
| 压缩 | `src/support/io/sink/compression.rs` | ZSTD/GZIP/Brotli/LZ4 |
| 断路器 | `src/support/io/sink/circuit_breaker.rs` | Sink 故障保护 |
| DI 集成 | `src/integrations/kit/` | trait-kit 依赖注入 |
| 适配器 | `src/integrations/infra/` | cache, config, database 适配器 |
| 错误类型 | `src/domain/types/error.rs` | `InklogError` |
| 日志记录 | `src/domain/types/log_record.rs` | `LogRecord` |
| 路径验证 | `src/validation/path.rs` | `PathValidator` |
| 日志消毒 | `src/validation/sanitize.rs` | `LogSanitizer` |

## Conventions

- **edition**: 2024, rust 1.85+
- **许可证**: MIT
- **格式化**: `rustfmt.toml` 配置 (hard tabs, 4-space width)
- **错误处理**: `thiserror` 用于错误类型, `anyhow` 用于错误上下文
- **异步**: tokio 运行时, `async/.await` 全异步
- **序列化**: `serde` derive macros
- **Feature flags**: `http`, `cli` (默认); `sqlite`, `postgres`, `mysql`, `duckdb`, `kit`, `i18n` (可选)
- **依赖必须通过 feature 门控**: 可选依赖使用 `optional = true` + feature flag
- **测试**: `serial_test` 用于并行测试隔离
- **DI**: trait-kit 框架用于依赖注入
- **TDD 开发流程**: Red → Green → Commit → Analyze → Next

## Anti-Patterns

- **禁止** 在 async 上下文中阻塞 (使用 `tokio::spawn`)
- **禁止** 使用 `std::thread` (使用 tokio tasks)
- **禁止** 提交 `logs/` 或 `temp/rust/` 文件
- **禁止** 修改外部依赖 (`oxcache`, `dbnexus`, `sea-orm`, `trait-kit` 等)
- **禁止** 向后兼容 (FORBIDDEN backward compatibility)
- **禁止** 修改外部依赖声明

## Commands

```bash
# 构建
cargo build --all-features

# 测试
cargo test --lib --all-features
cargo test --all-features --workspace

# 格式化
cargo fmt --all -- --check

# Clippy (warnings = errors)
cargo clippy --all-targets --all-features -- -D warnings

# 安全审计
cargo deny check

# 覆盖率 (目标: 95%+)
cargo tarpaulin --out Html --all-features

# 文档
cargo doc --all-features
```

## Notes

- `src/support/io/sink/file.rs` 是最大的模块 (~1513 行) — 修改时需谨慎
- DatabaseSink 支持 PostgreSQL/MySQL/SQLite (通过 Sea-ORM)
- CLI 入口在 `src/cli/mod.rs` (通过 `[[bin]]` 在 Cargo.toml 定义)
- 健康监控指标在 `src/support/observability/metrics.rs`
- duckdb feature 仅用于 `--all-features` 测试场景，DatabaseSink 不直接支持 duckdb 驱动
