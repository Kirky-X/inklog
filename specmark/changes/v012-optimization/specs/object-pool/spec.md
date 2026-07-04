# Spec — object-pool

> Delta spec for change `v012-optimization`. 覆盖 ObjectPool 重构能力域在此变更中的具体需求。

## Requirements

### R-object-pool-001: 移除 SHARED_RUNTIME

移除 `ObjectPool` 内部自建的 `tokio::runtime::Runtime`。

**验收标准：**
- `src/support/processing/object_pool.rs` 中不存在 `static SHARED_RUNTIME`
- 不存在 `fn get_runtime()`
- 不存在 `Lazy<tokio::runtime::Runtime>` 任何引用
- `once_cell::sync::Lazy` import 若仅用于 SHARED_RUNTIME 则一并删除

### R-object-pool-002: 构造方法改为 async

`ObjectPool` 的所有构造方法改为 async。

**验收标准：**
- `pub async fn new() -> Result<Self, InklogError>`
- `pub async fn with_config(config: ObjectPoolConfig) -> Result<Self, InklogError>`
- `pub async fn with_capacity(max_capacity: usize) -> Result<Self, InklogError>`（如保留）
- `ObjectPoolBuilder::build` 改为 `pub async fn build() -> Result<Self, InklogError>`
- 构造失败时返回 `Err(InklogError)`，不再 panic
- 不再使用 `Cache::default()` 静默回退

### R-object-pool-003: 实例方法改为 async

`ObjectPool` 的所有实例方法改为 async。

**验收标准：**
- `pub async fn get(&self, key: &K) -> Result<Option<V>, InklogError>`
- `pub async fn put(&self, key: &K, value: V) -> Result<(), InklogError>`
- `pub async fn remove(&self, key: &K) -> Result<Option<V>, InklogError>`（如保留）
- `pub async fn contains(&self, key: &K) -> Result<bool, InklogError>`（如保留）
- `pub fn len(&self) -> usize` 保持 sync（仅读 atomic）
- `pub fn is_empty(&self) -> bool` 保持 sync

### R-object-pool-004: 全局 LOG_RECORD_POOL / STRING_POOL 改为 async 初始化

`LOG_RECORD_POOL` 和 `STRING_POOL` 全局 static 改为 `tokio::sync::OnceCell`。

**验收标准：**
- `static LOG_RECORD_POOL: tokio::sync::OnceCell<Arc<ObjectPool<...>>>`
- `static STRING_POOL: tokio::sync::OnceCell<Arc<ObjectPool<...>>>`
- 提供 `pub async fn get_log_record_pool() -> &'static Arc<ObjectPool<...>>`
- 提供 `pub async fn get_string_pool() -> &'static Arc<ObjectPool<...>>`
- `lib.rs` re-export 更新：移除 `get_log_record`/`put_log_record`/`get_string_buffer`/`put_string_buffer` 的 sync 版本，改为 async 或提供新的 async API

### R-object-pool-005: 移除 dead_code 标记的未使用方法

移除 `#[allow(dead_code)]` 标记的方法。

**验收标准：**
- `src/support/processing/object_pool.rs` 中不存在 `#[allow(dead_code)]`
- `cargo check --all-features` 无 dead_code 警告
- 保留的方法：`new`、`with_config`、`builder`、`get`、`put`、`len`、`stats`（如使用）
- 删除的方法：`with_capacity`、`remove`、`contains`、`is_empty`、`capacity`（如未使用）

### R-object-pool-006: 调用方更新为 async

所有 `ObjectPool` 调用方更新为 async API。

**验收标准：**
- `src/domain/core/manager.rs` 中 `LOG_RECORD_POOL.get(...)` → `get_log_record_pool().await.get(...).await`
- `src/support/processing/template.rs` 中调用同步更新
- `src/support/processing/masking.rs` 中调用同步更新
- `benches/inklog_bench.rs` 中调用同步更新
- `examples/src/bin/object_pool.rs` 示例更新

## Constraints

- `ObjectPool` 仍为 `Clone`（内部 `Arc` 共享）
- `K: oxcache::CacheKey + Send + Sync + 'static` bound 保留
- `V: serde::Serialize + Deserialize + Send + Sync + Clone + 'static` bound 保留
- 不引入新的 `tokio::runtime::Runtime` 实例

## Out of Scope

- 不修改 `ObjectPoolConfig` 结构
- 不修改 `PoolMetrics` 结构（除非移除 dead_code 涉及）
- 不引入新的对象池类型
