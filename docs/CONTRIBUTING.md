# Inklog 贡献指南

感谢您对 Inklog 项目的兴趣!本指南将帮助您了解如何为项目做出贡献。

## 目录

- [贡献指南概述](#贡献指南概述)
- [快速开始](#快速开始)
- [开发环境](#开发环境)
- [构建项目](#构建项目)
- [TDD 开发流程](#tdd-开发流程)
- [Pre-commit Hooks](#pre-commit-hooks)
- [代码质量工具](#代码质量工具)
- [测试](#测试)
- [代码风格](#代码风格)
- [文档](#文档)
- [提交变更](#提交变更)
- [代码审查](#代码审查)
- [社区](#社区)
- [致谢](#致谢)
- [快速参考](#快速参考)

---

## 贡献指南概述

### 为什么贡献 Inklog?

Inklog 是一个企业级 Rust 日志基础设施项目,专为高性能、高安全性和高可靠性环境设计。贡献 Inklog 意味着:

- **学习**: 探索现代 Rust 异步编程、内存安全和系统设计
- **影响**: 为数百个依赖 Inklog 的项目做出贡献
- **成长**: 提升 Rust 技能,学习企业级软件开发最佳实践
- **连接**: 加入活跃的 Rust 开发者社区

### 贡献类型

我们欢迎多种类型的贡献:

| 类型 | 描述 | 适合人群 |
|------|------|----------|
| **代码贡献** | 新功能、bug 修复、性能优化 | 有 Rust 经验的开发者 |
| **文档** | 改进文档、添加示例、更新指南 | 所有贡献者 |
| **测试** | 添加测试用例、改进测试覆盖率 | 质量保证爱好者 |
| **问题报告** | 报告 bug、提出功能请求 | 所有用户 |
| **审查** | 审查 PR、提供反馈 | 高级贡献者 |
| **设计** | 架构设计、API 设计讨论 | 高级开发者 |

---

## 快速开始

### 首次贡献步骤

```bash
# 1. Fork 仓库
访问 https://github.com/Kirky-X/inklog,点击 "Fork" 按钮

# 2. 克隆您的 Fork
git clone https://github.com/YOUR_USERNAME/inklog.git
cd inklog

# 3. 添加上游仓库
git remote add upstream https://github.com/Kirky-X/inklog.git

# 4. 创建功能分支
git checkout -b feature/your-feature

# 5. 进行修改
# ... 进行代码修改 ...

# 6. 运行测试
cargo test --all-features

# 7. 提交变更
git add .
git commit -m "feat: add your feature"

# 8. 推送到您的 Fork
git push origin feature/your-feature

# 9. 创建 Pull Request
访问 GitHub,点击 "New Pull Request"
```

### 推荐工具

我们推荐以下工具来提高开发效率:

| 工具 | 用途 | 安装 |
|------|------|------|
| **rustup** | Rust 版本管理 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` |
| **cargo-watch** | 自动重新编译 | `cargo install cargo-watch` |
| **cargo-expand** | 宏展开调试 | `cargo install cargo-expand` |
| **fd** | 快速文件搜索 | `cargo install fd-find` |
| **ripgrep** | 代码搜索 | `cargo install ripgrep` |
| **exa** | 彩色 ls 替代 | `cargo install exa` |

---

## 开发环境

### 系统要求

| 要求 | 最低版本 | 推荐版本 |
|------|---------|---------|
| Rust | 1.85.0 | 最新稳定版 |
| Git | 2.0 | 最新版 |
| Cargo | 1.85.0 | 最新稳定版 |
| 内存 | 4GB | 8GB+ |
| 磁盘 | 2GB | 5GB+ |

### Rust 版本管理

```bash
# 检查当前版本
rustc --version
cargo --version

# 安装或更新 Rust
rustup update stable

# 安装特定版本
rustup install 1.85.0

# 设置默认版本
rustup default 1.85.0

# 查看已安装版本
rustup toolchain list
```

### 必需依赖

#### Linux (Ubuntu/Debian)

```bash
# 更新包列表
sudo apt update

# 安装构建工具
sudo apt install -y build-essential pkg-config libssl-dev

# 安装 Docker (用于集成测试)
sudo apt install -y docker.io
sudo systemctl start docker
sudo usermod -aG docker $USER
```

#### macOS

```bash
# 安装 Homebrew (如果未安装)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 安装依赖
brew install openssl pkg-config

# 设置 OpenSSL 环境变量
export OPENSSL_LIB_DIR=$(brew --prefix openssl)/lib
export OPENSSL_INCLUDE_DIR=$(brew --prefix openssl)/include
```

#### Windows

```bash
# 安装 Visual Studio C++ 构建工具
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# 安装 Rust for Windows
rustup default stable-msvc

# 安装 Perl (用于 OpenSSL 构建)
# https://strawberryperl.com/
```

### 可选依赖

| 依赖 | 用途 | 安装 |
|------|------|------|
| **Docker** | 数据库集成测试 | https://docs.docker.com/get-docker/ |
| **Docker Compose** | 多服务测试 | https://docs.docker.com/compose/install/ |
| **just** | 命令行任务运行 | `cargo install just` |

### 环境验证

```bash
# 验证 Rust 环境
rustc --version
cargo --version

# 验证构建工具
cc --version

# 验证 OpenSSL (如果需要)
pkg-config --modversion openssl

# 克隆并构建测试
git clone --depth 1 https://github.com/Kirky-X/inklog.git /tmp/inklog-test
cd /tmp/inklog-test
cargo check --all-features
```

---

## 构建项目

### 基本构建

```bash
# Debug 构建 (默认)
cargo build

# 完整特性构建
cargo build --all-features

# 发布构建
cargo build --release

# 构建特定包
cargo build -p inklog
```

### 构建诊断

```bash
# 检查代码 (不生成二进制文件)
cargo check
cargo check --all-features

# 检查文档
cargo doc
cargo doc --all-features
cargo doc --document-private-items

# 检查依赖
cargo tree
cargo tree -i dependency_name
```

### 常见构建问题

#### 问题 1: OpenSSL 找不到

```bash
# Linux
sudo apt install libssl-dev

# macOS
brew install openssl
export OPENSSL_ROOT_DIR=$(brew --prefix openssl)
export OPENSSL_LIB_DIR=$OPENSSL_ROOT_DIR/lib
export OPENSSL_INCLUDE_DIR=$OPENSSL_ROOT_DIR/include
```

#### 问题 2: 内存不足

```bash
# 减少并行构建任务
cargo build -j 2
```

---

## TDD 开发流程

本项目遵循 **TDD（测试驱动开发）** 循环。每个开发任务组必须按照以下步骤执行:

### 循环: Red → Green → Commit → Analyze → Next

#### 1. 定接口

先定义 trait / API 签名,不写实现:

```rust
pub trait LogSink: Send + Sync {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;
    async fn flush(&self) -> Result<(), InklogError>;
    async fn shutdown(&self) -> Result<(), InklogError>;
}
```

#### 2. 写测试 (Red)

基于接口编写单元测试,此时测试应失败:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sink_write() {
        let sink = MockSink::new();
        let record = LogRecord::new(LogLevel::Info, "test message");
        assert!(sink.write(&record).await.is_ok());
    }
}
```

#### 3. 写代码 (Green)

实现接口,使测试通过:

```rust
pub struct MockSink {
    records: Arc<Mutex<Vec<LogRecord>>>,
}

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
cargo test --features <对应特性> --lib
```

确保所有测试通过。

#### 5. Commit

```bash
git add . && git commit -m "feat(<模块>): <描述>"
```

#### 6. Gitnexus Analyze

用 gitnexus 工具分析本任务对其他模块的影响,识别需联动修改的代码:

```bash
npx gitnexus analyze
```

#### 7. 继续下一个

基于 analyze 结果调整后续任务,再开始下一轮循环。

### 测试要求

- 测试要验证正确行为的有意义属性（值、结构、副作用、错误类型）
- "所有测试通过"是必要条件但非充分条件
- 测试太弱时要明确指出并改进

---

## Pre-commit Hooks

本项目使用自定义 pre-commit 脚本 (`scripts/pre-commit`) 来确保代码质量。

### 安装

```bash
# 安装 pre-commit hook
./scripts/install-pre-commit.sh
```

### Hook 执行内容

pre-commit 脚本会在每次 `git commit` 时运行以下检查:

| 检查项 | 命令 | 说明 |
|--------|------|------|
| **代码格式化** | `cargo fmt -- --check` | 确保代码格式一致 |
| **Clippy 检查** | `cargo clippy --all-features -- -D warnings` | 所有警告视为错误 |
| **编译检查** | `cargo check --all-features` | 确保代码可编译 |
| **单元测试** | `cargo test --lib --all-features -- --test-threads=4` | 运行完整测试套件 |

> **注意**: pre-commit 脚本运行完整测试套件（837+ 测试），可能需要 30-60 秒。

### 禁止事项

- **禁止使用 `--no-verify`** 跳过 pre-commit hooks
- **禁止直接提交到 main 分支** — 必须创建 feature 分支
- **禁止通过注释 CI 步骤来绕过质量门禁**
- **禁止临时降低覆盖率门禁阈值以通过 CI**

### 分支策略

```bash
# 创建 feature 分支
git checkout -b feature/your-feature

# 提交变更
git add <specific-files>
git commit -m "feat(scope): description"

# 合并回 main
git checkout main
git merge feature/your-feature
```

---

## 代码质量工具

本项目使用以下工具进行代码质量保证:

### diting — 代码质量审查

用于代码质量审查任务,包括 review、audit、tech debt 分析和 agent 审计。

```bash
# 触发代码审查
# 在 PR 中使用 diting 进行自动化代码审查
```

**适用场景**:
- PR 审查 (pull request review)
- 代码质量审计 (audit)
- 技术债务分析 (tech debt)
- 过度工程检测 (over-engineering)
- 代码简化 (simplify)

### tiangang — SAST 安全审查

专业 SAST 安全审查工具集,运行 Semgrep/CodeQL 等扫描器产出统一报告。

```bash
# 发布前安全审查
# tiangang 扫描 0 个 CRITICAL 漏洞才允许继续
```

**适用场景**:
- 安全审查 (security audit)
- 漏洞扫描 (vulnerability scan)
- SAST 代码安全检查
- 发布前安全检查

**发布前强制流程**:
1. tiangang SAST 扫描 — 0 个 CRITICAL 漏洞才允许继续
2. diting 代码审查 — 无 HIGH 级别问题才允许打 tag

### kueiku — 方法论导航

用于需要结构化思考和决策的场景,提供分析方法论框架。

**适用场景**:
- 技术选型决策
- 根因分析
- 优先级排序
- 风险预演
- 架构优化分析

---

## 测试

### 测试类型

| 测试类型 | 运行命令 | 目的 |
|----------|----------|------|
| **单元测试** | `cargo test --lib` | 测试单个模块 |
| **集成测试** | `cargo test --test '*'` | 测试模块交互 |
| **文档测试** | `cargo test --doc` | 验证文档示例 |
| **基准测试** | `cargo bench` | 性能测试 |
| **全部测试** | `cargo test --all-features` | 完整测试套件 |

### 运行测试

```bash
# 运行所有测试
cargo test --all-features

# 运行特定测试
cargo test --all-features test_log_rotation
cargo test --all-features --test integration_file_sink

# 运行文档测试
cargo test --doc

# 并行运行测试
cargo test --all-features -- --test-threads=4

# 显示测试输出
cargo test --all-features -- --nocapture
```

### 测试数据库集成

```bash
# 启动 Docker 服务 (PostgreSQL, MySQL)
docker-compose up -d postgres mysql

# 运行集成测试
cargo test --all-features --test '*'

# 清理
docker-compose down
```

---

## 代码风格

### 代码格式化

我们使用 Rust 官方的 `rustfmt` 进行代码格式化:

```bash
# 格式化所有代码
cargo fmt --all

# 检查格式(不修改文件)
cargo fmt --all -- --check
```

### Clippy 检查

Clippy 是 Rust 的 Lint 工具,我们将其警告视为错误:

```bash
# 运行 Clippy (所有警告作为错误)
cargo clippy --all-targets --all-features -- -D warnings
```

### 命名约定

- **变量和函数**: snake_case (`file_path`, `write_log_record`)
- **结构体和枚举**: PascalCase (`LoggerManager`, `SinkStatus`)
- **常量**: UPPER_SNAKE_CASE (`DEFAULT_CHANNEL_CAPACITY`, `MAX_RETRY_ATTEMPTS`)

### 文档注释

所有公共 API 必须有文档注释:

```rust
/// 文件日志 Sink 实现
///
/// 提供:
/// - 自动日志轮转 (基于大小或时间)
/// - 压缩支持 (ZSTD, GZIP, Brotli, LZ4)
/// - AES-256-GCM 加密
/// - 断路器保护
pub struct FileSink {
    config: FileSinkConfig,
    // ...
}
```

### 错误处理规范

- **错误类型定义**: 使用 `thiserror`
- **错误上下文**: 使用 `anyhow::Context`
- **禁止** `unwrap()` 或 `expect()` 在生产代码中
- **错误必须显性化**: 抛出、返回或上报,严禁吞掉

### 异步编程规范

- 使用 Tokio 异步运行时
- **禁止** 在异步上下文中阻塞 (使用 `tokio::spawn` 或 `tokio::task::spawn_blocking`)
- **禁止** 使用 `std::thread` (使用 tokio tasks)

### 反模式 (Anti-Patterns)

| 反模式 | 正确做法 |
|--------|----------|
| 在 async 上下文中使用 `std::thread` | 使用 `tokio::spawn` |
| 阻塞操作在 async 函数中 | 使用 `tokio::task::spawn_blocking` |
| 提交 `logs/` 目录到 git | 添加到 `.gitignore` |
| 硬编码密钥 | 从环境变量读取 |
| 不处理错误 | 使用 `anyhow::Context` 或 `thiserror` |
| 使用 `unwrap()` 或 `expect()` | 适当的错误处理 |

---

## 文档

### 文档类型

- **README.md**: 项目概述和快速开始
- **CONTRIBUTING.md**: 贡献指南 (本文档)
- **CHANGELOG.md**: 变更日志
- **AGENTS.md**: AI Agent 指南
- **docs/ARCHITECTURE.md**: 系统架构和设计决策
- **docs/SECURITY.md**: 安全最佳实践和特性
- **Rustdoc**: API 文档 (`cargo doc`)

### 文档更新清单

当修改代码时,请同步更新以下文档:

- [ ] API 文档 (rustdoc 注释)
- [ ] README.md (如果影响用户可见功能)
- [ ] CHANGELOG.md (记录变更)
- [ ] 示例代码 (如果添加新功能)
- [ ] ARCHITECTURE.md (如果是架构变更)

> **规则**: 禁止"先发布再补文档"——文档是发布的一部分。

---

## 提交变更

### Pull Request 流程

#### 1. 准备分支

```bash
# 确保主分支是最新的
git checkout main
git pull origin main

# 创建功能分支
git checkout -b feature/your-feature
```

#### 2. 进行修改

```bash
# 进行代码修改
# ...

# 运行测试
cargo test --all-features

# 运行 Clippy
cargo clippy --all-targets --all-features -- -D warnings

# 格式化代码
cargo fmt --all
```

#### 3. 提交变更

```bash
# 添加修改的文件 (优先指定文件名,避免 git add .)
git add src/support/io/sink/file.rs tests/integration_file_sink.rs

# 提交 (遵循 Conventional Commits)
git commit -m "feat(file): add LZ4 compression support

- Add LZ4 codec to compression options
- Update benchmark results for LZ4
- Add integration tests for LZ4 compression

Closes #123"
```

#### 4. 推送到远程

```bash
# 推送分支到远程仓库
git push origin feature/your-feature
```

#### 5. 创建 Pull Request

1. 访问 GitHub 仓库
2. 点击 "New Pull Request"
3. 选择您的分支作为源
4. 填写 PR 模板

### Pull Request 模板

```markdown
## 描述

简要描述此 PR 的目的和实现的功能。

## 变更类型

- [ ] Bug 修复 (不破坏现有功能)
- [ ] 新功能 (不破坏现有功能)
- [ ] 破坏性变更 (导致现有功能不可用)
- [ ] 文档更新

## 相关 Issue

Closes #(issue number)

## 测试

- [ ] 添加了单元测试
- [ ] 添加了集成测试
- [ ] 现有测试通过
- [ ] 代码覆盖率 > 95%

## 检查清单

- [ ] 代码遵循项目风格规范
- [ ] 已通过 Clippy 检查
- [ ] 已通过 `cargo fmt`
- [ ] 添加了必要的文档
- [ ] 更新了 CHANGELOG.md
- [ ] 所有测试通过
```

### 自动化检查

您的 PR 将通过以下 CI/CD 检查:

| 检查 | 说明 | 失败处理 |
|------|------|----------|
| **格式化** | `cargo fmt -- --check` | 运行 `cargo fmt` |
| **Clippy** | `cargo clippy` | 修复警告 |
| **测试** | `cargo test --all-features` | 修复失败的测试 |
| **安全审计** | `cargo deny check` | 更新或移除不安全依赖 |

### PR 合并策略

- **Squash and Merge**: 用于功能分支,单个提交
- **Rebase and Merge**: 用于维护更新
- **Merge Commit**: 避免使用

---

## 代码审查

### 审查标准

代码审查关注以下方面:

1. **功能性**: 代码是否实现预期功能
2. **安全性**: 是否引入安全漏洞
3. **性能**: 是否有性能问题
4. **可读性**: 代码是否易于理解
5. **测试**: 测试是否充分
6. **文档**: 文档是否准确完整

### 审查响应时间

- **小改动**: 1-2 个工作日
- **中等改动**: 3-5 个工作日
- **大改动**: 1-2 周

---

## 社区

### 沟通渠道

| 渠道 | 用途 | 链接 |
|------|------|------|
| **GitHub Issues** | Bug 报告、功能请求 | https://github.com/Kirky-X/inklog/issues |
| **GitHub Discussions** | 问答、讨论、想法分享 | https://github.com/Kirky-X/inklog/discussions |
| **Pull Requests** | 代码贡献 | https://github.com/Kirky-X/inklog/pulls |

### 行为准则

- **尊重**: 尊重不同的观点和经验
- **包容**: 欢迎所有背景的贡献者
- **建设性**: 提供建设性的反馈
- **专注**: 关注对项目最有利的事情

---

## 致谢

感谢所有为 Inklog 项目做出贡献的人!

### 特别感谢

- **tracing**: Rust 结构化日志生态系统
- **tokio**: 异步运行时
- **Sea-ORM**: 异步 ORM
- **Rust 社区**: 提供优秀的工具和库

---

## 快速参考

### 常用命令

```bash
# 开发循环
cargo watch -x 'check'
cargo watch -x 'test --all-features'

# 完整检查
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# 发布准备
cargo build --release
cargo test --release
cargo doc --all-features

# 覆盖率
cargo tarpaulin --out Html --all-features

# 安全审计
cargo deny check
```

### 项目结构

```
inklog/
- src/
  - lib.rs              # 公共 API 入口
  - domain/             # 领域层
    - core/             # 核心日志管理 (manager.rs, container.rs)
    - config/           # 配置系统
    - types/            # 类型定义 (error, log_record)
    - db_provider.rs    # 数据库提供者
  - support/            # 支撑层
    - io/               # I/O (sink, log_adapter)
    - processing/       # 处理 (masking, template, object_pool)
    - observability/     # 可观测性 (metrics)
  - integrations/       # 集成层
    - kit/               # DI kit
    - infra/             # 基础设施适配器
    - dbnexus_adapter.rs # DB Nexus 适配器
  - cli/                # 命令行工具
  - validation/         # 验证 (path, sanitize)
  - log_level.rs        # 日志级别
- tests/                # 测试
- examples/             # 示例
- docs/                 # 文档
- scripts/              # 脚本 (pre-commit 等)
- Cargo.toml            # 项目配置
- README.md             # 项目说明
- CONTRIBUTING.md       # 贡献指南 (本文档)
- CHANGELOG.md          # 变更日志
- AGENTS.md             # AI Agent 指南
```

---

**文档版本**: 2.0
**最后更新**: 2026-07-11
**项目**: Inklog - Enterprise-grade Rust Logging Infrastructure

---

**有问题吗?** 请查看 [GitHub Discussions](https://github.com/Kirky-X/inklog/discussions) 或创建 [Issue](https://github.com/Kirky-X/inklog/issues)。
