# Spec — docs

> Delta spec for change `v012-optimization`. 覆盖文档一致性能力域在此变更中的具体需求。

## Requirements

### R-docs-001: CHANGELOG 重写

重写 `docs/CHANGELOG.md` 的 `[Unreleased]` 节为 `[0.1.2]`，移除所有过时引用。

**验收标准：**
- 不存在 `src/manager.rs` 引用（实际路径为 `src/domain/core/manager.rs`）
- 不存在 `src/sink/file.rs` 引用（实际路径为 `src/support/io/sink/file.rs`）
- 不存在 `ConfersAdapter` 引用（已删除）
- 不存在 S3 archive 相关条目（已在 v0.1.1 移除）
- 不存在 `v0.2.0` 标题（实际版本为 v0.1.2）
- 新增 `[0.1.2] - 2026-07-XX` 节，包含：依赖重构、LogSink async 化、ObjectPool 重构、错误处理、文档同步
- 底部链接更新：`[Unreleased]: .../compare/v0.1.2...HEAD`、`[0.1.2]: .../compare/v0.1.1...v0.1.2`

### R-docs-002: confers 残留清理

清理所有 `confers` 引用。

**验收标准：**
- `src/support/io/sink/file.rs:1359` 注释删除
- `src/domain/core/manager.rs:304` 注释删除
- `docs/ARCHITECTURE.md` 中 confers 引用删除或替换为 oxcache
- `README.md` 中 confers 引用删除
- `README_zh.md` 中 confers 引用删除
- `docs/USER_GUIDE.md` 中 confers 引用删除
- `docs/API_REFERENCE.md` 中 confers 引用删除
- `docs/CHANGELOG.md` 历史 [0.1.1] 节中的 confers 引用保留（历史记录）
- `grep -rn 'confers' src/` 返回 0 结果
- `grep -rn 'confers' docs/` 仅在 `docs/archive/` 或历史 CHANGELOG 节中存在

### R-docs-003: README 与 feature 列表更新

更新 `README.md` 和 `README_zh.md` 反映新的 feature 结构和 async API。

**验收标准：**
- feature 列表中不存在 `dbnexus`，改为 `sqlite`/`postgres`/`mysql`
- 依赖描述中不存在 `moka`/`dashmap`
- 依赖描述中 `oxcache` 版本为 `0.3.2`
- 示例代码中 sink 调用为 async（`.await`）
- 示例代码中 `ObjectPool::new()` 为 `ObjectPool::new().await?`

### R-docs-004: ARCHITECTURE/USER_GUIDE/API_REFERENCE 更新

更新架构文档、用户指南、API 参考以反映新的模块路径和 async API。

**验收标准：**
- 模块路径引用更新：`src/manager.rs` → `src/domain/core/manager.rs`，`src/sink/file.rs` → `src/support/io/sink/file.rs`，`src/masking.rs` → `src/support/processing/masking.rs`，`src/metrics.rs` → `src/support/observability/metrics.rs`，`src/object_pool.rs` → `src/support/processing/object_pool.rs`
- API 签名示例更新为 async
- feature 描述更新为 `sqlite`/`postgres`/`mysql`
- 不存在 `ConfersAdapter` 引用

### R-docs-005: 版本号同步

版本号在所有位置同步为 `0.1.2`。

**验收标准：**
- `Cargo.toml` 第 3 行：`version = "0.1.2"`
- `examples/Cargo.toml`：`inklog = { version = "0.1.2", ... }`
- `docs/CONTRIBUTING.md` 中版本引用同步（如有）
- `docs/CHANGELOG.md` 新增 `[0.1.2]` 节

## Constraints

- 不修改 `docs/archive/` 下文档（历史记录只读）
- 历史 CHANGELOG 节（`[0.1.0]`、`[0.1.1]`）保留原样
- 不引入新文档文件（仅更新现有）

## Out of Scope

- 不重写 `docs/CONTRIBUTING.md`（除非版本引用过时）
- 不修改 `docs/tdd.md`、`docs/test.md`、`docs/uat.md`、`docs/task.md`、`docs/prd.md`
- 不修改 `docs/SECURITY.md`
- 不创建新文档
