# ws-R14 治理复核记录 — inklog（R-inklog-008）

> Generated: 2026-10-01 · 依据: `FEATURE_AUDIT_REPORT.md` §3.2（inklog 隔离性遗留项）· 规格引用: R-inklog-008
> 复核范围: 审计四项遗留（docs.rs all-features、四后端 compile_error 守卫、compression→zstd、manager.rs http 误门控）逐条"修复或记录"

## 0. 结论总表

| # | 审计项（原严重度） | 结论 | 证据位置 |
|---|---|---|---|
| 1 | docs.rs `all-features = true` 与四后端互斥冲突（高） | **已修复**（树上既有） | `Cargo.toml` `[package.metadata.docs.rs]` |
| 2 | 四后端同开无编译期防护（高） | **已修复**（树上既有） | `src/lib.rs` compile_error 守卫 |
| 3 | `compression` 名实不符（中） | **已修复**（树上既有） | `Cargo.toml` `[features]` zstd 主名 + compression 兼容别名 |
| 4 | 安全审计日志被 http 误门控（中） | **已修复**（树上既有） | `src/domain/core/manager.rs` `security_logger_initialized` 已无条件记录 |
| 4b | `HttpServerConfig.enabled` 无 http feature 时静默无效（中，§3.2 同段） | **已修复**（本次纳入） | `src/domain/core/manager.rs` 构造期显性 warn |

## 1. docs.rs all-features（高）— 已修复

**审计原文**：项目 CI 已因互斥弃用 `--all-features`（ci.yml 用显式列表），docs.rs 配置却仍是 `all-features = true` → 全开必然编译失败。

**现状**：`Cargo.toml` 的 `[package.metadata.docs.rs]` 已改为显式 feature 清单（`default/http/cli/zstd/gzip/parquet/fast-masking/sqlite/kit/test-utils/otlp/net-sink/kms/dbnexus-audit/confers-audit/config-confers/otel`），并附注释说明四后端互斥红线与 sqlite 满足 kit 守卫的设计。

**本次复核动作**：逐名核对清单中的 feature 与 `[features]` 定义一致（17 项全部存在，含 `otel`/`confers-audit` 等新增项）；将清单中的 `compression` 别名替换为 `zstd` 主名（语义等价，与第 3 项改名后的命名契约一致，避免未来删除别名时清单失效）。

## 2. 四后端 compile_error 守卫（高）— 已修复

**审计原文**：lib.rs 只防"kit 无驱动"，不防多驱动混用 → 补 compile_error 互斥守卫（对齐 dbnexus 红线）。

**现状**：`src/lib.rs` 已有 embedded（sqlite/duckdb）与 server-side（postgres/mysql）互斥的 `compile_error!` 守卫，注释说明对齐上游 dbnexus 红线。

**实测验证**：`cargo check --features sqlite,postgres` 触发编译失败——报错来自上游 `dbnexus 0.6.0-rc.5` 的同名守卫（依赖先于本 crate 编译，上游守卫先触发）。inklog 侧守卫作为**版本漂移兜底**保留：当未来 dbnexus 升级/替换导致上游守卫缺失时，消费侧仍能给出可读错误。守卫语义正确，双层防护符合纵深防御意图。

## 3. compression→zstd（中）— 已修复

**审计原文**：`compression` 只启用 zstd，与 `gzip` 形成上位词混乱 → 改名 `zstd`，`compression = ["zstd"]` 兼容别名。

**现状**：`[features]` 中 `zstd = ["dep:zstd"]` 为主名（含注释：避免 zstd-sys 符号冲突的 optional 理由），`compression = ["zstd"]` 标注 deprecated 兼容别名；README 同步说明。

**实测验证**：`cargo check --features compression` 通过——别名仍可用，既有消费者无破坏。

## 4. manager.rs http 误门控（中）— 已修复

**审计原文（两项 manager.rs 遗留）**：

1. **安全审计日志被 http 误门控**（manager.rs 原 :376）：`security_logger_initialized` 的 `tracing::info!` 挂在 `#[cfg(feature = "http")]` 下，关掉 http 后安全事件消失。
   **现状**：该 cfg 已移除，`with_config_and_sinks` 中安全审计事件无条件记录（`src/domain/core/manager.rs`）。已复核确认无残留门控。

