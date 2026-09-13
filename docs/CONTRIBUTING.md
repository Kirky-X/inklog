# 🤝 inklog 贡献指南

感谢你对 inklog 的关注！本指南介绍如何搭建开发环境、执行测试与质量门禁，以及提交 Pull Request 的完整流程。

> 相关文档：[📖 用户指南](USER_GUIDE.md) · [📘 API 参考](API_REFERENCE.md) · [🏗️ 架构设计](ARCHITECTURE.md) · [🔒 安全文档](SECURITY.md) · [📋 更新日志](CHANGELOG.md)

<details open>
<summary>📑 目录</summary>

- [🎯 贡献概述](#-贡献概述)
- [🚀 快速开始](#-快速开始)
- [🖥️ 开发环境](#️-开发环境)
- [🔨 构建项目](#-构建项目)
- [🔄 TDD 开发流程](#-tdd-开发流程)
- [🪝 Git 钩子](#-git-钩子)
- [🧰 代码质量工具](#-代码质量工具)
- [🧪 测试](#-测试)
- [✍️ 代码风格](#️-代码风格)
- [📚 文档](#-文档)
- [📬 提交变更](#-提交变更)
- [👀 代码审查](#-代码审查)
- [🌐 社区](#-社区)
- [🙏 致谢](#-致谢)
- [⚡ 快速参考](#-快速参考)

</details>

---

## 🎯 贡献概述

inklog 是企业级 Rust 日志基础设施，面向高性能、高安全性与高可靠性环境。贡献 inklog 意味着：

- **学习**：接触现代 Rust 异步编程（tokio + crossbeam）、trait 依赖注入与系统设计；
- **影响**：为依赖 inklog 的下游项目改进日志基础设施；
- **成长**：练习 TDD、性能基准与安全工程的企业级实践。

### 贡献类型

| 类型 | 描述 | 适合人群 |
|------|------|----------|
| **代码贡献** | 新功能、bug 修复、性能优化 | 有 Rust 经验的开发者 |
| **文档** | 改进文档、添加示例、更新指南 | 所有贡献者 |
| **测试** | 添加测试用例、提升覆盖率 | 质量保证爱好者 |
| **问题报告** | 报告 bug、提出功能请求 | 所有用户 |
| **审查** | 审查 PR、提供反馈 | 熟悉代码库的贡献者 |
| **设计** | 架构与 API 设计讨论 | 核心贡献者 |

> 发现安全漏洞请勿公开披露，按 [🔒 安全文档](SECURITY.md) 的流程通过 `security@inklog.dev` 负责任上报。

## 🚀 快速开始

```bash
# 1. Fork 仓库（https://github.com/Kirky-X/inklog 点击 Fork），然后克隆你的 Fork
git clone https://github.com/YOUR_USERNAME/inklog.git
cd inklog

# 2. 添加上游仓库
git remote add upstream https://github.com/Kirky-X/inklog.git

# 3. 安装 Git 钩子（rustfmt/clippy/deny/私钥扫描等门禁）
bash scripts/install-pre-commit.sh

# 4. 创建功能分支
git checkout -b feature/your-feature

# 5. 进行修改，运行与 CI 一致的测试组合
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# 6. 以 Conventional Commits 风格提交并推送
git commit -m "feat: add your feature"
git push origin feature/your-feature

# 7. 在 GitHub 上打开 Pull Request
```

## 🖥️ 开发环境

### 系统要求

| 要求 | 版本 | 说明 |
|------|------|------|
| Rust | 1.97.1 | 仓库经 `rust-toolchain.toml` 固定，rustup 会自动安装 |
| protobuf 编译器 | 最新稳定版 | 含数据库 feature 的组合构建需要 `protoc` |
| lefthook | 最新稳定版 | Git 钩子管理器（`bash scripts/install-pre-commit.sh`） |
| Docker | 可选 | PostgreSQL / MySQL 容器级集成测试 |

### Rust 工具链

```bash
# rust-toolchain.toml 已固定版本，进入仓库目录即自动生效
rustc --version   # 应输出 1.97.1
cargo --version

# 手动安装/更新
rustup update stable
rustup toolchain list
```

### 平台依赖

#### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev protobuf-compiler

# 容器级集成测试（可选）
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
# 安装 Visual Studio C++ 构建工具
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# 安装 Perl（OpenSSL 构建需要）
# https://strawberryperl.com/

# 安装 protoc（数据库 feature 组合构建需要）
```

### 环境验证

```bash
rustc --version
cargo --version
protoc --version
cc --version

# 克隆并做一次快速检查
cargo check --workspace --features "sqlite http cli"
```

## 🔨 构建项目

### 基本构建

```bash
cargo build            # Debug 构建（默认，无可选 feature）
cargo build --release  # 发布构建
cargo build -p inklog  # 只构建主 crate
```

> ⚠️ **数据库后端互斥**：`sqlite` / `postgres` / `mysql` / `duckdb` 互斥（经 dbnexus 强制，embedded 与 server-side 驱动不得混用），本项目**不适用** `--all-features`。请按后端分组启用，CI 与本地验证均使用统一 feature 组合（见下文测试章节）。

### 构建诊断

```bash
cargo check                                            # 快速类型检查（不生成二进制）
cargo doc --no-deps                                    # 生成文档
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps         # 文档告警视为错误
cargo tree                                             # 依赖树
cargo tree -i <crate>                                  # 反查依赖来源
```

### 常见构建问题

| 问题 | 解决方案 |
|------|----------|
| 找不到 OpenSSL | Linux：`sudo apt install libssl-dev`；macOS：`brew install openssl && export OPENSSL_ROOT_DIR=$(brew --prefix openssl)` |
| 找不到 `protoc` | 安装 protobuf 编译器（`apt install protobuf-compiler` / `brew install protobuf`） |
| 内存不足 | 降低并行度：`cargo build -j 2` |
| 数据库后端 feature 冲突 | 检查是否同时启用了多个数据库后端 feature（互斥） |

## 🔄 TDD 开发流程

本项目遵循 **TDD（测试驱动开发）** 循环，每个功能变更按以下步骤执行：

### 循环：Red → Green → Commit → Analyze → Next

#### 1. 定接口

先定义 trait / API 签名，不写实现：

```rust
pub trait LogSink: Send + Sync {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;
    async fn flush(&self) -> Result<(), InklogError>;
    async fn shutdown(&self) -> Result<(), InklogError>;
}
```

#### 2. 写测试（Red）

基于接口编写单元测试，此时测试应编译失败或断言失败：

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

#### 3. 写实现（Green）

实现接口，使测试通过：

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

#### 4. 跑测试

```bash
cargo test --lib --features <对应特性>
```

#### 5. Commit

```bash
git add <涉及的具体文件>
git commit -m "feat(<模块>): <描述>"
```

#### 6. Analyze

分析本次变更对其他模块的影响（测试、文档、下游示例），识别需联动修改的代码，再进入下一轮循环。

### 测试要求

- 测试要验证正确行为的有意义属性（值、结构、副作用、错误类型），而非恒真断言；
- 「所有测试通过」是必要条件但非充分条件，测试太弱时要明确指出并改进；
- 测试编写需遵循库的真实运行时语义，详见 [🧪 测试场景](TEST_SCENARIOS.md) 的「测试编写要点」章节。

## 🪝 Git 钩子

本项目使用 [lefthook](https://github.com/evilmartians/lefthook) 管理 Git 钩子（配置见仓库根目录 `lefthook.yml`）。

### 安装

```bash
bash scripts/install-pre-commit.sh   # 等价于 lefthook install
```

### 钩子内容

| 阶段 | 检查 | 说明 |
|------|------|------|
| **pre-commit** | `cargo fmt --all -- --check` | 代码格式一致 |
| **pre-commit** | `cargo clippy --all-targets --features sqlite,http,cli,kit,compression,parquet,fast-masking -- -D warnings` | 所有警告视为错误 |
| **pre-commit** | `cargo deny check` | 依赖漏洞 / 许可证 / 重复依赖 |
| **pre-commit** | 私钥文件扫描 | 阻止 `BEGIN ... PRIVATE KEY` 等内容入库 |
| **commit-msg** | Conventional Commits 校验 | `feat: ...` / `fix: ...` 等格式强制 |
| **pre-push** | `cargo audit` | RustSec 安全公告 |
| **pre-push** | `cargo llvm-cov --fail-under-lines 80` | 行覆盖率 ≥ 80% 门禁 |

### 禁止事项

- **禁止使用 `--no-verify`** 跳过 Git 钩子；
- **禁止直接提交到 main 分支**，必须创建 feature 分支；
- **禁止通过注释 CI 步骤来绕过质量门禁**；
- **禁止临时降低覆盖率门禁阈值以通过 CI**。

### 分支策略

```bash
git checkout -b feature/your-feature   # 创建 feature 分支
git add <specific-files>               # 只暂存需要的文件
git commit -m "feat(scope): description"

# 合并经 Pull Request 完成，不直接 push 到 main
```

## 🧰 代码质量工具

| 工具 | 用途 | 常用命令 |
|------|------|----------|
| **rustfmt** | 代码格式化 | `cargo fmt --all` / `cargo fmt --all -- --check` |
| **Clippy** | Lint 检查（警告视为错误） | `cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings` |
| **cargo-deny** | 依赖漏洞 / 许可证 / 重复依赖 | `cargo deny check` |
| **cargo-audit** | RustSec 安全公告 | `cargo audit` |
| **cargo-llvm-cov** | 行覆盖率 | `cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 80` |

## 🧪 测试

### 测试类型

| 测试类型 | 位置 | 运行命令 |
|----------|------|----------|
| 单元测试 | `src/` 内联 `#[cfg(test)]` | `cargo test --lib` |
| 集成测试 | `tests/integration/`、`tests/unit_tests.rs` 等 | `cargo test --workspace --features <组合>` |
| E2E 场景 | `tests/e2e/e2e_advanced.rs`（226 场景 × 15 域） | 同上 |
| 组合测试 | `tests/combinations/` | 需 `sqlite` |
| 容器级 | `tests/docker/` | `docker compose -f docker/docker-compose.test.yml up -d` |
| 基准测试 | `benches/` | `cargo bench` |

### 运行命令（与 CI 一致）

```bash
# CI 测试门禁（数据库后端互斥，不适用 --all-features）
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# 运行单个测试
cargo test --lib --features sqlite test_log_rotation

# 显示测试输出
cargo test --lib --features sqlite -- --nocapture

# 容器级数据库测试
docker compose -f docker/docker-compose.test.yml up -d
cargo test --test docker --features sqlite
docker compose -f docker/docker-compose.test.yml down
```

> **本地化提示**：错误消息经 ICU/Fluent 按系统 locale 渲染。若测试断言英文消息文本，请设置 `INKLOG_LOCALE=en` 以固定输出语言。

测试金字塔、E2E 场景清单与静态门槛详见 [🧪 测试场景](TEST_SCENARIOS.md)；性能基准复现见 [⚡ 性能基线](PERFORMANCE.md)。

## ✍️ 代码风格

### 格式化与 Lint

```bash
cargo fmt --all                                                        # 格式化
cargo fmt --all -- --check                                             # 检查（不修改）
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings  # 零告警
```

### 命名约定

- **变量和函数**：snake_case（`file_path`、`write_log_record`）；
- **结构体和枚举**：PascalCase（`LoggerManager`、`SinkStatus`）；
- **常量**：UPPER_SNAKE_CASE（`PBKDF2_ITERATIONS`）。

### 文档注释

所有公共 API 必须有文档注释（rustdoc）：

```rust
/// 文件日志 Sink 实现。
///
/// 提供：
/// - 自动日志轮转（基于大小或时间）
/// - 压缩支持（Zstd / Gzip）
/// - AES-256-GCM 加密
/// - 断路器保护
pub struct FileSink {
    config: FileSinkConfig,
}
```

### 错误处理规范

- 错误类型用 `thiserror` 定义（`InklogError` / `InklogResult`）；
- **禁止**在生产代码中使用 `unwrap()` / `expect()`；
- 错误必须显性化：抛出、返回或上报，严禁吞掉。

### 异步编程规范

- 使用 Tokio 异步运行时；
- **禁止**在异步上下文中阻塞（必要时用 `tokio::task::spawn_blocking`）；
- `log` / `tracing` 双前端记录日志，业务代码零侵入。

### 反模式

| 反模式 | 正确做法 |
|--------|----------|
| 在 async 上下文中阻塞 | `tokio::task::spawn_blocking` |
| 提交 `logs/`、`target/` 等产物 | 依赖 `.gitignore`，不入库 |
| 硬编码密钥 | 从环境变量读取（见 [🔒 安全文档](SECURITY.md)） |
| 吞掉错误（忽略 `Result`） | `?` 传播或显式记录 |
| 使用 `unwrap()` / `expect()` | 显式错误处理 |
| 同时启用多个数据库后端 feature | 按后端分组启用（互斥） |

## 📚 文档

| 文档 | 内容 |
|------|------|
| [README.md](../README.md) | 项目概述与快速开始 |
| [docs/USER_GUIDE.md](USER_GUIDE.md) | 完整使用教程 |
| [docs/API_REFERENCE.md](API_REFERENCE.md) | 公共 API 逐项说明 |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | 架构与设计决策 |
| [docs/SECURITY.md](SECURITY.md) | 安全设计与报告流程 |
| [docs/CHANGELOG.md](CHANGELOG.md) | 版本变更记录 |
| Rustdoc | API 文档（`cargo doc --open`） |

### 文档更新清单

修改代码时请同步更新：

- [ ] 公共 API 的 rustdoc 注释；
- [ ] `docs/CHANGELOG.md`（记录变更）；
- [ ] `README.md`（如影响用户可见功能）；
- [ ] `docs/` 下对应专题文档（API 变更 → API_REFERENCE，架构变更 → ARCHITECTURE，安全相关 → SECURITY）；
- [ ] 示例代码（如新增功能）。

> **规则**：禁止「先发布再补文档」，文档是发布的一部分。

## 📬 提交变更

### Pull Request 流程

1. **准备分支**：从最新 main 创建 `feature/your-feature`；
2. **进行修改**：补测试、补文档，运行 `cargo fmt --all`、clippy 与测试组合；
3. **提交**：遵循 Conventional Commits（commit-msg 钩子强制校验）：

```bash
git add <具体文件>
git commit -m "feat(file): add zstd compression support

- Add zstd codec to compression options
- Add integration tests for compression
- Update docs/CHANGELOG.md"
```

4. **推送并打开 Pull Request**，填写 PR 描述（目的、变更类型、测试情况）。

### CI 检查

| 检查 | 说明 | 失败处理 |
|------|------|----------|
| **格式化** | `cargo fmt --all -- --check` | 运行 `cargo fmt --all` |
| **Clippy** | `-D warnings` 零告警 | 修复警告 |
| **测试** | CI feature 组合全量测试 | 修复失败测试 |
| **覆盖率** | llvm-cov 行覆盖 ≥ 80% | 补充测试 |
| **安全审计** | `cargo deny check` + `cargo audit` | 升级或替换依赖 |

### PR 合并策略

- **Squash and Merge**：功能分支（单个提交）；
- **Rebase and Merge**：维护性更新；
- **Merge Commit**：避免使用。

## 👀 代码审查

审查关注以下方面：

1. **功能性**：是否实现预期行为；
2. **安全性**：是否引入安全漏洞；
3. **性能**：是否影响写入主链路（对照 [⚡ 性能基线](PERFORMANCE.md)）；
4. **可读性**：代码是否易于理解；
5. **测试**：测试是否充分；
6. **文档**：文档是否准确完整。

### 审查响应时间

| 改动规模 | 响应时间 |
|----------|----------|
| 小改动 | 1-2 个工作日 |
| 中等改动 | 3-5 个工作日 |
| 大改动 | 1-2 周 |

## 🌐 社区

| 渠道 | 用途 | 链接 |
|------|------|------|
| **GitHub Issues** | Bug 报告、功能请求 | <https://github.com/Kirky-X/inklog/issues> |
| **GitHub Discussions** | 问答、讨论、想法分享 | <https://github.com/Kirky-X/inklog/discussions> |
| **Pull Requests** | 代码贡献 | <https://github.com/Kirky-X/inklog/pulls> |

### 行为准则

- **尊重**：尊重不同的观点和经验；
- **包容**：欢迎所有背景的贡献者；
- **建设性**：提供建设性的反馈；
- **专注**：关注对项目最有利的事情。

## 🙏 致谢

感谢 [tracing](https://github.com/tokio-rs/tracing)、[tokio](https://tokio.rs/)、[crossbeam](https://github.com/crossbeam-rs/crossbeam) 等 Rust 生态项目，以及所有为 inklog 做出贡献的开发者。完整致谢见 [README](../README.md#-致谢)。

## ⚡ 快速参考

### 常用命令

```bash
# 开发循环
cargo check --features sqlite
cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"

# 完整检查（提交前）
cargo fmt --all -- --check
cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking" -- -D warnings
cargo deny check

# 覆盖率
cargo llvm-cov --features "sqlite http cli kit compression gzip parquet fast-masking" --lib --fail-under-lines 80

# 基准
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
```

### 项目结构

```text
inklog/
├── src/
│   ├── lib.rs              # 公共 API 入口与 re-exports
│   ├── error.rs            # InklogError / InklogResult
│   ├── log_level.rs        # LogLevel
│   ├── domain/             # 领域层（core: manager/builder/subscriber/workers；config；types: log_record）
│   ├── support/            # 支撑层（io/sink 全家桶、processing: masking/template/object_pool、observability、audit_chain）
│   ├── validation/         # PathValidator / LogSanitizer
│   ├── integrations/       # 适配层（oxcache / confers / dbnexus / trait-kit、Mock 实现）
│   ├── i18n/               # Fluent + ICU 消息本地化
│   └── cli/                # inklog-cli（decrypt / generate / validate / query）
├── locales/                # i18n 资源（zh-CN / en）
├── tests/                  # 集成 / E2E / 组合 / docker / 性能测试
├── benches/                # criterion 基准（inklog_bench、rc4_pipeline_bench）
├── examples/               # inklog-examples crate（7 类 39 个示例）
├── docs/                   # 文档（本目录）
└── scripts/                # install-pre-commit.sh 等脚本
```

---

有问题吗？请到 [GitHub Discussions](https://github.com/Kirky-X/inklog/discussions) 提问，或创建 [Issue](https://github.com/Kirky-X/inklog/issues)。
