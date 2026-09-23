# Design — audit-hardening

## Context

审计已给出全部踩雷点的 file:line 证据。约束：63k 行存量代码、19 仓共享的 pre-commit 钩子（typos 自动改写、fmt/clippy 门禁）、错误消息必须走 fluent i18n（既有基线）、禁止绕过钩子提交。修复必须前向进行（不回退既有行为契约），公共 API 变更以新增方法/配置字段为主，破坏性重命名不做。

## Decision

### D1 脱敏正则与防护（masking.rs / subscriber.rs）

- 身份证：`[\dX]` → `[\dXx]`，两侧加数字/字母负向断言 `(?<![0-9A-Za-z])…(?![0-9A-Za-z])` 替代 `\b`（CJK 汉字属 Unicode 词字符导致 `电话138…` 漏报）。
- phone / bank_card / ssn 等依赖 `\b` 的数字规则同改负向断言；bank_card 收紧为 16–19 位（消除 13 位毫秒时间戳误伤），credit_card 保留 Luhn 路径。
- passport 收紧：`(?<![0-9a-fA-F])[EeGg][A-Za-z0-9]{8}(?![0-9a-fA-F])` 且后 8 位至少含 1 个字母（排除 git 短 SHA）。
- `SENSITIVE_FIELD_PATTERNS` 增补姓名词表（`name`/`surname`/`full_name`/`姓名`/`用户名`→ 不掩值本身错误高，仅掩 `id_name` 类组合？——定：直接纳入键名检测，命中即 `***MASKED***`，与 password 同策略）。
- `mask_value` / `mask_hashmap` / `sanitize_field_value` 增加 `MAX_MASK_DEPTH=16` 递归上限（超限打 `***TRUNCATED***`）；`mask()` 输入超 1 MiB 跳过并 `tracing::warn` + 计数。
- `mask_value` 的 `_ => {}` 空分支改为：Number/Bool 转 `to_string()` 后过掩码正则再回填；Null 保持跳过。

### D2 sink 掩码覆盖（新增 `support/processing/sink_masking.rs` 辅助）

- 新增共享构造函数 `shared_masker(masking_enabled: bool) -> Option<DataMasker>`，所有 sink 统一用它（替代各自 `DataMasker::new()` 硬编码）。
- net.rs：`write` 序列化前对 record 跑 masker（`masking_enabled` 门控，默认 true 与 file/console 一致）。
- otlp.rs：`encode_log_record` 入口先 mask message + fields。
- ring_buffered_file.rs：`write_blocking`/async write 在 `template.render` 前对 record 掩码。
- database_impl.rs：废弃"序列化后纯文本正则"，改 `mask_hashmap`（键检测参与）+ 字符串字段值掩码。
- 三个 sink 的 config 增加 `with_masker` 注入点（`DataMaskerBuilder` 自定义规则可用）；fast-masking feature 下 `DataMasker::builder().build()` 对 `is_literal=true` 规则自动构建 AcMasker 并在文档标注内置规则仍走正则。

### D3 可靠性（subscriber.rs / manager.rs / file.rs / workers.rs）

- fallback 补发：`LoggerSubscriber` 增加 `try_flush_fallback` 的运行期触发——manager 启动一个低频定时任务（60s 间隔）调用 subscriber 句柄；channel 恢复空闲（`capacity() > 半满`）时立即触发一次。subscriber 由 `Arc` 持有以便 manager 持引用。
- fsync：`FileSinkConfig` 新增 `fsync: bool`（默认 false），flush_batch_inner 完成写入后按配置 `sync_all()`。
- 空闲滞留：FileSink 后台轮转定时器线程（已有 60s tick）追加职责——发现 batch_buffer 非空且距上次 flush ≥ flush_interval_ms 即触发 flush。
- FileSink 性能：`inner` 持 `BufWriter`（容量 64KiB）；序列化在锁内、`writeln!`+flush 移到锁外（序列化产物 buffer 出锁后写）；worker 对同 sink 连续写合并为单次 `block_on`（批处理循环：drain 至多 N 条再逐条 write，仍逐条但共享一次调度——保持简单：维持现状仅做 BufWriter+锁外，block_on 摊销若实现风险高则记录 Consequence）。
- 对象池归还：worker 消费完 `Arc<LogRecord>` 后 `Arc::try_unwrap` 成功则 `put_log_record` 归还（fields 清空复用）。

