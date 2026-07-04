# v012-optimization

## Motivation

inklog v0.1.1 (2026-06-29 发布) 已修复 issue #1 的 statvfs 类型不匹配和路径依赖问题，但留下了若干技术债：

1. **依赖配置错误**：`Cargo.toml` 同时声明 `oxcache`、`moka`、`dashmap` 三个直接依赖，但 `moka`/`dashmap` 在源码中无任何 `use` 引用——它们实际是 `oxcache` 的 `memory` feature 的传递依赖。这违反了"通过特性使用而不是默认特性"的工程原则。
2. **feature 设计缺陷**：`dbnexus` feature 只硬编码 sqlite 后端，用户无法通过 inklog 的 feature gate 选择 postgres/mysql。
3. **架构反模式**：`ObjectPool` 自建 `tokio::runtime::Runtime`（`SHARED_RUNTIME`），`DatabaseSink::execute_async` 强制使用 `block_in_place`——单线程 tokio runtime 下会 panic。
4. **隐性 bug 与代码质量问题**：`OxCacheAdapter::default().expect()` 在库代码中 panic、`Cache::default()` 静默回退掩盖错误、大量 `#[allow(dead_code)]` 违反 YAGNI、`tracing::warn!` 吞错违反显性化原则。
5. **文档与代码 drift**：CHANGELOG `[Unreleased]` 引用已删除的 `src/manager.rs`、`ConfersAdapter`、S3 archive；8 个文件残留 `confers` 引用；`deny.toml` 未覆盖最新的 RUSTSEC-2026-0173。
6. **依赖过期**：`oxcache` 0.2.0 → 0.3.2（2026-07-02 发布）；`proc-macro-error2` unmaintained 警报未忽略。

v0.1.2 版本目标是系统性解决上述技术债，使 inklog 在依赖管理、架构设计、代码质量、文档一致性四个维度达到企业级标准。

## Scope

### 依赖管理
- 升级 `oxcache` 0.2.0 → 0.3.2，启用 `memory` feature 替代直接的 `moka`/`dashmap` 依赖
- 删除 `Cargo.toml` 中的 `moka`、`dashmap` 直接依赖
- 拆分 `dbnexus` feature 为 `sqlite`、`postgres`、`mysql` 三个独立 feature，每个透传 `dbnexus`/`sea-orm` 对应后端 feature
- 统一 `sea-orm` 使用 `runtime-tokio-rustls`（与 `dbnexus` 默认一致），按 feature 启用 `sqlx-sqlite`/`sqlx-postgres`/`sqlx-mysql`
- 添加 `RUSTSEC-2026-0173`（proc-macro-error2 unmaintained）到 `deny.toml` ignore 列表

### 架构重构
- **LogSink trait async 化**：`write`/`flush`/`shutdown` 改为 `async fn`
- 重构 `FileSink`、`ConsoleSink`、`RingBufferedFile`、`DatabaseSink` 全部 sink 实现为原生 async
- 移除 `ObjectPool::SHARED_RUNTIME`，强制要求调用方在 async 上下文使用
- 移除 `DatabaseSink::execute_async` 的 `block_in_place` hack
- `LoggerManager` 内部 sink 调用改为 async

### 代码质量
- 移除 `ObjectPool` 中所有 `#[allow(dead_code)]` 标记的未使用方法（YAGNI）
- `OxCacheAdapter::default()` 改为返回 `Result`，不再 panic
- `ObjectPool::with_config` 在 `Cache::builder().build()` 失败时返回 `Err` 而非静默用 `Cache::default()`
- `OxCacheAdapter` 的 `get`/`set`/`delete`/`exists` 错误处理改为返回 `Result`（移除 `tracing::warn!` + None/false 静默吞错）
- `Lazy::new` 的 `panic!` 改为 `Result` 返回或更优雅的失败模式

### 文档一致性
- 重写 `docs/CHANGELOG.md` `[Unreleased]` 节为 `[0.1.2]`，更新路径引用（`src/manager.rs` → `src/domain/core/manager.rs` 等）、移除 `ConfersAdapter`/S3 archive 引用
- 清理 8 个文件中的 `confers` 残留（docs × 6、src 注释 × 2）
- 同步更新 `README.md`、`README_zh.md`、`docs/ARCHITECTURE.md`、`docs/USER_GUIDE.md`、`docs/API_REFERENCE.md` 中的依赖描述和 feature 列表
- 在 GitHub issue #1 评论说明 v0.1.1+ 已修复，发布 v0.1.2 后关闭

### 测试与验证
- 全量回归测试：`cargo test --all-features --workspace` 通过
- 静态检查：`cargo clippy --all-targets --all-features -- -D warnings`、`cargo fmt --all -- --check`
- 安全审计：`cargo deny check`、`cargo audit` 通过（除 allow-listed advisories）
- 覆盖率：`cargo tarpaulin --out Html` ≥ 90%

### 版本发布
- `Cargo.toml` 版本号 0.1.1 → 0.1.2
- `examples/Cargo.toml` 版本号同步
- 打 tag `v0.1.2`

## Non-Goals

- **不引入新功能**：v0.1.2 是技术债清理版本，不增加新 API 或新 sink 类型
- **不重写 LoggerManager 核心**：仅修改 sink 调用方式（sync → async），不重构 LoggerManager 整体架构
- **不修改 dbnexus/oxcache 上游**：按 AGENTS.md "DO NOT modify external dependencies"，问题报告给上游但不 fork
- **不引入 metrics feature 的实际实现**：现有 `metrics = []` feature 保留为空，待后续版本实现
- **不替换 parquet 依赖**：parquet 仍为默认依赖，不改为 optional feature（影响范围过大，留给后续版本）
- **不修改 `[[bin]] path = "src/cli/mod.rs"` 非标准路径**：虽不优雅但工作正常，不在本版本动
- **不删除 `RUSTSEC-2023-0071`（rsa Marvin Attack）的 ignore**：上游无修复，保持现状

## Clarifications

- **[Functional Scope]** Q: oxcache 0.2.0 → 0.3.2 升级策略？
  A: 升级到 0.3.2 + 启用 memory feature + 删 moka/dashmap 直接依赖

- **[Functional Scope]** Q: dbnexus feature 重构方案？
  A: 拆分为 sqlite/postgres/mysql 三个独立 feature，废弃 dbnexus feature

- **[Functional Scope]** Q: DatabaseSink::execute_async 的 block_in_place 修复策略？
  A: 完全重写 DatabaseSink 为原生 async trait（LogSink trait 整体 async 化）

- **[Functional Scope]** Q: ObjectPool 自建 tokio runtime 如何处理？
  A: 移除自建 runtime，强制要求调用方在 async 上下文使用

- **[Functional Scope]** Q: proc-macro-error2 (RUSTSEC-2026-0173) unmaintained 处理方式？
  A: 添加到 deny.toml ignore 列表（无安全升级可用）

## NEEDS CLARIFICATION

无。所有歧义点已在 Clarifications 节解决。
