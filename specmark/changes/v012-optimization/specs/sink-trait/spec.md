# Spec — sink-trait

> Delta spec for change `v012-optimization`. 覆盖 LogSink trait async 化能力域在此变更中的具体需求。

## Requirements

### R-sink-trait-001: LogSink trait 改为 async trait

将 `LogSink` trait 的 `write`/`flush`/`shutdown` 方法改为 `async fn`。

**验收标准：**
- `src/support/io/sink/mod.rs` 中 `LogSink` trait 标注 `#[async_trait::async_trait]`
- `async fn write(&self, record: &LogRecord) -> Result<(), InklogError>`
- `async fn flush(&self) -> Result<(), InklogError>`
- `async fn shutdown(&self) -> Result<(), InklogError>`
- 非 async 方法（如 `name()`）保持 sync
- `async_trait` 已在 `Cargo.toml` 依赖中（无需新增）

### R-sink-trait-002: FileSink 实现改为 async LogSink

`FileSink` 实现 async `LogSink` trait。

**验收标准：**
- `src/support/io/sink/file.rs` 中 `impl LogSink for FileSink` 标注 `#[async_trait]`
- `write`/`flush`/`shutdown` 方法为 `async fn`
- 内部 `std::fs` 操作改为 `tokio::fs` 或用 `tokio::task::spawn_blocking` 包装
- 现有 `FileSink` 测试改为 `#[tokio::test] async fn`
- 测试覆盖：async write 后能读到内容、async flush 后缓冲清空、async shutdown 后资源释放

### R-sink-trait-003: ConsoleSink 实现改为 async LogSink

`ConsoleSink` 实现 async `LogSink` trait。

**验收标准：**
- `src/support/io/sink/console.rs` 中 `impl LogSink for ConsoleSink` 标注 `#[async_trait]`
- 所有方法为 `async fn`
- 测试改为 `#[tokio::test]`

### R-sink-trait-004: RingBufferedFile 实现改为 async LogSink

`RingBufferedFile` 实现 async `LogSink` trait。

**验收标准：**
- `src/support/io/sink/ring_buffered_file.rs` 中实现 async `LogSink`
- 测试改为 `#[tokio::test]`

### R-sink-trait-005: DatabaseSink 重写为原生 async

`DatabaseSink` 完全重写为原生 async，移除 `execute_async` 和 `block_in_place`。

**验收标准：**
- `src/support/io/sink/database/mod.rs` 中不存在 `fn execute_async`
- 不存在 `tokio::task::block_in_place` 调用
- `write`/`flush`/`shutdown` 直接 `await` 数据库操作
- circuit_breaker、fallback_sink 逻辑保留
- `DatabaseSink::write` 在 buffer 满或超时时触发 `flush_inner`，`flush_inner` 直接 `await database.insert_batch(...)`
- 测试用 `#[tokio::test(flavor = "multi_thread")]`（保持与 dbnexus permission 兼容）

### R-sink-trait-006: SinkRegistry 支持 async LogSink

`SinkRegistry` 的注册/获取逻辑兼容 async `LogSink`。

**验收标准：**
- `src/support/io/sink/registry.rs` 中 `Arc<dyn LogSink>` 仍可正常存储（async_trait 自动加 `Send` bound）
- 测试验证 async sink 可注册、可被获取并调用 `.await`

### R-sink-trait-007: LoggerManager 调用 sink 为 async

`LoggerManager` 内部所有 sink 调用改为 `.await`。

**验收标准：**
- `src/domain/core/manager.rs` 中 `sink.write(...)` → `sink.write(...).await`
- `sink.flush()` → `sink.flush().await`
- `sink.shutdown()` → `sink.shutdown().await`
- worker task 内部循环改为 async
- `LoggerManager::shutdown` 等公共方法可能需要改为 async（视调用链）

## Constraints

- `async_trait` 性能开销可接受（日志库非超低延迟场景）
- 所有 sink 实现必须 `Send + Sync`（async_trait 自动要求）
- `DatabaseSink` 在单线程 tokio runtime 下不再 panic（block_in_place 已移除）

## Out of Scope

- 不引入新的 sink 类型
- 不重构 LoggerManager 整体架构（仅 sink 调用方式变化）
- 不修改 sink 配置结构（`FileSinkConfig` 等）
