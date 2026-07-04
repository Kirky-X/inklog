# Design — v012-optimization

## Context

inklog 当前的架构问题集中在三个层面：

1. **依赖层**：`Cargo.toml` 同时声明 `oxcache`（仅启用 `macros` feature）和 `moka`/`dashmap`（直接依赖），但 `cache.rs` 调用 `OxCache::new()` 时实际需要 `oxcache` 的 `memory` feature（该 feature 内部才启用 `dep:moka`、`dep:dashmap`）。当前配置下，`OxCache::new()` 的行为依赖 cargo feature unification 才能工作，是隐性 bug。
2. **架构层**：`LogSink` trait 设计为 sync（`fn write(&self, ...) -> Result<()>`），但 `DatabaseSink` 内部需要 async 调用 dbnexus。当前用 `tokio::task::block_in_place(|| Handle::current().block_on(f))` 桥接，问题：
   - 单线程 tokio runtime 下 `block_in_place` 会 panic
   - 在 `spawn_blocking` 内调用会 deadlock
   - 与 `ObjectPool` 的 `Handle::try_current()` + fallback 不一致
3. **代码质量层**：库代码中存在多处 panic 路径（`Lazy::new().unwrap_or_else(panic!)`、`OxCacheAdapter::default().expect()`）和静默错误（`Cache::default()` fallback、`tracing::warn!` + None）。

约束：
- AGENTS.md 禁止 backward compatibility → 可以做破坏性 API 变更
- AGENTS.md 禁止 `std::thread` → 必须用 tokio
- AGENTS.md 要求 tarpaulin 覆盖率 ≥ 95%（项目实际目标 90%）
- rustc 1.93.1，sea-orm 必须 pin 到 rc.37（项目记忆）

## Decision

### D1: oxcache 依赖重构

**决策**：
```toml
# Cargo.toml
oxcache = { version = "0.3.2", default-features = false, features = ["macros", "memory"] }
# 删除：moka = { ... }
# 删除：dashmap = { ... }
```

**理由**：
- `default-features = false` 避免引入 `redis`/`compression`/`lua-script` 等不需要的传递依赖
- `memory` feature 启用 `dep:moka` + `dep:dashmap`，由 oxcache 统一管理版本
- `macros` feature 保留以支持 `oxcache::Cache` 派生宏

**API 兼容性**：需验证 0.2.0 → 0.3.2 的 `Cache::new()`、`Cache::builder().ttl().capacity().build()`、`Cache::get/set/delete/exists` 签名。若 `Cache::default()` 在 0.3.2 中行为变化，需同步调整 `ObjectPool::with_config` 的 fallback 逻辑（本来就要改）。

### D2: dbnexus feature 拆分

**决策**：
```toml
# Cargo.toml
[features]
default = ["http", "cli"]
sqlite = ["dep:dbnexus", "dep:sea-orm", "dbnexus/sqlite", "sea-orm/sqlx-sqlite"]
postgres = ["dep:dbnexus", "dep:sea-orm", "dbnexus/postgres", "sea-orm/sqlx-postgres"]
mysql = ["dep:dbnexus", "dep:sea-orm", "dbnexus/mysql", "sea-orm/sqlx-mysql"]
# 删除：dbnexus = ["dep:dbnexus", "dep:sea-orm"]
http = ["dep:axum"]
cli = ["dep:clap", "dep:glob"]
# ...

[dependencies]
dbnexus = { version = "0.2.0", default-features = false, features = ["sql-parser", "config-env", "macros", "permission"], optional = true }
sea-orm = { version = "2.0.0-rc.37", default-features = false, features = ["runtime-tokio-rustls", "with-chrono"], optional = true }
```

**理由**：
- `default-features = false` 在 dbnexus 上避免硬编码 sqlite；sea-orm 同理避免硬编码 runtime-tls
- 每个数据库 feature 同时启用 dbnexus 后端 + sea-orm 后端，确保版本一致
- `runtime-tokio-rustls` 替换 `runtime-tokio-native-tls`，与 dbnexus 默认一致
- 移除 `dbnexus` feature 作为 meta-feature，避免 feature 命名混淆

**代码影响**：
- 所有 `#[cfg(feature = "dbnexus")]` 改为 `#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]`
- 或定义内部宏：`#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]` 太长，可在 `lib.rs` 顶部定义 `#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))] pub const HAS_DATABASE: bool = true;` 或使用内部 cfg alias

### D3: LogSink trait async 化

**决策**：
```rust
// src/support/io/sink/mod.rs
#[async_trait::async_trait]
pub trait LogSink: Send + Sync {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError>;
    async fn flush(&self) -> Result<(), InklogError>;
    async fn shutdown(&self) -> Result<(), InklogError>;
}
```

