<div align="center">

<img src="docs/assets/inklog.png" alt="inklog" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/inklog?style=for-the-badge)](https://crates.io/crates/inklog) [![docs.rs](https://img.shields.io/docsrs/inklog?style=for-the-badge)](https://docs.rs/inklog) [![downloads](https://img.shields.io/crates/d/inklog?style=for-the-badge)](https://crates.io/crates/inklog) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/inklog/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/inklog?style=for-the-badge)](https://codecov.io/gh/Kirky-X/inklog) [![license](https://img.shields.io/crates/l/inklog?style=for-the-badge)](LICENSE)

[中文](README.md) | **English**

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

inklog is logging infrastructure built for production: application code keeps using the standard `log` / `tracing` macros while inklog takes over subscription, masking, dispatch, and persistence. Everything below maps to real code in this repository and the [docs/](docs/USER_GUIDE.md) documentation.

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>Async Pipeline</b><br><span style="color:#64748B">Bounded Crossbeam channels + dedicated worker pool; non-blocking senders with backpressure when full</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📁 <b>File Output</b><br><span style="color:#64748B">Size- and time-based rotation, <code>BufWriter</code> buffering, optional compression and encryption</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🗄️ <b>Database Output</b><br><span style="color:#64748B">Batched inserts, connection pooling, partitioned tables via the dbnexus adapter for four backends</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🎭 <b>Data Masking</b><br><span style="color:#64748B">Sensitive field-name detection + regex rule set, with <code>fast-masking</code> multi-pattern acceleration</span></td>
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

<details>
<summary>📦 Advanced Capability List (maps to <code>src/</code> modules)</summary>

- **Sink family** (`src/support/io/sink/`): `console`, `file` (rotation/compression/encryption), `database` (batch/partition/circuit breaker), `ring_buffered_file` (channel-buffered high throughput), `net` (TCP with optional TLS + UDP), `otlp`, `middleware`, `sampling`, `rate_limit`
- **Processing** (`src/support/processing/`): `template` engine, `masking` engine with rule registry, `object_pool` for LogRecord/string buffers
- **Observability** (`src/support/observability/`): `Metrics`, `HealthStatus`, `SinkHealthMonitor`, fallback state
- **Validation** (`src/validation/`): `PathValidator` path traversal protection, `LogSanitizer` log content sanitization
- **Tamper-evident archival** (`src/support/audit_chain.rs`): HMAC-SHA256 archive chain against deletion, reordering, and forgery
- **Integrations** (`src/integrations/`): `OxCacheAdapter`, `InklogConfigAdapter`, `DbNexusAdapter`, trait-kit `InklogModule`, dbnexus audit bridge, confers config loading with watch, confers audit bridge
- **CLI** (`src/cli/`): five subcommands: `decrypt`, `generate`, `validate`, `query`, `verify-chain` (`verify-chain` validates the archive audit chain via `--manifest` + `--key-env`)

</details>

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
inklog = "0.3.0-rc.11"
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

1. **Initialization**: `LoggerManager` installs the global tracing subscriber and the `log` crate frontend with process-wide singleton semantics (`init_inklog_logger()` is the convenience entry point).
2. **Recording**: business code uses standard macros such as `tracing::info!` with zero intrusion.
3. **Sinks**: the output abstraction (`LogSink` / `AsyncSink` traits) with built-in console / file / database / net / otlp implementations, extensible with your own.
4. **Configuration**: `InklogConfig` supports TOML files; `INKLOG_*` environment variable overrides apply only when loading via `InklogConfig::load_with_env_overrides()` (precedence: environment > file > defaults). `LoggerManager::new()` / `LoggerManager::load()` go through `load_sync()` and do not apply env overrides.
5. **Shutdown**: call `shutdown()` before exit so remaining records in the channel are flushed.

---

## 🎨 Feature Flags

`default = ["i18n", "cache"]`: the default build contains core capabilities plus internationalization and the cache backend; every other feature below must be enabled explicitly (per the `Cargo.toml` `[features]` definition). Opting out via `default-features = false` disables both i18n and cache, which prunes the whole icu/fluent dependency tree (oxcache's icu/fluent deps are not optional), with `tr`/`tr_args` falling back to the embedded English message table.

<details>
<summary>📋 Table</summary>

| Flag | Default | Description |
|------|:----:|------|
| `i18n` | ✅ | Internationalization runtime: ICU4X locale formatting + fluent-bundle translation + `INKLOG_LOCALE` override |
| `cache` | ✅ | Cache backend: oxcache adapter (`OxCacheAdapter`) and `ObjectPool`; oxcache's icu/fluent deps are not optional, so pruning icu/fluent requires disabling both `i18n` and `cache` (`default-features = false`) |
| `sqlite` | ❌ | SQLite database backend (via dbnexus, rustls runtime) |
| `postgres` | ❌ | PostgreSQL database backend (via dbnexus) |
| `mysql` | ❌ | MySQL database backend (via dbnexus) |
| `duckdb` | ❌ | DuckDB database backend (via dbnexus). 🚫 Currently unusable: upstream zip full-range yank breaks libduckdb-sys (zip ^6) resolution — do not enable |
| `http` | ❌ | Axum health and metrics endpoints (axum + axum-server, TLS via rustls) |
| `cli` | ❌ | `inklog-cli` command-line tool (clap + glob, implies `schema`) |
| `schema` | ❌ | Config JSON Schema export (schemars derive + `InklogConfig::json_schema()`); implicitly enabled by `cli` |
| `kit` | ❌ | trait-kit lifecycle and observability integration (`InklogModule`); requires at least one database backend feature |
| `zstd` | ❌ | Zstd compression for rotated log files (zstd); `compression` is a deprecated compatibility alias kept for existing `--features compression` usage |
| `gzip` | ❌ | Gzip compression backend (pure-Rust flate2; FileSink falls back to gzip for rotation when `compression` is off) |
| `parquet` | ❌ | Parquet/Arrow export for database sink archival |
| `fast-masking` | ❌ | Aho-Corasick accelerated multi-pattern masking |
| `secret-scan` | ❌ | Secret outbound masking gate: value-shape pattern registry (bare `sk-`/PEM/AKIA secrets) + value entropy scanning + per-pattern counting with fail-closed limit (`mask_checked`) + Chinese sensitive key names |
| `dbnexus-audit` | ❌ | dbnexus AuditStorage port adapter; audit events persisted through the inklog DB sink, combinable with any backend |
| `config-confers` | ❌ | Configuration loaded via confers + watch hot-reload of level and rotation parameters; confers 0.6.0-rc.6 is a non-optional core dependency (the `INKLOG_*` env override mapping goes through the confers source chain), this feature additionally enables confers' `toml`/`watch` features (kept for compatibility) |
| `confers-audit` | ❌ | inklog implementation of the confers `AuditSink` port; configuration audit events are written into the inklog structured sink; confers is a non-optional core dependency, this feature additionally enables confers' `audit` feature |
| `kms` | ❌ | KMS key providers (`EnvKeyProvider` / `ConfersKeyProvider` / Vault transit MVP); confers is a non-optional core dependency, this feature additionally enables confers' `encryption`/`cloud-kms` features |
| `net-sink` | ❌ | Network forwarding sinks (TCP with optional TLS + UDP, buffered reconnection) |
| `otlp` | ❌ | OTLP/HTTP JSON log export MVP (hand-written transport, zero new dependencies) |
| `otel` | ❌ | Real OpenTelemetry trace context: reads the SpanContext from span extensions (written by the host's tracing-opentelemetry layer); the host otel version must match inklog's (currently opentelemetry 0.33, paired with tracing-opentelemetry 0.34), otherwise it silently falls back to root-span derivation |
| `metrics-registry` | ❌ | General-purpose business metrics registry (CounterVec / GaugeVec / HistogramVec + Prometheus text export) |
| `test-utils` | ❌ | Test-facing mock exports (`MockCache` / `MockConfig` / `MockDatabaseAdapter`); excluded from default and all production combinations |

</details>

> ⚠️ **Database backend exclusivity**: `sqlite` / `postgres` / `mysql` / `duckdb` are mutually exclusive (enforced via dbnexus; embedded and server-side drivers must not be mixed). `--all-features` is not supported; enable features grouped by backend instead.
>
> 🚫 **Do not enable the `duckdb` feature** (since 2026-10-16): the upstream `zip` crate has been fully yanked on crates.io (only 9.0.3 remains). The `duckdb` chain resolves through `libduckdb-sys` (pinned to `zip ^6`), which can no longer be resolved — enabling this feature from scratch fails at dependency resolution. Existing projects with a committed `Cargo.lock` are unaffected. This will be lifted once duckdb-rs upstream publishes a release adapted to zip 9.x; until then use the `sqlite` / `postgres` / `mysql` backends instead.

---

## 📚 Documentation

<details>
<summary>📋 Table</summary>

| Document | Description |
|------|------|
| [📖 User Guide](docs/USER_GUIDE.md) | Complete tutorial from installation and configuration to advanced topics |
| [📘 API Reference](docs/API_REFERENCE.md) | Item-by-item coverage of core types, config structs, errors, and traits |
| [🏗️ Architecture](docs/ARCHITECTURE.md) | Layered design, DI architecture, data flow, and concurrency model |
| [📊 Performance Baseline](docs/PERFORMANCE.md) | Criterion benchmark environment, methodology, and official baseline numbers |
| [🧪 Test Scenarios](docs/TEST_SCENARIOS.md) | Test pyramid, E2E scenario definitions, and combination matrix |
| [🔒 Security](docs/SECURITY.md) | Security design, vulnerability reporting process, and compliance notes |
| [📋 Changelog](docs/CHANGELOG.md) | Version history maintained in Keep a Changelog format |
| [🤝 Contributing](docs/CONTRIBUTING.md) | Development environment, TDD workflow, and code style conventions |
| [📦 Online API Docs](https://docs.rs/inklog) | Latest documentation auto-generated on docs.rs |

</details>

---

## 💻 Examples

[`examples/`](examples/) is a standalone workspace crate (`inklog-examples`) with 40 examples in 7 categories. Run from the repository root:

<details>
<summary>📄 Code · bash</summary>

```bash
cargo run --package inklog-examples --bin <name>
```

</details>

#### Configuration (config)

| Example | Description | Feature |
|------|------|----------|
| `config_file` | Configuration file loading (Layer 1, local resources) | none |
| `config_inspect` | Config inspection: `sinks_enabled()` and `LoggerManager::load()` | none |
| `env_overrides` | Environment variable override loading | none |
| `confers_audit` | confers audit bridge: `AuditSink` port events fed into the inklog sink | `confers-audit` |

#### Core (core)

<details>
<summary>📋 Table</summary>

| Example | Description | Feature |
|------|------|----------|
| `basic` | Basic usage: init, levels, structured fields, health check, graceful shutdown | none |
| `builder` | Builder pattern configuration | none |
| `all_features` | Full feature demonstration | none |
| `production` | Production environment configuration | none |
| `template` | Log templates | none |
| `error_handling` | Error handling (Layer 0, zero dependencies) | none |
| `i18n` | Internationalized formatting | none |

</details>

#### Sinks & Output (sinks)

<details>
<summary>📋 Table</summary>

| Example | Description | Feature |
|------|------|----------|
| `console` | Console sink | none |
| `file` | File sink | none |
| `rotation` | Log rotation (Layer 1, local resources) | none |
| `ring_buffered_file` | ChannelBufferedFileSink (Layer 1, local resources) | none |
| `archive_format` | Archive format (Layer 0, zero dependencies) | none |
| `compression` | Zstd compression and decompression | `compression` |
| `parquet_archive` | Parquet archival | `parquet` + any database backend |
| `partition_strategy` | Database partitioning strategies | none |

</details>

#### Database (database)

| Example | Description | Feature |
|------|------|----------|
| `database` | Database sink with in-memory SQLite | any database backend (the example doc uses `sqlite`) |
| `database_pg_mysql` | PostgreSQL/MySQL database driver demo | none |
| `di_example` | Dependency injection pattern | `sqlite` (enforced via required-features) |

#### Infrastructure (infra)

<details>
<summary>📋 Table</summary>

| Example | Description | Feature |
|------|------|----------|
| `channel_strategy` | Adaptive channel strategy | none |
| `circuit_breaker` | Circuit breaker (Layer 2, external services) | none |
| `fallback` | Sink fallback/degradation | none |
| `log_adapter` | `log` crate adapter bridge (Layer 0, zero dependencies) | none |
| `log_level` | LogLevel parsing, comparison, and Display | none |
| `metrics` | Health monitoring and metrics collection (Layer 2, external services) | none |
| `object_pool` | Object pool (Layer 0, zero dependencies) | none |
| `output_format` | Output formats (Layer 0, zero dependencies) | none |
| `performance` | Performance demo | none |
| `rate_limiter` | Rate limiter (Layer 0, zero dependencies) | none |
| `runtime_ops` | LoggerManager runtime operations API | none |

</details>

#### Network (network)

| Example | Description | Feature |
|------|------|----------|
| `http` | HTTP health and metrics endpoint demo | `http` (enforced by required-features) |
| `http_auth` | HTTP authentication and IP allowlist configuration | none |
| `tls_config` | TLS configuration | none |

#### Security (security)

| Example | Description | Feature |
|------|------|----------|
| `encryption` | Log encryption | none |
| `log_sanitizer` | Log content sanitization (Layer 0, zero dependencies) | none |
| `masking` | Data masking | none |
| `path_validator` | Path validator (Layer 0, zero dependencies) | none |

<div align="center">

**[📂 Browse all examples →](examples/)**

</div>

---

## 🏗️ Architecture

inklog uses a layered async architecture: `domain` (manager, subscriber, and worker threads) passes records through `support::processing` for template rendering and masking into a bounded Crossbeam channel, from which dedicated threads dispatch to the sinks in `support::io::sink`; `integrations` adapts oxcache / confers / dbnexus / trait-kit onto traits, and `support::observability` exposes health and metric endpoints via the `http` feature. The full architecture diagram, layer responsibility table, and `src/` module tree comparison live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) ("Layered Architecture").

### 🔀 Core Pipeline

The full sequence of a log record — emitted via `tracing` macros, masked, sent non-blocking into the bounded channel (backpressure when full), persisted by the file / database sinks, and written back to metrics — is diagrammed in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) ("Core Pipeline"), together with the key parameters: default channel capacity 10,000 and 3 worker threads, tunable via `PerformanceConfig`; encryption, compression, and rotation fire per rotated file and never sit on the per-record hot path.

### 🧯 Failure Handling

Sink write failures go through the circuit breaker (default failure threshold 5, 30-second cooldown) into retry or the DB → File → Console three-level fallback, while the health-check thread patrols every 1 second (unhealthy-sink rebuilds observe a 30-second cooldown and require more than 3 consecutive failures), re-initializing unhealthy sinks and resetting the breaker. The full flow diagram is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) ("Failure degradation and self-healing").

---

## 🧪 Testing

### Test Strategy

<details>
<summary>📋 Table</summary>

| Type | Location | Description |
|------|------|------|
| Unit tests | inline `#[cfg(test)]` in `src/` | Boundary and error scenarios per module; mocks visible via cfg(test) |
| Integration tests | `tests/integration/`, `tests/cli_integration.rs` | Batch writes, HTTP, CLI, compression ratio, Parquet, auto recovery, and more; requires the `sqlite,http,cli,compression,parquet,test-utils` combination |
| Combination tests | `tests/combinations/` | Feature combination matrix and multi-sink fallback; requires `sqlite` |
| End-to-end tests | `tests/e2e/e2e_advanced.rs` | 228 tests across 21 scenario domains (see [docs/TEST_SCENARIOS.md](docs/TEST_SCENARIOS.md)) |
| Docker database integration | `tests/docker/` + `docker/docker-compose.test.yml` | PostgreSQL / MySQL / SQLite lifecycle verification |
| Performance tests | `tests/performance/` + `benches/` | Large-volume and long-running tests plus criterion benchmarks |

</details>

**Test suite size** (measured at HEAD, counted via `grep -rEc '#\[(tokio::)?test\b' --include='*.rs'`): 1,712 inline in `src/` + 564 in `tests/` + 41 in `examples/` for a total of **2,317 test functions**, plus 48 criterion benchmark functions (33 in `benches/inklog_bench.rs`, 7 in `benches/rc4_pipeline_bench.rs`, 8 in `benches/secret_scan_bench.rs`, counted by `.bench_function(` registrations; 1 of them gated behind the `otlp` feature, and `secret_scan_bench` requires the `secret-scan` feature).

### Commands (matching CI)

<details>
<summary>📄 Code · bash</summary>

```bash
# CI test gate (database backends are mutually exclusive; --all-features is unsupported)
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# Docker database integration tests
docker compose -f docker/docker-compose.test.yml up -d

# Coverage gate (CI requires ≥95% line coverage)
cargo llvm-cov --features "sqlite http cli kit zstd gzip parquet fast-masking" --lib --fail-under-lines 95

# Benchmarks
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
cargo bench --bench secret_scan_bench --features secret-scan
```

</details>

> **Locale note**: error messages are localized via ICU/Fluent based on the system locale. If tests assert English message text, set `INKLOG_LOCALE=en` (e.g. in CI or non-English environments) to pin the output language.

### Quality Gates

<details>
<summary>📄 Code · bash</summary>

```bash
cargo fmt --all -- --check                    # format check
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings  # zero warnings
cargo deny check                              # dependency advisories / licenses / duplicate crates
cargo audit                                   # security advisories (lefthook pre-push)
```

</details>

---

## 📊 Performance

### Baseline Numbers

The first official baseline (2026-09-11, criterion medians, covering the write / serialization / encryption paths) — full numbers, environment notes, and reproduction steps — is recorded in [docs/PERFORMANCE.md](docs/PERFORMANCE.md).

### Design Highlights

| Mechanism | Parameter | Effect |
|------|------|------|
| Bounded-channel backpressure | `channel_capacity` default 10,000 | Prevents memory exhaustion; channel usage exposed via metrics |
| Database batch writes | `batch_size` default 100, flush interval default 500 ms | docs/ARCHITECTURE.md reference: ~10,000 rows/s for batches of 100 vs ~100 rows/s for per-row inserts |
| Masking on demand | `masking_enabled` | Regex masking is the most expensive security step on the main path; enable only on sinks that need it |
| Zstd compression | levels 1-22, default 3 | ~3.5x ratio at the default level |

Reproduction steps are documented in the "Reproduction" section of [docs/PERFORMANCE.md](docs/PERFORMANCE.md).

---

## 🔒 Security

### Reporting a Vulnerability

Please do **not** disclose security vulnerabilities publicly; report responsibly following [docs/SECURITY.md](docs/SECURITY.md):

- **Preferred**: [GitHub Security Advisories](https://github.com/Kirky-X/inklog/security/policy) ("Report a vulnerability" private channel)
- **Alternative**: email [Kirky-X@outlook.com](mailto:Kirky-X@outlook.com) (handler: Kirky.X)
- **Response targets and coordinated disclosure**: see [docs/SECURITY.md](docs/SECURITY.md) ("Vulnerability Reporting Process").

### Security Design

Encryption at rest (AES-256-GCM), key derivation and memory zeroization, path and content protection, SQL injection protection, HTTP access control, tamper-evident archival, and the GDPR / HIPAA / PCI-DSS compliance mapping are documented item by item in [docs/SECURITY.md](docs/SECURITY.md) ("Security Design Overview").

### Supply Chain Security

The repository maintains [`deny.toml`](deny.toml); where `cargo deny check` (advisories / licenses / duplicate crates), `cargo audit` (RustSec advisories), and the pre-commit private-key scan run, and their scope, are documented in [docs/SECURITY.md](docs/SECURITY.md) ("Security Design Overview").

---

## 🗺️ Roadmap

Phased goals compiled from the existing release plan (timing may adjust with the overall workspace release schedule):

| Status | Goal | Notes |
|:----:|------|------|
| 📋 | v0.3.0 stable release | Complete the 0.3.0-rc.9 → 0.3.0 stable release |
| 📋 | Workspace dependency lockstep | trait-kit 0.5.0, oxcache 0.5.0, dbnexus 0.6.0 |
| ✅ | CI test matrix grouped by database backend | The Docker pipeline validates the four mutually exclusive backends (sqlite/postgres/mysql/duckdb); duckdb is embedded and needs no container service |
| ✅ | Provision the MySQL integration environment | A MySQL 8 service is provided in docker/docker-compose.test.yml; the CI mysql branch runs the integration tests |
| ✅ | Raise test coverage | llvm-cov CI-scope line coverage 95.4%+, gate raised in stages 80→85→90→95 to 95% |

---

## 🤝 Contributing

Contributions are welcome! See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for the complete guide.

### Development Setup

| Requirement | Notes |
|------|------|
| Rust 1.97.1 | pinned by `rust-toolchain.toml` |
| Protobuf compiler | `protoc` is required to build combinations with database features |
| Git hooks | run `lefthook install` (config in `lefthook.yml`); or `pip install pre-commit && pre-commit install` (reads `.pre-commit-config.yaml`) |

<details>
<summary>📄 Code · bash</summary>

```bash
git clone https://github.com/Kirky-X/inklog.git
cd inklog
lefthook install   # or pip install pre-commit && pre-commit install

# Run tests (same feature combination as CI)
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"
```

</details>

### Commit Conventions

- **Conventional Commits**: `feat: ...` / `fix: ...` / `docs: ...`, enforced by the commit-msg hook;
- **pre-commit hooks** (`.pre-commit-config.yaml`): rustfmt, clippy (`-D warnings`, zero warnings), `cargo check`, private-key/secret scans (detect-private-key, detect-secrets) and typos, etc.; `cargo deny check` lives in the lefthook config (`lefthook.yml`);
- **pre-push hooks**: `cargo audit` and the coverage ≥95% gate (same scope as the CI coverage job: explicit features + `--lib`, see lefthook.yml).

### Pull Request Process

1. Fork the repository and create a feature branch (`git checkout -b feature/your-feature`);
2. Make your changes and add doc comments for public APIs;
3. Run tests, clippy, and `cargo fmt --all`, ensuring everything passes;
4. Commit and push with Conventional Commits messages;
5. Open a Pull Request and pass all CI quality gates.

---

## 📋 Changelog

The full version history lives in [docs/CHANGELOG.md](docs/CHANGELOG.md) (Keep a Changelog format, semantic versioning).

### Recent Releases
- **0.3.0-rc.9** (2026-10-08): production API surface reduced — removed test-only public functions (`decrypt_file`, `reset_ops_hub_for_tests`); complexity hotspots converged via helper extraction, dead delegates removed with feature-gating replacing `allow` markers; governance records and the specmark directory moved out of git tracking; vulnerability reporting channels unified

- **0.3.0-rc.9** (2026-10-07): fix `E0433` in `create_compressed_output` on non-unix targets when neither zstd nor gzip is enabled (hit by limiteron with `default-features = false`); the CI build matrix now also checks the zero-feature build to prevent regressions
- **0.3.0-rc.6** (2026-10-06): full egress PII masking coverage with hardened regex rules (fancy-regex lookaround); ERROR/FATAL fallback re-emission, fsync crash consistency and 0600 file permissions; audit chain wired into rotation with `inklog-cli verify-chain`; service identity static fields; deferred-capabilities four items (Chinese name masking / FallbackJournal / CBFS promotion / real OTel trace context); masking detection-face and KV-redaction APIs; i18n feature-gating; CI database matrix extended with duckdb;
- **0.3.0-rc.4** (2026-09-14, including the original rc.3 batch): added `init_inklog_logger` singleton initialization, runtime level hot-reload (`set_level`), dynamic sink registration (`LoggerBuilder::add_sink`), `trace_id`/`span_id` correlation, `inklog-cli query` log search, network forwarding sinks (TCP/UDP), an OTLP export MVP, the tamper-evident archive chain, and the first performance baseline in `docs/PERFORMANCE.md` (version 0.3.0-rc.3 was skipped and never published; its content shipped with this release);
- **0.3.0-rc.2** (2026-09-03): integrated trait-kit 0.5.0-rc.2 and the i18n refactor with a version bump; removed the three mocks from the default public API (BREAKING; external test consumers must enable `test-utils`);
- **0.2.0** (2026-08-05): added the `compression` / `parquet` / `fast-masking` features and the i18n core module; added ChannelBufferedFileSink, circuit-breaker protection, and the ring-buffered file sink; edition 2024, MSRV 1.94.

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

Database, cache, configuration, and lifecycle integrations are provided by the sibling workspace projects dbnexus, oxcache, confers, and trait-kit; rate limiting, retry, and circuit-breaking primitives come from limiteron. Following the "self-developed libraries first" strategy, so that algorithm fixes for worker retries, token buckets, and circuit breakers converge on a single place in the libraries, limiteron 0.3.0-rc.6 (`default-features = false`, `retry` + `sync` features) and confers 0.6.0-rc.6 (`default-features = false`, `toml` + `watch` features) are both **non-optional core dependencies** — sink worker pool retry values, the three token bucket kernels, and the `INKLOG_*` environment override mapping are single-sourced to limiteron and confers respectively.

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
