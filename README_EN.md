<div align="center">

<img src="docs/assets/inklog.png" alt="inklog" width="180">

[![CI Status](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/inklog/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/inklog.svg)](https://crates.io/crates/inklog) [![Docs.rs](https://docs.rs/inklog/badge.svg)](https://docs.rs/inklog) [![Downloads](https://img.shields.io/crates/d/inklog.svg)](https://crates.io/crates/inklog) [![License](https://img.shields.io/crates/l/inklog.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/inklog/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/inklog)

[中文](README.md) | **English**

**Enterprise-grade Rust Logging Infrastructure**

[✨ Features](#-features) • [🚀 Quick Start](#-quick-start) • [📚 Documentation](#-documentation) • [💻 Examples](#-examples) • [🤝 Contributing](#-contributing)

<table style="width:100%; border-collapse: collapse; margin: 8px 0;">
<tr>
<td width="25%" align="center" style="padding: 12px; border-radius:8px; border:1px solid #E2E8F0; vertical-align:top;">
<b>⚡ Async Throughput</b><br>
<sub>Tokio runtime + bounded Crossbeam channels with batching and backpressure</sub>
</td>
<td width="25%" align="center" style="padding: 12px; border-radius:8px; border:1px solid #E2E8F0; vertical-align:top;">
<b>🔒 Security Built In</b><br>
<sub>AES-256-GCM encryption, PII masking, key zeroization, path traversal protection</sub>
</td>
<td width="25%" align="center" style="padding: 12px; border-radius:8px; border:1px solid #E2E8F0; vertical-align:top;">
<b>🎯 Multi-Target Output</b><br>
<sub>Console, file, database, TCP/UDP forwarding, OTLP export</sub>
</td>
<td width="25%" align="center" style="padding: 12px; border-radius:8px; border:1px solid #E2E8F0; vertical-align:top;">
<b>📊 Full Observability</b><br>
<sub>Health endpoints, Prometheus metrics, trace_id correlation</sub>
</td>
</tr>
</table>

</div>

---

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents (Click to expand)</summary>

- [✨ Features](#-features)
- [🚀 Quick Start](#-quick-start)
- [🎨 Feature Flags](#-feature-flags)
- [📚 Documentation](#-documentation)
- [💻 Examples](#-examples)
- [🏗️ Architecture](#️-architecture)
- [🔀 Core Pipeline](#-core-pipeline)
- [🧯 Failure Handling](#-failure-handling)
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

<table style="width:100%; border-collapse: collapse;">
<tr>
<td width="50%" style="vertical-align:top; padding: 8px;">

| Capability | Description |
|------|------|
| ⚡ **Async Pipeline** | Bounded Crossbeam channels + dedicated worker pool; non-blocking senders with backpressure when full |
| 📁 **File Output** | Size- and time-based rotation, `BufWriter` buffering, optional compression and encryption |
| 🗄️ **Database Output** | Batched inserts, connection pooling, partitioned tables via the dbnexus adapter for four backends |
| 🎭 **Data Masking** | Sensitive field-name detection + regex rule set, with `fast-masking` multi-pattern acceleration |
| 🎨 **Template Formatting** | Placeholders such as `{timestamp}` `{level}` `{message}` `{trace_id}` |
| 🧩 **Dependency Injection** | `Cache` / `Config` / `Database` trait abstractions with swappable, mockable adapters |

</td>
<td width="50%" style="vertical-align:top; padding: 8px;">

| Capability | Description |
|------|------|
| 🔁 **Reliability** | Circuit breaker, DB → File → Console three-level fallback, automatic recovery by a health-check thread |
| 🔀 **Dynamic Sinks** | Register third-party sinks with `LoggerBuilder::add_sink`, one channel per sink; middleware chain, sampler, and token-bucket rate-limit decorators |
| 🌡️ **Runtime Tuning** | `set_level` hot-reloads global and per-target levels via `tracing_subscriber::reload` |
| 🔍 **Log Search** | `inklog-cli query` searches local logs by time, level, and keyword (auto decrypt/unpack) |
| 🌐 **i18n** | Error messages rendered per system locale via Fluent + ICU (zh-CN / en) |
| 📈 **Observability** | Prometheus export of health, channel usage, pool metrics, and write-latency histogram |

</td>
</tr>
</table>

<details>
<summary>📦 Advanced Capability List (maps to <code>src/</code> modules)</summary>

- **Sink family** (`src/support/io/sink/`): `console`, `file` (rotation/compression/encryption), `database` (batch/partition/circuit breaker), `ring_buffered_file` (channel-buffered high throughput), `net` (TCP with optional TLS + UDP), `otlp`, `middleware`, `sampling`, `rate_limit`
- **Processing** (`src/support/processing/`): `template` engine, `masking` engine with rule registry, `object_pool` for LogRecord/string buffers
- **Observability** (`src/support/observability/`): `Metrics`, `HealthStatus`, `SinkHealthMonitor`, fallback state
- **Validation** (`src/validation/`): `PathValidator` path traversal protection, `LogSanitizer` log content sanitization
- **Tamper-evident archival** (`src/support/audit_chain.rs`): HMAC-SHA256 archive chain against deletion, reordering, and forgery
- **Integrations** (`src/integrations/`): `OxCacheAdapter`, `InklogConfigAdapter`, `DbNexusAdapter`, trait-kit `InklogModule`, dbnexus audit bridge, confers config loading with watch
- **CLI** (`src/cli/`): four subcommands: `decrypt`, `generate`, `validate`, `query`

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

```bash
cargo add inklog
```

Or declare it explicitly in `Cargo.toml` (`default = []`; only core capabilities are enabled by default):

```toml
[dependencies]
inklog = "0.3.0-rc.3"
```

### Minimal Runnable Example

Taken from [`examples/src/bin/core/basic.rs`](examples/src/bin/core/basic.rs):

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

### Core Concepts

1. **Initialization**: `LoggerManager` installs the global tracing subscriber with process-wide singleton semantics (`init_inklog_logger()` is the convenience entry point).
2. **Recording**: business code uses standard macros such as `tracing::info!` with zero intrusion.
3. **Sinks**: the output abstraction (`LogSink` / `AsyncSink` traits) with built-in console / file / database / net / otlp implementations, extensible with your own.
4. **Configuration**: `InklogConfig` supports TOML files and `INKLOG_*` environment variable overrides; precedence is environment > file > defaults.
5. **Shutdown**: call `shutdown()` before exit so remaining records in the channel are flushed.

---

## 🎨 Feature Flags

`default = []`: the default build contains only core capabilities; every feature below must be enabled explicitly (per the `Cargo.toml` `[features]` definition).

| Flag | Default | Description |
|------|:----:|------|
| `sqlite` | ❌ | SQLite database backend (via dbnexus, rustls runtime) |
| `postgres` | ❌ | PostgreSQL database backend (via dbnexus) |
| `mysql` | ❌ | MySQL database backend (via dbnexus) |
| `duckdb` | ❌ | DuckDB database backend (via dbnexus) |
| `http` | ❌ | Axum health and metrics endpoints (axum + axum-server, TLS via rustls) |
| `cli` | ❌ | `inklog-cli` command-line tool (clap + glob) |
| `kit` | ❌ | trait-kit lifecycle and observability integration (`InklogModule`); requires at least one database backend feature |
| `compression` | ❌ | Zstd compression for rotated log files (zstd) |
| `gzip` | ❌ | Gzip compression backend (pure-Rust flate2; FileSink falls back to gzip for rotation when `compression` is off) |
| `parquet` | ❌ | Parquet/Arrow export for database sink archival |
| `fast-masking` | ❌ | Aho-Corasick accelerated multi-pattern masking |
| `dbnexus-audit` | ❌ | dbnexus AuditStorage port adapter; audit events persisted through the inklog DB sink, combinable with any backend |
| `config-confers` | ❌ | Configuration loaded via confers + watch hot-reload of level and rotation parameters |
| `kms` | ❌ | KMS key providers (`EnvKeyProvider` / `ConfersKeyProvider` / Vault transit MVP) |
| `net-sink` | ❌ | Network forwarding sinks (TCP with optional TLS + UDP, buffered reconnection) |
| `otlp` | ❌ | OTLP/HTTP JSON log export MVP (hand-written transport, zero new dependencies) |
| `test-utils` | ❌ | Test-facing mock exports (`MockCache` / `MockConfig` / `MockDatabaseAdapter`); excluded from default and all production combinations |

> ⚠️ **Database backend exclusivity**: `sqlite` / `postgres` / `mysql` / `duckdb` are mutually exclusive (enforced via dbnexus; embedded and server-side drivers must not be mixed). `--all-features` is not supported; enable features grouped by backend instead.

---

## 📚 Documentation

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

---

## 💻 Examples

[`examples/`](examples/) is a standalone workspace crate (`inklog-examples`) with 39 examples in 7 categories. Run from the repository root:

```bash
cargo run --package inklog-examples --example <name>
```

#### Configuration (config)

| Example | Description | Feature |
|------|------|----------|
| `config_file` | Configuration file loading (Layer 1, local resources) | none |
| `config_inspect` | Config inspection: `sinks_enabled()` and `LoggerManager::load()` | none |
| `env_overrides` | Environment variable override loading | none |

#### Core (core)

| Example | Description | Feature |
|------|------|----------|
| `basic` | Basic usage: init, levels, structured fields, health check, graceful shutdown | none |
| `builder` | Builder pattern configuration | none |
| `all_features` | Full feature demonstration | none |
| `production` | Production environment configuration | none |
| `template` | Log templates | none |
| `error_handling` | Error handling (Layer 0, zero dependencies) | none |
| `i18n` | Internationalized formatting | none |

#### Sinks & Output (sinks)

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

#### Database (database)

| Example | Description | Feature |
|------|------|----------|
| `database` | Database sink with in-memory SQLite | any database backend (the example doc uses `sqlite`) |
| `database_pg_mysql` | PostgreSQL/MySQL database driver demo | none |
| `di_example` | Dependency injection pattern | `sqlite` (enforced via required-features) |

#### Infrastructure (infra)

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

#### Network (network)

| Example | Description | Feature |
|------|------|----------|
| `http` | HTTP health and metrics endpoint demo | none |
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

inklog uses a layered async architecture (see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and the `src/` module tree): the `domain` layer owns `LoggerManager` / `LoggerBuilder` and the configuration model, while `domain::core::subscriber` implements the tracing Subscriber and builds `LogRecord`s; records pass through `support::processing` for template rendering and masking before entering a bounded Crossbeam channel, from which the dedicated threads in `domain::core::workers` dispatch to the sinks in `support::io::sink`; the `integrations` layer adapts caching (oxcache), configuration (confers), and databases (dbnexus) onto the `Cache` / `Config` / `Database` traits via dependency inversion; `support::observability` aggregates health and metrics, exposed through the `http` feature.

```mermaid
flowchart TD
    APP["Application code<br/>tracing macros"] --> SUB["domain::core::subscriber<br/>LoggerSubscriber"]
    CFG["domain::config<br/>InklogConfig"] --> MGR["domain::core<br/>LoggerManager / LoggerBuilder"]
    MGR --> SUB
    MGR --> INT["integrations adapters<br/>OxCache / InklogConfig / DbNexus"]
    SUB --> PROC["support::processing<br/>template / masking / object_pool"]
    PROC --> CH["Bounded Crossbeam channel"]
    CH --> W["domain::core::workers<br/>file / database / health threads"]
    W --> SINK["support::io::sink<br/>console / file / database / net / otlp<br/>middleware / sampling / rate_limit"]
    SINK --> STORE["Storage backends<br/>filesystem / PostgreSQL / MySQL / SQLite / DuckDB"]
    W --> OBS["support::observability<br/>Metrics / HealthStatus"]
    OBS --> HTTP["HTTP endpoints<br/>health and Prometheus metrics"]
```

| Layer | Responsibility |
|------|------|
| `domain` | Log manager, builder, DI container, subscriber, worker threads, configuration model |
| `support` | Sink implementations, template and masking processing, object pool, metrics, validation, query, audit chain |
| `integrations` | Trait-based adapters for oxcache / confers / dbnexus / trait-kit, isolating external dependencies |
| `i18n` | Fluent + ICU message localization (zh-CN / en resources in `locales/`) |
| `cli` | `inklog-cli` binary: decrypt / generate / validate / query |

---

## 🔀 Core Pipeline

The full path of a log record from emission to persistence (distilled from the "Data Flow" chapter of [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)):

```mermaid
sequenceDiagram
    autonumber
    participant App as Application code
    participant Sub as LoggerSubscriber
    participant Chan as Bounded Crossbeam channel
    participant Worker as Worker threads
    participant FS as FileSink
    participant DS as DatabaseSink
    participant Met as Metrics
    participant HTTP as HTTP endpoints

    App->>Sub: record a log via tracing macros
    Sub->>Sub: build LogRecord and extract trace_id
    Sub->>Sub: mask sensitive fields by rule set
    Sub->>Chan: send LogRecord non-blocking
    Chan->>Worker: dispatch record from queue
    Worker->>FS: write with on-demand rotation, compression, encryption
    Worker->>DS: buffer and flush in batches
    Worker->>Met: update latency and sink health metrics
    Met-->>HTTP: expose health and Prometheus metrics
```

Key points:

- Senders only block when the channel is full (backpressure); default capacity is 10,000 with 3 worker threads, tunable via `PerformanceConfig`;
- The file thread performs blocking I/O, while the database thread runs its own dedicated tokio runtime;
- Encryption, compression, and rotation fire per rotated file and never sit on the per-record hot path.

---

## 🧯 Failure Handling

How sink failures are degraded and recovered (from the "Error Handling Flow" and "FileSink write flow" chapters of [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)):

```mermaid
flowchart TD
    W["Sink write"] --> OK["Write succeeded"]
    W --> ERR["Write failed"]
    ERR --> CB["Circuit breaker records failure"]
    CB --> THR{"Failure threshold reached"}
    THR -->|"yes"| DEG["Degraded output<br/>DB → File → Console fallback"]
    THR -->|"no"| RETRY["Retry up to three times<br/>with exponential backoff"]
    RETRY -->|"succeeded"| OK
    RETRY -->|"failed"| DEG
    DEG --> MET["Record failure metrics and update sink health"]
    MET --> HC["Health-check thread patrol<br/>every 10 seconds"]
    HC -->|"failures exceed threshold after cooldown"| RECOVER["Send sink recovery command"]
    RECOVER --> REINIT["Re-initialize sink<br/>reset circuit breaker"]
    REINIT --> OK
```

- **Circuit breaker**: default failure threshold of 5 with a 30-second cooldown; batch size halves in the half-open state;
- **Three-level fallback**: database failure degrades to file, file failure degrades to console;
- **Automatic recovery**: the health-check thread detects unhealthy sinks and triggers re-initialization, writing results back to metrics.

---

## 🧪 Testing

### Test Strategy

| Type | Location | Description |
|------|------|------|
| Unit tests | inline `#[cfg(test)]` in `src/` | Boundary and error scenarios per module; mocks visible via cfg(test) |
| Integration tests | `tests/integration/`, `tests/cli_integration.rs` | Batch writes, HTTP, CLI, compression ratio, Parquet, auto recovery, and more; requires the `sqlite,http,cli,compression,parquet,test-utils` combination |
| Combination tests | `tests/combinations/` | Feature combination matrix and multi-sink fallback; requires `sqlite` |
| End-to-end tests | `tests/e2e/e2e_advanced.rs` | 226 scenarios across 15 scenario domains (see [docs/TEST_SCENARIOS.md](docs/TEST_SCENARIOS.md)) |
| Docker database integration | `tests/docker/` + `docker/docker-compose.test.yml` | PostgreSQL / MySQL / SQLite lifecycle verification |
| Performance tests | `tests/performance/` + `benches/` | Large-volume and long-running tests plus criterion benchmarks |

**Test suite size** (as of v0.3.0-rc.3, counted by `#[test]` / `#[tokio::test]`): 1,255 inline in `src/` + 528 in `tests/` for a total of **1,783 test functions**, plus 18 criterion benchmark functions (15 in `benches/inklog_bench.rs`, 3 in `benches/rc4_pipeline_bench.rs`).

### Commands (matching CI)

```bash
# CI test gate (database backends are mutually exclusive; --all-features is unsupported)
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# Docker database integration tests
docker compose -f docker/docker-compose.test.yml up -d

# Coverage gate (CI requires ≥80% line coverage)
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 80

# Benchmarks
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
```

> **Locale note**: error messages are localized via ICU/Fluent based on the system locale. If tests assert English message text, set `INKLOG_LOCALE=en` (e.g. in CI or non-English environments) to pin the output language.

### Quality Gates

```bash
cargo fmt --all -- --check                    # format check
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings  # zero warnings
cargo deny check                              # dependency advisories / licenses / duplicate crates
cargo audit                                   # security advisories (lefthook pre-push)
```

---

## 📊 Performance

### Baseline Numbers

The first official baseline recorded in [docs/PERFORMANCE.md](docs/PERFORMANCE.md) (2026-09-11, criterion medians):

| Benchmark | Path | Median | Throughput |
|------|------|----------|------|
| `template_render_text` (default template + 2 fields) | Write | 320 ns | ~3.12 M rec/s |
| `mask_sensitive_fields` (regex + sensitive keys) | Write | 50.5 µs | ~19.8 K rec/s |
| `logrecord_to_json` (single record) | Serialization | 257 ns | ~3.89 M rec/s |
| `logrecord_batch_100_to_json` (batch of 100) | Serialization | 26.5 µs | ~3.77 M rec/s |
| `aes256gcm_roundtrip_1kb` (1 KiB encrypt/decrypt roundtrip) | Encryption | 506 ns | ~1.88 GiB/s |
| `pbkdf2_derive_600k` (PBKDF2-HMAC-SHA256 key derivation) | Encryption | 63.3 ms | ~15.8 ops/s |

**Environment notes** (as stated in docs/PERFORMANCE.md): WSL2 development laptop (linux 6.6.87.2), criterion 0.8, release profile (`opt-level=3`, `lto=fat`, `codegen-units=1`), quick baseline parameters `--warm-up-time 1 --measurement-time 2 --sample-size 10`. Numbers vary across machines and are provided for comparison, not as guarantees.

### Design Highlights

| Mechanism | Parameter | Effect |
|------|------|------|
| Bounded-channel backpressure | `channel_capacity` default 10,000 | Prevents memory exhaustion; channel usage exposed via metrics |
| Database batch writes | `batch_size` default 100, flush interval default 500 ms | docs/ARCHITECTURE.md reference: ~10,000 rows/s for batches of 100 vs ~100 rows/s for per-row inserts |
| Masking on demand | `masking_enabled` | Regex masking is the most expensive security step on the main path; enable only on sinks that need it |
| Zstd compression | levels 0-22, default 3 | ~3.5x ratio at the default level; encryption and key derivation fire per rotated file, not per record |

Reproduction steps are documented in the "Reproduction" section of [docs/PERFORMANCE.md](docs/PERFORMANCE.md).

---

## 🔒 Security

### Reporting a Vulnerability

Please do **not** disclose security vulnerabilities publicly; report responsibly following [docs/SECURITY.md](docs/SECURITY.md):

- **Preferred**: email [security@inklog.dev](mailto:security@inklog.dev)
- **Alternative**: [GitHub Security Advisories](https://github.com/Kirky-X/inklog/security/advisories)
- **Response targets**: acknowledgment within 24 hours, initial assessment within 48 hours, fix development in 7-14 days

### Security Design

| Capability | Implementation |
|------|------|
| Encryption at rest | AES-256-GCM authenticated encryption ([aes-gcm](https://crates.io/crates/aes-gcm)); ciphertext format `[nonce][ciphertext]` |
| Key memory safety | `zeroize` clears key material when it leaves scope |
| Key derivation | PBKDF2-HMAC-SHA256 with 600k iterations (minimum approved by security review; at most once per rotated file) |
| Path safety | `PathValidator` prevents path traversal and blocks user home directories and key files |
| Content sanitization | `LogSanitizer` guards against log injection; PII masking covers email, phone, ID, and card patterns |
| Access control | HTTP endpoint auth token cached at startup with fail-closed behavior; file permissions 0600 on Unix |
| Tamper-evident archives | `ArchiveChain` HMAC-SHA256 chain resisting deletion, reordering, and forgery |
| Compliance support | Encryption, masking, and audit design supports GDPR, HIPAA, PCI-DSS, and similar requirements (see [docs/SECURITY.md](docs/SECURITY.md)) |

### Supply Chain Security

The repository maintains [`deny.toml`](deny.toml): the CI security job and lefthook pre-push run `cargo deny check` (advisories / licenses / duplicate crates) and `cargo audit` (RustSec advisories) respectively; lefthook pre-commit includes a private-key file scan.

---

## 🗺️ Roadmap

Phased goals compiled from the existing release plan (timing may adjust with the overall workspace release schedule):

| Status | Goal | Notes |
|:----:|------|------|
| 📋 | v0.3.0 stable release | Complete the 0.3.0-rc.3 → 0.3.0 stable release |
| 📋 | Workspace dependency lockstep | trait-kit 0.5.0, oxcache 0.5.0, dbnexus 0.6.0 |
| 📋 | CI test matrix grouped by database backend | Backend features are mutually exclusive and must be validated per backend group |
| 📋 | Provision the MySQL integration environment | Integration tests for that backend are currently blocked by the missing MySQL service |
| 📋 | Raise test coverage | llvm-cov baseline is about 80%, moving toward the 95%+ target |

---

## 🤝 Contributing

Contributions are welcome! See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for the complete guide.

### Development Setup

| Requirement | Notes |
|------|------|
| Rust 1.97.1 | pinned by `rust-toolchain.toml` |
| Protobuf compiler | `protoc` is required to build combinations with database features |
| lefthook | install hooks with `bash scripts/install-pre-commit.sh` |

```bash
git clone https://github.com/Kirky-X/inklog.git
cd inklog
bash scripts/install-pre-commit.sh

# Run tests (same feature combination as CI)
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"
```

### Commit Conventions

- **Conventional Commits**: `feat: ...` / `fix: ...` / `docs: ...`, enforced by the commit-msg hook;
- **pre-commit hooks**: rustfmt, clippy (`-D warnings`, zero warnings), `cargo deny check`, private-key scan;
- **pre-push hooks**: `cargo audit` and the coverage ≥80% gate.

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

- **0.3.0-rc.3** (2026-09-10): added `init_inklog_logger` singleton initialization, runtime level hot-reload (`set_level`), dynamic sink registration (`LoggerBuilder::add_sink`), `trace_id`/`span_id` correlation, `inklog-cli query` log search, network forwarding sinks (TCP/UDP), an OTLP export MVP, the tamper-evident archive chain, and the first performance baseline in `docs/PERFORMANCE.md`;
- **0.3.0-rc.2** (2026-09-03): integrated trait-kit 0.5.0-rc.2 and the i18n refactor with a version bump; removed the three mocks from the default public API (BREAKING; external test consumers must enable `test-utils`);
- **0.2.0** (2026-08-05): added the `compression` / `parquet` / `fast-masking` features and the i18n core module; added ChannelBufferedFileSink, circuit-breaker protection, and the ring-buffered file sink; edition 2024, MSRV 1.94.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) with the additional [Commons Clause](LICENSE) condition: the software may not be sold without separate authorization. Copyright (c) 2026 Kirky.X.

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

Database, cache, configuration, and lifecycle integrations are provided by the sibling workspace projects dbnexus, oxcache, confers, and trait-kit.

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

**Built with ❤️ by the Inklog Team**

<sub>© 2026 Inklog Project. All rights reserved.</sub>

**[⬆ Back to Top](#-table-of-contents)**

</div>
