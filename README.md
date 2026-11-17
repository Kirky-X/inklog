<div align="center">

<img src="docs/assets/inklog.png" alt="inklog" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/inklog?style=for-the-badge)](https://crates.io/crates/inklog) [![docs.rs](https://img.shields.io/docsrs/inklog?style=for-the-badge)](https://docs.rs/inklog) [![downloads](https://img.shields.io/crates/d/inklog?style=for-the-badge)](https://crates.io/crates/inklog) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/inklog/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/inklog?style=for-the-badge)](https://codecov.io/gh/Kirky-X/inklog) [![license](https://img.shields.io/crates/l/inklog?style=for-the-badge)](LICENSE)

English | [中文](README_CN.md)

**Enterprise-grade Rust Logging Infrastructure**

[✨ Features](#-features) • [🚀 Quick Start](#-quick-start) • [📚 Documentation](#-documentation) • [💻 Examples](#-examples) • [🤝 Contributing](#-contributing)

</div>

---

<div align="center" style="padding: 32px; margin: 24px 0">

### ⚡ Structured Logging, Security Built In

Every log record travels a controlled path through capture, masking, and encryption to multi-sink persistence:

<table style="width:100%; border-collapse: collapse">
<tr><td align="center" width="25%" style="padding: 12px">⚡<br><b>Async Throughput</b><br><span style="color:#64748B">Tokio runtime + bounded channels, batching with backpressure</span></td><td align="center" width="25%" style="padding: 12px">🔒<br><b>Security Built In</b><br><span style="color:#64748B">AES-256-GCM encryption, PII masking, key zeroization</span></td><td align="center" width="25%" style="padding: 12px">🎯<br><b>Multi-Target Output</b><br><span style="color:#64748B">console, file, database, TCP/UDP forwarding, OTLP export</span></td><td align="center" width="25%" style="padding: 12px">📊<br><b>Full Observability</b><br><span style="color:#64748B">health endpoints, Prometheus metrics, trace_id correlation</span></td></tr>
</table>

</div>

---

## 📋 Table of Contents

<details open>
<summary>📑 Contents</summary>

- [✨ Features](#-features)
- [🚀 Quick Start](#-quick-start)
- [🎨 Feature Flags](#-feature-flags)
- [📚 Documentation](#-documentation)
- [💻 Examples](#-examples)
- [🏗️ Architecture](#️-architecture)
- [🧪 Testing](#-testing)
- [📊 Performance](#-performance)
- [🔒 Security](#-security)
- [🗺️ Roadmap](#️-roadmap)
- [🤝 Contributing](#-contributing)
- [📋 Changelog](#-changelog)
- [📄 License](#-license)
- [🙏 Acknowledgments](#-acknowledgments)
- [📞 Contact & Support](#-contact--support)
- [⭐ Star History](#-star-history)

</details>

---

## ✨ Features

inklog is logging infrastructure built for production: application code keeps using the standard `log` / `tracing` macros while inklog takes over subscription, masking, dispatch, and persistence. Everything below maps to real code in this repository and the [docs/](docs/en/USER_GUIDE.md) documentation.

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>Async Pipeline</b><br><span style="color:#64748B">Bounded Crossbeam channels + dedicated worker pool; non-blocking senders with backpressure when full</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📁 <b>File Output</b><br><span style="color:#64748B">Size- and time-based rotation, <code>BufWriter</code> buffering, optional compression and encryption</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🗄️ <b>Database Output</b><br><span style="color:#64748B">Batched inserts, connection pooling, partitioned tables via the dbnexus adapter for four backends</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🎭 <b>Data Masking</b><br><span style="color:#64748B">Source-level redaction (<code>mark_sensitive_fields</code>) replaces registered field values before fan-out; the regex rule set masks embedded PII as a fallback, with <code>fast-masking</code> multi-pattern acceleration</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🎨 <b>Template Formatting</b><br><span style="color:#64748B"><code>{timestamp}</code> <code>{level}</code> <code>{message}</code> <code>{trace_id}</code> placeholders</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧩 <b>Dependency Injection</b><br><span style="color:#64748B"><code>Cache</code> / <code>Config</code> / <code>Database</code> trait abstractions with swappable, mockable adapters</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔁 <b>Reliability</b><br><span style="color:#64748B">Circuit breaker, DB → File → Console three-level fallback, automatic recovery by a health-check thread</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🔀 <b>Dynamic Sinks</b><br><span style="color:#64748B">Register third-party sinks with <code>LoggerBuilder::add_sink</code>, one channel per sink; middleware chain, sampler, and token-bucket rate-limit decorators</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🌡️ <b>Runtime Tuning</b><br><span style="color:#64748B"><code>set_level</code> hot-reloads global and per-target levels via <code>tracing_subscriber::reload</code></span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🔍 <b>Log Search</b><br><span style="color:#64748B"><code>inklog-cli query</code> searches local logs by time, level, and keyword (auto decrypt/unpack)</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>i18n</b><br><span style="color:#64748B">Error messages rendered per system locale via Fluent + ICU (zh-CN / en)</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📈 <b>Observability</b><br><span style="color:#64748B">Prometheus export of health, channel usage, pool metrics, and write-latency histogram</span></td>
</tr>
</table>

> 🧱 For the advanced capability list mapped to `src/` modules, see the [🏗️ Architecture](docs/en/ARCHITECTURE.md) "Advanced Capability List" section.

---

## 🚀 Quick Start

### Requirements

| Requirement | Version |
|------|------|
| Rust | 1.97.1+ (pinned by `rust-toolchain.toml`) |
| Edition | 2024 |
| Platforms | Linux / macOS / Windows |

### Installation

<details>
<summary>📄 Code · bash</summary>

```bash
cargo add inklog
```

</details>

Or declare it explicitly in `Cargo.toml` (`default = ["i18n", "cache"]`; core capabilities plus internationalization and the cache backend are enabled by default):

<details>
<summary>📄 Code · toml</summary>

```toml
[dependencies]
inklog = "0.3.0"
```

</details>

### Minimal Runnable Example

Taken from [`examples/src/bin/core/basic.rs`](examples/src/bin/core/basic.rs):

<details>
<summary>📄 Code · rust</summary>

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with defaults and install the global subscriber (console sink, level info)
    let logger = LoggerManager::new().await?;

    tracing::info!("Hello, inklog!");
    tracing::info!(user_id = 42, action = "login", "structured fields example");

    // Drain the channel and shut every sink down before exiting
    logger.shutdown()?;
    std::mem::forget(logger); // already shut down explicitly, prevent double close on Drop
    Ok(())
}
```

</details>

### Core Concepts

The five core concepts — initialization, recording, sinks, configuration, and shutdown — are described item by item in the [📖 User Guide](docs/en/USER_GUIDE.md) "Core Concepts" section.

---

## 🎨 Feature Flags

For the full feature list, defaults, and how to enable them, see the [📖 User Guide](docs/en/USER_GUIDE.md) "🎨 Feature Flags" section; the database-backend exclusivity note (`--all-features` unsupported) is in the same section.

---

## 📚 Documentation

<details>
<summary>📋 Table</summary>

| Document | Description |
|------|------|
| [📖 User Guide](docs/en/USER_GUIDE.md) | Complete tutorial from installation and configuration to advanced topics |
| [📘 API Reference](docs/en/API_REFERENCE.md) | Item-by-item coverage of core types, config structs, errors, and traits |
| [🏗️ Architecture](docs/en/ARCHITECTURE.md) | Layered design, DI architecture, data flow, and concurrency model |
| [📊 Performance Baseline](docs/en/PERFORMANCE.md) | Criterion benchmark environment, methodology, and official baseline numbers |
| [🧪 Test Scenarios](docs/en/TEST_SCENARIOS.md) | Test pyramid, E2E scenario definitions, and combination matrix |
| [🔒 Security](docs/en/SECURITY.md) | Security design, vulnerability reporting process, and compliance notes |
| [📋 Changelog](docs/CHANGELOG.md) | Version history maintained in Keep a Changelog format |
| [🤝 Contributing](docs/en/CONTRIBUTING.md) | Development environment, TDD workflow, and code style conventions |
| [📦 Online API Docs](https://docs.rs/inklog) | Latest documentation auto-generated on docs.rs |

</details>

---

## 💻 Examples

[`examples/`](examples/) is a standalone workspace crate (`inklog-examples`) with 40 examples in 7 categories; the full list is in the [📖 User Guide](docs/en/USER_GUIDE.md) "💻 Complete Example List" section. Run from the repository root:

<details>
<summary>📄 Code · bash</summary>

```bash
cargo run --package inklog-examples --bin <name>
```

</details>

<div align="center">

**[📂 Browse all examples →](examples/)**

</div>

---

## 🏗️ Architecture

For the layered-architecture diagram and responsibility table, the core-pipeline sequence, and the failure-fallback and self-healing flow, see the [🏗️ Architecture](docs/en/ARCHITECTURE.md).

---

## 🧪 Testing

For the test pyramid baseline, E2E scenario definitions, combination matrix, and static gates, see [🧪 Test Scenarios](docs/en/TEST_SCENARIOS.md); for test commands and quality gates, see [🤝 Contributing](docs/en/CONTRIBUTING.md).

---

## 📊 Performance

For the criterion benchmark environment, methodology, and official baseline numbers (with reproduction steps), see the [📊 Performance Baseline](docs/en/PERFORMANCE.md).

---

## 🔒 Security

Please do **not** disclose security vulnerabilities publicly; report responsibly following [🔒 Security Documentation](docs/en/SECURITY.md). Security design, supply-chain security, and compliance notes live in the same document.

---

## 🗺️ Roadmap

Phased goals and current status are in the [📖 User Guide](docs/en/USER_GUIDE.md) "🗺️ Roadmap" section.

---

## 🤝 Contributing

Contributions are welcome! For the development environment, TDD workflow, Git hooks, and the Pull Request process, see [🤝 Contributing](docs/en/CONTRIBUTING.md).

---

## 📋 Changelog

The full version history lives in [docs/CHANGELOG.md](docs/CHANGELOG.md) (Keep a Changelog format, semantic versioning).

---

## 📄 License

This project is licensed under the [MIT](LICENSE) License.

Copyright (c) 2026 Kirky.X🌠

---

## 🙏 Acknowledgments

inklog builds on these outstanding open-source projects:

- [tokio](https://tokio.rs/): the async runtime
- [tracing](https://github.com/tokio-rs/tracing): structured logging and the Subscriber ecosystem
- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam): high-performance bounded channels
- [axum](https://github.com/tokio-rs/axum): HTTP health and metrics endpoints
- [serde](https://serde.rs/): serialization framework
- [RustCrypto](https://github.com/RustCrypto) (aes-gcm / sha2 / pbkdf2 / zeroize): cryptographic primitives
- [Project Fluent](https://projectfluent.org/) and [ICU](https://icu4x.unicode.org/): internationalized message formatting
- [criterion](https://github.com/bheisler/criterion.rs): statistically rigorous benchmarking

Database, cache, configuration, and lifecycle integrations are provided by the sibling workspace projects dbnexus, oxcache, confers, and trait-kit; rate limiting, retry, and circuit-breaking primitives come from limiteron. confers 0.6 (`default-features = false`, `toml` + `watch` features) stays a **non-optional core dependency** — the `INKLOG_*` environment override mapping is single-sourced to confers. limiteron is no longer a dependency from this release: the limiting kernel is the in-crate token bucket (capacity = rate, refilled at the rate per second, semantically equal to limiteron `TokenBucketLimiter::new(rate, rate)`), sink-level burst quota still uses `BurstLimiter` (pure-budget semantics, no clock, no refill), the sink worker pool retry counts are inklog-side constants, and the rate/target-grouped limit modules work in the default build — inklog depends on no external limiting library.

---

## 📞 Contact & Support

<div align="center" style="margin: 24px 0;">

<table style="width:100%; max-width: 600px;">
<tr>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/inklog/issues">
<div style="padding: 16px; border-radius:8px;">
<b style="color:#991B1B;">📋 Issues</b>
</div>
</a>
<br><span style="color:#64748B;">Report bugs and issues</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/inklog/discussions">
<div style="padding: 16px; border-radius:8px;">
<b style="color:#1E40AF;">💬 Discussions</b>
</div>
</a>
<br><span style="color:#64748B;">Ask questions and share ideas</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/inklog">
<div style="padding: 16px; border-radius:8px;">
<b style="color:#1E293B;">🐙 GitHub</b>
</div>
</a>
<br><span style="color:#64748B;">View source code</span>
</td>
</tr>
</table>

</div>

---

## ⭐ Star History

<div align="center">

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/inklog&type=Date)](https://star-history.com/#Kirky-X/inklog&Date)

### 💝 Support This Project

If you find this project useful, please consider giving it a ⭐️!

**Built with ❤️ by Kirky.X**

<sub>© 2026 Kirky.X🌠. All rights reserved.</sub>

**[⬆ Back to Top](#-table-of-contents)**

</div>
