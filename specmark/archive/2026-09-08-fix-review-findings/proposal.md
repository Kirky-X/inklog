<!-- domain: code -->
# fix-review-findings

## Motivation

对 `INKLOG_REVIEW_REPORT.md`（天罡 SAST + diting 安全审查报告）进行三路独立 subagent 核实后，确认报告整体结论可信、分级发现全部真实，但同时暴露出若干需要修复的真实问题：

1. **PBKDF2 迭代数"测试守卫"是同义反复**：`encryption.rs` 的 `test_pbkdf2_iteration_count_is_at_least_600k` 本地硬编码 `600_000` 调用 `pbkdf2_hmac`，与生产常量完全脱钩——把生产迭代数降到 1000 该测试照样通过，守卫形同虚设。
2. **IP 白名单 CIDR 解析失败完全静默**（报告 MED-001）：`http_server.rs` 中 `parse_cidr` 返回 `None` 时该条目被静默跳过，配置侧亦无前置校验——配置笔误（如 `10.0.0.0/33`）会导致合法 IP 被拒且无任何日志。
3. **测试容器缺安全加固**（报告 MED-002）：`docker/docker-compose.test.yml` 的 postgres/mysql/redis 三个服务均无 `security_opt: no-new-privileges` 与 `read_only`。
4. **45 处 GitHub Actions 可变标签**（报告 MED-003）：供应链攻击风险，应 pin 到完整 commit SHA。
5. **报告自身的量化数字不实**：unsafe 计数（25 vs 实测 269）、Trufflehog（201 vs 产物 297/实跑 352）、Semgrep 总数（79 vs 实跑 9）、cargo audit "0 发现"掩盖产物中 RUSTSEC-2026-0235（rkyv）历史记录、PBKDF2 守卫表述、"DB→File→Console 三级降级"链路描述不准确。报告若被引用会误导读者。
6. **LOW-001 文档缺口**：32 字节输入直用为原始密钥的兼容行为仅有行内注释与运行时 warn，rustdoc 层未显式标注。

## Scope

- `src/support/io/sink/encryption.rs`：PBKDF2 迭代数提取为具名常量并让测试真实守卫生产常量；`get_encryption_key` rustdoc 显式标注 32 字节兼容语义。
- `src/domain/core/http_server.rs`：IP 白名单条目解析失败时输出 `tracing::warn`（每次进程生命周期仅一次，避免请求热路径刷屏）。
- `src/domain/config/http.rs` + `src/domain/config/config.rs`：`ip_whitelist` 条目在配置校验期做格式前置校验（fail-fast，坏条目直接报错）。
- `docker/docker-compose.test.yml`：三个服务添加 `security_opt: ["no-new-privileges:true"]`、`read_only: true` 及必要的 tmpfs 挂载。
- `.github/workflows/*.yml`：全部 `uses:` 可变标签替换为完整 commit SHA（经 `git ls-remote` 解析并剥离 annotated tag，附原版本号注释）。
- `INKLOG_REVIEW_REPORT.md`：按独立核实的实测值修正不实数字与两处失实描述（unsafe 统计、Trufflehog/Semgrep 数字、cargo audit rkyv 历史、PBKDF2 守卫表述、降级链描述）。

## Non-Goals

- **参数化 SQL 迁移**（报告 LOW-003/路线图长期项）：受限于 dbnexus `batch_execute_in_transaction` 仅接受 `&str`，等待上游支持后另行立项。
- **SAST 全量重跑刷新报告的所有数字**：仅修正已核实为不实的条目，其余数字保持原样并标注来源；全量重扫属于 tiangang skill 的职责。
- **unsafe 全量清点入报告**：实测分布仅用于修正"25 处"这一处不实数字，不在报告中展开完整清单。
- **归档 `reviews/tiangang/` 历史产物**：产物为只读历史证据，不修改。

## NEEDS CLARIFICATION

（无 — 网络可达性已验证（`git ls-remote` 可解析 actions/checkout v7），所有发现均有明确修复方案，无需用户决策。）