### D4 门面与导出（log_adapter.rs / otlp.rs / manager.rs）

- log 门面：`log()` 内 `tracing::Span::current()` 非空时提取 span id（复用 subscriber.rs 的派生逻辑，抽成 `fn derive_ids(span) -> (Option<String>, Option<String>)` 共享）。
- log 门面 per-target：`LogAdapter` 持有 `EnvFilter` 快照（初始化与 set_level 时同步），`enabled()` 先查 filter。
- OTLP：`encode_log_record` 输出加 `traceId`（32hex）/`spanId`（16hex）/`severityNumber`（映射 SEVERE=17/ERROR=17? 标准：DEBUG=5 INFO=9 WARN=13 ERROR=17 FATAL=21）；https URL + `net-sink` feature 启用时走 rustls `StreamOwned`（复用 net.rs 模式），未启用时构造期报错提示启用 feature。
- set_level：manager `set_level` 末尾同步 `log::set_max_level(映射后的 LevelFilter)`。

### D5 安全合规（file.rs / file_sink.rs）

- 权限：FileSink 打开/创建文件改用 `OpenOptions::mode(0o600)`（unix）；`encrypt_file`/压缩输出 `File::create` → `create_validated_file`（已有 0600+O_NOFOLLOW）；目录 `create_dir_all` 后 `set_permissions(0o700)`（仅新建目录时）。
- 留存：`perform_cleanup` 重构为两个独立阶段——大小清理与年龄清理互不遮蔽；`FileSinkConfig::default` 的 `retention_days` 文档标注等保场景应设 ≥180 并在 `validate` 时对 `encrypt=true`（审计级）配置发 warn 提示 <180。

### D6 审计链与杂项

- audit_chain 接入：FileSink 增加 `audit_chain: Option<Mutex<ArchiveChain>>`（`audit_chain_enabled` 配置），轮转成功后 append `{rotated_path, sha256, record_count, timestamp}`，manifest 写 `<stem>.chain.jsonl`；CLI 增加 `inklog-cli verify-chain <manifest>` 子命令（重算 HMAC 链比对，密钥取环境变量）。
- net 退避：NetSink 持 `AtomicU32 consecutive_failures`，connect 失败后退避 `min(2^n * 100ms, 30s)`，成功清零；退避期间 write 直接入缓冲不尝试连接。
- 解压上限：query.rs 与 compression.rs 解压改流式 + `total_output_limit`（默认 1 GiB，超限报 `InklogError::DecompressionLimitExceeded`（i18n 键））。
- ops_event：file.rs 轮转失败/压缩失败/加密失败、workers.rs sink 降级与恢复、subscriber 丢弃计数新增阈值告警处调用 `publish_ops_event`（经 manager 句柄；FileSink 无 manager 引用——用 `tracing::warn!` 到 error sink 的既有通道 + ops 通道经 `OpsEventSender` 注入构造）。

## Alternatives Considered

- **AC 自动机加速内置正则规则**：不可行——aho-corasick 只匹配 literal，内置规则需要边界语义；保持"AC 仅服务 literal 自定义规则"，接通 builder 自动构建 + 文档如实。
- **fallback 改为持久化磁盘队列**：改动面大（新文件格式、清理），60s 周期补发已消除"运行期永不补发"的主缺陷，持久化队列留后续变更。
- **替换默认 FileSink 为 ChannelBufferedFileSink**：后者缺轮转/压缩/加密/掩码全链能力，先补掩码，替换本身另立变更。
- **全量 fsync 默认开启**：日志库吞吐优先，fsync 是审计场景需求，配置默认 false + 文档指引等保场景开启。

## Consequences

- 正面：三出口泄漏面关闭；ERROR 兜底真实生效；崩溃一致性可选达标；审计链从库能力变产品行为。
- 负面/技术债：FileSink 串行化根因（单写线程）仍在，BufWriter 仅摊销 syscall；masker 注入点与配置面扩大需要文档同步（README/docs/SECURITY.md）；OTLP TLS 依赖 net-sink feature 的 feature 矩阵组合需在 docs.rs metadata 验证。
- 跟进项：真 OTel 层集成、ChannelBufferedFileSink 转正、磁盘持久化 fallback 队列。
