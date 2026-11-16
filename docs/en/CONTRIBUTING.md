# 🤝 inklog Contributing Guide

Thank you for your interest in contributing to inklog! This guide covers setting up the development environment, running tests and quality gates, and the full Pull Request workflow.

> Related documents: [📖 User Guide](USER_GUIDE.md) · [📘 API Reference](API_REFERENCE.md) · [🏗️ Architecture](ARCHITECTURE.md) · [🔒 Security Documentation](SECURITY.md) · [📋 Changelog](../CHANGELOG.md)

<details open>
<summary>📑 Contents</summary>

- [🎯 Contribution Overview](#-contribution-overview)
- [🚀 Quick Start](#-quick-start)
- [🖥️ Development Environment](#️-development-environment)
- [🔨 Building the Project](#-building-the-project)
- [🔄 TDD Workflow](#-tdd-workflow)
- [🪝 Git Hooks](#-git-hooks)
- [🧰 Code Quality Tools](#-code-quality-tools)
- [🧪 Testing](#-testing)
- [✍️ Code Style](#️-code-style)
- [📚 Documentation](#-documentation)
- [📬 Submitting Changes](#-submitting-changes)
- [👀 Code Review](#-code-review)
- [🌐 Community](#-community)
- [🙏 Acknowledgments](#-acknowledgments)
- [⚡ Quick Reference](#-quick-reference)

</details>

---

## 🎯 Contribution Overview

inklog is enterprise-grade Rust logging infrastructure aimed at high-performance, high-security, high-reliability environments. Contributing to inklog means:

- **Learning**: working with modern Rust async programming (tokio + crossbeam), trait-based dependency injection, and systems design;
- **Impact**: improving the logging infrastructure for downstream projects that depend on inklog;
- **Growth**: practicing enterprise-grade TDD, performance benchmarking, and security engineering.

### Contribution Types

| Type | Description | Best for |
|------|------|----------|
| **Code** | new features, bug fixes, performance optimization | developers with Rust experience |
| **Documentation** | improve docs, add examples, update guides | all contributors |
| **Testing** | add test cases, raise coverage | QA enthusiasts |
| **Issue reports** | report bugs, request features | all users |
| **Review** | review PRs, provide feedback | contributors familiar with the codebase |
| **Design** | architecture and API design discussions | core contributors |

> If you discover a security vulnerability, do not disclose it publicly; follow the process in the [🔒 Security Documentation](SECURITY.md) and report responsibly, preferably via the private GitHub Security Advisories channel (alternative: email `Kirky-X@outlook.com`, handled by Kirky.X).

## 🚀 Quick Start

```bash
# 1. Fork the repository (click Fork at https://github.com/Kirky-X/inklog), then clone your fork
git clone https://github.com/YOUR_USERNAME/inklog.git
cd inklog

# 2. Add the upstream repository
git remote add upstream https://github.com/Kirky-X/inklog.git

# 3. Install Git hooks (choose one: lefthook or the Python pre-commit framework)
lefthook install
# or: pip install pre-commit && pre-commit install

# 4. Create a feature branch
git checkout -b feature/your-feature

# 5. Make changes and run the same test combination as CI
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# 6. Commit with Conventional Commits style and push
git commit -m "feat: add your feature"
git push origin feature/your-feature

# 7. Open a Pull Request on GitHub
```

## 🖥️ Development Environment

### System Requirements

| Requirement | Version | Notes |
|------|------|------|
| Rust | 1.97.1 | pinned by `rust-toolchain.toml`; rustup installs it automatically |
| protobuf compiler | latest stable | combinations with database features require `protoc` |
| pre-commit (or lefthook) | latest stable | Git hook manager: `lefthook install` (reads `lefthook.yml`); or `pip install pre-commit && pre-commit install` (reads `.pre-commit-config.yaml`) |
| Docker | optional | container-level PostgreSQL / MySQL integration tests |

### Rust Toolchain

```bash
# rust-toolchain.toml pins the version; it takes effect automatically inside the repo
rustc --version   # should print 1.97.1
cargo --version

# Manual install/update
rustup update stable
rustup toolchain list
```

### Platform Dependencies

#### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev protobuf-compiler

# Container-level integration tests (optional)
sudo apt install -y docker.io
sudo systemctl start docker
sudo usermod -aG docker $USER
```

#### macOS

```bash
brew install openssl pkg-config protobuf

export OPENSSL_LIB_DIR=$(brew --prefix openssl)/lib
export OPENSSL_INCLUDE_DIR=$(brew --prefix openssl)/include
```

#### Windows

```bash
# Install the Visual Studio C++ build tools
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Install Perl (needed to build OpenSSL)
# https://strawberryperl.com/

# Install protoc (needed for builds that include database features)
```

### Environment Verification

```bash
rustc --version
cargo --version
protoc --version
cc --version

# Clone and run a quick check
cargo check --workspace --features "sqlite http cli"
```

## 🔨 Building the Project

### Basic Builds

```bash
cargo build            # Debug build (default, no optional features)
cargo build --release  # release build
cargo build -p inklog  # build only the main crate
```

> ⚠️ **Database backend exclusivity**: `sqlite` / `postgres` / `mysql` / `duckdb` cannot be enabled together, and this project does **not** support `--all-features` (the reason is in [User Guide · Feature Flags](USER_GUIDE.md#-feature-flags)). Enable features grouped by backend; CI and local verification both use the unified feature combination (see the testing section below).

### Build Diagnostics

```bash
cargo check                                            # quick type check (no binary)
cargo doc --no-deps                                    # generate documentation
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps         # treat doc warnings as errors
cargo tree                                             # dependency tree
cargo tree -i <crate>                                  # trace where a dependency comes from
```

### Common Build Problems

| Problem | Solution |
|------|----------|
| OpenSSL not found | Linux: `sudo apt install libssl-dev`; macOS: `brew install openssl && export OPENSSL_ROOT_DIR=$(brew --prefix openssl)` |
| `protoc` not found | install the protobuf compiler (`apt install protobuf-compiler` / `brew install protobuf`) |
| Out of memory | reduce parallelism: `cargo build -j 2` |
| Database backend feature conflict | check whether multiple database backend features are enabled at once (they are mutually exclusive) |

## 🔄 TDD Workflow

This project follows the **TDD (test-driven development)** loop; every feature change goes through these steps:

### The Loop: Red → Green → Commit → Analyze → Next

#### 1. Define the interface

Define the trait / API signature first, without the implementation:

```rust
pub trait LogSink: Send + Sync {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;
    async fn flush(&self) -> Result<(), InklogError>;
    async fn shutdown(&self) -> Result<(), InklogError>;
}
```

#### 2. Write tests (Red)

Write unit tests against the interface; at this point the tests should fail to compile or fail their assertions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sink_write() {
        let sink = MockSink::new();
        let record = LogRecord::new(tracing::Level::INFO, "app".to_string(), "test".to_string());
        assert!(sink.write(&record).await.is_ok());
    }
}
```

#### 3. Write the implementation (Green)

Implement the interface so the tests pass:

```rust
pub struct MockSink {
    records: Arc<Mutex<Vec<LogRecord>>>,
}

#[async_trait::async_trait]
impl LogSink for MockSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
    // ...
}
```

#### 4. Run the tests

```bash
cargo test --lib --features <relevant-features>
```

#### 5. Commit

```bash
git add <the-specific-files>
git commit -m "feat(<module>): <description>"
```

#### 6. Analyze

Analyze the impact of this change on other modules (tests, docs, downstream examples), identify code that needs coordinated changes, then start the next loop iteration.

### Test Requirements

- Tests must verify meaningful properties of correct behavior (values, structure, side effects, error types), never tautological assertions;
- "All tests pass" is necessary but not sufficient; when tests are too weak, call it out and improve them;
- Tests must follow the library's real runtime semantics — see the "Test-Writing Rules" section of the [🧪 Test Scenarios](TEST_SCENARIOS.md).

## 🪝 Git Hooks

This project offers two optional Git hook configurations; pick one: Python [pre-commit](https://pre-commit.com/) (`.pre-commit-config.yaml`) or [lefthook](https://github.com/evilmartians/lefthook) (`lefthook.yml` at the repository root).

### Installation

```bash
lefthook install                     # enable the hooks in lefthook.yml
# or
pip install pre-commit && pre-commit install   # enable .pre-commit-config.yaml
```

### Hook Contents

The table below lists the hooks in `lefthook.yml`; Python pre-commit runs `cargo fmt --all -- --check`, `cargo check`, `cargo clippy --all-targets ... -- -D warnings` plus generic hooks (trailing-whitespace, detect-private-key, detect-secrets, typos, etc.) at the pre-commit stage, and does not include `cargo deny check`, commit-msg, or pre-push hooks.

| Stage | Check | Notes |
|------|------|------|
| **pre-commit** | `cargo fmt --all -- --check` | consistent code format |
| **pre-commit** | `cargo clippy --all-targets --no-default-features --features sqlite,http,cli,kit,compression,parquet,fast-masking -- -D warnings` | all warnings as errors (db drivers are mutually exclusive; `--all-features` unsupported) |
| **pre-commit** | `cargo deny check` | dependency advisories / licenses / duplicate crates |
| **pre-commit** | private-key file scan | blocks content such as `BEGIN ... PRIVATE KEY` from entering the repo |
| **commit-msg** | Conventional Commits validation | enforces the `feat: ...` / `fix: ...` format |
| **pre-push** | `cargo audit` | RustSec security advisories |
| **pre-push** | `cargo llvm-cov --features "sqlite http cli kit zstd gzip parquet fast-masking" --lib --fail-under-lines 95` | line coverage ≥ 95% gate |

### Prohibitions

- **Do not use `--no-verify`** to skip Git hooks;
- **Do not commit directly to the main branch**; always create a feature branch;
- **Do not bypass quality gates by commenting out CI steps**;
- **Do not temporarily lower the coverage gate threshold to pass CI**.

### Branch Strategy

```bash
git checkout -b feature/your-feature   # create a feature branch
git add <specific-files>               # stage only the files you need
git commit -m "feat(scope): description"

# Merging happens via Pull Request; never push directly to main
```

## 🧰 Code Quality Tools

| Tool | Purpose | Common commands |
|------|------|----------|
| **rustfmt** | code formatting | `cargo fmt --all` / `cargo fmt --all -- --check` |
| **Clippy** | lint checks (warnings as errors) | `cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings` |
| **cargo-deny** | dependency advisories / licenses / duplicate crates | `cargo deny check` |
| **cargo-audit** | RustSec security advisories | `cargo audit` |
| **cargo-llvm-cov** | line coverage | `cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 95` |

## 🧪 Testing

### Test Types

| Test type | Location | Command |
|----------|------|----------|
| Unit tests | inline `#[cfg(test)]` in `src/` | `cargo test --lib` |
| Integration tests | `tests/integration/`, `tests/unit_tests.rs`, etc. | `cargo test --workspace --features <combination>` |
| E2E scenarios | `tests/e2e/e2e_advanced.rs` (228 tests × 21 domains) | same as above |
| Combination tests | `tests/combinations/` | requires `sqlite` |
| Container level | `tests/docker/` | `docker compose -f docker/docker-compose.test.yml up -d` |
| Benchmarks | `benches/` | `cargo bench` |

### Commands (matching CI)

```bash
# CI test gate (database backends are mutually exclusive; --all-features unsupported)
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# Run a single test
cargo test --lib --features sqlite test_log_rotation

# Show test output
cargo test --lib --features sqlite -- --nocapture

# Container-level database tests
docker compose -f docker/docker-compose.test.yml up -d
cargo test --test docker --features sqlite
docker compose -f docker/docker-compose.test.yml down
```

> **Locale note**: error messages are rendered via ICU/Fluent based on the system locale. If tests assert English message text, set `INKLOG_LOCALE=en` to pin the output language.

For the test pyramid, the E2E scenario list, and static gates, see the [🧪 Test Scenarios](TEST_SCENARIOS.md); for benchmark reproduction, see the [⚡ Performance Baseline](PERFORMANCE.md).

## ✍️ Code Style

### Formatting and Lint

```bash
cargo fmt --all                                                        # Format
cargo fmt --all -- --check                                             # Check (no changes)
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings  # Zero warnings
```

### Naming Conventions

- **Variables and functions**: snake_case (`file_path`, `write_log_record`);
- **Structs and enums**: PascalCase (`LoggerManager`, `SinkStatus`);
- **Constants**: UPPER_SNAKE_CASE (`PBKDF2_ITERATIONS`).

### Documentation Comments

All public APIs must have documentation comments (rustdoc):

```rust
/// File log sink implementation.
///
/// Provides:
/// - Automatic log rotation (size- or time-based)
/// - Compression support (Zstd / Gzip)
/// - AES-256-GCM encryption
/// - Circuit-breaker protection
pub struct FileSink {
    config: FileSinkConfig,
}
```

### Error Handling Rules

- Define error types with `thiserror` (`InklogError` / `InklogResult`);
- **Never** use `unwrap()` / `expect()` in production code;
- Errors must be explicit: throw, return, or report them — never swallow them.

### Async Programming Rules

- Use the Tokio async runtime;
- **Never** block inside async contexts (use `tokio::task::spawn_blocking` when necessary);
- Record logs through the `log` / `tracing` dual frontends with zero intrusion into business code.

### Anti-Patterns

| Anti-pattern | Correct approach |
|--------|----------|
| Blocking in an async context | `tokio::task::spawn_blocking` |
| Committing artifacts such as `logs/`, `target/` | rely on `.gitignore`; never commit them |
| Hard-coded secrets | read from environment variables (see the [🔒 Security Documentation](SECURITY.md)) |
| Swallowing errors (ignoring `Result`) | propagate with `?` or record explicitly |
| Using `unwrap()` / `expect()` | explicit error handling |
| Enabling multiple database backend features at once | enable them grouped by backend (mutually exclusive) |

## 📚 Documentation

| Document | Content |
|------|------|
| [README_CN.md](../../README_CN.md) | project overview and quick start |
| [docs/USER_GUIDE.md](USER_GUIDE.md) | the complete usage tutorial |
| [docs/API_REFERENCE.md](API_REFERENCE.md) | item-by-item public API reference |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | architecture and design decisions |
| [docs/SECURITY.md](SECURITY.md) | security design and reporting process |
| [docs/CHANGELOG.md](../CHANGELOG.md) | version change history |
| Rustdoc | API documentation (`cargo doc --open`) |

### Documentation Update Checklist

When changing code, update in sync:

- [ ] rustdoc comments for public APIs;
- [ ] `docs/CHANGELOG.md` (record the change);
- [ ] `README.md` (if user-visible features are affected);
- [ ] the corresponding topic document under `docs/` (API changes → API_REFERENCE, architecture changes → ARCHITECTURE, security-related → SECURITY);
- [ ] example code (for new features).

> **Rule**: "ship first, document later" is forbidden — documentation is part of the release.

## 📬 Submitting Changes

### Pull Request Process

1. **Prepare the branch**: create `feature/your-feature` from the latest main;
2. **Make changes**: add tests and documentation; run `cargo fmt --all`, clippy, and the test combination;
3. **Commit**: follow Conventional Commits (enforced by the commit-msg hook):

```bash
git add <specific-files>
git commit -m "feat(file): add zstd compression support

- Add zstd codec to compression options
- Add integration tests for compression
- Update docs/CHANGELOG.md"
```

4. **Push and open the Pull Request**, filling in the PR description (purpose, change type, testing done).

### CI Checks

| Check | Description | On failure |
|------|------|----------|
| **Formatting** | `cargo fmt --all -- --check` | run `cargo fmt --all` |
| **Clippy** | `-D warnings`, zero warnings | fix the warnings |
| **Tests** | full test run with the CI feature combination | fix the failing tests |
| **Coverage** | llvm-cov line coverage ≥ 95% | add tests |
| **Security audit** | `cargo deny check` + `cargo audit` | upgrade or replace dependencies |

### PR Merge Strategy

- **Squash and Merge**: feature branches (single commit);
- **Rebase and Merge**: maintenance updates;
- **Merge Commit**: avoid.

## 👀 Code Review

Reviews focus on:

1. **Functionality**: does it implement the expected behavior;
2. **Security**: does it introduce vulnerabilities;
3. **Performance**: does it affect the write main path (compare against the [⚡ Performance Baseline](PERFORMANCE.md));
4. **Readability**: is the code easy to understand;
5. **Testing**: are the tests sufficient;
6. **Documentation**: is the documentation accurate and complete.

### Review Response Times

| Change size | Response time |
|----------|----------|
| small | 1-2 business days |
| medium | 3-5 business days |
| large | 1-2 weeks |

## 🌐 Community

| Channel | Purpose | Link |
|------|------|------|
| **GitHub Issues** | bug reports, feature requests | <https://github.com/Kirky-X/inklog/issues> |
| **GitHub Discussions** | Q&A, discussion, idea sharing | <https://github.com/Kirky-X/inklog/discussions> |
| **Pull Requests** | code contributions | <https://github.com/Kirky-X/inklog/pulls> |

### Code of Conduct

- **Respect**: respect different opinions and experiences;
- **Inclusiveness**: welcome contributors from all backgrounds;
- **Constructiveness**: provide constructive feedback;
- **Focus**: keep attention on what benefits the project most.

## 🙏 Acknowledgments

Thanks to [tracing](https://github.com/tokio-rs/tracing), [tokio](https://tokio.rs/), [crossbeam](https://github.com/crossbeam-rs/crossbeam) and other Rust ecosystem projects, and to everyone who has contributed to inklog. Full acknowledgments are in the [README](../../README.md#-acknowledgments).

## ⚡ Quick Reference

### Common Commands

```bash
# Development loop
cargo check --features sqlite
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# Full check (before committing)
cargo fmt --all -- --check
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings
cargo deny check

# Coverage
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 95

# Benchmarks
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
```

### Project Structure

```text
inklog/
├── src/
│   ├── lib.rs              # Public API entry and re-exports
│   ├── error.rs            # InklogError / InklogResult
│   ├── log_level.rs        # LogLevel
│   ├── domain/             # Domain layer (core: manager/builder/subscriber/workers; config; types: log_record)
│   ├── support/            # Support layer (full io/sink family, processing: masking/template/object_pool, observability, audit_chain)
│   ├── validation/         # PathValidator / LogSanitizer
│   ├── integrations/       # Adapter layer (oxcache / confers / dbnexus / trait-kit, Mock implementations)
│   ├── i18n/               # Fluent + ICU message localization
│   └── cli/                # inklog-cli（decrypt / generate / validate / query）
├── locales/                # i18n resources (zh-CN / en)
├── tests/                  # Integration / E2E / combinations / docker / performance tests
├── benches/                # criterion benchmarks (inklog_bench, rc4_pipeline_bench)
├── examples/               # inklog-examples crate (7 categories, 40 examples)
└── docs/                   # Documentation (this directory)
```

---

Have questions? Ask on [GitHub Discussions](https://github.com/Kirky-X/inklog/discussions) or open an [Issue](https://github.com/Kirky-X/inklog/issues).