**理由**：
- `async_trait` 已是依赖（`async-trait = "0.1"`），无需新增
- `DatabaseSink::write` 可直接 `await database.insert_batch(...)`，移除 `execute_async` 和 `block_in_place`
- `FileSink`/`ConsoleSink` 本来就是 sync I/O，改为 async 后用 `tokio::task::spawn_blocking` 包装 fs/console 操作（仅在必要时）
- `LoggerManager` 内部 sink 调用改为 async，`spawn` worker task 消费 log channel

**代码影响范围**：
- `src/support/io/sink/mod.rs`：trait 定义
- `src/support/io/sink/file.rs`：FileSink 实现
- `src/support/io/sink/console.rs`：ConsoleSink 实现
- `src/support/io/sink/ring_buffered_file.rs`：RingBufferedFile 实现
- `src/support/io/sink/database/mod.rs`：DatabaseSink 实现（最大改动）
- `src/support/io/sink/registry.rs`：sink 注册表
- `src/domain/core/manager.rs`：LoggerManager 调用 sink 的所有方法
- `tests/` 下所有 sink 测试
- `examples/src/bin/*.rs` 中所有调用 sink 的示例

**风险**：
- `async_trait` 在 hot path 有微小性能开销（heap allocation），但对日志库可接受
- 现有 sync 测试需改为 `#[tokio::test]`

### D4: ObjectPool 移除 SHARED_RUNTIME

**决策**：
```rust
// 移除：
// static SHARED_RUNTIME: Lazy<tokio::runtime::Runtime> = ...;
// fn get_runtime() -> &'static tokio::runtime::Runtime { ... }

// ObjectPool::with_config 改为：
pub fn with_config(config: ObjectPoolConfig) -> Result<Self, InklogError> {
    // 必须在 async 上下文调用
    let cache = futures::executor::block_on(async {
        let mut builder = Cache::builder();
        builder = builder.capacity(config.max_capacity as u64);
        if let Some(ttl_secs) = config.ttl_secs {
            builder = builder.ttl(Duration::from_secs(ttl_secs));
        }
        builder.build().await
    }).map_err(|e| InklogError::CacheError(format!("Failed to build cache: {}", e)))?;
    Ok(Self { cache: Arc::new(cache), ... })
}

// 或更激进：ObjectPool::with_config 改为 async fn
pub async fn with_config(config: ObjectPoolConfig) -> Result<Self, InklogError> {
    let mut builder = Cache::builder();
    builder = builder.capacity(config.max_capacity as u64);
    if let Some(ttl_secs) = config.ttl_secs {
        builder = builder.ttl(Duration::from_secs(ttl_secs));
    }
    let cache = builder.build().await
        .map_err(|e| InklogError::CacheError(format!("Failed to build cache: {}", e)))?;
    Ok(Self { cache: Arc::new(cache), ... })
}
```

**采用 async fn 版本**：与 D3 一致，全 async 化。`new()`/`with_capacity()` 也改为 async。`get`/`put`/`remove`/`contains` 等 method 改为 async。

**代码影响**：
- `LOG_RECORD_POOL`/`STRING_POOL` 两个 `Lazy<LogRecordPool>`/`Lazy<StringPool>` static 改为 `OnceCell<Arc<ObjectPool<...>>>` 或 `tokio::sync::OnceCell`，需要 async 初始化
- 所有调用 `LOG_RECORD_POOL.get(...)` 的地方改为 `LOG_RECORD_POOL.get(...).await`
- `benches/inklog_bench.rs` 同步更新

### D5: 错误处理显性化

**决策**：
- `OxCacheAdapter::default()` 移除（Default trait 不允许返回 Result，删除 Default 实现）
- `OxCacheAdapter::new()` 保持 `Result<Self, InklogError>` 返回
- `Cache` trait 的 `get` 改为 `async fn get(&self, key: &str) -> Result<Option<String>, InklogError>`，移除 `tracing::warn!` 静默
- `Cache` trait 的 `delete`/`exists` 改为返回 `Result<bool, InklogError>` 或 `Result<(), InklogError>`
- `ObjectPool::with_config` 在 cache build 失败时返回 `Err`，移除 `Cache::default()` fallback

### D6: 文档与 CHANGELOG 修复

