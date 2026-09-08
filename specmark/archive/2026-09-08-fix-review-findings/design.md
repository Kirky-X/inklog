# Design — fix-review-findings

## Context

独立核实（3 路并行 subagent，约 60 条断言）确认 `INKLOG_REVIEW_REPORT.md` 的分级发现全部真实，同时暴露出测试守卫失效、CIDR 静默失败、CI/容器加固缺口与报告数字失实等问题。相关现状：

- `src/support/io/sink/encryption.rs`：`derive_key_from_password`（约 117-135 行）在 `pbkdf2_hmac` 调用中内联硬编码 `600_000`；测试 `test_pbkdf2_iteration_count_is_at_least_600k`（约 351-362 行）本地硬编码同值直接调 `pbkdf2_hmac`，与生产路径无关联。
- `src/domain/core/http_server.rs`：白名单检查（147-155 行）通配符条目已带 "." 后缀防误匹配；CIDR 条目经 `parse_cidr`（171-173 行，`cidr.parse::<IpNet>().ok()`）失败返回 `None` 即静默不匹配，全文件无 warn。
- `src/domain/config/http.rs`：`ip_whitelist: Vec<String>`（29 行附近），配置校验 `InklogConfig::validate` 的 HTTP 段（config.rs:410-418 附近）仅查端口，未校验白名单条目格式。
- `docker/docker-compose.test.yml`：postgres（12-37）、mysql（41-66）、redis（70-89）三服务，均无 `security_opt`/`read_only`。
- `.github/workflows/`：6 个文件共 45 处 `uses:`，全部可变标签；`git ls-remote` 已验证可解析（actions/checkout v7 → 3d3c42e5…）。
- `INKLOG_REVIEW_REPORT.md`：未纳入 git（untracked），量化数字多处与实测不符。

## Decision

### D1 — PBKDF2 迭代数常量化 + 真实守卫

提取 `pub(crate) const PBKDF2_ITERATIONS: u32 = 600_000;`（encryption.rs 顶部，附 OWASP 出处注释）。`derive_key_from_password` 的 `pbkdf2_hmac` 调用改用该常量。测试重写为双向守卫：

1. `assert!(PBKDF2_ITERATIONS >= 600_000)`——常量被调低即失败；
2. 经生产路径 `derive_key_from_password(password, Some(fixed_salt))`（固定盐保证确定性）得到密钥，与本地 `pbkdf2_hmac(password, salt, PBKDF2_ITERATIONS)` 结果比对——若生产派生路径绕开常量（回归本次缺陷形态），比对失败。

可见性取 `pub(crate)`：守卫测试在同 crate 测试模块内，无需扩大公共 API。

### D2 — IP 白名单：配置期 fail-fast + 运行期一次性 warn

两层防线：

- **配置期（主防线）**：`InklogConfig::validate` 的 HTTP 段新增白名单条目格式校验——每条须能解析为 `IpAddr` 或 `IpNet`（与 http_server 运行期解析语义一致），否则返回描述性错误（含条目原文与序号）。坏配置在启动时拒绝，而非运行期静默丢条目。
- **运行期（兜底）**：`http_server.rs` 在白名单条目解析失败时经进程级 `AtomicBool`（`AtomicU8` 计数，仅首次）输出 `tracing::warn!`（含条目原文）——避免每请求刷屏，同时覆盖绕过配置校验直接构造 `HttpServer` 的路径。

### D3 — Docker 测试容器加固

三个服务统一追加：`security_opt: ["no-new-privileges:true"]`、`read_only: true`，并为镜像必需的可写路径挂 `tmpfs`（postgres：`/var/lib/postgresql/data`、`/var/run/postgresql`、`/tmp`；mysql：`/var/lib/mysql`、`/var/run/mysqld`、`/tmp`；redis：`/data`、`/tmp`）。测试容器数据本就 ephemeral，tmpfs 不损失语义。本地无 Docker 环境时不实跑，由 `test-docker.yml` CI 工作流验证；若 CI 因镜像需要额外可写路径失败，按镜像报错补 tmpfs 路径（预期至多一轮）。

### D4 — Actions 可变标签 pin SHA

对 `.github/workflows/*.yml` 中全部唯一 `owner/repo@ref` 组合：`git ls-remote https://github.com/<owner>/<repo> refs/tags/<ref> 'refs/tags/<ref>^{}'` 解析——优先取 `^{}` 剥离后的 commit SHA（annotated tag 场景），轻量标签直接取值；非 tag 引用（如分支名）取对应 ref 的 SHA。替换为 `uses: owner/repo@<sha> # <原ref>` 保留可读性。已验证网络可达；45 处仅涉及少数唯一组合，逐个解析后脚本化替换。

### D5 — 报告数字修正（INKLOG_REVIEW_REPORT.md）

按独立核实实测值修正，不实数字旁标注"已按 <日期> 独立复核修正"：

- unsafe：25 → 实测全仓约 269（几乎全部位于 `#[cfg(test)]`，生产仅 file.rs:591 一处 Windows FFI），分文件数字同步替换；
- Trufflehog：201/88/66/37/10 → 标注产物 297 条（Postgres=122、FTP=64、URI=56、Roaring=45 等）与复跑漂移说明（含 target/ 噪声）；
- Semgrep：总数 79 → 标注复跑 9 条（全部为产物自引用），保留可复现的 45/8/1 三项；
- cargo audit：补充 RUSTSEC-2026-0235（rkyv）历史记录与"9 月 6-7 日移除 rkyv 后清零"的时序说明；
- PBKDF2 守卫表述：改为"已修复为真实守卫（引用生产常量）"；
- 降级链：改为"DB→Console 与 File→Console，重试 3 次后丢弃并计数"；
- MED-002 文件名补 `docker/` 前缀；A08 表计数同步替换。

## Alternatives Considered

- **CIDR 仅运行期 warn（不做配置期校验）**：实现最小，但坏配置每次启动都靠日志才发现，fail-fast 更符合本仓库已有模式（file_sink 尺寸/加密 env 校验均在 validate()）。采用两层并存，配置期为主。
- **PBKDF2 测试仅断言常量数值**：比双断言简单，但无法检测"派生路径绕开常量"这一本次缺陷的实际形态（内联硬编码）。采用双断言。
- **Docker 仅加 security_opt 不加 read_only**：零破坏风险但不满足报告建议的加固基线。tmpfs 方案风险可控（CI 验证兜底），采用完整加固。
- **SHA pin 保留 tag 于注释之外不改**：违反报告 MED-003 的建议本体，放弃。
- **不改报告（保持原样作为历史记录）**：报告 untracked 且日期为今日，会持续被引用；保留失实数字的害处大于"历史不可变"的收益。修正并标注复核来源。

## Consequences

- PBKDF2 守卫从"形同虚设"变为可真实拦截迭代数回归；`PBKDF2_ITERATIONS` 成为 crate 内单一事实来源。
- IP 白名单配置错误从"静默失效"变为"启动报错 + 运行期一次告警"；已有合法配置（全部条目可解析）不受影响。
- Docker 加固在 CI 的 test-docker 工作流首次运行时存在失败可能（镜像需额外可写路径），修复方式明确（补 tmpfs）。
- SHA pin 后依赖升级从"改 tag"变为"换 SHA"，升级摩擦增加；注释保留原 tag 缓解可读性。
- 报告修正后与实测一致，但其中 Trufflehog/Semgrep 数字天然随扫描时点漂移，已加标注说明。
