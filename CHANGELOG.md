# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-01-01

### Added

#### Core Features
- **LoggerManager**: Async logger manager with dual initialization support
  - `LoggerManager::new()` for zero-dependency initialization
  - `LoggerManager::from_file()` for config file loading (requires `confers` feature)
  - `LoggerManager::builder()` for fluent builder pattern
  - `LoggerManager::load()` for automatic config discovery (requires `confers`)

- **Console Sink**: High-performance console logging with colored output
  - Synchronous fast path (<50μs latency)
  - ANSI color support (ERROR=red, WARN=yellow, INFO=green)
  - Automatic TTY detection (disabled colors for non-TTY)
  - Configurable stderr分流 for error/warn levels

- **File Sink**: Persistent file logging with rotation and compression
  - Size-based rotation (configurable threshold)
  - Time-based rotation (hourly/daily/weekly)
  - Zstd compression (configurable level 1-22)
  - AES-256-GCM encryption with per-file nonces
  - Automatic cleanup with retention policies
  - Magic header format for encrypted files

- **Database Sink**: Multi-database support with batch writes
  - SQLite, PostgreSQL, MySQL support via SeaORM
  - Batch insert with configurable size (default: 100)
  - Timeout-based flush (default: 500ms)
  - Monthly partitioning for PostgreSQL
  - Parquet format export for analytics

- **S3 Archive**: Cloud archiving to S3-compatible storage
  - Scheduled archiving with cron expressions
  - Automatic local retention with cleanup
  - Multiple storage classes (Standard, IA, Glacier)
  - Multipart upload for large files
  - SHA256 checksum verification

- **HTTP Monitoring**: Built-in HTTP server for observability
  - `/health` endpoint for health checks
  - `/metrics` endpoint for Prometheus-compatible metrics
  - Configurable host, port, and paths

- **Metrics**: Comprehensive performance monitoring
  - Logs written/dropped counters
  - Latency histograms
  - Channel usage gauges
  - Sink health status

- **Masking**: Sensitive data protection
  - Field name matching (password, token, etc.)
  - Regex pattern matching (email, phone, etc.)
  - Configurable masking rules

#### Configuration
- Dual initialization: zero-dependency default + config file
- Environment variable overrides (40+ variables)
- Full TOML configuration support
- Configuration validation with error messages
- Feature-gated configuration loading

#### Performance
- Async architecture with tokio runtime
- crossbeam-channel for zero-copy message passing
- Bounded channel with backpressure (default: 10,000)
- 3-thread worker pool (Dispatcher + File + DB)
- Object pool for memory optimization

### Changed

- **Breaking**: Renamed initialization methods
  - `LoggerManager::init()` → `LoggerManager::new()` or `LoggerManager::from_file()`
- **Breaking**: Configuration field names normalized to snake_case
- **Performance**: Console output latency improved to <50μs
- **Performance**: Throughput improved to >3.6M ops/s

### Fixed

- Configuration environment variable override mechanism
- HTTP server error handling with configurable modes
- Archive metadata recording with checksum and status
- Scheduler stability with tokio-cron-scheduler
- Removed unwrap() calls in critical paths

### Removed

- Legacy `init()` API (replaced with dual initialization)

### Security

- AES-256-GCM encryption for log files
- Base64-encoded key management via environment variables
- File permissions set to 600 (owner-only)
- Sensitive data masking for passwords, tokens, etc.

### Documentation

- PRD (Product Requirements Document)
- TDD (Technical Design Document)
- TASK (Development Tasks)
- TEST (Test Specifications)
- UAT (User Acceptance Testing)
- Quickstart Guide
- Configuration Reference
- Troubleshooting Guide

### Performance Benchmarks

| Metric | Target | Actual |
|--------|--------|--------|
| Console latency | <50μs | ~1μs |
| Channel enqueue | <5μs | ~0.5μs |
| Throughput | 500/s | ~3.6M/s |
| Memory usage | <30MB | ~15MB |

### Compatibility

- **Rust**: 1.70+
- **Platforms**: Linux, macOS, Windows
- **Databases**: SQLite 3.35+, PostgreSQL 12+, MySQL 8.0+
- **S3**: AWS S3, MinIO, DigitalOcean Spaces

### Example Usage

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Zero-dependency default
    let _logger = LoggerManager::new().await?;
    
    // Or with builder pattern
    let _logger = LoggerManager::builder()
        .level("info")
        .enable_console(true)
        .enable_file("app.log")
        .channel_capacity(10000)
        .build()
        .await?;
    
    tracing::info!("Hello, inklog!");
    Ok(())
}
```

### CLI Tools

```bash
# Generate config template
inklog-cli generate -o config.toml

# Validate configuration
inklog-cli validate -c config.toml

# Decrypt encrypted logs
inklog-cli decrypt -i encrypted.log.enc -o recovered.log
```

---

## [0.0.0] - 2025-12-30

### Added

- Initial project scaffolding
- Basic Cargo configuration
- CI/CD workflows

<!-- Links -->
[Unreleased]: https://github.com/inklog/inklog/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/inklog/inklog/releases/tag/v0.1.0
[0.0.0]: https://github.com/inklog/inklog/releases/tag/v0.0.0
