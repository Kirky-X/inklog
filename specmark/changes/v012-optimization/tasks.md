# Tasks — v012-optimization

任务按执行/依赖顺序排列。`apply` 强制严格顺序执行，遇阻 PAUSE 不跳过。
每个产代码任务遵循 TDD 五步循环：Red（写失败测试）→ Green（写实现）→ Refactor → Commit → Verify。

---

## Phase 1: 依赖配置清理

- [x] [T001] [P0] 升级 `oxcache` 0.2.0 → 0.3.3 并启用 `memory`/`serialization`/`tracing` feature，删除 `moka`/`dashmap` 直接依赖
  - 文件：`Cargo.toml` 第 59-61 行
  - 改动：`oxcache = { version = "0.3.3", default-features = false, features = ["macros", "memory", "serialization", "tracing"] }`，删除 `moka = { ... }` 和 `dashmap = "6.1"` 两行（注：0.3.3 需要 serialization+tracing feature 解决上游 cfg-gate 缺陷）
  - 验证：`cargo check --all-features` 通过，`cargo tree -i moka` 显示只通过 oxcache 引入
  - TDD：写测试验证 `OxCacheAdapter::new()` 仍能正常 set/get（已有测试覆盖）

- [x] [T002] [P0] 拆分 `dbnexus` feature 为 `sqlite`/`postgres`/`mysql` 三个独立 feature
  - 文件：`Cargo.toml` 第 19-26 行 `[features]` 节
  - 改动：移除 `dbnexus = ["dep:dbnexus", "dep:sea-orm"]`，新增三行：
    - `sqlite = ["dep:dbnexus", "dep:sea-orm", "dbnexus/sqlite", "sea-orm/sqlx-sqlite"]`
    - `postgres = ["dep:dbnexus", "dep:sea-orm", "dbnexus/postgres", "sea-orm/sqlx-postgres"]`
    - `mysql = ["dep:dbnexus", "dep:sea-orm", "dbnexus/mysql", "sea-orm/sqlx-mysql"]`
  - 更新 `[lints.rust]` 的 `check-cfg` 添加新 feature 值
  - 验证：`cargo check --features sqlite`、`cargo check --features postgres`、`cargo check --features mysql` 各自通过

- [x] [T003] [P0] 切换 `sea-orm` TLS 后端 `runtime-tokio-native-tls` → `runtime-tokio-rustls`
  - 文件：`Cargo.toml` 第 70 行
  - 改动：`sea-orm = { version = "2.0.0-rc.37", default-features = false, features = ["runtime-tokio-rustls", "with-chrono"], optional = true }`
  - 验证：`cargo check --features sqlite` 通过，无 native-tls 残留

- [x] [T004] [P0] 添加 `RUSTSEC-2026-0173`（proc-macro-error2 unmaintained）到 `deny.toml` ignore 列表
  - 文件：`deny.toml` 第 5-8 行
  - 改动：在 `ignore` 数组追加 `"RUSTSEC-2026-0173", # proc-macro-error2 unmaintained (transitive via validator_derive, no safe upgrade available)`
  - 顺手清理：移除 5 个 `advisory-not-detected` 警告项（`RUSTSEC-2025-0134` rustls-pemfile / `RUSTSEC-2023-0071` rsa / `RUSTSEC-2023-0086` lexical-core / `RUSTSEC-2021-0145` atty / `RUSTSEC-2024-0375` atty unmaintained）—这些依赖在依赖升级后已不再引入
  - 验证：`cargo deny check advisories` 输出 `advisories ok`

---

## Phase 2: cfg 替换（dbnexus → sqlite/postgres/mysql）

