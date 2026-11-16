<div align="center">

<img src="docs/assets/inklog.png" alt="inklog" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/inklog?style=for-the-badge)](https://crates.io/crates/inklog) [![docs.rs](https://img.shields.io/docsrs/inklog?style=for-the-badge)](https://docs.rs/inklog) [![downloads](https://img.shields.io/crates/d/inklog?style=for-the-badge)](https://crates.io/crates/inklog) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/inklog/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/inklog?style=for-the-badge)](https://codecov.io/gh/Kirky-X/inklog) [![license](https://img.shields.io/crates/l/inklog?style=for-the-badge)](LICENSE)

[English](README.md) | 中文

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

inklog 是面向生产环境的日志基础设施：应用代码继续使用 `log` / `tracing` 标准宏，由 inklog 接管订阅、脱敏、分发与落盘。下表为主要能力，全部与仓库代码和 [docs/](docs/cn/USER_GUIDE.md) 文档对应。

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

> 🧱 进阶能力与 `src/` 模块对照清单见 [🏗️ 架构文档](docs/cn/ARCHITECTURE.md)「进阶能力清单」。

---

## 🚀 快速开始

### 环境要求

| 要求 | 版本 |
|------|------|
| Rust | 1.97.1+（仓库经 `rust-toolchain.toml` 固定） |
| edition | 2024 |
| 平台 | Linux / macOS / Windows |

### 安装

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo add inklog
```

</details>

或在 `Cargo.toml` 中显式声明（`default = ["i18n", "cache"]`，默认启用核心能力 + 国际化 + 缓存后端）：

<details>
<summary>📄 代码 · toml</summary>

```toml
[dependencies]
inklog = "0.3.0"
```

</details>

### 最小可运行示例

出自 [`examples/src/bin/core/basic.rs`](examples/src/bin/core/basic.rs)：

<details>
<summary>📄 代码 · rust</summary>

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

</details>

### 核心概念

初始化、记录、Sink、配置、关闭五大核心概念的逐项说明见 [📖 用户指南](docs/cn/USER_GUIDE.md)「核心概念」。

---

## 🎨 特性标志

完整 feature 清单、默认值与启用方式见 [📖 用户指南](docs/cn/USER_GUIDE.md)「🎨 特性标志」；数据库后端互斥（不适用 `--all-features`）说明同见该节。

---

## 📚 文档

<details>
<summary>📋 表格</summary>

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/cn/USER_GUIDE.md) | 从安装、配置详解到高级主题的完整教程 |
| [📘 API 参考](docs/cn/API_REFERENCE.md) | 核心类型、配置结构体、错误类型与 trait 的逐项说明 |
| [🏗️ 架构文档](docs/cn/ARCHITECTURE.md) | 分层设计、依赖注入架构、数据流与并发模型 |
| [📊 性能基线](docs/cn/PERFORMANCE.md) | criterion 基准环境、方法与正式基线数字 |
| [🧪 测试场景](docs/cn/TEST_SCENARIOS.md) | 测试金字塔、E2E 场景定义与组合矩阵 |
| [🔒 安全文档](docs/cn/SECURITY.md) | 安全设计、漏洞报告流程与合规性说明 |
| [📋 更新日志](docs/CHANGELOG.md) | 按 Keep a Changelog 格式维护的版本记录 |
| [🤝 贡献指南](docs/cn/CONTRIBUTING.md) | 开发环境、TDD 流程与代码风格约定 |
| [📦 在线 API 文档](https://docs.rs/inklog) | docs.rs 自动生成的最新文档 |

</details>

---

## 💻 示例

[`examples/`](examples/) 是 workspace 内的独立 crate（`inklog-examples`），按目录分为 7 类共 40 个示例；完整清单见 [📖 用户指南](docs/cn/USER_GUIDE.md)「💻 完整示例清单」。在仓库根目录运行：

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo run --package inklog-examples --bin <名称>
```

</details>

<div align="center">

**[📂 浏览全部示例 →](examples/)**

</div>

---

## 🏗️ 架构

分层架构图与分层职责、核心执行链路时序、故障降级与自愈流程的完整说明见 [🏗️ 架构文档](docs/cn/ARCHITECTURE.md)。

---

## 🧪 测试

测试金字塔基线、E2E 场景定义、组合矩阵与静态门槛见 [🧪 测试场景](docs/cn/TEST_SCENARIOS.md)；测试运行命令与质量门禁见 [🤝 贡献指南](docs/cn/CONTRIBUTING.md)。

---

## 📊 性能

criterion 基准环境、方法与正式基线数字（含复现步骤）见 [📊 性能基线](docs/cn/PERFORMANCE.md)。

---

## 🔒 安全

发现安全漏洞请**不要**公开披露，按 [🔒 安全文档](docs/cn/SECURITY.md) 流程负责任上报；安全设计、供应链安全与合规性说明见同文档。

---

## 🗺️ 开发路线图

阶段性目标与当前状态见 [📖 用户指南](docs/cn/USER_GUIDE.md)「🗺️ 开发路线图」。

---

## 🤝 参与贡献

欢迎贡献！开发环境、TDD 流程、Git 钩子与 Pull Request 流程见 [🤝 贡献指南](docs/cn/CONTRIBUTING.md)。

---

## 📋 更新日志

完整版本记录见 [docs/CHANGELOG.md](docs/CHANGELOG.md)（Keep a Changelog 格式，语义化版本）。

---

## 📄 许可证

本项目基于 [MIT](LICENSE) 许可证发布。

Copyright (c) 2026 Kirky.X🌠

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

数据库、缓存、配置与生命周期集成分别由同工作区项目 dbnexus、oxcache、confers、trait-kit 提供；限流/重试/熔断原理由 limiteron 提供。confers 0.6（`default-features = false`，`toml` + `watch` 特性）仍是**非 optional 核心依赖**——INKLOG_* 环境变量覆盖映射单源于 confers。limiteron 依赖自本版起**移除**：限流内核改由仓内令牌桶自持（容量 = 速率、每秒补充速率个令牌，语义等价 limiteron `TokenBucketLimiter::new(rate, rate)`），sink 级突发配额仍用 `BurstLimiter`（纯预算语义，无时钟无补充），sink worker 池重试数值为 inklog 侧常量；按速率/按 target 分组的限流模块默认构建即生效，inklog 不再依赖任何外部限流库。

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

**由 ❤️ Kirky.X 构建**

<sub>© 2026 Kirky.X🌠. 版权所有。</sub>

**[⬆ 返回顶部](#-目录)**

</div>
