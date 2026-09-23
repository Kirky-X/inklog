<!-- domain: code -->
# audit-hardening

## Motivation

2026-09-23 对 inklog 的"大厂日志面试题"逐维审计（异步性能、trace 上下文、脱敏、加密、可靠性合规、系统设计六个专题）发现 20+ 处具体踩雷点，其中 6 处为高危：三出口 sink 绕过 PII 掩码导致转发场景明文泄漏、ERROR/FATAL 兜底缓冲运行期不补发、OTLP 导出丢弃链路 ID、FileSink 锁内同步 IO、DB sink 敏感键检测失效、文件权限与留存合规缺陷。这些问题使"企业级日志基础设施"的宣称在脱敏覆盖、崩溃一致性、链路关联三个维度上不成立，需全部修复。

## Scope

按模块分 6 组修复，共 16 个问题组（M01–M16，RICE 排序定优先级）：

1. **脱敏管线**（masking.rs / sanitize.rs / subscriber.rs）：正则缺陷（CJK 边界漏报、身份证小写 x、时间戳/git SHA 误伤）、键名检测补姓名词表、递归深度与匹配上限、非字符串 JSON 值掩码、fast-masking 接通。
2. **sink 掩码覆盖**（net.rs / otlp.rs / ring_buffered_file.rs / database_impl.rs / console.rs / file.rs）：net/otlp/ChannelBufferedFileSink 三出口接入 DataMasker；DB sink 键值遍历式掩码；sink 层 masker 注入点。
3. **可靠性**（subscriber.rs / manager.rs / file.rs / workers.rs）：ERROR/FATAL 兜底缓冲运行期补发、可配 fsync + 空闲期周期 flush、FileSink BufWriter 与锁外 IO、对象池归还路径。
4. **门面与导出**（log_adapter.rs / otlp.rs / manager.rs）：log 门面桥接 tracing span 上下文、log 门面 per-target 过滤、OTLP 编码补 traceId/spanId/severityNumber + 可选 TLS、set_level 同步 log::set_max_level。
5. **安全合规**（file.rs / path.rs / file_sink.rs）：日志/密文/压缩产物 0600 权限、年龄清理分支修复 + 等保 6 个月留存配置。
6. **审计链与杂项**（audit_chain.rs / file.rs / cli / net.rs / query.rs / compression.rs / manager.rs）：审计链接入轮转管线 + 验链 CLI、net 重连指数退避、解压大小上限、内部故障发布 ops_event。

## Non-Goals

- **中文姓名值模式识别**：无上下文的 2–4 汉字串无法与普通中文区分，值模式必然大面积误伤；仅补键名检测（name/surname/姓名 等词表）。
- **真实 OpenTelemetry 层集成**（读 span extensions / W3C traceparent 提取）：trace_id 派生机制保持现状，仅保证已捕获的 ID 不在导出处丢失；真 OTel 集成留待独立变更。
- **ChannelBufferedFileSink 接入 manager 替换默认 FileSink**：改动面过大，本轮仅让该 sink 具备掩码能力，替换主路径另立变更。
- **版本号发布与 CHANGELOG**：修复以代码+测试交付，release 流程走既有 GitHub 工作流（v* tag），不在本变更内。