- [x] [T005] [P0] 替换所有源码中 `#[cfg(feature = "dbnexus")]` 为 `#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]`
  - 文件：`src/support/io/sink/database/mod.rs`、`src/domain/core/manager.rs`、`src/domain/core/container.rs`、`src/integrations/infra/database.rs`、`src/integrations/kit/keys.rs`、`src/support/io/sink/mod.rs`、`src/integrations/infra/mod.rs`、`src/integrations/kit/mod.rs`
  - 测试/基准/示例同步：`benches/inklog_bench.rs`、`tests/integration_tests.rs`、`tests/unit_tests.rs`、`tests/unit/sink/{mod,entity_test}.rs`、`tests/integration/additional_tests.rs`、`tests/integration/batch/batch_write_test.rs`、`tests/performance/benchmark_test.rs`、`tests/combinations/complex_features_test.rs`、`tests/docker/main.rs`、`examples/src/bin/{database,parquet_archive}.rs`、`examples/src/lib.rs`、`examples/README.md`
  - 配套修复：`examples/Cargo.toml` 拆分 `dbnexus` feature 为 `sqlite`/`postgres`/`mysql` 三独立 feature 并正确转发；`.github/workflows/test-docker.yml` 将 `--features dbnexus` 改为 `--features ${{ matrix.db }}`
  - 特殊处理：`tests/docker/main.rs` 内部属性 `#![cfg(...)]` 单独 Edit；`tests/combinations/complex_features_test.rs` 的 `#[cfg(all(feature = "aws", feature = "dbnexus"))]` 改为 `#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]`（aws feature 已移除）
  - 文本引用：剩余 7 处 `--features dbnexus` 文档/eprintln 引用全部改为 `--features sqlite`（默认示例数据库）
  - 验证：`grep -rn 'feature = "dbnexus"' src/ tests/ benches/ examples/` 返回 0 结果；`grep -rn --features dbnexus` 全仓返回 0 结果；`cargo fmt --all -- --check` 通过；`cargo clippy --all-features --all-targets -- -D warnings` 退出码 0 无警告

---

## Phase 3: LogSink trait async 化

- [ ] [T006] [P0] 重写 `LogSink` trait 为 async trait
  - 文件：`src/support/io/sink/mod.rs`
  - TDD：
    - Red：写测试 `tokio::test` 验证 async write/flush/shutdown 可调用
    - Green：定义 `#[async_trait::async_trait] pub trait LogSink { async fn write(&self, record: &LogRecord) -> Result<(), InklogError>; async fn flush(&self) -> Result<(), InklogError>; async fn shutdown(&self) -> Result<(), InklogError>; }`
    - Refactor：保留 `LogSink::name()` 等非 async 方法
    - Commit：`refactor(sink): migrate LogSink trait to async (write/flush/shutdown)`
    - Verify：`cargo check --all-features`

- [ ] [T007] [P0] 重构 `FileSink` 实现为 async LogSink
  - 文件：`src/support/io/sink/file.rs`
  - TDD：
    - Red：写 async 测试 `#[tokio::test] async fn test_file_sink_async_write()`
    - Green：`#[async_trait] impl LogSink for FileSink { async fn write(...) { ... } async fn flush(...) { ... } async fn shutdown(...) { ... } }`，移除 sync 实现
    - Refactor：内部 `std::fs` 操作用 `tokio::fs` 或 `tokio::task::spawn_blocking` 包装
    - Commit：`refactor(file-sink): migrate FileSink to async LogSink trait`
    - Verify：`cargo test --features sqlite --lib file_sink`

- [ ] [T008] [P0] 重构 `ConsoleSink` 实现为 async LogSink
  - 文件：`src/support/io/sink/console.rs`
  - TDD：同 T007 模式，Red 测试 → Green 实现 → Refactor → Commit `refactor(console-sink): migrate ConsoleSink to async LogSink trait` → Verify

- [ ] [T009] [P0] 重构 `RingBufferedFile` 实现为 async LogSink
  - 文件：`src/support/io/sink/ring_buffered_file.rs`
  - TDD：同 T007 模式，Commit `refactor(ring-buffer): migrate RingBufferedFile to async LogSink trait`

- [ ] [T010] [P0] 重写 `DatabaseSink` 为原生 async，移除 `execute_async` 和 `block_in_place`
  - 文件：`src/support/io/sink/database/mod.rs`
  - TDD：
    - Red：写 async 测试验证 `DatabaseSink::write` 直接 await `database.insert_batch(...)`
    - Green：`#[async_trait] impl LogSink for DatabaseSink { async fn write(...) { ... self.database.insert_batch(...).await ... } }`，删除 `fn execute_async`
    - Refactor：保留 circuit_breaker、fallback_sink 逻辑
    - Commit：`refactor(database-sink): rewrite DatabaseSink as native async, remove block_in_place`
    - Verify：`cargo test --features sqlite --lib database_sink`

- [ ] [T011] [P0] 更新 `SinkRegistry` 处理 async LogSink
  - 文件：`src/support/io/sink/registry.rs`
  - TDD：Red 测试 async 注册/获取 → Green 实现 → Commit `refactor(sink-registry): support async LogSink` → Verify

- [ ] [T012] [P0] 更新 `LoggerManager` 内部 sink 调用为 async
  - 文件：`src/domain/core/manager.rs`
  - 改动：所有 `sink.write(...)` 改为 `sink.write(...).await`，`sink.flush()` 改为 `sink.flush().await`，`sink.shutdown()` 改为 `sink.shutdown().await`
  - 影响：worker task 已是 `tokio::spawn`，内部循环改为 async
  - Commit：`refactor(manager): update sink calls to async`
  - Verify：`cargo test --all-features --lib`

