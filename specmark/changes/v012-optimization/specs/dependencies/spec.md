# Spec — dependencies

> Delta spec for change `v012-optimization`. 覆盖依赖管理能力域在此变更中的具体需求。

## Requirements

### R-dependencies-001: oxcache 升级与 feature 配置

将 `oxcache` 从 0.2.0 升级到 0.3.2，启用 `memory` feature 替代直接的 `moka`/`dashmap` 依赖。

**验收标准：**
- `Cargo.toml` 中 `oxcache` 版本为 `"0.3.2"`
- `oxcache` 配置为 `default-features = false, features = ["macros", "memory"]`
- `Cargo.toml` 中不再出现 `moka` 或 `dashmap` 直接依赖声明
- `cargo tree -i moka` 输出仅显示通过 `oxcache` 引入
- `cargo check --all-features` 通过

### R-dependencies-002: dbnexus feature 拆分

将 `dbnexus` feature 拆分为 `sqlite`、`postgres`、`mysql` 三个独立 feature。

**验收标准：**
- `Cargo.toml` `[features]` 节中不存在 `dbnexus = [...]`
- 新增 `sqlite = ["dep:dbnexus", "dep:sea-orm", "dbnexus/sqlite", "sea-orm/sqlx-sqlite"]`
- 新增 `postgres = ["dep:dbnexus", "dep:sea-orm", "dbnexus/postgres", "sea-orm/sqlx-postgres"]`
- 新增 `mysql = ["dep:dbnexus", "dep:sea-orm", "dbnexus/mysql", "sea-orm/sqlx-mysql"]`
- `cargo check --features sqlite` 通过
- `cargo check --features postgres` 通过
- `cargo check --features mysql` 通过
- `cargo check --no-default-features --features sqlite,http,cli` 通过（默认 features 之外也能用）

### R-dependencies-003: sea-orm TLS 一致性

将 `sea-orm` 的 TLS 后端从 `runtime-tokio-native-tls` 切换到 `runtime-tokio-rustls`。

**验收标准：**
- `Cargo.toml` 中 `sea-orm` features 列表为 `["runtime-tokio-rustls", "with-chrono"]`
- `cargo tree -e features -i sea-orm` 不再显示 `runtime-tokio-native-tls`
- `cargo check --features sqlite` 通过

### R-dependencies-004: deny.toml 安全审计忽略列表更新

添加 `RUSTSEC-2026-0173`（proc-macro-error2 unmaintained）到 `deny.toml` 的 ignore 列表。

**验收标准：**
- `deny.toml` `ignore` 数组包含 `"RUSTSEC-2026-0173"` 项
- 注释说明："proc-macro-error2 unmaintained (transitive via validator_derive/sea-bae/dbnexus-macros, no safe upgrade available)"
- `cargo deny check advisories` exit code 0

## Constraints

- 不修改 `dbnexus`、`oxcache` 上游源码
- `sea-orm` 必须 pin 在 `2.0.0-rc.37`（rustc 1.93.1 兼容性）
- `dbnexus` 必须 pin 在 `0.2.0`（无新版本可用）
- 不 yank v0.1.0（避免破坏现有用户）

## Out of Scope

- 不引入 parquet 作为 optional feature（留给后续版本）
- 不替换 validator（即使其依赖 proc-macro-error2）
- 不修复 RUSTSEC-2023-0071（rsa Marvin Attack，上游无修复）