**决策**：
- `docs/CHANGELOG.md` `[Unreleased]` 节重写为 `[0.1.2]`，移除所有过时引用（`src/manager.rs`、`ConfersAdapter`、S3 archive、`v0.2.0`），按实际改动重写
- `docs/CHANGELOG.md` 链接更新：`[Unreleased]: .../compare/v0.1.2...HEAD`、`[0.1.2]: .../compare/v0.1.1...v0.1.2`
- 8 个文件的 `confers` 引用清理：
  - `docs/ARCHITECTURE.md`、`README.md`、`README_zh.md`、`docs/CHANGELOG.md`、`docs/USER_GUIDE.md`、`docs/API_REFERENCE.md`：移除 confers 描述
  - `src/support/io/sink/file.rs:1359` 注释：移除 "confers derive generates..."
  - `src/domain/core/manager.rs:304` 注释：移除 "使用 confers 自动生成的方法..."

### D7: Issue #1 处理

**决策**：在 GitHub issue #1 评论说明 v0.1.1+ 已修复（statvfs → statfs，confers/path 依赖移除），v0.1.2 发布后关闭 issue。不 yank v0.1.0（避免破坏现有用户）。

## Alternatives Considered

### A1: 保留 dbnexus feature 作为 meta-feature
**方案**：`dbnexus = ["sqlite"]`，新增 `postgres`/`mysql` 作为独立 feature。
**未选原因**：feature 命名混淆——`dbnexus` 既是库名又是特性名；用户无法仅启用 postgres 而不启用 sqlite。AGENTS.md FORBIDDEN backward compatibility，所以直接拆分更干净。

### A2: 引入 AsyncLogSink 新 trait 与 LogSink 并存
**方案**：保留 sync LogSink，新增 `async trait AsyncLogSink`，DatabaseSink 实现两者。
**未选原因**：双 trait 增加维护成本，调用方需选择用哪个。AGENTS.md FORBIDDEN backward compatibility，直接重构 LogSink 为 async 更彻底。

### A3: ObjectPool 保留 SHARED_RUNTIME，仅在文档警告
**方案**：保留现状，在 lib.rs 注释说明单线程 runtime 限制。
**未选原因**：库不应自建 runtime——这是反模式，跨 runtime 引用 Future 易出问题。文档警告不能替代正确设计。

### A4: DatabaseSink 用 spawn_blocking + channel 解耦
**方案**：sync write 入队 mpsc channel，独立 worker task 异步消费。
**未选原因**：channel 介入引入背压/丢弃语义复杂度，且 LogSink trait sync 签名仍是问题。直接 async 化更彻底。

### A5: oxcache 保留 0.2.0，仅修复 feature 配置
**方案**：`oxcache = { version = "0.2.0", default-features = false, features = ["macros", "memory"] }`，删 moka/dashmap 直接依赖。
**未选原因**：Rule 17 要求"项目依赖优先使用最新稳定版本"。0.3.2 是 2 天前发布，应跟进。

## Consequences

### 正面影响
- 依赖配置正确：moka/dashmap 通过 oxcache memory feature 传递，不再依赖 cargo feature unification
- feature 设计语义清晰：`sqlite`/`postgres`/`mysql` 用户可按需启用
- 架构一致性：所有 sink 都是原生 async，无 block_in_place hack
- 错误显性化：库代码无 panic 路径，无静默吞错
- 文档与代码一致：CHANGELOG 路径、依赖描述、feature 列表全部对齐
- 安全审计通过：cargo-deny / cargo-audit 无未忽略的失败项

### 负面影响
- **破坏性 API 变更**：LogSink trait 签名变化、ObjectPool 构造方法改 async、Cache trait 方法签名变化——所有 sink 实现者和 sink 调用方需适配
- **测试改动量大**：所有 sink 测试改为 `#[tokio::test]`，ObjectPool 测试改为 async
- **示例代码改动**：`examples/src/bin/*.rs` 中调用 ObjectPool/sink 的示例需更新
- **tarpaulin 覆盖率可能短期下降**：新增 async 测试代码路径若覆盖不全，可能影响覆盖率

### 技术债
- `parquet` 仍是默认依赖（未改为 optional），增加编译时间
- `[[bin]] path = "src/cli/mod.rs"` 非标准路径保留
- `metrics = []` feature 仍为空，待后续版本实现
- `RUSTSEC-2023-0071`（rsa Marvin Attack）仍在 ignore 列表，等上游修复

### 后续跟进项
- v0.2.0 计划：实现 metrics feature、parquet 改 optional、LoggerManager 重构
- 监控 oxcache 0.3.x 后续版本是否有 breaking change
- 监控 sea-orm 2.0.0 stable 发布（当前 rc.37）
- 监控 RUSTSEC-2023-0071 和 RUSTSEC-2026-0173 上游修复进度
