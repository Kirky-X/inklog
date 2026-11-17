# 📖 inklog User Guide

This guide covers the complete usage of inklog (enterprise-grade Rust logging infrastructure): installation, configuration in detail, common examples, advanced topics, and troubleshooting. Read it together with the [📘 API Reference](API_REFERENCE.md) for API details.

> Related documents: [📘 API Reference](API_REFERENCE.md) · [🏗️ Architecture](ARCHITECTURE.md) · [⚡ Performance Baseline](PERFORMANCE.md) · [🔒 Security Documentation](SECURITY.md)

<details open>
<summary>📑 Contents</summary>

- [🧭 Project Overview](#-project-overview)
- [✨ Core Features](#-core-features)
- [📦 Installation](#-installation)
- [🚀 Quick Start](#-quick-start)
- [⚙️ Configuration Reference](#️-configuration-reference)
- [💡 Common Examples](#-common-examples)
- [🛠️ Advanced Topics](#️-advanced-topics)
- [🌍 Environment Variables](#-environment-variables)
- [✅ Best Practices](#-best-practices)
- [🔧 Troubleshooting](#-troubleshooting)
- [❓ FAQ](#-faq)
- [🔗 Related Resources](#-related-resources)
- [🧭 Core Concepts](#-core-concepts)
- [🎨 Feature Flags](#-feature-flags)
- [💻 Complete Example List](#-complete-example-list)
- [🗺️ Roadmap](#️-roadmap)

</details>

---

## 🧭 Project Overview

inklog is a logging infrastructure library designed for Rust production environments: application code keeps using the standard `log` / `tracing` macros while inklog takes over subscription, masking, dispatch, and persistence.

### Design Philosophy

| Philosophy | Description |
|------|------|
| **High performance** | Bounded Crossbeam channels + a dedicated worker thread pool; non-blocking senders with backpressure when the queue is full |
| **Security first** | AES-256-GCM encryption, PII data masking, key zeroization in memory |
| **Flexible configuration** | Multi-target output (console, file, database, network, OTLP) |
| **Observability** | Health monitoring, Prometheus metrics export, latency histograms |
| **Production ready** | Automatic rotation, circuit breaker, failure fallback with automatic recovery, graceful shutdown |

### Core Concepts

Five core concepts — initialization (`LoggerManager` installs the global tracing subscriber and the `log` crate frontend, process-wide singleton semantics), recording (zero-intrusion standard macros), sinks (the output-target abstraction and built-in implementations), configuration (TOML + environment variable precedence), and shutdown (`shutdown()` drains the channel) — are described item by item in [🧭 Core Concepts](#-core-concepts) at the end of this guide.

## ✨ Core Features

### Core Capabilities (always available)

| Capability | Description |
|------|------|
| **Async pipeline** | Bounded Crossbeam channels + a dedicated worker thread pool, with backpressure control |
| **Multi-target output** | Write to console, file, database, and other targets simultaneously |
| **Structured logging** | tracing ecosystem integration, `trace_id` / `span_id` correlation |
| **Template formatting** | placeholder templates such as `{timestamp}` `{level}` `{message}` `{trace_id}` |
| **File rotation** | automatic size- and time-based rotation |
| **Data masking** | sensitive field-name detection + a regex rule set (PII coverage) |
| **Runtime hot-reload** | `set_level` adjusts global and per-target log levels instantly |
| **Health monitoring** | sink status, channel usage, and metric tracking |
| **Dynamic sinks** | `LoggerBuilder::add_sink` registers third-party sinks, one channel per sink |
| **i18n** | error messages rendered per system locale via Fluent + ICU (`i18n` feature, on by default; when disabled, falls back to the embedded English message table) |

### Optional Capabilities (feature-gated)

| Capability | Description |
|------|------|
| **Database sink** | PostgreSQL, MySQL, SQLite, DuckDB (via dbnexus; batched persistence, partitioned tables) |
| **i18n opt-out** | `default-features = false` prunes the icu/fluent dependency tree; `tr`/`tr_args` fall back to the embedded English message table |
| **Compression** | Zstd (`zstd`; `compression` is a deprecated alias) / Gzip (`gzip`) compression of rotated files |
| **Encryption** | AES-256-GCM encryption of rotated archives |
| **Parquet export** | analytics-ready columnar archive format |
| **HTTP endpoints** | Axum health check and Prometheus metrics endpoints |
| **Network forwarding** | TCP (optional TLS) + UDP sinks, disconnect buffering and automatic reconnection |
| **OTLP export** | OTLP/HTTP JSON log export |
| **CLI tooling** | `inklog-cli`: decrypt / generate / validate / query / verify-chain (audit-chain verification, `--manifest` + `--key-env`; verification also requires the `<stem>.chain.seal.json` seal next to the manifest — a missing seal or a mismatch with the manifest fails with exit code 2; an empty manifest fails the same way (exit code 2), regardless of whether a key is set; when the seal's entry count disagrees with the manifest, it re-reads once before judging — if the re-read pair matches, it reports "rotation occurred during verification" and exits with the actual result; if it still mismatches, it emits a diagnosis distinct from tampering (manifest and seal from different generations), still exiting 2) |
| **Sampling and rate limiting** | sampler (N-of-1), token-bucket rate limiting, middleware-chain decorators, configurable sampling policy (per_level rates + per_target_prefix rules), per-target tiered rate limiting (rate_limit.rules prefix quota groups, longest prefix first) |
| **Tamper-evident archival** | HMAC-SHA256 archive chain against deletion, reordering, and forgery; truncation to a non-empty prefix is detected by the out-of-band seal (total entry count + tail hash) |

## 📦 Installation

Add the following to `Cargo.toml` (`default = ["i18n", "cache"]`; core capabilities + internationalization + the cache backend are enabled by default):

```toml
[dependencies]
inklog = "0.3.0"
```

### Enabling Optional Features

```toml
# HTTP endpoint
inklog = { version = "0.3.0", features = ["http"] }

# CLI tool
inklog = { version = "0.3.0", features = ["cli"] }

# Database support (pick one of four, mutually exclusive)
inklog = { version = "0.3.0", features = ["sqlite"] }
inklog = { version = "0.3.0", features = ["postgres"] }
inklog = { version = "0.3.0", features = ["mysql"] }
inklog = { version = "0.3.0", features = ["duckdb"] }

# Compression and performance
inklog = { version = "0.3.0", features = ["zstd", "gzip", "parquet", "fast-masking"] }

# Integration and extension
inklog = { version = "0.3.0", features = ["net-sink", "otlp", "kms", "config-confers", "dbnexus-audit", "kit"] }
```

For the full feature list and descriptions, see [🎨 Feature Flags](#-feature-flags) at the end of this guide.

> ⚠️ **Database backend exclusivity**: `sqlite` / `postgres` / `mysql` / `duckdb` cannot be enabled together and `--all-features` is unsupported; the reason and the per-backend grouping are in [🎨 Feature Flags](#-feature-flags) at the end of this guide.

> Note: TOML configuration file loading (`LoggerManager::from_file` / `load`) is built in and needs no extra feature.

## 🚀 Quick Start

### Basic Logging

The simplest usage — initialize the logging system with the default configuration:

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with default config and install the global subscriber (console sink, level info)
    let logger = LoggerManager::new().await?;

    tracing::trace!("这是一条追踪消息");
    tracing::debug!("这是一条调试消息");
    tracing::info!("这是一条信息消息");
    tracing::info!(user_id = 42, action = "login", "结构化字段示例");
    tracing::warn!("这是一条警告消息");
    tracing::error!("这是一条错误消息");

    // Drain the channel and shut down all sinks before exit
    logger.shutdown()?;
    std::mem::forget(logger); // Already shut down explicitly; prevents a duplicate shutdown in Drop
    Ok(())
}
```

You can also use the standard `log` crate macros (bridged by the built-in `LogLogger`):

```rust
log::info!("应用已启动");
log::error!("发生错误: {}", "详情");
```

Process-wide singleton initialization (re-initialization returns a clear error):

```rust
inklog::init_inklog_logger().await?;
```

### File Logging

Configure file output with automatic rotation and compression:

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure the log directory exists
    std::fs::create_dir_all("logs")?;

    // Configure the file sink
    let file_config = FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        max_size: "10MB".into(),       // Rotate when the file reaches 10MB
        rotation_time: "daily".into(), // Rotate daily
        keep_files: 7,                 // Keep 7 rotated files
        compress: true,                // Compress rotated files (Zstd via the compression feature; falls back to gzip when disabled)
        encrypt: false,                // No encryption
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("应用已启动");
    log::error!("发生错误: {}", "详情");

    Ok(())
}
```

### Database Logging

Write logs to a database with batched inserts (requires any one database backend feature):

```rust
use inklog::config::DatabaseDriver;
use inklog::{DatabaseSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_sink = DatabaseSinkConfig {
        enabled: true,
        driver: DatabaseDriver::SQLite,
        url: "sqlite://logs/app.db".to_string(),
        pool_size: 5,
        batch_size: 100,           // Write 100 logs per batch
        flush_interval_ms: 1000,   // Flush once per second
        ..Default::default()
    };

    let config = InklogConfig {
        database_sink: Some(database_sink),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("日志已写入数据库");

    Ok(())
}
```

## ⚙️ Configuration Reference

### Basic Configuration Structure

`InklogConfig` is the root configuration struct containing all sub-configurations:

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "info".into(),
        format: "{timestamp} [{level}] {target} - {message}".to_string(),
        masking_enabled: true,
        ..Default::default()
    },
    console_sink: Some(inklog::config::ConsoleSinkConfig::default()),
    file_sink: None,
    database_sink: None,
    performance: inklog::config::PerformanceConfig::default(),
    http_server: None,
    ..Default::default()
};
```

#### InklogConfig Fields

| Field | Type | Default | Description |
|------|------|--------|------|
| `global` | `GlobalConfig` | `default()` | global configuration |
| `console_sink` | `Option<ConsoleSinkConfig>` | `Some(default())` | console sink configuration |
| `file_sink` | `Option<FileSinkConfig>` | `None` | file sink configuration |
| `database_sink` | `Option<DatabaseSinkConfig>` | `None` | database sink configuration |
| `performance` | `PerformanceConfig` | `default()` | performance configuration |
| `http_server` | `Option<HttpServerConfig>` | `None` | HTTP server configuration |
| `target_levels` | `HashMap<String, String>` | `{}` | per-target level presets (e.g. `{"hyper" = "warn"}`) |

#### Global Configuration (GlobalConfig)

| Field | Type | Default | Description |
|------|------|--------|------|
| `level` | `String` | `"info"` | log level: `trace`, `debug`, `info`, `warn`, `error`, `fatal` |
| `format` | `String` | `"{timestamp} [{level}] {target} - {message}"` | log format template |
| `masking_enabled` | `bool` | `true` | whether data masking is enabled |
| `auto_fallback` | `bool` | `true` | whether sinks degrade automatically on failure |
| `fallback_initial_delay_ms` | `u64` | `1000` | wait before the first retry (milliseconds) |
| `fallback_max_delay_ms` | `u64` | `60000` | retry delay cap (milliseconds) |
| `fallback_max_retries` | `u32` | `10` | maximum retry count |
| `output_format` | `OutputFormat` | `Text` | output format: `Text` (template) or `Json` (NDJSON) |
| `fallback_journal` | `bool` | `false` | persist ERROR/FATAL overflow to a fallback journal (replayed at startup against loss) |
| `fallback_journal_path` | `String` | `"logs/fallback.journal"` | journal file path |
| `fallback_journal_encrypt` | `bool` | `false` | AES-256-GCM journal encryption (fails explicitly at construction when the key is missing/invalid) |
| `fallback_journal_key_env` | `Option<String>` | `None` | journal key environment variable name (default `INKLOG_JOURNAL_KEY`; takes precedence over the key file) |
| `fallback_journal_key_file` | `Option<String>` | `None` | journal key file path (unix permissions must be 0600) |
| `fallback_journal_push_addr` | `Option<String>` | `None` | journal push address (`host:port`; disconnect buffering, failures never back-pressure persistence) |

#### Template Variables

| Variable | Description |
|------|------|
| `{timestamp}` | ISO 8601 timestamp |
| `{level}` | log level |
| `{target}` | log target (module/file) |
| `{message}` | log message |
| `{fields}` | additional structured fields (JSON) |
| `{file}` | source file name |
| `{line}` | source line number |
| `{thread_id}` | thread ID |
| `{trace_id}` | trace ID (extracted from the current tracing span) |
| `{span_id}` | span ID |

### File Output Configuration

#### The Two File Write Paths (ChannelBufferedFileSink Promotion)

The default file configuration goes through `ChannelBufferedFileSink` (CBFS): a dedicated flush thread (100 ms forced flush by default) + a configurable backpressure policy (`Block` by default) + PII masking + size/time rotation + compressed/encrypted archives + the audit chain + retention cleanup (rotation naming and archive post-processing share the same implementation as FileSink). Only two capabilities remain exclusive to `FileSink`:

| Capability | ChannelBufferedFileSink | FileSink |
|------|-------------------------|----------|
| PII masking | ✓ | ✓ |
| Dedicated flush thread / batched flushing | ✓ | batched flushing (no dedicated thread) |
| Backpressure policy (Block/DropOldest/DropNewest) | ✓ | fixed drop + count |
| Size/time rotation | ✓ (time is a rolling-interval approximation, counted from the last rotation) | ✓ (time is calendar-aligned: daily = midnight next day, monthly = same day next month) |
| Compressed / encrypted archives | ✓ (reuses FileSink post-processing) | ✓ |
| Audit chain (`audit_chain_enabled`) | ✓ | ✓ |
| Retention cleanup (`retention_days`/`keep_files`) | ✓ (reuses `FileSink::perform_cleanup`) | ✓ |
| Per-batch `fsync` | ✗ | ✓ |
| `output_format = "Json"` (NDJSON egress) | ✗ | ✓ |

Setting `fsync = true` or `output_format = "Json"` falls back to `FileSink` automatically. The encrypted-archive key comes from **env first, then `encryption_key_file`** (unix permissions 0600); a missing/invalid key fails explicitly at construction.

#### FileSinkConfig Fields

| Field | Type | Default | Description |
|------|------|--------|------|
| `enabled` | `bool` | `true` | whether the file sink is enabled |
| `path` | `PathBuf` | `"logs/app.log"` | log file path |
| `max_size` | `String` | `"100MB"` | maximum file size before rotation (e.g. `"10MB"`, `"500KB"`) |
| `rotation_time` | `String` | `"daily"` | time rotation strategy: `"hourly"`, `"daily"`, `"weekly"`. FileSink is calendar-aligned (daily = midnight next day, etc.); the default path CBFS approximates with a rolling interval (counted from the last rotation) |
| `keep_files` | `u32` | `30` | number of rotated files to keep |
| `compress` | `bool` | `true` | whether to compress rotated files |
| `compression_level` | `i32` | `3` | compression level (1-22; higher compresses more; out-of-range values are rejected at configuration validation) |
| `encrypt` | `bool` | `false` | whether to encrypt rotated archives |
| `encryption_key_env` | `Option<String>` | `None` | environment variable name of the encryption key (takes precedence over `encryption_key_file`) |
| `encryption_key_file` | `Option<String>` | `None` | encryption key file path (unix permissions must be 0600; overridden when env is set) |
| `retention_days` | `u32` | `30` | days to keep logs |
| `max_total_size` | `String` | `"1GB"` | maximum total size of the log directory |
| `cleanup_interval_minutes` | `u64` | `60` | interval for cleaning old logs (minutes) |
| `batch_size` | `usize` | `100` | records buffered before writing |
| `flush_interval_ms` | `u64` | `100` | maximum flush interval (milliseconds) |
| `masking_enabled` | `bool` | `true` | whether masking is enabled for file output |
| `output_format` | `OutputFormat` | `Text` | output format: `Text` or `Json` |

#### File Rotation Example

```rust
let file_config = FileSinkConfig {
    enabled: true,
    path: "logs/app.log".into(),
    max_size: "50MB".into(),        // Rotate at 50MB
    rotation_time: "daily".into(),  // Rotate daily
    keep_files: 14,                 // Keep 14 rotated files
    compress: true,                 // Compress old logs
    ..Default::default()
};
```

#### Rotated File Naming

When rotation triggers, the current file is renamed into a timestamped archive (timestamp format `%Y%m%d_%H%M%S`):

```text
logs/
├── app.log                      # Current active file (plaintext)
├── app_20260913_143022.log      # Rotated
├── app_20260912_080000.log.zst  # Compressed (compression / gzip feature)
└── app_20260911_080000.log.zst.enc  # Compressed and encrypted (encrypt = true)
```

> Encryption and compression apply only to **rotated archives**; the active file stays in plaintext; the processing order is compress first (`.zst`), then encrypt (`.enc`). Decrypt with `inklog-cli decrypt` — see the [🔒 Security Documentation](SECURITY.md).

### Database Output Configuration

#### DatabaseSinkConfig Fields

| Field | Type | Default | Description |
|------|------|--------|------|
| `name` | `String` | `"default"` | sink name |
| `enabled` | `bool` | `false` | whether the database sink is enabled |
| `driver` | `DatabaseDriver` | `SQLite` | database driver |
| `url` | `String` | `"sqlite::memory:"` | database connection URL |
| `pool_size` | `u32` | `10` | connection pool size (forced to 1 for SQLite) |
| `batch_size` | `usize` | `100` | batch size |
| `flush_interval_ms` | `u64` | `500` | flush interval (milliseconds) |
| `partition` | `PartitionStrategy` | `Monthly` | partition strategy: `Monthly`, `Yearly` |
| `table_name` | `String` | `"logs"` | log table name |
| `archive_format` | `ArchiveFormat` | `Json` | archive export format: `Json`, `Parquet`, `Csv` |
| `parquet_config` | `ParquetConfig` | `default()` | Parquet export configuration (`parquet` feature) |
| `permissions_path` | `Option<String>` | `None` | RBAC permission configuration file path |
| `admin_role` | `String` | `"admin"` | admin role name for DDL and write operations |

> `DatabaseSinkConfig` can be configured without any feature, but actually persisting requires one of `sqlite` / `postgres` / `mysql` / `duckdb`.

#### Database Driver Types

| Driver | String form | URL example |
|------|------------|----------|
| `PostgreSQL` | `"postgres"` | `postgres://user:pass@localhost/logs` |
| `MySQL` | `"mysql"` | `mysql://user:pass@localhost/logs` |
| `SQLite` | `"sqlite"` | `sqlite://logs/app.db` |
| `DuckDB` | `"duckdb"` | path to a DuckDB database file |

### Data Masking Configuration

inklog automatically masks sensitive personal information (PII): 21 built-in regex rules (credit card numbers validated with Luhn) + sensitive field-name detection, applied in priority order.

#### Masking Effect Examples

| Data type | Before | After |
|---------|---------|---------|
| Email address | `user@example.com` | `**@**.***` |
| Chinese mobile number | `13812345678` | `***-****-****` |
| Credit card number (Luhn-validated) | `4111111111111111` | `****-****-****-1111` |
| ID card number | `110101199001011234` | `******1234` |
| US SSN (hyphenated / spaced / compact forms, last four kept) | `123-45-6789` | `***-**-6789` |

#### Enabling Data Masking

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "info".into(),
        format: "{timestamp} {level} {message}".to_string(),
        masking_enabled: true,  // Enable data masking (on by default)
        ..Default::default()
    },
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;

// Sensitive data is masked automatically
log::info!("用户邮箱: user@example.com");
// Output: 用户邮箱: **@**.***
```

#### Source-Level Sensitive Field Redaction

The regex rule set above masks sensitive **content** found in messages and field values. When a structured field's **name** is known in advance, inklog provides a second, earlier layer: source-level redaction replaces the whole field value before the record enters the pipeline — the raw value never reaches any sink or downstream middleware.

```rust
use inklog::{LoggerManager, mark_sensitive_fields};

// Register process-level sensitive fields BEFORE the logger is built
// (each subscriber snapshots the registry at construction time).
mark_sensitive_fields(["email", "phone"]);

let _logger = LoggerManager::builder().level("info").build().await;

tracing::info!(target: "app::auth", message = "login", email = "user@example.com", phone = "13812345678");
// Field values on disk: "email": "<redacted:email>", "phone": "<redacted:phone>"
```

Per-subscriber declaration (merged with the process-level registry) is available via `LoggerSubscriber::with_sensitive_fields(["email"])`; the `SensitiveRedaction` middleware itself is exported from the crate root for custom sink pipelines.

Two-layer semantics:

| Layer | Matches | When it runs | Result |
|-------|---------|--------------|--------|
| Source redaction (`mark_sensitive_fields` / `with_sensitive_fields`) | field **names**, exact match | first in the middleware chain, before user middleware and governance | whole value → `<redacted:{field}>` |
| Regex masking (PII rules, `masking_enabled`) | value **content** | sink egress | rule replacement (e.g. `[CARD_NUM]`) |

Boundaries:

- The placeholder template defaults to `<redacted:{field}>`; `SensitiveRedaction::with_placeholder_template` customizes it (e.g. `"{field}:redacted"`).
- A field whose value is a JSON object or array is replaced **wholesale** with the placeholder string — nested values are not traversed or partially masked.
- Non-string values (numbers, booleans, objects) are replaced as well: any registered field name yields the placeholder regardless of value type.
- The source layer runs before the regex layer: a placeholder contains no raw value, so the regex content rules leave it untouched (marker idempotence).
- If a registered field's **name** is itself a sink-side sensitive field name (e.g. `api_token` — the `token` token), the sink egress name-based masking replaces the whole value with `***MASKED***`: the raw value still never appears, but the on-disk marker differs from the source placeholder. Prefer registered names that are not sink-sensitive names when the `<redacted:{field}>` marker must survive verbatim.

#### Custom Masking Rules

Customize rules via `MaskRuleBuilder` / `DataMaskerBuilder` / `MaskRuleRegistry`; the full API is in the [📘 API Reference](API_REFERENCE.md):

```rust
use inklog::{MaskRule, MaskRuleBuilder};

let custom_rule = MaskRule::builder("employee_id")
    .pattern(r"\bEMP-\d{6}\b")
    .replacement("EMP-***")
    .priority(30)
    .build()
    .expect("Invalid pattern");
```

#### Sink Egress Masking Contract

The PII masking that the console / file sinks perform under the `masking_enabled` gate uses the **inklog built-in rule set** constructed by `DataMasker::new()` (21 regex rules + sensitive field-name detection, in priority order). These built-in rules are **not the same source** as consumer-managed masking rules in business code: the masking a consumer performs at its own egress points (e.g. API responses, database columns) and the built-in mask at the inklog sink egress are independent in both rule set and masked form — never assume the two outputs match. To reuse consumer-managed rules on the sink side, build a `DataMasker` from a registry and inject it via `with_masker` (see below).

**Marker idempotence contract**: masking output carries markers such as `***REDACTED***`, `***MASKED***`, `[REDACTED]`. Marker protection applies only at the **value** level: a value that is already a marker is not masked again (when multiple masking layers stack — e.g. a global sanitizer has already processed the record and the sink egress PII mask processes it again — you will not get REDACTED nested inside REDACTED). **The whole message** is never short-circuited on markers — the marker vocabulary can be forged by user input, and whole-message short-circuiting would exempt a record containing one injected marker from all masking (including the secret-scan gate): bare secrets and PII in the same message are masked as usual.

**Mask when safety cannot be proven**: when a single message exceeds 1 MiB (scanning skipped) or the regex engine errors (e.g. the `fancy-regex` backtracking budget is exhausted), `DataMasker::mask` does not pass the original text through; it redacts wholesale as `***REDACTED_OVERSIZED***` / `***REDACTED_ENGINE_ERROR***` and leaves a warn — both skipped scanning and engine errors mean "does this content contain sensitive values" is undecidable. Deployments that need per-record discrimination can use `DataMasker::mask_checked` instead (returns `Err` when the input is oversized or hits exceed the gate limit, letting the caller decide to drop or degrade).

**Custom rule-set injection (`with_registry` alignment)**: `DataMasker::builder().with_registry(registry)` **replaces** all built-in rules with the registry's rules (`add_rule` / `disable_builtin` keep composing on top of the replacement); obtain a registry via `MaskRuleRegistry::with_builtins()` (loads the built-in rules) or `load_from_toml` (parses TOML rules). The resulting `DataMasker` takes effect when injected through the sink's `with_masker`:

```rust
use inklog::{DataMasker, MaskRuleRegistry, sink::console::ConsoleSink};

let registry = MaskRuleRegistry::with_builtins();
let masker = DataMasker::builder().with_registry(registry).build();

let sink = ConsoleSink::new(console_config, template).with_masker(masker);
// FileSink / ChannelBufferedFileSink likewise provide with_masker injection
```

Note: derived instances created by `FileSink` rotation do not inherit the injected masker (they fall back to the built-in rule set); for `ChannelBufferedFileSink`, injection overrides the default construction controlled by `base_config.masking_enabled`.

### HTTP Server Configuration

#### HttpServerConfig Fields

| Field | Type | Default | Description |
|------|------|--------|------|
| `enabled` | `bool` | `false` | whether the HTTP server is enabled (requires the `http` feature) |
| `host` | `String` | `"127.0.0.1"` | listening host address (an IP literal: plain IPv4 like `127.0.0.1`, or bracketed IPv6 like `[::1]`) |
| `port` | `u16` | `9090` | listening port |
| `metrics_path` | `String` | `"/metrics"` | Prometheus metrics endpoint path |
| `health_path` | `String` | `"/health"` | health check endpoint path |
| `error_mode` | `HttpErrorMode` | `Strict` | startup-failure handling: `Strict` returns an error, `Warn` logs a warning and continues |
| `auth` | `Option<HttpAuthConfig>` | `None` | authentication configuration (token environment variable, cached at startup, fail-closed on failure) |
| `ip_whitelist` | `Option<Vec<String>>` | `None` | IP allowlist |
| `tls` | `Option<TlsConfig>` | `None` | TLS configuration (`cert_path` / `key_path`) |

#### HTTP Server Example

> **Non-loopback binding requires access control**: when `http_server` binds a non-loopback address (`0.0.0.0` / `[::]` / intranet IP) with neither `auth` nor `ip_whitelist`, configuration validation returns `Err` outright — `/metrics` and `/health` must not be exposed to the network without access control.

> **`host` must be a bindable address literal**: only plain IPv4 (`127.0.0.1`) and bracketed IPv6 (`[::1]` / `[::]`) are accepted. Hostnames (`localhost`) and malformed bracket spellings (`[::1`, `[[::1]]`) cannot form a socket address, so configuration validation returns `Err` — such addresses would fail to bind at runtime anyway. Loopback determination and runtime binding share the same parsing, so "judged loopback but cannot bind" cannot happen. An allowlist entry that permits all addresses (the CIDR `0.0.0.0/0`) does not constitute access control.

```rust
use inklog::config::{HttpErrorMode, HttpServerConfig};
use inklog::{InklogConfig, LoggerManager};

let config = InklogConfig {
    http_server: Some(HttpServerConfig {
        enabled: true,
        host: "0.0.0.0".to_string(),
        port: 8080,
        metrics_path: "/metrics".to_string(),
        health_path: "/health".to_string(),
        error_mode: HttpErrorMode::Strict,
        // Non-loopback binding: one of the access controls is required
        ip_whitelist: Some(vec!["10.0.0.0/8".to_string()]),
        ..Default::default()
    }),
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;

// Health endpoint: http://0.0.0.0:8080/health
// Metrics endpoint: http://0.0.0.0:8080/metrics
```

`LoggerBuilder` can also wire access control directly (`http_auth` / `http_ip_whitelist`):

```rust
let _logger = LoggerManager::builder()
    .enable_http_server(true)
    .http_host("0.0.0.0")
    .http_port(8080)
    .http_auth("INKLOG_HTTP_AUTH_TOKEN")   // token value is read from the environment variable
    .http_ip_whitelist(vec!["10.0.0.0/8".to_string()])
    .build()
    .await?;
```

When the peer address is loopback (including the default `127.0.0.1` binding and requests from the local machine under a wildcard binding), the server requires the target host to be a loopback host (`localhost` / `127.0.0.1` / `[::1]`) — blocking the DNS rebinding path of reading endpoint responses through a local port. The target host comes from the `Host` header, falling back to the URI authority when absent (HTTP/2 only carries `:authority`); when both exist they must point at the same host.

#### Health Check Endpoint (/health)

Returns the health status as JSON (the structure corresponds to `HealthStatus` / `MetricsSnapshot`):

```json
{
  "overall_status": "Healthy",
  "sinks": {
    "console": {
      "status": "Healthy",
      "last_error": null,
      "consecutive_failures": 0
    }
  },
  "channel_usage": 0.1,
  "uptime_seconds": 1234,
  "metrics": {
    "logs_written": 1000,
    "logs_dropped": 0,
    "channel_blocked": 0,
    "sink_errors": 0,
    "avg_latency_us": 150,
    "latency_distribution": [10, 50, 100, 500, 2000],
    "active_workers": 3
  }
}
```

#### Prometheus Metrics Endpoint (/metrics)

Returns metrics in Prometheus format (excerpt):

```text
# HELP inklog_logs_written_total Total logs successfully written
# TYPE inklog_logs_written_total counter
inklog_logs_written_total 1000

# HELP inklog_sink_errors_total Total sink errors
# TYPE inklog_sink_errors_total counter
inklog_sink_errors_total 0

# HELP inklog_write_latency_us Log write latency in microseconds
# TYPE inklog_write_latency_us histogram
inklog_write_latency_us_bucket{le="50"} 950
```

### Performance Configuration

#### PerformanceConfig Fields

| Field | Type | Default | Description |
|------|------|--------|------|
| `channel_capacity` | `usize` | `10000` | log channel capacity |
| `worker_threads` | `usize` | `3` | number of worker threads |
| `channel_strategy` | `ChannelStrategy` | `Fixed` | channel capacity strategy: `Fixed`, `Adaptive` (scales by usage) |
| `expand_threshold_percent` | `u8` | `80` | expansion trigger usage (%) |
| `shrink_threshold_percent` | `u8` | `20` | shrink trigger usage (%) |
| `shrink_wait_seconds` | `u64` | `30` | wait before shrinking (seconds) |
| `min_capacity` | `usize` | `1000` | minimum channel capacity |
| `max_capacity` | `usize` | `50000` | maximum channel capacity |
| `rate_limit` | `Option<u64>` | `None` | global log rate cap (records/second) — effective in the default build; when unset, rate limiting is inactive (with an explicit startup warning) |

#### Performance Tuning Example

```rust
use inklog::{InklogConfig, config::PerformanceConfig};

let config = InklogConfig {
    performance: PerformanceConfig {
        channel_capacity: 20000,  // Increase channel capacity
        worker_threads: 4,        // Increase worker threads
        ..Default::default()
    },
    ..Default::default()
};

let _logger = LoggerManager::with_config(config).await?;
```

For tuning experience and official benchmark numbers, see the [⚡ Performance Baseline](PERFORMANCE.md).

## 💡 Common Examples

### Multiple Levels and Structured Fields

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;

    // Log at different levels
    tracing::info!("信息消息：应用已启动");
    tracing::warn!("警告消息：配置文件使用默认值");
    tracing::error!("错误消息：无法连接到数据库");

    // Use target to identify the log source
    tracing::info!(target: "auth", "用户登录");
    tracing::info!(target: "database", "查询执行");

    Ok(())
}
```

### File Logging and Rotation

For the complete file-sink initialization example (including the `FileSinkConfig` configuration and graceful shutdown), see [Quick Start · File Logging](#file-logging); for all rotation fields of `FileSinkConfig` and archive naming rules, see [File Output Configuration](#file-output-configuration).

### Encrypted Logging

Encrypt rotated archives with AES-256-GCM (set the key environment variable first):

```bash
# Generate a 32-byte high-entropy key (Base64-encoded)
export INKLOG_ENCRYPTION_KEY=$(openssl rand -base64 32)
```

```rust
use inklog::{FileSinkConfig, InklogConfig, LoggerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("logs")?;

    let file_config = FileSinkConfig {
        enabled: true,
        path: "logs/secure.log".into(),
        max_size: "10MB".into(),
        rotation_time: "daily".into(),
        keep_files: 5,
        encrypt: true,  // Enable encryption (applies to rotated archives)
        encryption_key_env: Some("INKLOG_ENCRYPTION_KEY".into()),
        ..Default::default()
    };

    let config = InklogConfig {
        file_sink: Some(file_config),
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("敏感操作：支付处理");

    Ok(())
}
```

> Key requirements: exactly 32 bytes after Base64 decoding, with Shannon entropy ≥ 4.0 (low-entropy keys are rejected with an error at startup). For key rotation and decryption, see the [🔒 Security Documentation](SECURITY.md).

### Custom Formatting

```rust
use inklog::{InklogConfig, config::GlobalConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InklogConfig {
        global: GlobalConfig {
            level: "debug".into(),
            // Custom format: includes file location info
            format: "[{timestamp}] [{level}] {target} - {message} | {file}:{line}".to_string(),
            masking_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let _logger = LoggerManager::with_config(config).await?;

    log::info!("使用自定义格式的日志");

    Ok(())
}
```

> Placeholders support only the undecorated `{name}` form (`timestamp`, `level`, `target`, `message`, `file`, `line`, `thread_id`, `trace_id`, `span_id`, `fields`); format decorations such as width/alignment (e.g. `{level:>5}`) are not recognized and are output literally as written.

### Running Examples (cargo run --bin)

The `examples/` crate provides 40 runnable examples in 7 categories; the full list is in [💻 Complete Example List](#-complete-example-list) at the end. Run them with:

```bash
cargo run --package inklog-examples --bin <name>
```

Selected examples:

| Example | Topic | Feature |
|------|------|----------|
| `basic` | init, levels, structured fields, graceful shutdown | none |
| `builder` | Builder-pattern configuration | none |
| `rotation` | size- and time-based file rotation | none |
| `ring_buffered_file` | channel-buffered file sink (high throughput) | none |
| `masking` | data masking | none |
| `encryption` | log encryption | none |
| `circuit_breaker` | sink circuit breaker and failure recovery | none |
| `database` | database sink (in-memory SQLite) | `sqlite` |
| `compression` | Zstd compression and decompression | `compression` |
| `metrics` | health monitoring and metrics collection | none |

> Database-related examples need the corresponding feature (e.g. `sqlite`); check the required-features configuration in `examples/Cargo.toml` before running.

## 🛠️ Advanced Topics

### Runtime Level Hot-Reload

Adjust global or per-target log levels without restarting the process (via `tracing_subscriber::reload` swapping the EnvFilter; additional `RUST_LOG` directives survive rebuilds):

```rust
use inklog::LoggerManager;

let manager = LoggerManager::new().await?;

// Adjust the global level
manager.set_level(None, "debug")?;

// Adjust the level for a specific target (e.g., quiet a third-party crate)
manager.set_level(Some("hyper"), "warn")?;

// Inspect the currently active filter
println!("{}", manager.current_level_filter_string());
```

### Dynamic Sink Registration

Once a third-party sink implements the `LogSink` trait, register it via `LoggerBuilder::add_sink` with zero core changes; each dynamic sink owns an independent channel and never preempts the others:

```rust
use inklog::{AsyncSink, LoggerBuilder};
use std::sync::Arc;

let logger = LoggerBuilder::new()
    .level("info")
    .add_sink(Arc::new(MyCustomSink::new()))  // Custom sink (impl LogSink)
    .build()
    .await?;
```

For the complete custom-sink implementation steps, see the "Extension Points" section of the [🏗️ Architecture](ARCHITECTURE.md).

### Multi-Target Output

```rust
use inklog::{DatabaseSinkConfig, FileSinkConfig, InklogConfig};

let config = InklogConfig {
    console_sink: Some(inklog::config::ConsoleSinkConfig {
        enabled: true,
        colored: true,
        ..Default::default()
    }),
    file_sink: Some(FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        ..Default::default()
    }),
    database_sink: Some(DatabaseSinkConfig {
        enabled: true,
        url: "sqlite::memory:".to_string(),
        ..Default::default()
    }),
    ..Default::default()
};
```

### Batched Writes

The database sink buffers logs and persists them in a batch when either condition is met: the count reaches `batch_size`, or `flush_interval_ms` elapses:

```rust
let database_sink = DatabaseSinkConfig {
    enabled: true,
    batch_size: 500,        // Write every 500 entries
    flush_interval_ms: 2000, // or every 2 seconds
    ..Default::default()
};
```

### Sink Failure Recovery

inklog provides a circuit breaker, three-level fallback (DB → File → Console), and automatic recovery; the health-check thread patrols every 1 second (unhealthy-sink rebuild cooldown 30 s, requiring more than 3 consecutive failures) and rebuilds sinks automatically when conditions are met.

```rust
// Manually recover a specific sink
logger.recover_sink("file")?;

// Recover all unhealthy sinks
let recovered = logger.trigger_recovery_for_unhealthy_sinks()?;
println!("已恢复的 Sink: {:?}", recovered);

// Inspect health status
let health = logger.get_health_status();
println!("整体状态: {:?}", health.overall_status);
println!("Channel 使用率: {:.2}%", health.channel_usage * 100.0);
```

For the complete failure-handling and recovery flow, see the "Failure Fallback and Self-Healing" section of the [🏗️ Architecture](ARCHITECTURE.md).

### Testing with Mock Implementations

inklog provides three mock implementations — `MockCache` / `MockConfig` / `MockDatabaseAdapter` — for unit and integration tests, with no real infrastructure services needed.

> Mock types are gated by the `test-utils` feature (since v0.3.0, the default public API contains no mocks); external test consumers must enable it explicitly:

```toml
[dev-dependencies]
inklog = { version = "0.3.0", features = ["test-utils"] }
```

#### Dependency Injection Test Pattern

```rust
use inklog::{LoggerDependencies, LoggerManager, MockCache, MockConfig, MockDatabaseAdapter};
use std::sync::Arc;

#[tokio::test]
async fn test_with_mocks() -> Result<(), Box<dyn std::error::Error>> {
    let database = Arc::new(MockDatabaseAdapter::new());

    let deps = LoggerDependencies {
        cache: Some(Arc::new(MockCache::new())),
        config: Some(Arc::new(MockConfig::new().with_value("level", "debug"))),
        custom_sinks: Vec::new(), // Required: leave empty when no third-party sinks (populated when registered via LoggerBuilder::add_sink)
        database: Some(database.clone()),
    };

    // LoggerManager::with_dependencies is deprecated since 0.3.0 (DI entry converged to the builder; calls emit a deprecation warning):
    // New code should use the LoggerManager::builder() chain instead
    let logger = LoggerManager::with_dependencies(deps).await?;

    log::info!("Test message");

    // Verify the logs were written to the mock database
    assert_eq!(database.record_count(), 1);
    Ok(())
}
```

#### MockCache Usage Example

```rust
use inklog::MockCache;

#[tokio::test]
async fn test_cache_operations() -> Result<(), inklog::InklogError> {
    let cache = MockCache::new();

    cache.set("key1", "value1".to_string()).await?;
    assert_eq!(cache.get("key1").await?, Some("value1".to_string()));

    assert!(cache.exists("key1").await?);
    assert!(!cache.exists("nonexistent").await?);

    assert!(cache.delete("key1").await?);
    assert_eq!(cache.get("key1").await?, None);

    // Delay simulation (timeout test scenario)
    let slow_cache = MockCache::with_delay(100); // 100ms delay
    Ok(())
}
```

#### MockConfig Usage Example

```rust
use inklog::MockConfig;

#[test]
fn test_config_operations() {
    let config = MockConfig::new()
        .with_value("level", "debug")
        .with_value("port", "8080");

    assert_eq!(config.get_string("level"), Some("debug".to_string()));
    assert_eq!(config.get_int("port"), Some(8080));

    // Modify config at runtime (dynamic config test scenario)
    config.set("level", "error");
    assert_eq!(config.get_string("level"), Some("error".to_string()));
}
```

#### MockDatabaseAdapter Usage Example

```rust
use inklog::{LogRecord, MockDatabaseAdapter};

#[tokio::test]
async fn test_database_operations() {
    let db = MockDatabaseAdapter::new();

    let records = vec![
        LogRecord::new(tracing::Level::INFO, "app".to_string(), "Test message 1".to_string()),
        LogRecord::new(tracing::Level::ERROR, "app".to_string(), "Test message 2".to_string()),
    ];

    // Bulk insert
    let count = db.insert_batch(&records).await.unwrap();
    assert_eq!(count, 2);

    // Health status control (degradation test scenario)
    db.set_healthy(false);
    assert!(!db.is_healthy().await);

    // Clear records (test isolation)
    db.clear();
    assert_eq!(db.record_count(), 0);
}
```

#### Test Isolation Best Practices

```rust
use inklog::{LoggerDependencies, LoggerManager, MockCache, MockConfig, MockDatabaseAdapter};
use std::sync::Arc;

struct TestContext {
    database: Arc<MockDatabaseAdapter>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            database: Arc::new(MockDatabaseAdapter::new()),
        }
    }

    async fn create_logger(&self) -> Result<LoggerManager, inklog::InklogError> {
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: Some(Arc::new(MockConfig::new())),
            custom_sinks: Vec::new(), // Required field
            database: Some(self.database.clone()),
        };
        // Deprecated entry point (since 0.3.0): dependency-injection demo for isolated tests only; calls emit a deprecation warning;
        // production code should use LoggerManager::builder() instead
        LoggerManager::with_dependencies(deps).await
    }

    fn reset(&self) {
        self.database.clear(); // Clear the database
    }
}

#[tokio::test]
async fn test_isolated() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TestContext::new();
    let _logger = ctx.create_logger().await?;

    // Run the test...

    ctx.reset(); // Reset state without affecting other tests
    Ok(())
}
```

## 🌍 Environment Variables

inklog supports overriding configuration via `INKLOG_*` environment variables (higher priority than the configuration file). Environment variable overrides apply only when loading via `InklogConfig::load_with_env_overrides()`; `LoggerManager::new()` and `LoggerManager::load()` only go through `load_sync()` to read the configuration file and do not apply the overrides listed in this section.

### Global Configuration

| Environment variable | Description | Example |
|----------|------|--------|
| `INKLOG_GLOBAL_LEVEL` | log level | `INKLOG_GLOBAL_LEVEL=debug` |
| `INKLOG_GLOBAL_FORMAT` | log format | `INKLOG_GLOBAL_FORMAT="{timestamp} {message}"` |
| `INKLOG_GLOBAL_MASKING_ENABLED` | enable data masking | `INKLOG_GLOBAL_MASKING_ENABLED=true` |
| `INKLOG_GLOBAL_AUTO_FALLBACK` | enable automatic fallback | `INKLOG_GLOBAL_AUTO_FALLBACK=true` |
| `INKLOG_GLOBAL_SERVICE_NAME` | service identity: service name (the static field `service_name` injected into every record) | `INKLOG_GLOBAL_SERVICE_NAME=orders` |
| `INKLOG_GLOBAL_SERVICE_INSTANCE` | service identity: instance identifier (field `service_instance`) | `INKLOG_GLOBAL_SERVICE_INSTANCE=orders-7f3a` |
| `INKLOG_GLOBAL_SERVICE_ENV` | service identity: deployment environment (field `service_env`) | `INKLOG_GLOBAL_SERVICE_ENV=prod` |
| `INKLOG_GLOBAL_SERVICE_VERSION` | service identity: service version (field `service_version`) | `INKLOG_GLOBAL_SERVICE_VERSION=1.2.3` |
| `INKLOG_GLOBAL_STATIC_FIELDS` | additional static key-values (comma-separated `key=value`, injected into every record; values must not contain `,`, unparseable segments are skipped with a warning) | `INKLOG_GLOBAL_STATIC_FIELDS=region=cn-north-1,az=a` |
| `INKLOG_LOCALE` | message localization language (zh-CN / en) | `INKLOG_LOCALE=en` |

> Note: `service_name`/`service_instance`/`service_env`/`service_version` are reserved field keys; applications should avoid naming business fields with the `service_` prefix (an event carrying an explicitly same-named field overrides the static identity and affects log attribution).

### File Sink

| Environment variable | Description | Example |
|----------|------|--------|
| `INKLOG_FILE_SINK_ENABLED` | enable the file sink | `INKLOG_FILE_SINK_ENABLED=true` |
| `INKLOG_FILE_SINK_PATH` | log file path | `INKLOG_FILE_SINK_PATH=logs/app.log` |
| `INKLOG_FILE_SINK_MAX_SIZE` | maximum file size | `INKLOG_FILE_SINK_MAX_SIZE=100MB` |

### Database Sink

| Environment variable | Description | Example |
|----------|------|--------|
| `INKLOG_DATABASE_SINK_URL` | database connection URL | `INKLOG_DATABASE_SINK_URL=postgres://localhost/logs` |
| `INKLOG_DATABASE_SINK_POOL_SIZE` | connection pool size | `INKLOG_DATABASE_SINK_POOL_SIZE=10` |
| `INKLOG_DATABASE_SINK_BATCH_SIZE` | batch size | `INKLOG_DATABASE_SINK_BATCH_SIZE=100` |
| `INKLOG_DATABASE_SINK_FLUSH_INTERVAL_MS` | flush interval (milliseconds) | `INKLOG_DATABASE_SINK_FLUSH_INTERVAL_MS=1000` |
| `INKLOG_DATABASE_SINK_TABLE_NAME` | log table name | `INKLOG_DATABASE_SINK_TABLE_NAME=logs` |

### HTTP Server

| Environment variable | Description | Example |
|----------|------|--------|
| `INKLOG_HTTP_SERVER_ENABLED` | enable the HTTP server | `INKLOG_HTTP_SERVER_ENABLED=true` |
| `INKLOG_HTTP_SERVER_HOST` | listening host | `INKLOG_HTTP_SERVER_HOST=0.0.0.0` |
| `INKLOG_HTTP_SERVER_PORT` | listening port | `INKLOG_HTTP_SERVER_PORT=8080` |
| `INKLOG_HTTP_SERVER_METRICS_PATH` | metrics endpoint path | `INKLOG_HTTP_SERVER_METRICS_PATH=/metrics` |
| `INKLOG_HTTP_SERVER_HEALTH_PATH` | health check path | `INKLOG_HTTP_SERVER_HEALTH_PATH=/health` |
| `INKLOG_HTTP_SERVER_ERROR_MODE` | error handling mode | `INKLOG_HTTP_SERVER_ERROR_MODE=strict` |

### Performance Configuration

| Environment variable | Description | Example |
|----------|------|--------|
| `INKLOG_PERFORMANCE_CHANNEL_CAPACITY` | channel capacity | `INKLOG_PERFORMANCE_CHANNEL_CAPACITY=10000` |
| `INKLOG_PERFORMANCE_WORKER_THREADS` | number of worker threads | `INKLOG_PERFORMANCE_WORKER_THREADS=3` |

### Others

| Environment variable | Description |
|----------|------|
| `INKLOG_CONFIG_PATH` | configuration file path (highest lookup priority for `LoggerManager::load()`) |
| `INKLOG_ENCRYPTION_KEY` | encryption key (Base64 or raw 32 bytes) |
| `INKLOG_DECRYPT_KEY` | CLI decryption key (the default variable name for `inklog-cli decrypt --key-env`) |

## ✅ Best Practices

### Choosing Log Levels

| Level | Use case | Example |
|------|----------|--------|
| `trace` | detailed execution tracing | function entry/exit, variable values |
| `debug` | debugging information | intermediate variables, logic branches |
| `info` | routine information | application startup, user actions |
| `warn` | warning conditions | falling back to defaults, degraded functionality |
| `error` | error conditions | exceptions, failed operations |

### Formatting Recommendations

| Environment | Recommended format |
|------|----------|
| Production | `{timestamp} [{level}] {target} - {message}` |
| Development | `[{timestamp}] [{level}] {target} - {message} \| {file}:{line}` |
| Structured collection | `output_format = "Json"` (NDJSON) |

### File Rotation Strategies

| Scenario | Recommended configuration |
|------|----------|
| High-frequency logs | `max_size = "50MB"`, `rotation_time = "hourly"`, `keep_files = 24` |
| Regular logs | `max_size = "100MB"`, `rotation_time = "daily"`, `keep_files = 30` |
| Low-frequency logs | `max_size = "500MB"`, `rotation_time = "weekly"`, `keep_files = 12` |

### Performance Optimization

- **High-throughput scenarios**: `channel_capacity = 50000`, `worker_threads = 8`; rotation compression can be turned off (`compress = false`);
- **Balanced configuration (recommended)**: `channel_capacity = 10000`, `worker_threads = 4`, `compress = true`;
- **Low-latency scenarios**: reduce `channel_capacity` (e.g. 1000) to shorten queue residence time;
- **Enable masking on demand**: regex masking is the most expensive security step on the main path (about 160x template rendering); enable it only on sinks that need it — quantitative evidence in the [⚡ Performance Baseline](PERFORMANCE.md).

### Security Best Practices

1. **Never hard-code keys in code**; inject via environment variables or KMS (the `kms` feature);
2. **Enable data masking** (`masking_enabled = true`, on by default);
3. **Enable rotated-archive encryption** (`encrypt = true` + `encryption_key_env`);
4. **Restrict log file permissions** (inklog sets `0600` on encrypted files automatically; directories should be `0700`).

For the full security design, see the [🔒 Security Documentation](SECURITY.md).

## 🔧 Troubleshooting

### No Log Output

**Problem**: logs are not output anywhere.

**Check**:

1. The log level is set too high (e.g. global `error` while emitting `info`);
2. The sink in question is not enabled (e.g. `file_sink: None`);
3. The host application installed the global subscriber first (inklog detects this and skips with a fallback; logs flow to the already-installed subscriber).

```rust
use inklog::{InklogConfig, config::GlobalConfig};

let config = InklogConfig {
    global: GlobalConfig {
        level: "debug".into(),  // Lower the level
        ..Default::default()
    },
    console_sink: Some(inklog::config::ConsoleSinkConfig {
        enabled: true,  // Make sure the console sink is enabled
        ..Default::default()
    }),
    ..Default::default()
};
```

### File Rotation Not Working

**Problem**: log files are not rotating.

**Check**:

1. `max_size` is too large and the threshold is never reached;
2. the `rotation_time` interval is too long;
3. insufficient disk space triggers the protection logic.

```rust
let file_config = inklog::FileSinkConfig {
    enabled: true,
    path: "logs/app.log".into(),
    max_size: "10MB".into(),        // Lower the threshold
    rotation_time: "hourly".into(), // Shorten the interval
    keep_files: 10,
    ..Default::default()
};
```

### Database Connection Failure

**Problem**: the database sink cannot connect.

**Check**:

1. The database URL is wrong (verify with local SQLite first: `sqlite://logs/app.db`);
2. the database service is not running or the network is unreachable;
3. authentication fails;
4. multiple database backend features are enabled at once (mutually exclusive; compile/run behavior becomes anomalous).

```rust
let database_sink = inklog::DatabaseSinkConfig {
    enabled: true,
    url: "sqlite://logs/app.db".to_string(), // Use a local database for troubleshooting
    pool_size: 5,
    batch_size: 10,
    flush_interval_ms: 1000,
    ..Default::default()
};
```

### Encrypted Logs Cannot Be Read

**Problem**: encrypted archives fail to decrypt.

**Check**:

1. The key is lost or mismatched (the v2 format reads the salt from the file header for deterministic derivation; the key must match the one used for encryption);
2. the key format is wrong (must be exactly 32 bytes after Base64 decoding, or raw 32 bytes);
3. "password mode" files from the v1 era cannot be recovered because no salt was stored — migrate to v2.

```bash
# Decrypt with the CLI tool
inklog-cli decrypt \
  --input logs/app_20260913_143022.log.zst.enc \
  --output decrypted/ \
  --key-env INKLOG_DECRYPT_KEY
```

## ❓ FAQ

### Is inklog compatible with the standard `log` crate?

Yes. inklog ships a built-in `LogLogger` bridge; standard macros like `log::info!` and `log::error!` work alongside the `tracing` macros.

### How do I output to both file and console?

`console_sink` is enabled by default; just configure `file_sink` as well — see [Multi-Target Output](#multi-target-output) for an example.

### How do I disable data masking?

```rust
let config = InklogConfig {
    global: inklog::config::GlobalConfig {
        masking_enabled: false, // Disable masking
        ..Default::default()
    },
    ..Default::default()
};
```

### Can encryption and compression be used together?

Yes. Rotated archives are processed **compressed first (`.zst`), then encrypted (`.enc`)**; both apply to rotated files. The active log file stays in plaintext (by design, to keep real-time write performance).

### How do I change the log format?

```rust
let config = InklogConfig {
    global: inklog::config::GlobalConfig {
        format: "[{timestamp}] {level}: {message}".to_string(),
        ..Default::default()
    },
    ..Default::default()
};
```

Available template variables are listed in the "Template Variables" table (including `{trace_id}` / `{span_id}`).

### How do I adjust the log level at runtime without restarting?

Use `LoggerManager::set_level` (see the "Runtime Level Hot-Reload" section); both global and per-target levels take effect immediately.

### How do I search local log files?

Use `inklog-cli query` (the `cli` feature): search by time, level, and keyword, with decryption and unpacking handled automatically:

```bash
inklog-cli query --path logs/ --level error --since 2026-09-13T00:00:00Z --grep "payment"
```

## 🔗 Related Resources

### Project Documentation

| Document | Description |
|------|------|
| [📘 API Reference](API_REFERENCE.md) | item-by-item reference of core types, configuration structs, and traits |
| [🏗️ Architecture](ARCHITECTURE.md) | layered design, data flow, and the concurrency model |
| [⚡ Performance Baseline](PERFORMANCE.md) | criterion benchmark environment and baseline numbers |
| [🧪 Test Scenarios](TEST_SCENARIOS.md) | test pyramid and E2E scenario definitions |
| [🔒 Security Documentation](SECURITY.md) | security design, vulnerability reporting, and compliance |
| [📋 Changelog](../CHANGELOG.md) | version change history |
| [📦 Online API Docs](https://docs.rs/inklog) | the latest documentation auto-generated on docs.rs |

### Ecosystem

- [tracing](https://github.com/tokio-rs/tracing): structured logging and the Subscriber ecosystem
- [Tokio](https://tokio.rs/): the async runtime
- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam): high-performance bounded channels
- [Axum](https://github.com/tokio-rs/axum): HTTP endpoints

### Community Support

- [GitHub Issues](https://github.com/Kirky-X/inklog/issues): report bugs and issues
- [GitHub Discussions](https://github.com/Kirky-X/inklog/discussions): ask questions and share ideas

## 🧭 Core Concepts

1. **Initialization**: `LoggerManager` installs the global tracing subscriber and the `log` crate frontend with process-wide singleton semantics (`init_inklog_logger()` is the convenience entry point).
2. **Recording**: business code uses standard macros such as `tracing::info!` with zero intrusion.
3. **Sinks**: the output abstraction (`LogSink` / `AsyncSink` traits) with built-in console / file / database / net / otlp implementations, extensible with your own.
4. **Configuration**: `InklogConfig` supports TOML files; `INKLOG_*` environment variable overrides apply only when loading via `InklogConfig::load_with_env_overrides()` (precedence: environment > file > defaults). `LoggerManager::new()` / `LoggerManager::load()` go through `load_sync()` and do not apply env overrides.
5. **Shutdown**: call `shutdown()` before exit so remaining records in the channel are flushed.

## 🎨 Feature Flags

`default = ["i18n", "cache"]`: the default build contains core capabilities plus internationalization and the cache backend; every other feature below must be enabled explicitly (per the `Cargo.toml` `[features]` definition). Opting out via `default-features = false` disables both i18n and cache, which prunes the whole icu/fluent dependency tree (oxcache's icu/fluent deps are not optional), with `tr`/`tr_args` falling back to the embedded English message table.

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
| `fast-masking` | ❌ | Deprecated no-op compatibility alias: Aho-Corasick acceleration ships always-on with trait-kit/masking; the feature name is kept only for existing `--features` usage |
| `secret-scan` | ❌ | Secret outbound masking gate: value-shape pattern registry (bare `sk-`/PEM/AKIA secrets) + value entropy scanning + per-pattern counting with fail-closed limit (`mask_checked`) + Chinese sensitive key names |
| `dbnexus-audit` | ❌ | dbnexus AuditStorage port adapter; audit events persisted through the inklog DB sink, combinable with any backend |
| `config-confers` | ❌ | Configuration loaded via confers + watch hot-reload of level and rotation parameters; confers 0.6 is a non-optional core dependency (the `INKLOG_*` env override mapping goes through the confers source chain), this feature additionally enables confers' `toml`/`watch` features (kept for compatibility) |
| `confers-audit` | ❌ | inklog implementation of the confers `AuditSink` port; configuration audit events are written into the inklog structured sink; confers is a non-optional core dependency, this feature additionally enables confers' `audit` feature |
| `oxcache-audit` | ❌ | inklog implementation of the oxcache `AuditEventPublisher` port (`OxcacheAuditPublisher`): cache audit events are handed to a single writer task over a bounded channel and written into the injected sink |
| Rate limiting | ✅ | The rate-based and per-target quota limiting modules (`RateLimiter` / `TargetRateLimiter`) and the sink-level rate-limit decorator (`RateLimitedSink`) work in the default build: the kernel is the in-crate token bucket (capacity = rate, refilled at the rate per second), and sink-level burst quota uses `BurstLimiter`. inklog depends on no external rate-limiting library; extensions go through the crate's own `SinkRateLimit` port |
| `kms` | ❌ | KMS key providers (`EnvKeyProvider` / `ConfersKeyProvider` / Vault transit MVP); confers is a non-optional core dependency, this feature additionally enables confers' `encryption`/`cloud-kms` features |
| `net-sink` | ❌ | Network forwarding sinks (TCP with optional TLS + UDP, buffered reconnection) |
| `insecure-tls` | ❌ | Allows disabling TLS certificate verification (`danger_accept_invalid_certs`) in **release builds**; off by default — without certificate verification the link is encrypted but unauthenticated, so a man-in-the-middle can read and tamper with the exported logs. Debug builds (`debug_assertions`) may use the switch without this feature |
| `otlp` | ❌ | OTLP/HTTP JSON log export MVP (hand-written transport, zero new dependencies) |
| `otel` | ❌ | Real OpenTelemetry trace context: reads the SpanContext from span extensions (written by the host's tracing-opentelemetry layer); the host otel version must match inklog's (currently opentelemetry 0.33, paired with tracing-opentelemetry 0.34), otherwise it silently falls back to root-span derivation |
| `metrics-registry` | ❌ | General-purpose business metrics registry (CounterVec / GaugeVec / HistogramVec + Prometheus text export) |
| `test-utils` | ❌ | Test-facing mock exports (`MockCache` / `MockConfig` / `MockDatabaseAdapter`); excluded from default and all production combinations |

> ⚠️ **Database backend exclusivity**: `sqlite` / `postgres` / `mysql` / `duckdb` are mutually exclusive (enforced via dbnexus; embedded and server-side drivers must not be mixed). `--all-features` is not supported; enable features grouped by backend instead.
>
> 🚫 **Do not enable the `duckdb` feature** (since 2026-10-16): the upstream `zip` crate has been fully yanked on crates.io (only 9.0.3 remains). The `duckdb` chain resolves through `libduckdb-sys` (pinned to `zip ^6`), which can no longer be resolved — enabling this feature from scratch fails at dependency resolution. Existing projects with a committed `Cargo.lock` are unaffected. This will be lifted once duckdb-rs upstream publishes a release adapted to zip 9.x; until then use the `sqlite` / `postgres` / `mysql` backends instead.

## 💻 Complete Example List

[`examples/`](../../examples/) is a standalone workspace crate (`inklog-examples`) with 40 examples in 7 categories. Run from the repository root:

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
| `rate_limiter` | Rate limiter (Layer 0, zero dependencies) | default (no feature needed) |
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

**[📂 Browse all examples →](../../examples/)**

</div>

## 🗺️ Roadmap

Phased goals compiled from the existing release plan (timing may adjust with the overall workspace release schedule):

| Status | Goal | Notes |
|:----:|------|------|
| ✅ | v0.3.0 stable release | The 0.3.0 stable release is published (2026-11-14) |
| ✅ | Workspace dependency lockstep | trait-kit 0.5, oxcache 0.5, dbnexus 0.6 (all stable, caret ranges in Cargo.toml) |
| ✅ | CI test matrix grouped by database backend | The Docker pipeline validates the four mutually exclusive backends (sqlite/postgres/mysql/duckdb); duckdb is embedded and needs no container service |
| ✅ | Provision the MySQL integration environment | A MySQL 8 service is provided in docker/docker-compose.test.yml; the CI mysql branch runs the integration tests |
| ✅ | Raise test coverage | llvm-cov CI-scope line coverage 95.79% (re-measured locally 2026-11-16), gate raised in stages 80→85→90→95 to 95% |

---

**[⬆ Back to Top](#-inklog-user-guide)**
