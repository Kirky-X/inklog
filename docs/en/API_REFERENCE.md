# 📘 inklog API Reference

This document is an item-by-item reference of the public API across inklog's `0.3.x` release line: core types, configuration structs, error types, health monitoring, the sink family, dependency-injection traits, and test mocks. For the usage tutorial see the [📖 User Guide](USER_GUIDE.md); for internal design see the [🏗️ Architecture](ARCHITECTURE.md).

<details open>
<summary>📑 Contents</summary>

- [📭 Overview](#-overview)
- [🚀 Top-Level Convenience Functions](#-top-level-convenience-functions)
- [🧩 Core Types](#-core-types)
- [⚙️ Configuration Structs](#️-configuration-structs)
- [⚠️ Error Types](#️-error-types)
- [🩺 Health Monitoring Types](#-health-monitoring-types)
- [🧾 Log Record Types](#-log-record-types)
- [🔌 Sink Types](#-sink-types)
- [🧪 Dependency Injection Types](#-dependency-injection-types)
- [🎭 Mock Implementations (Testing)](#-mock-implementations-testing)
- [🏭 Adapter Implementations](#-adapter-implementations)
- [💡 Example Reference](#-example-reference)

</details>

---

## 📭 Overview

### Public API Overview

| Category | Type | Description |
|------|------|------|
| Core | `LoggerManager` | the core log manager, coordinating log collection and routing |
| Core | `LoggerBuilder` | fluent builder for creating managers |
| Core | `LoggerDependencies` | dependency-injection container |
| Core | `InklogContainer` / `InklogContainerBuilder` | DI container |
| Configuration | `InklogConfig` and sub-configs | root configuration and per-sink configurations |
| Errors | `InklogError` / `InklogResult` | error enum and result alias |
| Health | `HealthStatus` / `Metrics` / `SinkHealthMonitor` | health status and metrics |
| Sinks | `LogSink` / `AsyncSink` and built-in implementations | output-target abstraction and implementations |
| Records | `LogRecord` | the log record structure |
| Injection | `Cache` / `Config` / `Database` traits | infrastructure abstractions |
| Adapters | `OxCacheAdapter` / `InklogConfigAdapter` / `DbNexusAdapter` | production adapters |
| Testing | `MockCache` / `MockConfig` / `MockDatabaseAdapter` | test mocks (the `test-utils` feature) |

### Importing the Public API

```rust
use inklog::{
    // Core types
    LoggerManager,
    LoggerBuilder,
    LoggerDependencies,
    InklogConfig,
    LogRecord,
    LogLevel,

    // Configuration types
    config::{
        GlobalConfig,
        ConsoleSinkConfig,
        FileSinkConfig,
        DatabaseSinkConfig,
        DatabaseDriver,
        HttpServerConfig,
        PerformanceConfig,
        ParquetConfig,
    },

    // Error types
    InklogError,
    InklogResult,

    // Health monitoring
    HealthStatus,
    Metrics,
    SinkStatus,
    SinkHealth,

    // Sink abstractions
    LogSink,
    AsyncSink,
};
```

## 🚀 Top-Level Convenience Functions

### `init_inklog_logger`

Initializes with the default configuration and installs the global log frontend, with process-wide singleton semantics (re-initialization returns a clear error).

**Signature**

```rust
pub async fn init_inklog_logger() -> Result<(), InklogError>
```

**Example**

```rust
inklog::init_inklog_logger().await?;
tracing::info!("Hello, inklog!");
```

### `init_inklog_logger_with_config`

Initializes with a custom configuration; semantics match [`init_inklog_logger`](#init_inklog_logger).

**Signature**

```rust
pub async fn init_inklog_logger_with_config(config: InklogConfig) -> Result<(), InklogError>
```

## 🧩 Core Types

### LoggerManager

The core log manager, coordinating log collection, masking, dispatch, and routing to the sinks, and responsible for installing the global tracing subscriber and the `log` crate frontend.

#### Methods

##### `new`

Creates a `LoggerManager` with the default configuration and installs the global frontend.

```rust
pub async fn new() -> Result<Self, InklogError>
```

##### `with_config`

Creates a `LoggerManager` with the given configuration and installs the global frontend.

```rust
pub async fn with_config(config: InklogConfig) -> Result<Self, InklogError>
```

**Example**

```rust
use inklog::{FileSinkConfig, InklogConfig};

let config = InklogConfig {
    file_sink: Some(FileSinkConfig {
        enabled: true,
        path: "logs/app.log".into(),
        ..Default::default()
    }),
    ..Default::default()
};

let logger = LoggerManager::with_config(config).await?;
```

##### `with_dependencies`

Creates a `LoggerManager` with dependency injection.

```rust
pub async fn with_dependencies(deps: LoggerDependencies) -> Result<Self, InklogError>
```

> **⚠️ Deprecated** (`#[deprecated(since = "0.3.0", note = "use LoggerManager::builder() instead")]`): the DI entry point has been converged; this method is kept only for backward compatibility and simply forwards to the internal build logic. New code should use [`builder()`](#builder) (the `LoggerBuilder` supports third-party sink registration such as `add_sink`); if you must write a `LoggerDependencies` literal by hand, note that `custom_sinks` is a **required** field (use `Vec::new()` when there are no third-party sinks).

##### `builder`

Creates a `LoggerBuilder` instance.

```rust
pub fn builder() -> LoggerBuilder
```

##### `from_file`

Loads the TOML configuration from the given path and initializes.

```rust
pub async fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, InklogError>
```

##### `load`

Loads the configuration file from the default locations and initializes. Lookup precedence (highest to lowest):

1. the path given by `$INKLOG_CONFIG_PATH`;
2. `./inklog_config.toml` (current directory);
3. `~/.config/inklog/config.toml` (user configuration directory);
4. the system configuration path (Unix: `/etc/inklog/config.toml`).

```rust
pub async fn load() -> Result<Self, InklogError>
```

##### `build_detached`

Builds a `LoggerManager` without installing the global subscriber, returning the manager/subscriber/EnvFilter triple so tests and benchmarks can assemble it themselves (e.g. thread-level `set_default`).

```rust
pub async fn build_detached(
    config: InklogConfig,
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    database: Option<Arc<dyn Database>>,
) -> Result<(Self, LoggerSubscriber, tracing_subscriber::filter::EnvFilter), InklogError>
```

A `build_detached_with_sinks` variant can also register dynamic third-party sinks.

##### `set_level`

Runtime level hot-reload: swaps the EnvFilter via `tracing_subscriber::reload`, taking effect immediately in-process; supports global and per-target upserts, and additional `RUST_LOG` directives survive rebuilds.

```rust
pub fn set_level(&self, target: Option<&str>, level: &str) -> Result<(), InklogError>
```

| Parameter | Description |
|------|------|
| `target` | `None` adjusts the global level; `Some("hyper")` adjusts a specific target |
| `level` | target level (`trace` / `debug` / `info` / `warn` / `error`) |

##### `get_health_status`

Returns a snapshot of the current health status.

```rust
pub fn get_health_status(&self) -> HealthStatus
```

##### `recover_sink`

Sends a recovery command to the named sink (re-initializes and resets the circuit breaker).

```rust
pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError>
```

##### `trigger_recovery_for_unhealthy_sinks`

Recovers all unhealthy sinks and returns the list of recovered sink names.

```rust
pub fn trigger_recovery_for_unhealthy_sinks(&self) -> Result<Vec<String>, InklogError>
```

##### `effective_channel_capacity` / `channel_len`

Queries the effective channel capacity and the current backlog count.

```rust
pub fn effective_channel_capacity(&self) -> usize
pub fn channel_len(&self) -> usize
```

##### `publish_ops_event`

Broadcasts an internal ops event to all sink channels.

```rust
pub fn publish_ops_event(&self, kind: &str, sink: Option<&str>, detail: serde_json::Value)
```

##### `current_level_filter_string`

Returns the currently effective EnvFilter directive string.

```rust
pub fn current_level_filter_string(&self) -> String
```

##### `cache` / `database`

Accesses the injected cache / database implementations (returns `None` when not injected).

```rust
pub fn cache(&self) -> Option<Arc<dyn Cache>>
pub fn database(&self) -> Option<Arc<dyn Database>>
```

##### `shutdown`

Graceful shutdown: uses a 2-second timeout send (`send_timeout`) for the stop signal and keeps polling worker handles after timeout, so it never hangs; waits for remaining records in the channel to drain, then shuts down all sinks.

```rust
pub fn shutdown(&self) -> Result<(), InklogError>
```

### LoggerBuilder

A fluent builder for creating `LoggerManager`s. Invalid parameters are deferred to `build()`, which uniformly reports `ConfigError` (the builder returns a `Result`).

#### Methods

##### Basic Configuration

| Method | Signature | Description |
|------|------|------|
| `new` | `pub fn new() -> Self` | create the builder |
| `level` | `pub fn level(self, level: impl Into<String>) -> Self` | global log level |
| `format` | `pub fn format(self, format: impl Into<String>) -> Self` | format template |
| `console` | `pub fn console(self, enabled: bool) -> Self` | enable/disable the console sink |
| `file` | `pub fn file(self, path: impl Into<PathBuf>) -> Self` | enable the file sink and set its path |
| `database` | `pub fn database(self, url: impl Into<String>) -> Self` | enable the database sink and set its URL |
| `channel_capacity` | `pub fn channel_capacity(self, capacity: usize) -> Self` | channel capacity |
| `worker_threads` | `pub fn worker_threads(self, threads: usize) -> Self` | number of worker threads |

##### Database Refinements (require a database backend feature)

| Method | Signature | Description |
|------|------|------|
| `with_driver` | `pub fn with_driver(self, driver: DatabaseDriver) -> Self` | database driver |
| `with_pool_size` | `pub fn with_pool_size(self, pool_size: u32) -> Self` | connection pool size |
| `with_batch_size` | `pub fn with_batch_size(self, batch_size: usize) -> Self` | batch size |
| `with_flush_interval_ms` | `pub fn with_flush_interval_ms(self, flush_interval_ms: u64) -> Self` | flush interval |
| `with_table_name` | `pub fn with_table_name(self, table_name: impl Into<String>) -> Self` | log table name |
| `with_admin_role` | `pub fn with_admin_role(self, admin_role: impl Into<String>) -> Self` | admin role name |

##### File Refinements

| Method | Signature | Description |
|------|------|------|
| `file_max_size` | `pub fn file_max_size(self, max_size: impl Into<String>) -> Self` | maximum size per file (e.g. `"50MB"`) |
| `file_compress` | `pub fn file_compress(self, compress: bool) -> Self` | whether to compress rotated files |
| `file_rotation_time` | `pub fn file_rotation_time(self, rotation: impl Into<String>) -> Self` | time rotation strategy |
| `file_keep_files` | `pub fn file_keep_files(self, keep: u32) -> Self` | number of rotated files to keep |

##### Console Refinements

| Method | Signature | Description |
|------|------|------|
| `console_colored` | `pub fn console_colored(self, colored: bool) -> Self` | whether to colorize output |
| `console_stderr_levels` | `pub fn console_stderr_levels(self, levels: &[&str]) -> Self` | levels routed to stderr (default `["error", "warn"]`) |

##### HTTP Server Refinements (require the `http` feature)

| Method | Signature | Description |
|------|------|------|
| `enable_http_server` | `pub fn enable_http_server(self, enabled: bool) -> Self` | enable HTTP endpoints |
| `http_host` / `http_port` | `pub fn http_host(self, host: impl Into<String>) -> Self` / `pub fn http_port(self, port: u16) -> Self` | listening address and port |
| `http_metrics_path` / `http_health_path` | `pub fn http_metrics_path(self, path: impl Into<String>) -> Self` etc. | endpoint paths |
| `http_error_mode` | `pub fn http_error_mode(self, mode: impl Into<String>) -> Self` | error mode (`"warn"` / `"strict"`) |

##### Dependency Injection and Dynamic Sinks

| Method | Signature | Description |
|------|------|------|
| `cache` | `pub fn cache(self, cache: Arc<dyn Cache>) -> Self` | inject a custom Cache |
| `config` | `pub fn config(self, config: Arc<dyn Config>) -> Self` | inject a custom Config |
| `with_database` | `pub fn with_database(self, database: Arc<dyn Database>) -> Self` | inject a custom Database (requires a database backend feature) |
| `add_sink` | `pub fn add_sink(self, sink: Arc<dyn AsyncSink>) -> Self` | register a dynamic third-party sink (one channel per sink) |

##### `build`

Builds and returns a `LoggerManager` (installing the global frontend).

```rust
pub async fn build(self) -> Result<LoggerManager, InklogError>
```

**Example**

```rust
use inklog::LoggerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = LoggerBuilder::new()
        .level("debug")
        .file("logs/app.log")
        .channel_capacity(20000)
        .worker_threads(4)
        .build()
        .await?;
    Ok(())
}
```

## ⚙️ Configuration Structs

### InklogConfig

The root configuration struct, supporting TOML files; `INKLOG_*` environment variable overrides take effect via `InklogConfig::load_with_env_overrides()` (`load_sync()` only reads the configuration file and does not apply env overrides).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InklogConfig {
    pub global: GlobalConfig,
    pub console_sink: Option<ConsoleSinkConfig>,   // defaults to Some(default())
    pub file_sink: Option<FileSinkConfig>,
    pub database_sink: Option<DatabaseSinkConfig>,
    pub performance: PerformanceConfig,
    pub http_server: Option<HttpServerConfig>,
    pub target_levels: HashMap<String, String>,    // per-target level presets
    pub sampling: SamplingConfig,                  // sampling policy (default empty = builtin fallback)
    pub rate_limit: RateLimitConfig,               // per-target prefix quota groups (default empty = not wired)
}
```

> Entries in `target_levels` merge into the EnvFilter: lower precedence than `RUST_LOG`, higher than the global default level.
>
> `sampling` configures the retention rules under rate-limit pressure: `per_level` (level → N-of-1 sampling rate) and
> `per_target_prefix` (target prefix → `{ keep_level, sample_every_n }`, longest prefix
> first). When unconfigured, the built-in fallback applies (drop non-critical levels, keep 1/100 of ERROR/FATAL); records
> that hit no rule fall back to the built-in fallback too.
>
> `rate_limit.rules` configures quota-based rate limiting grouped by target prefix: prefix → the group's
> tokens per second (a shared budget within the group, longest prefix first, ASCII case-insensitive). Records hitting a group are adjudicated
> independently by the group bucket (once admitted they no longer enter the global `performance.rate_limit` limit), while non-matching targets keep
> the existing global path. Group-budget exhaustion and global rate-limit rejection share the same pressure semantics: ERROR/FATAL are
> kept by 1-in-100 sampling (the `sampling` policy takes precedence), and non-critical levels count as `logs_dropped`.
> When unconfigured, the whole path is not wired and global behavior is unchanged.
>
> ✅ Rate limiting works in the default build: the limiting kernel of `performance.rate_limit` and
> `rate_limit.rules` (token buckets by rate / by target prefix group) ships with the crate;
> when the feature is off, the configuration is still accepted but inactive, and an explicit warning is printed at startup.
>
> Operations note: group quotas should be significantly lower than the global `performance.rate_limit` budget — an oversized group
> quota effectively exempts that prefix from global rate limiting; keep the rule count ≤ 64 (lookup is a per-record linear scan).
> Configuration JSON Schema (the `schema` feature): `InklogConfig::json_schema()` exports programmatically
> (regenerated on every call, O(schema) allocations; embedders may cache it themselves), and `inklog-cli generate
> --schema [--output <file>]` writes the file (default `config_schema.json`, committed and protected against drift by
> a drift test and CI `git diff --exit-code`); `SinkMetadata.config_schema` is filled by the built-in
> `FileSinkFactory` with the real schema of `FileSinkConfig` when the `schema` feature is enabled
> (process-level cache); in all other cases (feature off, third-party factories not providing it) it is `null`.
>
> The Schema omits serde alias key names: `masking_enabled` on the `file/console` sinks
> (alias `pii_masking_enabled`) and `global.sanitizer_enabled` (alias
> `sanitizer_enabled`) appear in `properties` only under their primary names; the aliases are mentioned only in the field description
> text — for machine validation against the schema, only the primary names are legal keys; alias deserialization compatibility
> is still guaranteed by the loader (schema validation and the deserialization acceptance domain are defined by the primary names).


#### Methods

| Method | Signature | Description |
|------|------|------|
| `validate` | `pub fn validate(&self) -> Result<(), InklogError>` | validates the configuration (level validity, non-empty paths, etc.) |
| `sinks_enabled` | `pub fn sinks_enabled(&self) -> Vec<&'static str>` | queries the names of enabled sinks |

### GlobalConfig

```rust
pub struct GlobalConfig {
    pub level: String,                    // default "info"
    pub format: String,                   // default "{timestamp} [{level}] {target} - {message}"
    pub masking_enabled: bool,            // default true (new name `sanitizer_enabled` recommended; the old name is a serde compatibility alias and serialization always emits the old name)
    pub auto_fallback: bool,              // default true
    pub fallback_initial_delay_ms: u64,   // default 1000
    pub fallback_max_delay_ms: u64,       // default 60000
    pub fallback_max_retries: u32,        // default 10
    pub output_format: OutputFormat,      // default Text
}
```

The `OutputFormat` enum: `Text` (template rendering), `Json` (NDJSON, colors disabled automatically).

### ConsoleSinkConfig

```rust
pub struct ConsoleSinkConfig {
    pub enabled: bool,               // default true
    pub colored: bool,               // default true (auto-disabled when NO_COLOR / TERM=dumb)
    pub stderr_levels: Vec<String>,  // default ["error", "warn"]
    pub masking_enabled: bool,       // default true (new name `pii_masking_enabled` recommended; the old name is a serde compatibility alias and serialization always emits the old name)
    pub secret_scan_enabled: bool,   // default false; requires the `secret-scan` feature and stacks with masking_enabled (if PII masking is off, no masking happens at all)
    pub output_format: OutputFormat, // default Text
}
```

### FileSinkConfig

```rust
pub struct FileSinkConfig {
    pub enabled: bool,                        // default true
    pub path: PathBuf,                        // default "logs/app.log"
    pub max_size: String,                     // default "100MB"
    pub rotation_time: String,                // default "daily" (hourly / daily / weekly)
    pub keep_files: u32,                      // default 30
    pub compress: bool,                       // default true
    pub compression_level: i32,               // default 3 (1-22)
    pub encrypt: bool,                        // default false
    pub encryption_key_env: Option<String>,   // default None
    pub encryption_key_file: Option<String>,  // default None; path to the encryption key file (takes effect when `encrypt` is on; an env setting overrides it) — unix permissions must be 0600, group/other-readable is explicitly rejected at key parsing time
    pub retention_days: u32,                  // default 30
    pub max_total_size: String,               // default "1GB"
    pub cleanup_interval_minutes: u64,        // default 60
    pub batch_size: usize,                    // default 100
    pub flush_interval_ms: u64,               // default 100
    pub fsync: bool,                          // default false; `sync_all` (fsync) after each batch write: crash-consistency hardening; off by default for throughput, recommended for compliance/audit scenarios
    pub audit_chain_enabled: bool,            // default false; archive audit chain (HMAC-SHA256 forward chain): after a successful rotation appends `{path, sha256, timestamp}` to the `<stem>.chain.jsonl` manifest and writes through the chain seal `<stem>.chain.seal.json` (total entry count + tail hash, to detect truncation); detects archive tampering/deletion/reordering/truncation; the key is read from `INKLOG_AUDIT_KEY`; when missing, the chain is disabled with a warning
    pub masking_enabled: bool,                // default true (new name `pii_masking_enabled` recommended; the old name is a serde compatibility alias)
    pub output_format: OutputFormat,          // default Text
}
```

> Compression requires the `compression` (Zstd) or `gzip` (flate2) feature; with neither enabled, rotated files stay uncompressed. Encryption and compression apply only to rotated archives (compress first, then encrypt); the active file stays in plaintext.

### ChannelBufferedConfig

The configuration input of `ChannelBufferedFileSink` (the channel-buffered high-throughput file sink, `ring_buffered_file`):

```rust
pub struct ChannelBufferedConfig {
    pub base_config: FileSinkConfig,                 // default FileSinkConfig::default() (rotation/compression/encryption/masking all inherit its semantics)
    pub channel_capacity: usize,                     // default 10_000 (bounded channel capacity between producers and the writer thread)
    pub backpressure_strategy: BackpressureStrategy, // default Block
    pub flush_batch_size: usize,                     // default 1000 (batching threshold)
    pub flush_interval_ms: u64,                      // default 100 (time-slice threshold)
}
```

`BackpressureStrategy` (`Debug/Clone/Copy/PartialEq/Eq/Default`, default `Block`) determines behavior when the channel fills:

| Variant | Behavior |
|------|------|
| `Block` | `write()` takes the async blocking path and waits for room (no logs lost, at the cost of slowing producers) |
| `DropNewest` | when `try_send` finds the channel full, the **newly arrived** entry is dropped and counted |
| `DropOldest` | when `try_send` finds the channel full, the **oldest** entry at the queue head is evicted first to make room; if there is still no room, the entry is counted as dropped |

Drop counts are exposed via `metrics() -> ChannelBufferedMetrics`; undelivered entries caused by an already-shut-down sink (disconnected channel) are **not counted as drops**. Under all three strategies, whenever an entry is not delivered, `LogSink::write()` returns `Err(InklogError::ChannelError)` (the message carries the current drop count) instead of silently succeeding.

### DatabaseSinkConfig

```rust
pub struct DatabaseSinkConfig {
    pub name: String,                     // default "default"
    pub enabled: bool,                    // default false
    pub driver: DatabaseDriver,           // default SQLite
    pub url: String,                      // default "sqlite::memory:"
    pub pool_size: u32,                   // default 10 (auto-set to 1 for SQLite)
    pub batch_size: usize,                // default 100
    pub flush_interval_ms: u64,           // default 500
    pub partition: PartitionStrategy,     // default Monthly
    pub table_name: String,               // default "logs"
    pub archive_format: ArchiveFormat,    // default Json
    pub parquet_config: ParquetConfig,    // default default()
    pub permissions_path: Option<String>, // default None (RBAC permission config)
    pub admin_role: String,               // default "admin"
    pub secret_scan_enabled: bool,        // default false; requires the `secret-scan` feature; ignored when not enabled (validation emits a warn)
}
```

> `secret_scan_enabled` (the outbound gate of the console / database built-in sinks): when on, bare credentials (`sk-` series, AWS AKIA, PEM blocks, etc.) are attributed and masked without needing a sensitive field name, effective before persistence/output. For declarative wiring and default values, see the [Security Documentation](SECURITY.md).

> **DSN masking and lossless export**: both `Debug` and `Serialize` mask the password in `url` as `***` (the configuration object is often printed or echoed as a whole). The serialized output is a **masked view**; a serialize→deserialize round trip loses the password; for lossless round trips (backup, migration, moving across environments) use `to_lossless_json()` / `to_lossless_toml()` — both outputs contain the original password, so exported files must be treated as credentials.
>
> **env overrides never swallow the password**: the env override in `load_with_env_overrides()` goes through a "serialize → patch → deserialize" round trip, which would replace the password with the masked placeholder — losing it from the configuration. That path has been rewritten to preserve the original DSN; a DSN supplied via `INKLOG_DATABASE_SINK_URL` takes effect with its original value.
>
> **The masked view cannot authenticate**: a DSN whose password slot is the placeholder (`***`) can never authenticate; both `DatabaseSinkConfig::validate()` and configuration-loading validation reject that form explicitly (a placeholder password passed via `INKLOG_DATABASE_SINK_URL` is ignored as an invalid value, with a warning).

#### DatabaseDriver

```rust
#[derive(Default)]
pub enum DatabaseDriver {
    PostgreSQL, // serde: "postgres"
    MySQL,      // serde: "mysql"
    #[default]
    SQLite,     // serde: "sqlite"
    DuckDB,     // serde: "duckdb"
}
```

Implements `FromStr` and `Display` (`"postgres"` / `"mysql"` / `"sqlite"` / `"duckdb"`, parsed case-insensitively); input outside the legal values returns a localized (i18n) error message.

The `DbNexusAdapter` driver is not parsed from this enum directly; it comes from dbnexus's `DbConfig::database_type()` (the same source dbnexus uses to pick the pool backend): the scheme is case-insensitive (`MYSQL://` and `mysql://` are synonyms), and for an unknown scheme or an unsupported backend (Ladybug/Neo4j), construction returns `Err(InklogError::ConfigError)` — no guessed semantics. The table-creation dialect and write escaping share that driver's source.

#### PartitionStrategy

| Variant | serde value | Description |
|------|----------|------|
| `Monthly` | `"monthly"` | partition by month (default) |
| `Yearly` | `"yearly"` | partition by year |

#### ArchiveFormat

| Variant | Description |
|------|------|
| `Json` | JSON archive (default) |
| `Parquet` | Parquet/Arrow archive (requires the `parquet` feature) |
| `Csv` | CSV archive |

#### ParquetConfig

| Field | Type | Default | Description |
|------|------|--------|------|
| `compression_level` | `i32` | `3` | Zstd compression level (1-22) |
| `encoding` | `String` | `"PLAIN"` | encoding: `PLAIN`, `DICTIONARY`, `RLE` |
| `max_row_group_size` | `usize` | `10000` | row-group size (rows) |
| `max_page_size` | `usize` | `1048576` | page size (bytes) |
| `include_fields` | `Vec<String>` | `[]` | included field list (empty = all) |

Available fields: `id`, `timestamp`, `level`, `target`, `message`, `fields`, `file`, `line`, `thread_id`.

### HttpServerConfig

HTTP endpoint configuration (requires the `http` feature).

```rust
pub struct HttpServerConfig {
    pub enabled: bool,                    // default false
    pub host: String,                     // default "127.0.0.1"
    pub port: u16,                        // default 9090
    pub metrics_path: String,             // default "/metrics"
    pub health_path: String,              // default "/health"
    pub error_mode: HttpErrorMode,        // default Strict
    pub auth: Option<HttpAuthConfig>,     // default None
    pub ip_whitelist: Option<Vec<String>>,// default None
    pub tls: Option<TlsConfig>,           // default None
}
```

`host` accepts only bindable IP literals: plain IPv4 (`127.0.0.1`) or bracketed IPv6 (`[::1]` / `[::]`). Hostnames (`localhost`) and malformed bracket spellings cannot form a socket address and are rejected outright by configuration validation.

| Related type | Field/variant | Description |
|----------|-----------|------|
| `HttpErrorMode` | `Strict` / `Warn` | return an error on startup failure / log a warning and continue |
| `HttpAuthConfig` | `enabled: bool`, `token_env: String` | auth token environment variable, cached at startup, fail-closed when acquisition fails |
| `TlsConfig` | `cert_path: String`, `key_path: String` | TLS certificate and private key paths (rustls) |

### PerformanceConfig

```rust
pub struct PerformanceConfig {
    pub channel_capacity: usize,        // default 10000
    pub worker_threads: usize,          // default 3
    pub channel_strategy: ChannelStrategy, // default Fixed
    pub expand_threshold_percent: u8,   // default 80
    pub shrink_threshold_percent: u8,   // default 20
    pub shrink_wait_seconds: u64,       // default 30
    pub min_capacity: usize,            // default 1000
    pub max_capacity: usize,            // default 50000
    pub rate_limit: Option<u64>,        // default None (entries/sec cap)
}
```

The `ChannelStrategy` enum: `Fixed` (fixed capacity), `Adaptive` (scales between `min_capacity` and `max_capacity` by usage).

## ⚠️ Error Types

### InklogError

```rust
#[derive(Error)]
pub enum InklogError {
    #[error("Configuration error: {}", sanitized(.0))]
    ConfigError(String),
    #[error("IO error: {}", sanitized_owned(.0.to_string()))]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {}", sanitized_owned(.0.to_string()))]
    SerializationError(#[from] serde_json::Error),
    #[error("Database error: {}", sanitized(.message))]
    DatabaseError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("Cache error: {}", sanitized(.0))]
    CacheError(String),
    #[error("Encryption error: {}", sanitized(.message))]
    EncryptionError {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("Shutdown error: {}", sanitized(.0))]
    Shutdown(String),
    #[error("Channel error: {}", sanitized(.0))]
    ChannelError(String),
    #[error("Compression error: {}", sanitized(.0))]
    CompressionError(String),
    #[error("Runtime error: {}", sanitized(.0))]
    RuntimeError(String),
    #[error("HTTP server error: {}", sanitized(.0))]
    HttpServerError(String),
    #[error("Unknown error: {}", sanitized(.0))]
    Unknown(String),
}
```

> **All error-text egress points are masked**: `Display` (`to_string()` / `format!("{err}")`),
> `Debug` (`{:?}`), `localized_message()`, and `safe_message()` share the same
> masking rules — paths, DSNs, SQL fragments, and credentials never leak through any egress point.

| Variant | Description |
|------|------|
| `ConfigError` | configuration errors (invalid level, empty path, failed builder validation, etc.) |
| `IoError` | I/O operation errors (converted automatically from `std::io::Error`) |
| `SerializationError` | serialization errors (converted automatically from `serde_json::Error`) |
| `DatabaseError` | database operation errors (with an optional underlying source) |
| `CacheError` | cache operation errors |
| `EncryptionError` | encryption/decryption errors (with an optional underlying source) |
| `Shutdown` | errors during shutdown |
| `ChannelError` | channel communication errors |
| `CompressionError` | compression errors |
| `RuntimeError` | runtime errors |
| `HttpServerError` | HTTP server errors |
| `Unknown` | unknown errors |

Error messages are rendered per system locale via Fluent + ICU (zh-CN / en); pin the language with `INKLOG_LOCALE`.

### InklogResult

```rust
pub type InklogResult<T> = Result<T, InklogError>;
```

## 🩺 Health Monitoring Types

### HealthStatus

```rust
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub overall_status: SinkStatus,
    pub sinks: HashMap<String, SinkHealth>,
    pub channel_usage: f64,           // 0.0 - 1.0
    pub uptime_seconds: u64,
    pub metrics: MetricsSnapshot,
    pub pool_stats: Option<PoolStats>,
    pub encryption_key_valid: bool,
}
```

### SinkStatus

```rust
#[derive(Debug, Serialize, Clone, PartialEq, Default)]
pub enum SinkStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { error: String },
    #[default]
    NotStarted,
}
```

Method:

```rust
pub fn is_operational(&self) -> bool  // Healthy or Degraded counts as operational
```

### SinkHealth

```rust
#[derive(Debug, Serialize, Clone)]
pub struct SinkHealth {
    pub status: SinkStatus,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
}
```

### Metrics

The metrics collector (internally atomic counters and histograms; reads are lock-free).

#### Methods

| Method | Signature | Description |
|------|------|------|
| `new` | `pub fn new() -> Self` | create an instance |
| `logs_written` / `inc_logs_written` | `pub fn logs_written(&self) -> u64` etc. | read / increment the written total |
| `logs_dropped` / `inc_logs_dropped` | same | dropped total |
| `channel_blocked` / `inc_channel_blocked` | same | channel-blocked count |
| `sink_errors` / `inc_sink_error` | same | sink error total |
| `active_workers` | `pub fn active_workers(&self) -> i64` | number of active worker threads |
| `record_latency` | `pub fn record_latency(&self, duration: Duration)` | record a write latency (into the histogram) |
| `record_pool_metrics` | `pub fn record_pool_metrics(&self, total: u64, active: u64, idle: u64)` | record connection-pool metrics |
| `update_sink_health` | `pub fn update_sink_health(&self, name: &str, healthy: bool, error: Option<String>)` | update a sink's health status |
| `sink_started` / `sink_degraded` | `pub fn sink_started(&self, name: &str)` etc. | sink lifecycle events |
| `get_status` | `pub fn get_status(&self, channel_len: usize, channel_cap: usize) -> HealthStatus` | aggregate a health snapshot |
| `export_prometheus` | `pub fn export_prometheus(&self) -> String` | export Prometheus text format |

### MetricsSnapshot

```rust
pub struct MetricsSnapshot {
    pub logs_written: u64,
    pub logs_dropped: u64,
    pub sampled_out: u64,
    pub user_middleware_dropped: u64,
    pub channel_blocked: u64,
    pub sink_errors: u64,
    pub db_batch_size: i64,
    pub db_batch_records_total: u64,
    pub avg_latency_us: u64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
    pub latency_distribution: Vec<u64>,
    pub active_workers: i64,
    pub pool_hit_rate: f64,
}
```

### PoolStats

Database connection pool status snapshot (passed through by `DbNexusAdapter::pool_status()`).

### Prometheus Metrics

| Metric | Type | Description |
|--------|------|------|
| `inklog_logs_written_total` | counter | total successfully written |
| `inklog_logs_dropped_total` | counter | dropped total |
| `inklog_sampled_out_total` | counter | total sampled out (the sampling breakdown of `logs_dropped`) |
| `inklog_user_middleware_dropped_total` | counter | total dropped by user governance middleware |
| `inklog_sink_errors_total` | counter | sink error total |
| `inklog_channel_blocked_total` | counter | channel-blocked count |
| `inklog_active_workers` | gauge | current active worker threads |
| `inklog_avg_latency_us` | gauge | average processing latency (microseconds) |
| `inklog_latency_p50_us` / `inklog_latency_p95_us` / `inklog_latency_p99_us` | gauge | processing latency P50/P95/P99 (microseconds) |
| `inklog_write_latency_us` | histogram | write latency (microseconds, with bucket/sum/count) |
| `inklog_sink_healthy` | gauge | per-sink health status |
| `inklog_uptime_seconds` | gauge | uptime |
| `inklog_pool_hit_rate` | gauge | object pool hit rate (0-100) |
| `inklog_db_batch_size` | gauge | database batch size |
| `inklog_db_batch_records_total` | counter | total records written in database batches |
| `inklog_db_pool_total` / `inklog_db_pool_active` / `inklog_db_pool_idle` | gauge | connection pool total/active/idle |

## 🧾 Log Record Types

### LogRecord

```rust
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub level: String,               // "trace"/"debug"/"info"/"warn"/"error"
    pub target: String,              // module path
    pub message: String,
    pub fields: HashMap<String, Value>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub thread_id: String,
    pub trace_id: Option<String>,    // extracted from the current tracing span
    pub span_id: Option<String>,     // derived as 16-char hex along the parent chain when OTel is absent
}
```

#### `new`

```rust
pub fn new(level: tracing::Level, target: String, message: String) -> Self
```

**Example**

```rust
use inklog::LogRecord;

let record = LogRecord::new(
    tracing::Level::INFO,
    "my_app::auth".to_string(),
    "User logged in".to_string(),
);
```

> **Egress scopes**: `Debug` (`{:?}`) is a masked view — the message passes the sanitizer (message-level sensitive values) and then the masker (PII and sensitive-key field values), and the source path is masked by the same scope. `Serialize` stays verbatim: pre-persistence masking is the sink's responsibility (the semantic surface of the `masking_enabled` / `secret_scan_enabled` switches); rewriting at the serialization layer would silently change persisted data the consumer explicitly configured.

### LogLevel

The `LogLevel` enum implements `FromStr` and `Display` for level parsing and comparison (invalid input returns `LogLevelParseError`).

## 🛡️ Secret Outbound Masking Gate (feature `secret-scan`)

The `secret-scan` feature (off by default) provides three types, attached at the masking point before sink writes (at the same layer as encryption). When enabled, `DataMasker` key-name detection additionally covers Chinese sensitive key names (keys ending with `密码`/`令牌`/`密钥`/`口令`/`凭证`/`凭据`/`私钥`), and the gate is injected via `DataMasker::builder().with_secret_scan(gate)`.

### SecretPatternRegistry

A value-shape regex registry: recognizes bare secrets without key-name context (the `sk-` prefix, PEM private-key blocks, AWS AKIA shapes, GitHub/Slack/Stripe/Google tokens, the three-part JWT form, and other built-in shapes — nine classes in total).

```rust
pub struct SecretPatternRegistry { /* ... */ }

impl SecretPatternRegistry {
    pub fn with_builtins() -> Self;                                        // nine builtin pattern families
    pub fn register(&mut self, name: &str, pattern: &str, replacement: &str)
        -> Result<(), InklogError>;                                        // duplicate-name check + regex compile validation
    pub fn patterns(&self) -> &[SecretPattern];
    pub fn scan(&self, text: &str) -> Vec<SecretMatch>;                    // detection only: pattern name + byte range
}

pub struct SecretMatch { pub pattern: String, pub start: usize, pub end: usize }
```

### EntropyScanner

A value entropy scanner: applies Shannon-entropy judgment to text tokens (base64 core alphabet `[A-Za-z0-9+/]`, ≥20 characters by default), with a configurable threshold. Separated by domain from `encryption::validate_key_entropy` — that one validates encryption key bytes (bits/byte), this one probes log content (bits/char).

```rust
pub struct EntropyScanner { /* ... */ }

impl EntropyScanner {
    pub fn new() -> Self;                                                  // threshold 4.5 bits/char, minimum 20 chars
    pub fn with_config(threshold_bits: f64, min_token_chars: usize) -> Self;
    pub fn shannon_entropy(input: &str) -> f64;
    pub fn scan(&self, text: &str) -> Vec<(usize, usize)>;                 // byte ranges of high-entropy tokens
}
```

### SecretScanGate

The outbound gate: combines the registry and entropy scanning, accumulates per-pattern counts, and fails closed over the limit. `Clone` shares the counters (when sinks each hold a clone, aggregation is global).

```rust
pub struct SecretScanGate { /* ... */ }

impl SecretScanGate {
    pub fn new(registry: SecretPatternRegistry) -> Self;
    pub fn with_entropy(self, scanner: EntropyScanner) -> Self;            // disabled by default
    pub fn with_match_limit(self, limit: usize) -> Self;                   // default 64 (per-input, per-pattern hit cap)
    pub fn mask(&self, text: &str) -> String;                              // fail-closed: replace + count (oversized input fully masked)
    pub fn mask_checked(&self, text: &str) -> Result<String, InklogError>; // over-limit returns Err, output unavailable
    pub fn scan(&self, text: &str) -> Vec<SecretMatch>;                    // detection only, no counting
    pub fn hit_counts(&self) -> Vec<(String, u64)>;
    pub fn hit_count(&self, pattern: &str) -> Option<u64>;
    pub fn total_hits(&self) -> u64;
}
```

### `DataMasker::mask_checked`

```rust
#[cfg(feature = "secret-scan")]
pub fn mask_checked(&self, text: &str) -> Result<String, InklogError>
```

The fail-closed exit: when a gate is attached, the Ok path shares the rule pipeline with `mask` (the gate's output continues through fast-masking and the rule set, with identical replacement semantics, so no PII slips through); if any pattern's hits within a single input exceed the gate limit, it returns `Err(InklogError::SecretScanLimit)`; if the input exceeds the 1 MiB masking cap, it returns `Err(InklogError::SecretScanOversizedInput)` (`mask` redacts the same input wholesale as `***REDACTED_OVERSIZED***` — the two exits fork here). Without a gate, it degrades to `mask` (always `Ok`). The existing `mask` signature is unchanged (wholesale redaction over the limit, backward compatible).

**Example**

```rust
use inklog::{DataMasker, EntropyScanner, SecretPatternRegistry, SecretScanGate};

let gate = SecretScanGate::new(SecretPatternRegistry::with_builtins())
    .with_entropy(EntropyScanner::new())
    .with_match_limit(64);
let probe = gate.clone();
let masker = DataMasker::builder().with_secret_scan(gate).build();

let out = masker.mask_checked("token=sk-proj-abcdefghijklmnopqrstuvwxyz123456")?;
assert_eq!(probe.hit_count("openai_sk"), Some(1));
```

## 🔌 Sink Types

### LogSink trait

The unified abstraction for all output targets; methods take `&self` (interior mutability is guaranteed by the implementer with Mutex/RwLock/atomics):

```rust
#[async_trait]
pub trait LogSink: Send + Sync {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;
    async fn flush(&self) -> Result<(), InklogError>;
    fn is_healthy(&self) -> bool { true }  // healthy by default; overriding recommended
    async fn shutdown(&self) -> Result<(), InklogError>;
}
```

### AsyncSink trait

A marker sub-trait of `LogSink` (blanket impl: every `LogSink` implements it automatically), used for dynamic sink registration via `LoggerBuilder::add_sink` and trait-object upcasting:

```rust
pub trait AsyncSink: LogSink {}
impl<T: LogSink + ?Sized> AsyncSink for T {}
```

### Built-in Sinks at a Glance

| Sink | Module | Feature | Description |
|------|------|---------|------|
| `ConsoleSink` | `support::io::sink::console` | none | ANSI colors, stderr level routing, NO_COLOR support |
| `FileSink` | `support::io::sink::file` | none (compression/encryption by feature) | rotation, compression, AES-256-GCM encryption, circuit breaker, disk-space management |
| `DatabaseSink` | `support::io::sink::database` | one of `sqlite`/`postgres`/`mysql`/`duckdb` | batched persistence, partitioned tables, circuit breaker |
| `ChannelBufferedFileSink` | `support::io::sink::ring_buffered_file` | none | channel-buffered high-throughput file sink |
| `TcpSink` / `UdpSink` | `support::io::sink::net` | `net-sink` | TCP (optional TLS, rustls) + UDP (NDJSON), disconnect buffering and automatic reconnection |
| `OtlpSink` | `support::io::sink::otlp` | `otlp` | OTLP/HTTP JSON log export |
| `SamplingSink` | `support::io::sink::sampling` | none | sampling decorator (level threshold + N-of-1 + keyword allowlist exemption) |
| `RateLimitedSink` | `support::io::sink::rate_limit` | none (extensions go through the crate's own `SinkRateLimit` port) | rate-limit decorator (default burst-quota kernel `BurstLimiter`) |
| `MiddlewareSink` | `support::io::sink::middleware` | none | middleware-chain decorator (filter / transform composition) |

### Related Types

| Type | Description |
|------|------|
| `CircuitBreaker` / `CircuitBreakerConfig` / `CircuitState` | circuit breaker (default failure threshold 5, cooldown 30 s; Closed/Open/HalfOpen state machine) |
| `SinkRegistry` | sink registry and lookup |
| `RotationStrategy` / `SizeBasedRotation` / `TimeBasedRotation` / `CompositeRotation` | rotation strategy composition |
| `Rotatable` / `RotationContext` / `RotationResult` | rotation abstractions |
| `CompressionStrategy` / `NoCompression` / `ZstdCompression` / `GzipCompression` | compression backend abstractions |
| `SinkFactory` / `FileSinkFactory` / `SinkMetadata` / `SinkWriteOutcome` | sink factories and metadata |
| `Sampler` | the sampler (`should_emit(&LogRecord) -> bool`) |
| `SamplingPolicy` | the sampling policy under rate-limit pressure (per_level rates + per_target_prefix rules; `should_emit` returns `Option<bool>`, `None` = no rule hit, falling back to the built-in default) |
| `SamplingConfig` / `TargetSamplingRule` | sampling policy configuration (`InklogConfig.sampling`, validated at load time) |
| `SinkRateLimit` / `NoOpRateLimit` / `TokenBucketRateLimit` | the rate-limit port and implementations (object-safe, for upper-layer implementations to inject); the `TokenBucketRateLimit` kernel is the in-crate `BurstLimiter` (pure budget, no clock, no refill) |
| `BurstLimiter` | the burst-quota counter (atomic CAS decrements, starting at `capacity`, rejecting at zero) |
| `MiddlewareChain` / `MiddlewareVerdict` / `RecordMiddleware` / `EnrichMiddleware` / `LevelFilterMiddleware` | middleware combinators |

## 🧪 Dependency Injection Types

### LoggerDependencies

```rust
#[derive(Default)]
pub struct LoggerDependencies {
    pub cache: Option<Arc<dyn Cache>>,
    pub config: Option<Arc<dyn Config>>,
    /// Third-party sinks registered dynamically (registered via `LoggerBuilder::add_sink`)
    pub custom_sinks: Vec<Arc<dyn LogSink>>,
    #[cfg(feature = "database")]
    pub database: Option<Arc<dyn Database>>,
}
```

Uninjected dependencies use default implementations (`OxCacheAdapter` / `InklogConfigAdapter` / no database).

> The struct derives `Default`, so hand-written literals can use `..Default::default()` as a backstop; but **field names must not be omitted**: `custom_sinks` and `database` are plain fields with no `#[serde(default)]`-style mechanism — a literal missing either fails to compile. The `database` gate is the `database` feature (which implies `dep:dbnexus`, providing the core non-driver APIs), not an `any(...)` combination of driver features.

### Cache trait

```rust
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, InklogError>;
    async fn set(&self, key: &str, value: String) -> Result<(), InklogError>;
    async fn delete(&self, key: &str) -> Result<bool, InklogError>;
    async fn exists(&self, key: &str) -> Result<bool, InklogError>;
}
```

### Config trait

```rust
pub trait Config: Send + Sync {
    fn get_string(&self, key: &str) -> Option<String>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_bool(&self, key: &str) -> Option<bool>;
    fn get_float(&self, key: &str) -> Option<f64>;
}
```

Supported configuration key paths (dot-separated hierarchy): `global.level`, `global.format`, `global.masking_enabled`, `global.auto_fallback`, `global.fallback_initial_delay_ms`, `global.fallback_max_delay_ms`, `global.fallback_max_retries`, `global.output_format`, `file_sink.*` (enabled/path/max_size/rotation_time/keep_files/compress/compression_level/encrypt/encryption_key_env/retention_days/max_total_size/cleanup_interval_minutes/batch_size/flush_interval_ms/masking_enabled/output_format), `console_sink.*` (enabled/colored/stderr_levels/masking_enabled).

### Database trait

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError>;
    async fn is_healthy(&self) -> bool;
}
```

## 🎭 Mock Implementations (Testing)

The three mocks are gated by the `test-utils` feature (visible directly to inline src tests via `cfg(test)`; external consumers must enable it explicitly):

```toml
[dev-dependencies]
inklog = { version = "0.3.0", features = ["test-utils"] }
```

### MockCache

An in-memory Cache based on `RwLock<HashMap>`.

| Method | Description |
|------|------|
| `new()` | create an empty MockCache |
| `with_delay(ms: u64)` | create a MockCache with simulated delay (timeout scenarios) |
| The four Cache trait methods | `get` / `set` / `delete` / `exists`, returning `Result` |

### MockConfig

An in-memory Config based on `RwLock<HashMap>`.

| Method | Description |
|------|------|
| `new()` | create an empty MockConfig |
| `with_value(key: &str, value: &str)` | add a configuration value fluently |
| `set(key: &str, value: &str)` | modify a configuration value at runtime |
| The four Config trait methods | `get_string` / `get_int` / `get_bool` / `get_float` |

### MockDatabaseAdapter

An in-memory Database based on `RwLock<Vec<LogRecord>>`.

| Method | Description |
|------|------|
| `new()` | create a healthy MockDatabaseAdapter |
| `set_healthy(healthy: bool)` | set the health status (fallback scenarios) |
| `record_count()` / `stored_count()` | number of stored records |
| `get_records()` | get all records (test verification) |
| `clear()` | clear records (test isolation) |
| The two Database trait methods | `insert_batch` / `is_healthy` |

## 🏭 Adapter Implementations

| Adapter | Implemented trait | Dependency | Notes |
|--------|--------------|------|------|
| `OxCacheAdapter` | `Cache` | oxcache (built-in) | high-performance in-memory cache with TTL and capacity (`builder()` / `ttl()` / `capacity()`; `new()` returns a `Result`) |
| `InklogConfigAdapter` | `Config` | none (available by default) | chained key-path access based on `InklogConfig` |
| `DbNexusAdapter` | `Database` | one of `sqlite`/`postgres`/`mysql`/`duckdb` | connection pool management (dbnexus); `pool_status() -> PoolStatus` passes through the pool snapshot |
| `InklogModule` | trait-kit module | `kit` + a database backend | trait-kit lifecycle and observability integration |
| `InklogAuditStorage` | the dbnexus AuditStorage port | `dbnexus-audit` | audit events persisted through the inklog DB sink (path `inklog::integrations::InklogAuditStorage`) |

## 💡 Example Reference

The `examples/` crate provides 40 runnable examples in 7 categories (full list in [User Guide · Complete Example List](USER_GUIDE.md#-complete-example-list)); run them with:

```bash
cargo run --package inklog-examples --bin <name>
```

Selected examples matching this document:

| Example | Corresponding API |
|------|----------|
| `basic` | `LoggerManager::new` / `shutdown` / `get_health_status` |
| `builder` | the full `LoggerBuilder` method set |
| `config_file` | `InklogConfig` TOML loading (`from_file` / `load`) |
| `env_overrides` | `INKLOG_*` environment variable overrides |
| `rotation` | `FileSinkConfig` rotation fields |
| `ring_buffered_file` | `ChannelBufferedFileSink` |
| `metrics` | `Metrics` / `HealthStatus` / `export_prometheus` |
| `circuit_breaker` | `CircuitBreaker` / `recover_sink` |
| `di_example` | `LoggerDependencies` injection (requires `sqlite`) |
| `runtime_ops` | `set_level` / `publish_ops_event` and other runtime APIs |

---

**[⬆ Back to Top](#-inklog-api-reference)**