---

## Phase 4: ObjectPool 重构

- [ ] [T013] [P0] 移除 `ObjectPool` 的 `SHARED_RUNTIME` 和 `get_runtime`
  - 文件：`src/support/processing/object_pool.rs` 第 41-55 行
  - 改动：删除 `static SHARED_RUNTIME: Lazy<tokio::runtime::Runtime>` 和 `fn get_runtime()`
  - 验证：编译失败，提示需要 async context（预期）

- [ ] [T014] [P0] 将 `ObjectPool` 的 `new`/`with_config`/`builder` 改为 async（不含 `with_capacity`，该方法在 T022 中删除）
  - 文件：`src/support/processing/object_pool.rs`
  - TDD：
    - Red：写 `#[tokio::test] async fn test_object_pool_async_new()`
    - Green：`pub async fn with_config(config: ObjectPoolConfig) -> Result<Self, InklogError>`，`pub async fn new() -> Result<Self, InklogError>`
    - Refactor：`builder().build()` 改为 `async fn build()`
    - Commit：`refactor(object-pool): convert constructors to async, remove SHARED_RUNTIME`
    - Verify：`cargo test --lib object_pool`

- [ ] [T015] [P0] 将 `ObjectPool` 的 `get`/`put` 改为 async（`len` 保持 sync 仅读 atomic；`remove`/`contains`/`is_empty`/`capacity` 在 T022 中删除）
  - 文件：`src/support/processing/object_pool.rs`
  - TDD：Red async 测试 → Green 实现 → Commit `refactor(object-pool): convert methods to async` → Verify

- [ ] [T016] [P0] 更新 `LOG_RECORD_POOL`/`STRING_POOL` 全局 static 为 async 初始化
  - 文件：`src/support/processing/object_pool.rs` 底部
  - 改动：`static LOG_RECORD_POOL: Lazy<LogRecordPool>` → `static LOG_RECORD_POOL: tokio::sync::OnceCell<Arc<LogRecordPool>>`，提供 `pub async fn get_log_record_pool() -> &'static Arc<LogRecordPool>` 异步初始化函数
  - 同理 `STRING_POOL`
  - 更新 `lib.rs` 中的 re-export：`pub use support::processing::{get_log_record, get_string_buffer, ...}` 改为 async
  - Commit：`refactor(object-pool): convert LOG_RECORD_POOL/STRING_POOL to async OnceCell`
  - Verify：`cargo check --all-features`

- [ ] [T017] [P0] 更新 `ObjectPool` 所有调用方为 async
  - 文件：`src/domain/core/manager.rs`、`src/support/processing/template.rs`、`src/support/processing/masking.rs`、`benches/inklog_bench.rs`
  - 改动：`LOG_RECORD_POOL.get(&key)` → `get_log_record_pool().await.get(&key).await`
  - Commit：`refactor: update ObjectPool callers to async API`
  - Verify：`cargo test --all-features --lib`

---

## Phase 5: 错误处理显性化

- [ ] [T018] [P1] 移除 `OxCacheAdapter` 的 `Default` impl（避免 panic）
  - 文件：`src/integrations/infra/cache.rs` 第 158-162 行
  - 改动：删除 `impl Default for OxCacheAdapter { fn default() -> Self { Self::new().expect(...) } }`
  - TDD：Red 测试验证 `OxCacheAdapter::default()` 不再可调用 → Green 删除 impl → Commit `fix(cache): remove panicking Default impl for OxCacheAdapter`
  - Verify：`cargo check --all-features`

- [ ] [T019] [P1] 更新 `Cache` trait 方法签名返回 `Result`，移除 `tracing::warn!` 静默吞错
  - 文件：`src/integrations/infra/cache.rs`
  - TDD：
    - Red：写测试验证 `cache.get("missing").await` 返回 `Ok(None)` 而非 `None`，验证 `cache.get` 错误时返回 `Err`
    - Green：`async fn get(&self, key: &str) -> Result<Option<String>, InklogError>; async fn set(&self, key: &str, value: String) -> Result<(), InklogError>; async fn delete(&self, key: &str) -> Result<bool, InklogError>; async fn exists(&self, key: &str) -> Result<bool, InklogError>;`
    - Refactor：`OxCacheAdapter` 实现直接 `?` 传播错误，移除 `match ... tracing::warn! ... None`
    - Commit：`refactor(cache): return Result from Cache trait methods, eliminate silent error swallowing`
    - Verify：`cargo test --lib cache`

