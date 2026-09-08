# Spec — security-report

> Main spec for capability `security-report`.

## Requirements

### R-security-report-001: 量化数字与实测对齐

报告中以下数字按 2026-09-08 独立复核实测值修正，并标注复核来源与日期：

- unsafe 计数：全仓约 269 处（几乎全部位于 `#[cfg(test)]`；src/ 内 178 处测试 + 1 处生产 FFI），分文件值（decrypt.rs test=20、file.rs 34 test + 1 prod、encryption.rs test=15 等）同步替换原 25/14/8/6
- Trufflehog：标注 8 月 19 日产物为 297 条（Postgres=122、FTP=64、URI=56、Roaring=45 等）、复跑约 352 条（含 target/ 噪声），原 201/88/66/37/10 无产物支撑
- Semgrep：总数 79 标注为不可复现（原命令实跑 9 条、产物 4 条），保留可复现的 mutable-action-tag=45、aws-key=8、facebook-oauth=1
- cargo audit：补充 RUSTSEC-2026-0235（rkyv）历史记录及"2026-09-06/07 移除 rkyv 后清零"的时序说明

**验收标准：**
- 修正后报告中不再出现"25 处 unsafe""201 条"等失实数字的原表述
- 每处修正附带"（2026-09-08 独立复核修正）"类标注

### R-security-report-002: 两处失实描述修正

- PBKDF2 守卫表述：由"测试用例确保 600,000 次迭代不被意外降低"改为反映修复后状态"测试引用生产常量 PBKDF2_ITERATIONS，可真实拦截迭代数回归"（随本变更 T001/T002 落地后成立）
- 降级链描述：由"DB → File → Console 三级降级，失败时拒绝而非放行"改为"DB→Console 与 File→Console 降级，重试 3 次后丢弃记录并计数"（A04 表同步）
- MED-002 文件路径补 `docker/` 前缀；A08 表 unsafe 行与 4.1 节数字同步

**验收标准：**
- `grep -n "DB → File → Console" INKLOG_REVIEW_REPORT.md` 零命中
- `grep -n "docker-compose.test.yml" INKLOG_REVIEW_REPORT.md` 均含 docker/ 前缀
- 结论章"零 CRITICAL/HIGH"与 95/100 评分保留（核实支持该判定）

## Constraints

- 只修正经独立核实为不实的条目，不改写报告结论与评分；修正处保留原值并注明，保证可追溯

## Out of Scope

- 全量重跑 SAST 刷新全部数字；报告结构重组；reviews/tiangang/ 历史产物修改
