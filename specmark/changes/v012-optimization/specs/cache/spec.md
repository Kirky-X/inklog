# Spec — cache

> Delta spec for change `v012-optimization`. 覆盖 Cache 适配器错误处理能力域在此变更中的具体需求。

## Requirements

### R-cache-001: 移除 OxCacheAdapter 的 Default 实现

移除 `OxCacheAdapter` 的 `Default` impl，避免在库代码中 panic。

**验收标准：**
- `src/integrations/infra/cache.rs` 中不存在 `impl Default for OxCacheAdapter`
- 调用方使用 `OxCacheAdapter::new()?` 而非 `OxCacheAdapter::default()`
- 测试中 `OxCacheAdapter::default()` 调用改为 `OxCacheAdapter::new().expect("...")` 或 `?` 传播
- `cargo check --all-features` 通过

### R-cache-002: Cache trait 方法返回 Result

`Cache` trait 的所有方法返回 `Result`，移除 `tracing::warn!` 静默吞错。

**验收标准：**
- `async fn get(&self, key: &str) -> Result<Option<String>, InklogError>`
- `async fn set(&self, key: &str, value: String) -> Result<(), InklogError>`（保持不变）
- `async fn delete(&self, key: &str) -> Result<bool, InklogError>`
- `async fn exists(&self, key: &str) -> Result<bool, InklogError>`
- `OxCacheAdapter` 实现中不存在 `tracing::warn!` 吞错（直接 `?` 传播或 `map_err`）
- `MockCache` 实现匹配新签名

### R-cache-003: OxCacheAdapter 错误传播

`OxCacheAdapter` 的方法实现直接传播 oxcache 错误，不静默处理。

**验收标准：**
- `get` 方法：`self.inner.get(...).await.map_err(|e| InklogError::CacheError(...))`
- `set` 方法：保持现有 `?` 传播
- `delete` 方法：`self.inner.delete(...).await.map_err(...)` 返回 `Result`
- `exists` 方法：`self.inner.exists(...).await.map_err(...)` 返回 `Result`
- 错误信息包含原始 oxcache 错误上下文

### R-cache-004: MockCache 实现更新

`MockCache` 实现匹配新 `Cache` trait 签名。

**验收标准：**
- `MockCache` 的所有方法返回 `Result<_, InklogError>`
- `MockCache` 内部 `RwLock<HashMap>` 操作失败时返回 `InklogError`（理论上不会失败，但保留 Result 签名以匹配 trait）
- 测试中所有 `cache.get(...).await` 改为 `cache.get(...).await?` 或 `cache.get(...).await.unwrap()`

## Constraints

- `Cache` trait 仍标注 `#[async_trait]`
- `OxCacheAdapter` 和 `MockCache` 仍为 `Send + Sync`
- 错误类型统一使用 `InklogError::CacheError(String)`

## Out of Scope

- 不修改 `OxCacheAdapterBuilder`（TTL/capacity 配置逻辑保留）
- 不引入新的 cache 实现
- 不修改 oxcache 上游 API