2. **`HttpServerConfig.enabled` 无 http feature 时静默无效**（manager.rs 原 :452）：无 http feature 时 HTTP 监控服务器整体不编译，配置项被静默跳过，Strict 报错路径也在门内。
   **现状（本次纳入）**：构造函数新增 `#[cfg(not(feature = "http"))]` 分支——`http_server.enabled = true` 但 http feature 未编译时输出 `tracing::warn!`（event = `http_server_config_ignored`），消除运维侧"配了监控却看不到端口"的无提示失效。
   **实测验证**：`cargo check`（default，无 http）与 `cargo check --features http` 两种组合均编译通过；warn 分支仅在无 http 组合参与编译。

## 5. 附带核对（审计 §3.2 其余项，非本次范围）

- **`database` 内部聚合 feature**：已存在（`database = []`，四后端均启用），谓词收敛已落地——审计中"谓词重复粘贴约 100 处"的建议已在前序提交消化。
- **unexpected_cfgs 手工清单**：`[workspace.lints.rust]` 已改为 `unexpected_cfgs = { level = "warn" }` + 注释说明 feature 名由 cargo 自动注册——审计中"check-cfg 清单冗余可删"已消化。
- **http_server 模块门上移、examples required-features、kit 多后端测试面**：属 §3.2 [低] 项，不在 R-inklog-008 四项范围内，未复核（如需可另行立项）。

## 6. 验证基线

- `cargo check`（default / http / compression 三组合）通过
- `cargo check --features sqlite,postgres` 按预期触发互斥编译失败（上游守卫先报错）
- CI 口径 lib 测试全绿（见 T018 覆盖率工作同批验证）

## 7. 附带发现（复核过程中识别，超出本复核四项范围，已上报）

**`SinkHealthMonitor::handle_recovery` 的 `max_retries` 封顶分支当前不可达**（`src/support/observability/metrics.rs`）：达到降级阈值时 `retries` 计数被重置为 0，此后任何一次中途失败都会重新进入 `Fallback` 态并再次清零——从 `Fallback` 态出发的健康检查 `attempt` 恒为 1，`attempt > max_retries` 的封顶 warn 与 `FallbackAction::None` 返回永不执行。属防御性死分支而非行为缺陷（无限重试由"恢复尝试必经 Fallback 重入"的语义自然限制），修复需变更状态机语义，超出治理复核授权范围。已同步标注于 [TEST_SCENARIOS.md](../docs/TEST_SCENARIOS.md) 不可测组合表。

## 8. 覆盖率门禁分阶段抬升留痕（R-inklog-007，本批补充）

规格要求 `--fail-under-lines` 分阶段抬升 80→85→90→95 且「每阶段提交独立且绿」。实际留痕如下（不可回溯处如实注明）：

| 阶段 | 阈值 | 同口径行覆盖实测 | 留痕 |
| --- | --- | --- | --- |
| 基线 | 80 | 未归档 | git 在案：tarpaulin 80 → llvm-cov 80（ci.yml 历史），lefthook 同值 |
| 85 | 85 | 未归档，不可回溯 | **无独立提交与实测记录**——规格「每阶段提交独立且绿」未满足，85/90 两级从未落盘 |
| 90 | 90 | 未归档，不可回溯 | 同上 |
| 终值（T018 批次） | 95 | 95.40% | 随 T018 批次自 80 一次抬入；逐文件明细见本批 llvm-cov 输出，静态门槛表留档（TEST_SCENARIOS.md） |

- 红线复核：抬升未删除既有测试、未放松任何断言（见 TEST_SCENARIOS.md 约束声明），全部增量来自新增测试。
- 终值余量：95.40% 对 95 门槛留 0.4pp 余量，避免门禁常红。
- 口径修正（本批）：pre-push 门禁由裸 default 改为与 CI 同口径（显式 features + `--lib`）——裸 default 下 `tests/cli_integration.rs` 的 `#![cfg(feature = "cli")]` 空 target 零覆盖数据，llvm-cov 报告阶段以 "no coverage data found" 崩溃。