- [ ] [T020] [P1] 更新 `MockCache` 实现以匹配新 `Cache` trait 签名
  - 文件：`src/integrations/infra/cache.rs` 第 336-374 行
  - TDD：Red → Green → Commit `refactor(cache): update MockCache to new Result-returning trait`
  - Verify：所有 cache 测试通过

- [ ] [T021] [P1] 修复 `ObjectPool::with_config` 的 `Cache::default()` 静默回退
  - 文件：`src/support/processing/object_pool.rs`
  - 改动：在 T014 中已改为返回 `Result`，本任务确保 `builder.build().await` 失败时返回 `Err`，不再用 `Cache::default()` fallback
  - Commit：`fix(object-pool): eliminate Cache::default() silent fallback, propagate build errors`
  - Verify：`cargo test --lib object_pool`

---

## Phase 6: 代码清理

- [ ] [T022] [P1] 移除 `ObjectPool` 中所有 `#[allow(dead_code)]` 标记的未使用方法
  - 文件：`src/support/processing/object_pool.rs`
  - 改动：删除 `with_capacity`、`remove`、`contains`、`is_empty`、`capacity`、`stats` 等 `#[allow(dead_code)]` 方法
  - 保留：`new`、`with_config`、`builder`、`get`、`put`、`len` 等实际使用的方法
  - Commit：`refactor(object-pool): remove dead code methods (YAGNI)`
  - Verify：`cargo check --all-features` 无 dead_code 警告

- [ ] [T023] [P1] 清理 `confers` 残留引用
  - 文件：`src/support/io/sink/file.rs:1359`、`src/domain/core/manager.rs:304`
  - 改动：删除两处注释 `// Note: confers derive generates...` 和 `// 使用 confers 自动生成的方法加载配置`
  - Commit：`docs: remove stale confers references in source comments`
  - Verify：`grep -rn 'confers' src/` 仅返回 0 结果

---

## Phase 7: 文档同步

- [ ] [T024] [P0] 重写 `docs/CHANGELOG.md` `[Unreleased]` 节为 `[0.1.2]`
  - 文件：`docs/CHANGELOG.md`
  - 改动：
    - 移除 [Unreleased] 中所有过时引用：`src/manager.rs`、`src/sink/file.rs`、`ConfersAdapter`、S3 archive 相关项、`v0.2.0` 标题
    - 新增 `[0.1.2] - 2026-07-XX` 节，按实际改动重写：依赖重构（oxcache 升级、dbnexus feature 拆分、sea-orm TLS）、LogSink trait async 化、ObjectPool 重构、错误处理显性化、文档同步
    - 更新底部链接：`[Unreleased]: .../compare/v0.1.2...HEAD`、`[0.1.2]: .../compare/v0.1.1...v0.1.2`
  - Commit：`docs: rewrite CHANGELOG for v0.1.2 release`
  - Verify：`grep -n 'v0.2.0\|ConfersAdapter\|src/manager.rs' docs/CHANGELOG.md` 返回 0 结果

- [ ] [T025] [P1] 更新 `README.md` 和 `README_zh.md`
  - 文件：根目录 `README.md`、`README_zh.md`
  - 改动：
    - 移除 `confers` 引用
    - 更新 feature 列表：`sqlite`/`postgres`/`mysql` 替代 `dbnexus`
    - 更新依赖描述：移除 moka/dashmap
    - 更新示例代码：sink 调用改为 async（如 `sink.write(&record).await`）
  - Commit：`docs: update README/README_zh for v0.1.2`
  - Verify：`grep -n 'confers\|dbnexus feature' README*.md` 返回 0 结果

- [ ] [T026] [P1] 更新 `docs/ARCHITECTURE.md`、`docs/USER_GUIDE.md`、`docs/API_REFERENCE.md`
  - 文件：`docs/ARCHITECTURE.md`、`docs/USER_GUIDE.md`、`docs/API_REFERENCE.md`
  - 改动：
    - 移除 `confers` 引用
    - 更新模块路径：`src/manager.rs` → `src/domain/core/manager.rs`、`src/sink/file.rs` → `src/support/io/sink/file.rs`
    - 更新 feature 描述：`sqlite`/`postgres`/`mysql`
    - 更新 API 示例：async 签名
  - Commit：`docs: update ARCHITECTURE/USER_GUIDE/API_REFERENCE for v0.1.2`
  - Verify：`grep -rn 'confers' docs/` 返回 0 结果（archive/ 除外）

---

