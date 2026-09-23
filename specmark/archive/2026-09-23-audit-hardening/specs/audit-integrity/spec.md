# Spec — audit-integrity

> Delta spec for change `audit-hardening`. 覆盖此变更引入/修改的审计链与运行健壮性需求。

## Requirements

### R-audit-001: 审计链接入轮转管线
FileSink 启用 `audit_chain_enabled` 后，每次成功轮转向 HMAC-SHA256 前向链 append 条目 `{rotated_path, sha256, timestamp}`，并将链落盘为 `<stem>.chain.jsonl`。
**验收标准：**
- 触发轮转后 manifest 文件新增一条目，`verify_entries` 校验通过。
- 篡改 manifest 任一字节（条目删除/重排/内容修改/换盐）后 `verify_entries` 失败（既有审计链测试语义复用）。
- 未启用配置时不产生 manifest、无性能开销。

### R-audit-002: 验链 CLI
`inklog-cli verify-chain <manifest>` 子命令重算 HMAC 链并输出 ok 或 tampered（含首个失败条目号），密钥读 `INKLOG_AUDIT_KEY` 环境变量。
**验收标准：**
- 对 R-audit-001 产物执行输出 ok；篡改后输出 tampered 与条目号。
- 环境变量缺失时返回明确错误（i18n）。

### R-audit-003: net 重连指数退避
TCP 连接失败后按 `min(2^n × 100ms, 30s)` 指数退避；退避期间 write 直接入缓冲不发起 connect；连接成功清零计数。
**验收标准：**
- 连续失败 N 次后退避间隔按指数增长并封顶 30s（测试以注入时钟/计数断言）。
- 成功重连后缓冲按 FIFO 补发（既有语义不回归）。

### R-audit-004: 解压大小上限
查询与内存解压路径施加 `total_output_limit`（默认 1 GiB），流式读取超限即返回 `query-decompression_limit_exceeded` 错误。
**验收标准：**
- 高压缩比样本解压触发 limit 错误而非无界内存分配。
- 正常日志文件（< 上限）解压内容与现状逐字节一致。

### R-audit-005: 内部故障 ops 事件
file sink 轮转/压缩/加密失败与 worker sink 降级/恢复路径发布结构化 ops 事件（经构造注入的通道句柄，manager 为默认装配点）。
**验收标准：**
- 注入测试句柄后触发轮转失败，收到 `sink_degraded` 事件（kind/timestamp/sink/detail 字段完整）。
- ops 通道满时丢弃并计 `channel_blocked`，不反压主日志链路（既有语义）。

## Constraints

- HMAC 链实现、常量时间比较、盐一致性校验等既有密码学行为零修改（只接入不重写）。
- 退避状态不引入跨线程锁（原子量足够）。
- 解压上限常量可被调用方覆盖，库内默认 1 GiB。

## Out of Scope

- 审计链跨进程续链（加载既有 manifest 延链）——本轮只保证单实例链完整并落盘。
- UDP 路径退避（无连接语义，不适用）。
