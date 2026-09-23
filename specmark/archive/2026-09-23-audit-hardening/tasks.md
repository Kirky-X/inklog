# Tasks — audit-hardening

<!-- 任务按模块分组排序（同文件改动集中），P 标签来自 kueiku RICE 排序（P0=Top5: M13/M06/M12/M02/M09） -->

- [x] [T001] [P1] M03 脱敏正则缺陷修复：masking.rs 中 id_card 支持 `[\dXx]` 并加两侧 `(?<![0-9A-Za-z])…(?![0-9A-Za-z])` 断言替代 `\b`；phone/ssn/bank_card 同改断言，bank_card 收紧为 16–19 位；passport 加 hex 排除断言且要求后 8 位含字母。新增测试断言：`"电话13812345678"`（汉字紧贴）被掩码、小写 x 结尾身份证被掩码、13 位毫秒时间戳不被 bank_card 掩码、git 短 SHA `e1a2b3c4d` 不被 passport 掩码、`E12345678` 仍被掩码 → src/support/processing/masking.rs
- [x] [T002] [P1] M04 掩码 DoS 防护：masking.rs `mask_value`/`mask_hashmap` 与 subscriber.rs `sanitize_field_value` 增加 MAX_MASK_DEPTH=16 递归上限（超限值替换为 `***TRUNCATED***`）；`mask()` 对输入超 1 MiB 的字符串跳过并 `tracing::warn`。测试：1000 层嵌套 JSON 掩码正常返回不栈溢出；2 MiB 输入原样返回 → src/support/processing/masking.rs, src/domain/core/subscriber.rs
- [x] [T003] [P1] M02b 非字符串值掩码：masking.rs `mask_value` 的 `_ => {}` 分支改为 Number/Bool 调 `to_string()` 后过掩码正则、命中则替换为掩码 Value，Null 保持跳过。测试：字段值为 `Value::Number(13812345678)` 时输出变掩码字符串 → src/support/processing/masking.rs
- [x] [T004] [P0] M02a DB sink 键检测修复：database_impl.rs 放弃"先 to_string 再纯文本正则"，改用 `mask_hashmap`（键名检测参与）处理 fields 后再序列化。测试：fields 含 `password` 键（值 16+ 字符）落库 JSON 中值为 `***MASKED***` → src/integrations/infra/database_impl.rs
- [x] [T005] [P1] M01a net sink 接入掩码：net.rs `write` 序列化前对 record 执行 DataMasker（`masking_enabled` 配置门控，与 console/file 语义一致）。测试：向 net sink 写入含手机号的记录，TCP 服务端收到的 JSON 无明文手机号 → src/support/io/sink/net.rs
- [x] [T006] [P1] M01b otlp sink 接入掩码：otlp.rs `encode_log_record` 入口先掩码 message 与 fields。测试：OTLP 编码产物 body/attributes 无明文手机号 → src/support/io/sink/otlp.rs
- [x] [T007] [P1] M01c ChannelBufferedFileSink 接入掩码：ring_buffered_file.rs 两条写路径在 `template.render` 前对 record 掩码。测试：经该 sink 写出的文件无明文手机号 → src/support/io/sink/ring_buffered_file.rs
- [x] [T008] [P2] M05 fast-masking 接线与 masker 注入：DataMasker builder 在 fast-masking feature 下自动为 `is_literal=true` 规则构建 AcMasker；console/file/net/otlp/ring_buffered sink 增加公开 `with_masker` 注入点。测试（feature=fast-masking）：literal 自定义规则命中 AC 路径完成掩码；注入自定义规则的 sink 输出体现该规则 → src/support/processing/masking.rs, src/support/io/sink/{console,net,otlp,ring_buffered_file}.rs
- [x] [T009] [P0] M12 set_level 级别同步：manager.rs `set_level` 末尾按新级别调用 `log::set_max_level`（复用 install 时的 LevelFilter 映射）。测试：init 后 `set_level(debug)`，经 `log::debug!` 的记录能通过 LogAdapter enabled 检查落盘 → src/domain/core/manager.rs
- [x] [T010] [P1] M10a log 门面 trace 上下文桥接：subscriber.rs 抽出 `derive_trace_ids(span) -> (Option<String>, Option<String>)` 共享函数，log_adapter.rs `log()` 在 `tracing::Span::current()` 非空时填充 trace_id/span_id。测试：在 `info_span` 内调 `log::info!`，落盘记录 trace_id 非空 → src/domain/core/subscriber.rs, src/support/io/log_adapter.rs
- [x] [T011] [P2] M10b log 门面 per-target 过滤：LogAdapter 持有 EnvFilter 快照（init 与 set_level 时同步），`enabled()` 先查 target 指令再查全局 max_level。测试：`target_levels` 设 `myapp=warn` 后 `log::info!(target: "myapp", ...)` 被丢弃 → src/support/io/log_adapter.rs, src/domain/core/manager.rs
- [x] [T012] [P1] M11a OTLP 链路字段：otlp.rs 编码输出追加 `traceId`（32 hex）/`spanId`（16 hex）/`severityNumber`（TRACE=5 DEBUG=5 INFO=9 WARN=13 ERROR=17 FATAL=21）。测试：带 trace 上下文的记录编码后 JSON 含宽度正确的三字段 → src/support/io/sink/otlp.rs
- [x] [T013] [P2] M11b OTLP TLS：otlp.rs https URL 在 net-sink feature 下经 rustls `StreamOwned` 传输（复用 net.rs 连接模式），feature 未启用且 URL 为 https 时构造返回明确错误。测试：https URL 无 net-sink feature 时构造错误信息含 feature 提示 → src/support/io/sink/otlp.rs
- [x] [T014] [P0] M06 fallback 运行期补发：manager.rs 持有 subscriber `Arc` 并启动 60s 周期任务调用 `try_flush_fallback`；async 通道从满恢复至半满时立即触发一次。测试：先打满 channel 使 ERROR 进 fallback，恢复后等待一个周期断言 ERROR 记录补发到 file sink → src/domain/core/manager.rs, src/domain/core/subscriber.rs
- [x] [T015] [P0] M09 对象池归还：workers.rs 消费端写完后 `Arc::try_unwrap` 成功即 `put_log_record` 归还（清空 fields 复用）。测试：写 100 条后 thread-local 池命中（复用计数非零，池暴露 `len()` 供测试） → src/domain/core/workers.rs, src/support/processing/object_pool.rs
- [x] [T016] [P1] M07 崩溃一致性：file_sink.rs 配置新增 `fsync: bool`（默认 false），file.rs flush_batch_inner 写完按配置 `sync_all()`；file.rs 轮转定时器 tick 追加空闲 flush（batch_buffer 非空且距上次 flush ≥ flush_interval_ms 即刷）。测试：fsync=true 写入后立即读文件可得记录；空闲期一个 tick 内 batch 落盘 → src/domain/config/file_sink.rs, src/support/io/sink/file.rs
- [x] [T017] [P1] M08 FileSink 写路径优化：inner 改持 BufWriter（64 KiB），flush_batch_inner 中序列化在锁内完成、文件写入移到锁外执行。测试：多线程并发写 1000 条后行数正确且无交错损坏 → src/support/io/sink/file.rs
- [x] [T018] [P0] M13 文件权限加固：file.rs 日志文件创建加 `mode(0o600)`（unix），encrypt_file/压缩产物输出改 `create_validated_file`，新建目录 `set_permissions(0o700)`。测试（unix）：sink 文件、.enc、.zst 产物 mode 均为 0o600 → src/support/io/sink/file.rs, src/support/io/sink/compression.rs
- [x] [T019] [P1] M14 留存清理修复：file.rs `perform_cleanup` 拆为独立的大小清理与年龄清理两阶段（年龄清理不再被 max_total_size 分支遮蔽，parse_size 为 None 时年龄清理仍执行）；file_sink.rs validate 对 `encrypt=true 且 retention_days<180` 发 warn（等保提示）。测试：max_total_size 未超限时过期文件仍被删除 → src/support/io/sink/file.rs, src/domain/config/file_sink.rs
- [x] [T020] [P2] M15a 审计链接入轮转：file.rs 增加 `audit_chain_enabled` 配置与 `Option<Mutex<ArchiveChain>>`，轮转成功后 append `{path, sha256, timestamp}` 并落盘 `<stem>.chain.jsonl`。测试：触发轮转后 manifest 新增一条目且 `verify_entries` 通过 → src/support/io/sink/file.rs, src/support/audit_chain.rs
- [x] [T021] [P2] M15b 验链 CLI：cli 增加 `verify-chain <manifest>` 子命令（密钥读 INKLOG_AUDIT_KEY 环境变量，重算 HMAC 链输出 ok/tampered 与条目号）。测试：assert_cmd 对 T020 产物 verify 输出 ok，篡改 manifest 后输出 tampered → src/cli/main.rs, src/cli/cli_impl.rs
- [x] [T022] [P2] M16a net 重连退避：net.rs 增加 `consecutive_failures: AtomicU32`，connect 失败退避 `min(2^n*100ms, 30s)` 期间 write 直接入缓冲，成功清零。测试：模拟连接失败 N 次断言退避间隔按指数增长且封顶 30s → src/support/io/sink/net.rs
- [x] [T023] [P2] M16b 解压大小上限：query.rs 与 compression.rs 解压改流式读取并施加 `total_output_limit`（默认 1 GiB），超限返回新增 i18n 错误 `query-decompression_limit_exceeded`。测试：构造高压缩比样本解压触发 limit 错误而非 OOM → src/support/query.rs, src/support/io/sink/compression.rs, src/i18n/
- [x] [T024] [P2] M16c 内部 ops 事件发布：file.rs 轮转/压缩/加密失败、workers.rs sink 降级与恢复路径调用 ops 事件发布（经构造注入的 OpsEvent 通道句柄，manager 为默认装配点）。测试：注入测试句柄后触发轮转失败，断言收到 `sink_degraded` 事件 → src/support/io/sink/file.rs, src/domain/core/workers.rs, src/domain/core/manager.rs

