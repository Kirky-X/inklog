<div align="center">

<img src="docs/assets/inklog.png" alt="inklog" width="180">

[![CI Status](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/inklog.svg)](https://crates.io/crates/inklog) [![Docs.rs](https://docs.rs/inklog/badge.svg)](https://docs.rs/inklog) [![Downloads](https://img.shields.io/crates/d/inklog.svg)](https://crates.io/crates/inklog) [![License](https://img.shields.io/crates/l/inklog.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/inklog/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/inklog)

**中文** | [English](README_EN.md)

**企业级 Rust 日志基础设施**

[✨ 功能特性](#-功能特性) • [🚀 快速开始](#-快速开始) • [📚 文档](#-文档) • [💻 示例](#-示例) • [🤝 参与贡献](#-参与贡献)

</div>

---

<div align="center" style="padding: 32px; margin: 24px 0">

### ⚡ 结构化日志，安全内建

一条日志从采集、脱敏、加密到多端落盘，全链路可控：

<table style="width:100%; border-collapse: collapse">
<tr><td align="center" width="25%" style="padding: 12px">⚡<br><b>异步高吞吐</b><br><span style="color:#64748B">Tokio 异步 + 有界通道，批量写入与背压控制</span></td><td align="center" width="25%" style="padding: 12px">🔒<br><b>安全内建</b><br><span style="color:#64748B">AES-256-GCM 加密、PII 脱敏、密钥内存清零</span></td><td align="center" width="25%" style="padding: 12px">🎯<br><b>多目标输出</b><br><span style="color:#64748B">控制台、文件、数据库、TCP/UDP 转发、OTLP 导出</span></td><td align="center" width="25%" style="padding: 12px">📊<br><b>全链路可观测</b><br><span style="color:#64748B">健康检查端点、Prometheus 指标、trace_id 关联</span></td></tr>
</table>

</div>

---

## 📋 目录

<details open>
<summary>📑 目录</summary>

- [✨ 功能特性](#-功能特性)
- [🚀 快速开始](#-快速开始)
- [🎨 特性标志](#-特性标志)
- [📚 文档](#-文档)
- [💻 示例](#-示例)
- [🏗️ 架构](#️-架构)
- [🧪 测试](#-测试)
- [📊 性能](#-性能)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#️-开发路线图)
- [🤝 参与贡献](#-参与贡献)
- [📋 更新日志](#-更新日志)
- [📄 许可证](#-许可证)
- [🙏 致谢](#-致谢)
- [📞 联系与支持](#-联系与支持)
- [⭐ Star 历史](#-star-历史)

</details>

---

## ✨ 功能特性

inklog 是面向生产环境的日志基础设施：应用代码继续使用 `log` / `tracing` 标准宏，由 inklog 接管订阅、脱敏、分发与落盘。下表为主要能力，全部与仓库代码和 [docs/](docs/USER_GUIDE.md) 文档对应。

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>异步管线</b><br><span style="color:#64748B">Crossbeam 有界通道 + 专用工作线程池，发送端非阻塞、队列满时背压</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📁 <b>文件输出</b><br><span style="color:#64748B">按大小与时间轮转、<code>BufWriter</code> 缓冲、可选压缩与加密</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🗄️ <b>数据库输出</b><br><span style="color:#64748B">批量落库、连接池、分区表支持，经 dbnexus 适配四种后端</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🎭 <b>数据脱敏</b><br><span style="color:#64748B">敏感字段名检测 + 正则规则库，<code>fast-masking</code> 加速多模式匹配</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🎨 <b>模板格式化</b><br><span style="color:#64748B"><code>{timestamp}</code> <code>{level}</code> <code>{message}</code> <code>{trace_id}</code> 等占位符模板</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧩 <b>依赖注入</b><br><span style="color:#64748B"><code>Cache</code> / <code>Config</code> / <code>Database</code> trait 抽象，适配器可替换、可 Mock</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔁 <b>可靠性</b><br><span style="color:#64748B">断路器、DB → File → Console 三级降级、健康检查线程自动恢复</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🔀 <b>动态 Sink</b><br><span style="color:#64748B"><code>LoggerBuilder::add_sink</code> 注册第三方 Sink，每 Sink 独立通道；中间件链、采样器、令牌桶限流装饰器</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🌡️ <b>运行时热调</b><br><span style="color:#64748B"><code>set_level</code> 经 <code>tracing_subscriber::reload</code> 即时调整全局与 per-target 级别</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🔍 <b>日志检索</b><br><span style="color:#64748B"><code>inklog-cli query</code> 按时间、级别、关键词检索本地日志（自动解密解包）</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>i18n</b><br><span style="color:#64748B">错误消息经 Fluent + ICU 按系统 locale 渲染（zh-CN / en）</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📈 <b>可观测性</b><br><span style="color:#64748B">健康状态、通道水位、连接池与写延迟直方图的 Prometheus 导出</span></td>
</tr>
</table>

<details>
<summary>📦 进阶能力清单（对应 <code>src/</code> 模块）</summary>

- **Sink 家族**（`src/support/io/sink/`）：`console`、`file`（轮转/压缩/加密）、`database`（批量/分区/断路器）、`ring_buffered_file`（通道缓冲高吞吐）、`net`（TCP 可 TLS + UDP）、`otlp`、`middleware`、`sampling`、`rate_limit`
- **处理层**（`src/support/processing/`）：`template` 模板引擎、`masking` 脱敏引擎与规则注册表、`object_pool` LogRecord/字符串对象池
- **可观测性**（`src/support/observability/`）：`Metrics`、`HealthStatus`、`SinkHealthMonitor`、回退状态
- **校验**（`src/validation/`）：`PathValidator` 路径穿越防护、`LogSanitizer` 日志内容净化
- **归档防篡改**（`src/support/audit_chain.rs`）：归档 HMAC-SHA256 链，防删除、重排与伪造
- **集成适配**（`src/integrations/`）：`OxCacheAdapter`、`InklogConfigAdapter`、`DbNexusAdapter`、trait-kit `InklogModule`、dbnexus 审计桥、confers 配置与 watch
- **CLI**（`src/cli/`）：`decrypt`、`generate`、`validate`、`query` 四个子命令

</details>

---

## 🚀 快速开始

### 环境要求

| 要求 | 版本 |
|------|------|
| Rust | 1.97.1+（仓库经 `rust-toolchain.toml` 固定） |
| edition | 2024 |
| 平台 | Linux / macOS / Windows |

### 安装

```bash
cargo add inklog
```

或在 `Cargo.toml` 中显式声明（`default = []`，默认仅启用核心能力）：

```toml
[dependencies]
inklog = "0.3.0-rc.3"
```

### 最小可运行示例

出自 [`examples/src/bin/core/basic.rs`](examples/src/bin/core/basic.rs)：

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 默认配置初始化并安装全局 subscriber（Console Sink，级别 info）
    let logger = LoggerManager::new().await?;

    tracing::info!("Hello, inklog!");
    tracing::info!(user_id = 42, action = "login", "结构化字段示例");

    // 退出前排空通道并关闭全部 Sink
    logger.shutdown()?;
    std::mem::forget(logger); // 已显式关闭，阻止 Drop 重复关闭
    Ok(())
}
```

### 核心概念

1. **初始化**：`LoggerManager` 负责安装全局 tracing subscriber 与 `log` crate 前端，进程内单例语义（`init_inklog_logger()` 提供便捷入口）。
2. **记录**：业务代码使用 `tracing::info!` 等标准宏，无侵入。
3. **Sink**：输出目标抽象（`LogSink` / `AsyncSink` trait），console / file / database / net / otlp 内置实现，可自定义。
4. **配置**：`InklogConfig` 支持 TOML 文件与 `INKLOG_*` 环境变量覆盖，优先级为环境变量 > 配置文件 > 默认值。
5. **关闭**：应用退出前调用 `shutdown()`，等待通道中剩余日志全部写入。

---

## 🎨 特性标志

`default = []`：默认组合只包含核心能力，下列 feature 全部按需显式启用（依据 `Cargo.toml` `[features]` 定义）。

| 标志 | 默认 | 说明 |
|------|:----:|------|
| `sqlite` | ❌ | SQLite 数据库后端（经 dbnexus，rustls 运行时） |
| `postgres` | ❌ | PostgreSQL 数据库后端（经 dbnexus） |
| `mysql` | ❌ | MySQL 数据库后端（经 dbnexus） |
| `duckdb` | ❌ | DuckDB 数据库后端（经 dbnexus） |
| `http` | ❌ | Axum HTTP 健康与指标端点（axum + axum-server，TLS 走 rustls） |
| `cli` | ❌ | `inklog-cli` 命令行工具（clap + glob） |
| `kit` | ❌ | trait-kit 生命周期与可观测集成（`InklogModule`），需至少一个数据库后端 feature |
| `compression` | ❌ | Zstd 压缩轮转日志文件（zstd） |
| `gzip` | ❌ | Gzip 压缩后端（flate2 纯 Rust；未启用 `compression` 时 FileSink 轮转回退 gzip） |
| `parquet` | ❌ | Parquet/Arrow 导出（数据库 Sink 归档） |
| `fast-masking` | ❌ | Aho-Corasick 多模式脱敏加速 |
| `dbnexus-audit` | ❌ | dbnexus AuditStorage 端口适配器，审计事件经 inklog DB sink 落库，可与任一后端组合 |
| `config-confers` | ❌ | 配置经 confers 加载 + watch 热更新级别与轮转参数 |
| `kms` | ❌ | KMS 密钥提供者（`EnvKeyProvider` / `ConfersKeyProvider` / Vault transit MVP） |
| `net-sink` | ❌ | 网络转发 Sink（TCP 可 TLS + UDP，断线缓冲与自动重连） |
| `otlp` | ❌ | OTLP/HTTP JSON 日志导出 MVP（手写传输，零新增依赖） |
| `test-utils` | ❌ | 测试面 mock 导出（`MockCache` / `MockConfig` / `MockDatabaseAdapter`），不入 default 与任何生产组合 |

> ⚠️ **数据库后端互斥**：`sqlite` / `postgres` / `mysql` / `duckdb` 互斥（经 dbnexus 强制，embedded 与 server-side 驱动不得混用），不适用 `--all-features`，请按后端分组启用。

---

## 📚 文档

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/USER_GUIDE.md) | 从安装、配置详解到高级主题的完整教程 |
| [📘 API 参考](docs/API_REFERENCE.md) | 核心类型、配置结构体、错误类型与 trait 的逐项说明 |
| [🏗️ 架构文档](docs/ARCHITECTURE.md) | 分层设计、依赖注入架构、数据流与并发模型 |
| [📊 性能基线](docs/PERFORMANCE.md) | criterion 基准环境、方法与正式基线数字 |
| [🧪 测试场景](docs/TEST_SCENARIOS.md) | 测试金字塔、E2E 场景定义与组合矩阵 |
| [🔒 安全文档](docs/SECURITY.md) | 安全设计、漏洞报告流程与合规性说明 |
| [📋 更新日志](docs/CHANGELOG.md) | 按 Keep a Changelog 格式维护的版本记录 |
| [🤝 贡献指南](docs/CONTRIBUTING.md) | 开发环境、TDD 流程与代码风格约定 |
| [📦 在线 API 文档](https://docs.rs/inklog) | docs.rs 自动生成的最新文档 |

---

## 💻 示例

[`examples/`](examples/) 是 workspace 内的独立 crate（`inklog-examples`），按目录分为 7 类共 39 个示例。在仓库根目录运行：

```bash
cargo run --package inklog-examples --example <名称>
```

#### 配置（config）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `config_file` | 配置文件加载（Layer 1 本地资源） | 无 |
| `config_inspect` | 配置检查：`sinks_enabled()` 与 `LoggerManager::load()` | 无 |
| `env_overrides` | 环境变量覆盖配置加载 | 无 |

#### 核心（core）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `basic` | 基础用法：初始化、级别、结构化字段、健康检查、优雅关闭 | 无 |
| `builder` | Builder 模式配置 | 无 |
| `all_features` | 完整功能演示 | 无 |
| `production` | 生产环境配置 | 无 |
| `template` | 日志模板 | 无 |
| `error_handling` | 错误处理（Layer 0 零依赖） | 无 |
| `i18n` | 国际化格式化 | 无 |

#### Sink 与输出（sinks）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `console` | Console Sink | 无 |
| `file` | File Sink | 无 |
| `rotation` | 日志轮转（Layer 1 本地资源） | 无 |
| `ring_buffered_file` | ChannelBufferedFileSink（Layer 1 本地资源） | 无 |
| `archive_format` | 归档格式（Layer 0 零依赖） | 无 |
| `compression` | Zstd 压缩与解压缩 | `compression` |
| `parquet_archive` | Parquet 归档 | `parquet` + 任一数据库后端 |
| `partition_strategy` | 数据库分区策略 | 无 |

#### 数据库（database）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `database` | Database Sink（SQLite 内存库） | 任一数据库后端（示例文档用 `sqlite`） |
| `database_pg_mysql` | PostgreSQL/MySQL 数据库驱动演示 | 无 |
| `di_example` | 依赖注入模式 | `sqlite`（经 required-features 强制） |

#### 基础设施（infra）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `channel_strategy` | 自适应 Channel 策略 | 无 |
| `circuit_breaker` | 断路器（Layer 2 外部服务） | 无 |
| `fallback` | Sink 降级机制 | 无 |
| `log_adapter` | `log` crate 适配桥（Layer 0 零依赖） | 无 |
| `log_level` | LogLevel 解析、比较与 Display | 无 |
| `metrics` | 健康监控与指标收集（Layer 2 外部服务） | 无 |
| `object_pool` | 对象池（Layer 0 零依赖） | 无 |
| `output_format` | 输出格式（Layer 0 零依赖） | 无 |
| `performance` | 性能演示 | 无 |
| `rate_limiter` | 速率限制器（Layer 0 零依赖） | 无 |
| `runtime_ops` | LoggerManager 运行时操作 API | 无 |

#### 网络（network）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `http` | HTTP 健康检查与指标端点演示 | 无 |
| `http_auth` | HTTP 认证与 IP 白名单配置 | 无 |
| `tls_config` | TLS 配置 | 无 |

#### 安全（security）

| 示例 | 说明 | 特性要求 |
|------|------|----------|
| `encryption` | 日志加密 | 无 |
| `log_sanitizer` | 日志内容净化（Layer 0 零依赖） | 无 |
| `masking` | 数据脱敏 | 无 |
| `path_validator` | 路径验证器（Layer 0 零依赖） | 无 |

<div align="center">

**[📂 浏览全部示例 →](examples/)**

</div>

---

## 🏗️ 架构

inklog 采用分层异步架构：`domain`（管理器、Subscriber 与工作线程）经 `support::processing` 完成模板渲染与脱敏后进入 Crossbeam 有界通道，由专用线程分发给 `support::io::sink` 各 Sink；`integrations` 以 trait 适配 oxcache / confers / dbnexus / trait-kit，`support::observability` 经 `http` feature 暴露健康与指标端点。分层架构图、分层职责表与模块树对照见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)「分层架构」。

### 🔀 核心执行链路

一条日志从 `tracing` 标准宏记录、脱敏、非阻塞进入有界通道（满时背压）到文件 / 数据库 Sink 落盘并回写指标的完整时序图，以及关键要点（默认通道容量 10000、3 个工作线程，可经 `PerformanceConfig` 调整；加密、压缩与轮转按轮转文件触发，不占用单条记录写入热路径），见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)「核心执行链路」。

### 🧯 故障降级与自愈

Sink 写入失败经断路器（默认失败阈值 5 次、冷却 30 秒）重试或触发 DB → File → Console 三级降级，健康检查线程每 10 秒巡检并自动重建不健康 Sink、重置断路器。完整流程图见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)「故障降级与自愈」。

---

## 🧪 测试

### 测试策略

| 类型 | 位置 | 说明 |
|------|------|------|
| 单元测试 | `src/` 内联 `#[cfg(test)]` | 各模块边界与异常场景，Mock 经 cfg(test) 直接可见 |
| 集成测试 | `tests/integration/`、`tests/cli_integration.rs` | 覆盖批量写入、HTTP、CLI、压缩比、Parquet、自动恢复等，需 `sqlite,http,cli,compression,parquet,test-utils` 组合 |
| 组合测试 | `tests/combinations/` | feature 组合矩阵与多 Sink 降级，需 `sqlite` |
| 端到端测试 | `tests/e2e/e2e_advanced.rs` | 226 个场景、15 个场景域（见 [docs/TEST_SCENARIOS.md](docs/TEST_SCENARIOS.md)） |
| Docker 数据库集成 | `tests/docker/` + `docker/docker-compose.test.yml` | PostgreSQL / MySQL / SQLite 生命周期验证 |
| 性能测试 | `tests/performance/` + `benches/` | 大容量、长时间运行测试与 criterion 基准 |

**测试规模**（截至 v0.3.0-rc.3，按 `#[test]` / `#[tokio::test]` 统计）：`src/` 内联 1,255 个 + `tests/` 目录 528 个，共 **1,783 个测试函数**；另有 criterion 基准函数 18 个（`benches/inklog_bench.rs` 15 个、`benches/rc4_pipeline_bench.rs` 3 个）。

### 运行命令（与 CI 一致）

```bash
# CI 测试门禁（数据库后端互斥，不适用 --all-features）
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# Docker 数据库集成测试
docker compose -f docker/docker-compose.test.yml up -d

# 覆盖率门禁（CI 要求 ≥80% 行覆盖）
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 80

# 基准测试
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
```

> **本地化提示**：错误消息经 ICU/Fluent 按系统 locale 渲染。若测试断言英文消息文本，请设置 `INKLOG_LOCALE=en`（如 CI 或非英文系统环境）以固定输出语言。

### 代码质量门禁

```bash
cargo fmt --all -- --check                    # 格式检查
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings  # 零告警
cargo deny check                              # 依赖漏洞 / 许可证 / 重复依赖
cargo audit                                   # 安全公告（lefthook pre-push）
```

---

## 📊 性能

### 基线数字

首份正式基线（2026-09-11，criterion 中位数，覆盖写入 / 序列化 / 加密三条路径）的完整数字、环境口径与「复现」步骤见 [docs/PERFORMANCE.md](docs/PERFORMANCE.md)。

### 设计要点

| 机制 | 参数 | 效果 |
|------|------|------|
| 有界通道背压 | `channel_capacity` 默认 10000 | 防止内存溢出，通道水位经指标暴露 |
| 数据库批量写入 | `batch_size` 默认 100，刷新间隔默认 500 ms | docs/ARCHITECTURE.md 参考值：批量 100 条约 10,000 行/s，逐条插入约 100 行/s |
| 脱敏按需开启 | `masking_enabled` | 正则脱敏是主链路中最贵的安全环节，建议仅在需要的 sink 开启 |
| Zstd 压缩 | 级别 0-22，默认 3 | 默认级别压缩比约 3.5x |

复现方式见 [docs/PERFORMANCE.md](docs/PERFORMANCE.md)「复现」章节。

---

## 🔒 安全

### 漏洞报告

发现安全漏洞请**不要**公开披露，按 [docs/SECURITY.md](docs/SECURITY.md) 流程负责任上报：

- **首选**：邮件 [security@inklog.dev](mailto:security@inklog.dev)
- **备选**：[GitHub Security Advisories](https://github.com/Kirky-X/inklog/security/advisories)
- **响应时限与协调披露流程**：见 [docs/SECURITY.md](docs/SECURITY.md)「漏洞报告流程」。

### 安全设计

静态加密（AES-256-GCM）、密钥派生与内存清零、路径与内容防护、SQL 注入防护、HTTP 访问控制、归档防篡改链，以及对 GDPR / HIPAA / PCI-DSS 的合规映射，逐项说明见 [docs/SECURITY.md](docs/SECURITY.md)「安全设计概览」。

### 供应链安全

仓库维护 [`deny.toml`](deny.toml)；`cargo deny check`（漏洞 / 许可证 / 重复依赖）、`cargo audit`（RustSec 公告）与 pre-commit 私钥扫描的运行位置与口径见 [docs/SECURITY.md](docs/SECURITY.md)「安全设计概览」。

---

## 🗺️ 开发路线图

以下为既有发布安排整理的阶段性目标（节奏随工作区整体发布计划调整）：

| 状态 | 目标 | 说明 |
|:----:|------|------|
| 📋 | v0.3.0 正式发布 | 完成 0.3.0-rc.3 → 0.3.0 正式版 |
| 📋 | 工作区依赖传导同步 | trait-kit 0.5.0、oxcache 0.5.0、dbnexus 0.6.0 |
| 📋 | CI 测试矩阵按数据库后端分组 | 后端 feature 互斥，需按后端拆分验证组合 |
| 📋 | 补齐 MySQL 集成测试环境 | 当前缺少 MySQL 服务导致该后端集成测试阻塞 |
| 📋 | 提升测试覆盖率 | llvm-cov 基线约 80%，向 95%+ 目标提升 |

---

## 🤝 参与贡献

欢迎贡献！完整流程见 [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)。

### 开发环境

| 要求 | 说明 |
|------|------|
| Rust 1.97.1 | 经 `rust-toolchain.toml` 固定 |
| protobuf 编译器 | 含数据库 feature 的组合构建需要 `protoc` |
| lefthook | `bash scripts/install-pre-commit.sh` 安装钩子 |

```bash
git clone https://github.com/Kirky-X/inklog.git
cd inklog
bash scripts/install-pre-commit.sh

# 运行测试（与 CI 相同的 feature 组合）
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"
```

### 提交约定

- **Conventional Commits**：`feat: ...` / `fix: ...` / `docs: ...` 等，commit-msg 钩子强制校验；
- **pre-commit 钩子**：rustfmt、clippy（`-D warnings` 零告警）、`cargo deny check`、私钥扫描；
- **pre-push 钩子**：`cargo audit` 与覆盖率 ≥80% 门禁。

### Pull Request 流程

1. Fork 仓库并创建功能分支（`git checkout -b feature/your-feature`）；
2. 进行修改，为公共 API 补充文档注释；
3. 运行测试、clippy 与 `cargo fmt --all`，确保全部通过；
4. 以 Conventional Commits 风格提交并推送；
5. 打开 Pull Request 并通过 CI 全部质量门禁。

---

## 📋 更新日志

完整版本记录见 [docs/CHANGELOG.md](docs/CHANGELOG.md)（Keep a Changelog 格式，语义化版本）。

### 最近版本

- **0.3.0-rc.3**（2026-09-10）：新增 `init_inklog_logger` 单例初始化、运行时级别热调（`set_level`）、动态 Sink 注册（`LoggerBuilder::add_sink`）、`trace_id`/`span_id` 追踪关联、`inklog-cli query` 日志检索、网络转发 Sink（TCP/UDP）、OTLP 导出 MVP、归档防篡改链与 `docs/PERFORMANCE.md` 首份性能基线；
- **0.3.0-rc.2**（2026-09-03）：集成 trait-kit 0.5.0-rc.2 与 i18n 重构并升级版本号；默认公共 API 移除三个 Mock（BREAKING，外部测试消费者需启用 `test-utils`）；
- **0.2.0**（2026-08-05）：新增 `compression` / `parquet` / `fast-masking` feature 与 i18n 核心模块；新增 ChannelBufferedFileSink、断路器保护与环形缓冲文件 Sink；edition 2024、MSRV 1.94。

---

## 📄 许可证

本项目基于 [MIT License](LICENSE) 发布，附加 [Commons Clause](LICENSE) 条件：未经单独授权，不得销售本软件。版权所有 (c) 2026 Kirky.X。

---

## 🙏 致谢

inklog 的实现依赖这些优秀的开源项目：

- [tokio](https://tokio.rs/)：异步运行时
- [tracing](https://github.com/tokio-rs/tracing)：结构化日志与 Subscriber 生态
- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam)：高性能有界通道
- [axum](https://github.com/tokio-rs/axum)：HTTP 健康与指标端点
- [serde](https://serde.rs/)：序列化框架
- [RustCrypto](https://github.com/RustCrypto)（aes-gcm / sha2 / pbkdf2 / zeroize）：密码学原语
- [Project Fluent](https://projectfluent.org/) 与 [ICU](https://icu4x.unicode.org/)：国际化消息格式
- [criterion](https://github.com/bheisler/criterion.rs)：统计严谨的基准测试框架

数据库、缓存、配置与生命周期集成分别由同工作区项目 dbnexus、oxcache、confers、trait-kit 提供。

---

## 📞 联系与支持

<div align="center" style="margin: 24px 0;">

<table style="width:100%; max-width: 600px;">
<tr>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/inklog/issues">
<div style="padding: 16px; border-radius:8px;">
<b style="color:#991B1B;">📋 Issues</b>
</div>
</a>
<br><span style="color:#64748B;">报告 bug 和问题</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/inklog/discussions">
<div style="padding: 16px; border-radius:8px;">
<b style="color:#1E40AF;">💬 Discussions</b>
</div>
</a>
<br><span style="color:#64748B;">提问和分享想法</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/inklog">
<div style="padding: 16px; border-radius:8px;">
<b style="color:#1E293B;">🐙 GitHub</b>
</div>
</a>
<br><span style="color:#64748B;">查看源代码</span>
</td>
</tr>
</table>

</div>

---

## ⭐ Star 历史

<div align="center">

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/inklog&type=Date)](https://star-history.com/#Kirky-X/inklog&Date)

### 💝 支持本项目

如果您觉得这个项目有用，请考虑给它一个 ⭐️！

**由 ❤️ Inklog 团队构建**

<sub>© 2026 Inklog Project. 版权所有。</sub>

**[⬆ 返回顶部](#-目录)**

</div>
