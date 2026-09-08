# inklog 验收报告

## 阶段 1：依赖升级（30d5d1f）

- 升级内容：zstd 0.13→0.14（zstd-safe 8.0）、dirs 6.0→7.0、icu 2.2→2.3.1、tokio/regex 写法统一 1.53/1.13
- 回归结果：四驱动组+http/cli/compression/parquet/fast-masking 零告警 / lib 990 过 / clippy 零告警 / fmt 净
- 不可升级项/例外：
- 无不可升级项
- 下游传导：下游仓本地构建自动拉取上游新码（自下而上顺序执行中逐仓覆盖）

## 阶段 2：测试金字塔 + E2E 固化

- **死文件复活**：tests/integration、combinations、performance 深目录聚合此前无编译入口
  （Cargo 仅自动发现顶层 `*.rs` 与 `*/main.rs`），从未被 cargo test 运行——`[[test]]` 显式注册
  复活；e2e_advanced 迁入 `tests/e2e/` 目录承载（e2e_* 不得裸放顶层）。
- **测试修复**：integration 7 处失败 + combinations 9 处 + performance 3 处全部按真实行为
  核正（详见 docs/TEST_SCENARIOS.md §4：with_config 全局单次语义统一口径 / 弱密钥熵校验 /
  validation_errors 延迟校验 / adaptive 瞬态峰值采样 / log 宏进程级绑定 smoke 化等）。
- **库修复 ×3**：① shutdown 死锁（send→send_timeout(2s)，worker block_on 依赖调用方驱动
  线程的死锁环，gdb 实证；修复后 integration 复验 ×5 含串行全绿）；② LoggerSubscriber
  手动 impl Clone（AtomicU64 无 derive）；③ 默认 DbNexusAdapter 创建上移 build_detached
  （修 runtime-in-runtime panic）。
- **静态门槛清理**：clippy 告警 ×19 清零（module_inception ×6 含 stability/verification
  承载文件改名 long_running.rs/file_sink.rs、approx PI、恒真断言 ×6、is_multiple_of ×3、
  trim/split ×2、冗余 struct update ×1）。
- **CI 同步**：ci.yml test/clippy 双 job 补 test-utils（integration 目标 required-features
  含它，缺失时 cargo 静默跳过目标）；deny.toml 补 5 项 licenses clarify（inklog/oxcache/
  oxcache_macros/trait-kit/dbnexus，MIT hash 绑定，path 相对被 clarify crate 目录）。
- **全量结果**（CI 主口径 + test-utils）：lib 1009 / unit_tests 63 / integration 95+1i /
  combinations 20 / performance 11+3i / integration_tests 34+1i / docker 25 /
  cli_integration 9 / e2e_advanced 226 —— **0 failed**；fmt/clippy/doc/deny/audit 全净。
- 场景固化：docs/TEST_SCENARIOS.md（金字塔基线/目标落点/15 mod E2E 场景/核正 12 项/矩阵/门槛）。


## 阶段 3：文档-代码一致性（a45f588）

- 核对范围：README/README_EN/docs/*.md/examples/README + src/lib.rs html_root_url
（版本号 43 处统一 rc.2 全仓口径 / MSRV 文字声明 16 处对齐 1.97.1 / feature 名与
Cargo.toml 一致 / 徽章动态化 / examples 清单零缺失 / API 面类型级+方法级对照零缺失 /
文档相对链接零断链）
- 本仓修复：API_REFERENCE+ARCHITECTURE 虚构 http_server(host,port) 改写为实际 enable_http_server/http_host/http_port；CONTRIBUTING 1.94→1.97.1 ×4
- 验证：纯文档变更；sdforge（唯一代码属性变更 html_root_url）cargo check --workspace
--all-features 通过
- 依赖清单同步：base/scripts/dep_versions.json 重新生成（258 项；阶段 1 后 crates.io
新发布项保持阶段 1 基线不追新，发布稳定性优先，脚本 INTERNAL_CRATES 补 trait-kit-derive）