## Phase 1: Convergence

_由 /specmark converge 于 2026-09-23 生成。仅追加：不要编辑之前的任务。_

**发现缺口：** 5 (CRITICAL: 0 | HIGH: 0 | MEDIUM: 5 | LOW: 2)
**追加任务：** 5（跳过：2 个 LOW/unrequested，记录为叙述）
**未请求范围（按原样接受）：** 无

**验收标准检查（5 个 delta spec）：**
| R-ID | 验收条件 | 状态 |
|------|----------|------|
| R-masking-001..006 | CJK 边界/深度上限/非字符串值/DB 键值/三出口/注入+AC | ✓ PASS（既有+新增测试全绿） |
| R-rel-001/002/004/005 | fallback 补发/fsync/并发完整性/对象池归还 | ✓ PASS |
| R-rel-003 | 空闲期一个 tick 内落盘（可注入间隔） | ✗ 部分——逻辑已实现但 tick 硬编码 60s，无可注入间隔的自动化测试 |
| R-trace-001..005 | 门面桥接/per-target/级别同步/OTLP 字段/https 门 | ✓ PASS |
| R-sec-001 | 日志/加密/压缩产物 0600 | ✗ 部分——压缩产物 0600 缺自动化测试 |
| R-sec-002/003 | 年龄清理独立/等保 warn | ✓ PASS（warn 触发断言记 ? UNKNOWN，人工确认） |
| R-audit-001 | 链接入轮转+manifest 验证+篡改检测 | ✓ PASS |
| R-audit-002 | verify-chain CLI ok 路径自动化 | ✗ 部分——仅人工冒烟（tampered exit 2 已验证），ok 路径无断言测试 |
| R-audit-003/004 | 退避指数封顶/解压上限 | ✓ PASS |
| R-audit-005 | 轮转/压缩/加密失败发 ops 事件 | ✗ 部分——压缩/加密已接，轮转 rename 失败路径未接 publish |
| Constraints | 新增错误/警告消息走 fluent i18n | ✗ 部分——masking 超限 warn 与留存 warn 为硬编码英文 |