## Phase 8: 版本发布与 Issue 闭环

- [ ] [T027] [P0] 更新 `Cargo.toml` 版本号 0.1.1 → 0.1.2
  - 文件：`Cargo.toml` 第 3 行
  - 改动：`version = "0.1.2"`
  - Commit：`chore: bump version to 0.1.2`
  - Verify：`cargo read-manifest | grep version`

- [ ] [T028] [P0] 更新 `examples/Cargo.toml` 版本号同步
  - 文件：`examples/Cargo.toml`
  - 改动：`inklog = { version = "0.1.2", ... }`
  - Commit：`chore(examples): sync inklog dependency to 0.1.2`

- [ ] [T029] [P1] 更新 `examples/src/bin/*.rs` 中所有 sink/ObjectPool 调用为 async
  - 文件：`examples/src/bin/basic.rs`、`builder.rs`、`file.rs`、`console.rs`、`database.rs`、`database_pg_mysql.rs`、`object_pool.rs`、`metrics.rs`、`circuit_breaker.rs` 等
  - 改动：所有 `sink.write(...)` → `sink.write(...).await`，`ObjectPool::new()` → `ObjectPool::new().await?`
  - Commit：`refactor(examples): update all examples to async sink/ObjectPool API`
  - Verify：`cargo build --all-features --examples`

- [ ] [T030] [P1] 在 GitHub issue #1 评论说明 v0.1.1+ 已修复，v0.1.2 发布后关闭
  - 操作：使用 `gh issue comment 1 --repo Kirky-X/inklog --body "..."` 评论
  - 内容：说明 v0.1.1 已用 `nix::sys::statfs` 替代 `statvfs`、移除 confers/path 依赖、oxcache 已发布到 crates.io；v0.1.2 进一步优化依赖与架构
  - 关闭：`gh issue close 1 --repo Kirky-X/inklog --reason completed`
  - 注意：此任务需用户确认 GitHub token 可用后执行；可在 v0.1.2 crates.io 发布后执行

---

## Phase 9: 全量回归测试

- [ ] [T031] [P0] 运行 `cargo test --all-features --workspace`，确保所有测试通过
  - 命令：`cargo test --all-features --workspace 2>&1 | tee /tmp/test-output.log`
  - 验证：exit code 0，无 FAIL
  - 失败处理：PAUSE，逐项修复后重跑

- [ ] [T032] [P0] 运行 `cargo clippy --all-targets --all-features -- -D warnings`
  - 命令：`cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/clippy-output.log`
  - 验证：exit code 0，无 warning
  - 失败处理：PAUSE，逐项修复

- [ ] [T033] [P0] 运行 `cargo fmt --all -- --check`
  - 命令：`cargo fmt --all -- --check 2>&1 | tee /tmp/fmt-output.log`
  - 验证：exit code 0
  - 失败处理：运行 `cargo fmt --all` 修复后重跑

- [ ] [T034] [P0] 运行 `cargo deny check`
  - 命令：`cargo deny check 2>&1 | tee /tmp/deny-output.log`
  - 验证：advisories/bans/licenses/sources 全部 ok
  - 失败处理：PAUSE，检查是否漏加 ignore 或新 advisory

- [ ] [T035] [P0] 运行 `cargo audit`
  - 命令：`cargo audit --file Cargo.lock 2>&1 | tee /tmp/audit-output.log`
  - 验证：仅允许已 ignore 的 advisories（RUSTSEC-2024-0436、RUSTSEC-2025-0134、RUSTSEC-2023-0071、RUSTSEC-2023-0086、RUSTSEC-2021-0145、RUSTSEC-2024-0375、RUSTSEC-2026-0173）

- [ ] [T036] [P0] 运行 `cargo tarpaulin --out Html --all-features`，验证覆盖率 ≥ 90%
  - 命令：`cargo tarpaulin --out Html --all-features 2>&1 | tee /tmp/tarp-output.log`
  - 验证：`INFO File::coverage` 行显示 `>=90.00%`
  - 失败处理：PAUSE，分析未覆盖代码路径，补充测试

---

## Phase 10: 收尾

- [ ] [T037] [P0] 提交所有变更并打 tag `v0.1.2`
  - 命令：
    - `git status` 确认无未提交变更
    - `git log --oneline -10` 确认 commit 历史
    - `git tag v0.1.2`
    - `git push origin v0.1.2`（用户确认后）
  - 注意：crates.io 发布需用户执行 `cargo publish --dry-run` 验证后 `cargo publish`

## Phase N: Convergence
<仅由 /specmark converge 追加，propose 不写本节>
