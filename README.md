<div align="center">

# 🚀 Inklog

**Enterprise-grade Rust logging infrastructure**

[![Crates.io](https://img.shields.io/crates/v/inklog.svg)](https://crates.io/crates/inklog)
[![Documentation](https://docs.rs/inklog/badge.svg)](https://docs.rs/inklog)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-green.svg)](LICENSE-MIT)

<p align="center">
  <a href="#-features">Features</a> •
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-documentation">Documentation</a> •
  <a href="#-examples">Examples</a> •
  <a href="#-contributing">Contributing</a>
</p>

</div>

---

## ✨ Features

### Core Features

- **High-performance async logging** - Multi-threaded with minimal overhead
- **Multiple output targets** - Console, file, database, and S3 archiving
- **Structured logging** - JSON and custom format support with field extraction
- **Log rotation & compression** - Automatic file rotation with ZSTD/GZIP
- **Encryption support** - AES-GCM encryption for sensitive logs
- **S3 archiving** - Automatic log archiving with lifecycle management
- **Health monitoring** - Built-in metrics and HTTP health endpoints
- **Auto-recovery** - Automatic recovery from sink failures

### Advanced Features

- **Enterprise-ready** - Production-tested with comprehensive error handling
- **Security-first** - Encryption, secure key management, and audit logging
- **Cloud-native** - AWS S3 integration and container-friendly design
- **Easy integration** - Simple API with extensive configuration options

---

## 🚀 Quick Start

### Installation

```toml
[dependencies]
inklog = "0.2"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use inklog::{LoggerManager, InklogConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = InklogConfig::default();
    let _logger = LoggerManager::with_config(config).await?;

    log::info!("Application started");
    log::warn!("This is a warning");
    log::error!("This is an error");

    Ok(())
}
```

Run with:

```bash
cargo run
```

---

## 📚 Documentation

| Resource | Description |
|----------|-------------|
| [📖 User Guide](docs/USER_GUIDE.md) | Complete usage guide |
| [🚀 Quick Start](docs/quickstart.md) | Get started in 5 minutes |
| [📋 Config Reference](docs/config-reference.md) | Detailed configuration options |
| [🏗️ Architecture](docs/ARCHITECTURE.md) | System design documentation |
| [❓ FAQ](docs/FAQ.md) | Frequently asked questions |
| [🐛 Troubleshooting](docs/troubleshooting.md) | Common problems and solutions |
| [📘 API Reference](https://docs.rs/inklog) | Rust API documentation |

---

## 🎨 Examples

| Example | Description |
|---------|-------------|
| [basic.rs](examples/basic.rs) | Basic logging setup |
| [file_logging.rs](examples/file_logging.rs) | File rotation and compression |
| [database_logging.rs](examples/database_logging.rs) | Database output |
| [custom_format.rs](examples/custom_format.rs) | Custom log format |
| [encryption.rs](examples/encryption.rs) | Encrypted logs |
| [s3_archive.rs](examples/s3_archive.rs) | S3 archiving |
| [http_health.rs](examples/http_health.rs) | Health endpoints |

Run examples:

```bash
cargo run --example basic
cargo run --example file_logging
cargo run --example database_logging
```

---

## ⚙️ Configuration

### Minimal Config

```toml
[global]
level = "info"
```

### Standard Config

```toml
[global]
level = "info"
format = "{timestamp} [{level}] {target} - {message}"
masking_enabled = true

[console_sink]
enabled = true
colored = true

[file_sink]
enabled = true
path = "logs/app.log"
max_size = "100MB"
rotation_time = "daily"
compress = true
```

See [Config Reference](docs/config-reference.md) for all options.

---

## 🧪 Testing

```bash
# Run all tests
cargo test --all-features

# Run with coverage
cargo tarpaulin --out Html

# Run clippy
cargo clippy --all-features
```

---

## 🤝 Contributing

Contributions are welcome! Please see [Contributing Guide](docs/CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

## 📄 License

Licensed under MIT or Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

---

## 🙏 Acknowledgments

Built on top of excellent projects:

- [tracing](https://github.com/tokio-rs/tracing) - Structured tracing framework
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [serde](https://github.com/serde-rs/serde) - Serialization framework
- [aws-sdk-rust](https://github.com/awslabs/aws-sdk-rust) - AWS SDK
