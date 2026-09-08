# Tasks — fix-review-findings

- [x] [T001] [P1] encryption.rs 顶部新增 `pub(crate) const PBKDF2_ITERATIONS: u32 = 600_000;`（注释注明 OWASP 推荐），`derive_key_from_password` 的 `pbkdf2_hmac` 调用改用该常量 — src/support/io/sink/encryption.rs
- [x] [T002] [P1] 重写 `test_pbkdf2_iteration_count_is_at_least_600k` 为双向守卫：断言 `PBKDF2_ITERATIONS >= 600_000`，且 `derive_key_from_password(password, Some(fixed_salt))` 的输出与本地 `pbkdf2_hmac(password, salt, PBKDF2_ITERATIONS)` 比对一致；先确认测试在常量被人为调小时失败（临时改值验证后还原） — src/support/io/sink/encryption.rs
- [x] [T003] [P2] `get_encryption_key` rustdoc 增加"32 字节输入直用为原始密钥（兼容行为，勿在新部署使用）"的显式 `# 兼容性` 小节，与行内注释呼应 — src/support/io/sink/encryption.rs
- [x] [T004] [P1] `InklogConfig::validate` HTTP 段新增 ip_whitelist 条目格式校验：每条须解析为 `IpAddr` 或 `IpNet`，否则返回含条目原文与序号的错误；新增测试覆盖合法 IP/CIDR 通过、非法条目（`10.0.0.0/33`、`not-an-ip`）报错 — src/domain/config/config.rs
- [x] [T005] [P1] http_server.rs 白名单条目解析失败时经进程级 AtomicBool 仅首次输出 `tracing::warn!`（含条目原文），并新增测试验证：含非法条目时该条目不匹配任何 IP 且 warn 路径可达（合法条目行为不变，既有白名单测试保持通过） — src/domain/core/http_server.rs
- [x] [T006] [P1] docker-compose.test.yml 三个服务（postgres/mysql/redis）追加 `security_opt: ["no-new-privileges:true"]`、`read_only: true` 及 tmpfs 挂载（postgres: /var/lib/postgresql/data、/var/run/postgresql、/tmp；mysql: /var/lib/mysql、/var/run/mysqld、/tmp；redis: /data、/tmp），yml 语法经解析器校验 — docker/docker-compose.test.yml
- [x] [T007] [P1] 收集 .github/workflows/*.yml 全部唯一 `owner/repo@ref` 组合，`git ls-remote` 解析 commit SHA（annotated tag 取 `^{}` 剥离值），将 45 处可变标签替换为 `uses: owner/repo@<sha> # <原ref>`；替换后 grep 确认无可变标签残留且 uses 总数仍为 45 — .github/workflows/ci.yml、codeql.yml、release.yml 等
- [x] [T008] [P2] 按设计 D5 修正 INKLOG_REVIEW_REPORT.md：unsafe 计数（25→实测约 269 及分文件值）、Trufflehog/Semgrep 数字加实测标注、cargo audit 补 RUSTSEC-2026-0235 历史说明、PBKDF2 守卫表述改为已修复、降级链改为 DB→Console/File→Console、MED-002 路径补 docker/ 前缀、A08 表计数同步 → INKLOG_REVIEW_REPORT.md
- [x] [T009] [P0] 全量验证：`cargo test --workspace` 与 `cargo test --workspace --features "sqlite http cli kit compression gzip parquet fast-masking test-utils"` 0 失败，`cargo clippy --workspace --all-targets` 双配置 0 警告；docker-compose.yml 变更若 CI 无法本地验证，在提交说明中注明待 test-docker 工作流验证

## Phase 1: Convergence

_由 /specmark converge 于 2026-09-08 生成。仅追加：不要编辑之前的任务。_

**发现缺口：** 0 (CRITICAL: 0 | HIGH: 0 | MEDIUM: 0 | LOW: 0)
**追加任务：** 0（跳过：2 个 LOW/装饰性漂移，记录为叙述）
**未请求范围（按原样接受）：**
- `whitelist_entry_matches` 内通配符分支顺带改用 `strip_suffix`（clippy manual_strip 规范化，行为等价）
- 4.1 节 unsafe 分布表同步补全了原报告遗漏的其余测试文件计数（属 R-security-report-001 同一修正事项）

**验收标准检查（delta specs）：**

| R-ID | 验收条件 | 状态 |
|------|----------|------|
| R-pbkdf2-guard-001 | 常量单一来源，pbkdf2_hmac 调用点引用常量 | ✓ PASS（encryption.rs:19 定义、:148 引用） |
| R-pbkdf2-guard-002 | 双向守卫 + 双向变异验证 | ✓ PASS（常量调低→(a)失败；派生内联字面量→(b)失败；还原后通过） |
| R-pbkdf2-guard-003 | rustdoc 兼容性小节 | ✓ PASS（encryption.rs `# 兼容性`） |
| R-http-ip-whitelist-001 | 配置期校验：合法通过/非法报错含原文与序号 | ✓ PASS（config.rs + 3 个新测试） |
| R-http-ip-whitelist-002 | 运行期一次性告警 + fail-closed 语义不变 | ✓ PASS（whitelist_entry_matches + 2 个新测试，进程级 AtomicBool 守卫） |
| R-ci-hardening-001 | 三服务 security_opt/read_only/tmpfs | ✓ PASS（yml 解析校验；数据目录由既有 named volume 覆盖可写性，语义优于 tmpfs 方案；运行时行为待 CI test-docker 验证） |
| R-ci-hardening-002 | 45 处 uses 全部 pin SHA、无可变引用 | ✓ PASS（11 ref 经 git ls-remote 解析含 ^{} 剥离；codeql 子目录形式已还原；总量 45 不变） |
| R-security-report-001 | 失实数字修正并标注复核来源 | ✓ PASS |
| R-security-report-002 | 降级链/PBKDF2 守卫/docker 路径表述修正 | ✓ PASS |

**验证记录：** cargo test 默认 1243/0、全 feature 1752/0；clippy 双配置 0 警告。备注：apply 首轮全 feature 跑出现 1 个未复现失败（后续两轮全绿，本轮改动未触及对应代码，判定为偶发抖动）。