**LOW/unrequested 跳过记录：** ① masking 超限跳过只发 warn 无计数器指标（LOW，指标语义已由 logs_dropped 覆盖）；② worker 恢复事件 detail 中 attempts 恒为 0（LOW，重置后取值，信息性字段）。

- [x] [T025] [P1] 新增两条警告消息 i18n 化：masking.rs mask() 超限跳过 warn 与 file_sink.rs validate 留存 warn 改走 fluent 键（masking-limit-exceeded / file-retention-compliance-hint，en/zh 成对） → src/support/processing/masking.rs, src/domain/config/file_sink.rs, locales/
- [x] [T026] [P1] 轮转定时器 tick 间隔参数化（默认 60s，测试可传 1s），补空闲 flush 自动化测试：写 1 条后空闲，一个 tick 内断言落盘 → src/support/io/sink/file.rs
- [x] [T027] [P2] 压缩产物 0600 权限自动化测试（gzip feature 门控，对 .gz 产物 mode 断言） → src/support/io/sink/compression.rs
- [x] [T028] [P1] rotate_inner rename 失败路径发布 sink_degraded ops 事件（与压缩/加密失败同语义） → src/support/io/sink/file.rs
- [x] [T029] [P2] verify-chain ok 路径自动化测试：以 T020 同法生成真实 manifest，调用 verify_chain_manifest 断言退出码 0 → src/cli/cli_impl.rs
