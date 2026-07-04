# Spec — database

> Delta spec for change `v012-optimization`. 覆盖 DatabaseSink 与 dbnexus 集成能力域在此变更中的具体需求。

## Requirements

### R-database-001: cfg 替换 dbnexus → sqlite/postgres/mysql

所有 `#[cfg(feature = "dbnexus")]` 替换为 `#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]`。

**验收标准：**
- `src/support/io/sink/database/mod.rs` 所有 `#[cfg(feature = "dbnexus")]` 替换
- `src/domain/core/manager.rs` 所有 `#[cfg(feature = "dbnexus")]` 替换
- `src/domain/core/container.rs` 所有 `#[cfg(feature = "dbnexus")]` 替换
- `src/support/io/sink/entity.rs` 所有 `#[cfg(feature = "dbnexus")]` 替换
- `src/integrations/infra/database.rs` 所有 `#[cfg(feature = "dbnexus")]` 替换
- `#[cfg(not(feature = "dbnexus"))]` 替换为 `#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]`
- `grep -rn 'feature = "dbnexus"' src/` 返回 0 结果

### R-database-002: DatabaseSink 重写为原生 async

`DatabaseSink` 重写为原生 async，移除 `execute_async` 和 `block_in_place`。

**验收标准：**
- `src/support/io/sink/database/mod.rs` 中不存在 `fn execute_async`
- 不存在 `tokio::task::block_in_place` 调用
- `impl LogSink for DatabaseSink` 的 `write`/`flush`/`shutdown` 直接 `await` 数据库操作
- `flush_inner` 改为 `async fn flush_inner(&self, inner: &mut DatabaseSinkInner) -> Result<(), InklogError>`，直接 `await database.insert_batch(...)`
- circuit_breaker 逻辑保留：失败时 `record_failure()`，成功时 `record_success()`
- fallback_sink 逻辑保留：DB 失败时写入 FileSink

### R-database-003: 多数据库后端支持

通过 inklog feature gate 支持 postgres/mysql/sqlite 三种后端。

**验收标准：**
- 启用 `--features sqlite` 时编译通过，dbnexus 启用 sqlite 后端
- 启用 `--features postgres` 时编译通过，dbnexus 启用 postgres 后端
- 启用 `--features mysql` 时编译通过，dbnexus 启用 mysql 后端
- 同时启用 `--features sqlite,postgres,mysql` 时编译通过（多后端共存）
- `DatabaseDriver` 枚举（在 `src/domain/config/config.rs`）保留 postgres/mysql/sqlite 变体
- `DbNexusAdapter::new(url, pool_size)` 接受任何后端 URL，由 dbnexus 自动识别

## Constraints

- `DatabaseSink::write` 在 buffer 满（`current_batch_size`）或超时（`DEFAULT_FLUSH_INTERVAL_MS`）时触发 flush
- 自适应批处理逻辑（`adjust_batch_size`）保留：根据成功率和延迟动态调整 batch_size
- dbnexus `permission` feature 启用时，DDL 用 `execute_raw_ddl`，DML 用 `execute_raw`（项目记忆）
- `DatabaseSink::write` 仍要求 multi-thread runtime（`#[tokio::test(flavor = "multi_thread")]`）

## Out of Scope

- 不修改 `Database` trait（`integrations::infra::Database`）
- 不修改 `DbNexusAdapter` 内部实现（仅 cfg 替换）
- 不引入事务支持
- 不引入连接池配置项
