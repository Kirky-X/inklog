# Spec — reliability

> Delta spec for change `audit-hardening`. 覆盖此变更引入/修改的写入可靠性需求。

## Requirements

### R-rel-001: ERROR/FATAL 兜底缓冲运行期补发
async 通道满/超时时进入 fallback_buffer 的 ERROR/FATAL 记录，在运行期（不依赖进程退出）被补发到目标 sink。
**验收标准：**
- channel 打满导致 ERROR 进入 fallback 后，恢复投递能力的一个触发周期（≤60s 定时，或通道从满恢复至半满的立即触发）内该记录出现在 file sink 输出。
- 补发去重语义保留（已投递通道不重复补发）。
- fallback 容量与 LRU 淘汰行为不回归（上限 100）。

### R-rel-002: 可配 fsync
`FileSinkConfig` 新增 `fsync: bool`（默认 false），启用时每次批量 flush 后对文件 `sync_all()`。
**验收标准：**
- `fsync=true` 时写入返回后立即读取文件可见记录（断电窗口语义由测试以即时可见性近似验证）。
- 默认 false 时行为与现状一致（无 sync_all 调用路径）。

### R-rel-003: 空闲期周期 flush
FileSink 既有后台定时 tick 追加空闲 flush：batch_buffer 非空且距上次 flush ≥ flush_interval_ms 时执行刷盘。
**验收标准：**
- 停止产生日志后，最多一个 tick 周期（≤60s，测试中可注入更短间隔）内 batch 中记录落盘可读。
- 空闲且 batch 为空时 tick 不产生 IO。

### R-rel-004: FileSink 锁外 IO
`flush_batch_inner` 的序列化在锁内完成，文件写入移出写锁；文件句柄改用 64 KiB BufWriter。
**验收标准：**
- 多线程并发写 1000 条后文件行数正确、每行 JSON 完整无交错。
- 既有 file sink 功能测试全绿。

### R-rel-005: 对象池归还
worker 消费端在 `Arc<LogRecord>` 引用计数归一（`Arc::try_unwrap` 成功）时将记录归还 thread-local 池。
**验收标准：**
- 连续写 100 条后池非空（复用发生），池暴露的测试计数器非零。
- 归还前 fields 清空，复用记录不携带上一条残留数据（既有日志内容正确性测试全绿）。

## Constraints

- 有界 channel 容量、send_timeout(100ms) 背压语义、丢弃计数指标（logs_dropped/channel_blocked）不回归。
- graceful shutdown 排水（file/db 30s、console 5s）行为不变。
- 不引入新的无条件锁竞争：归还路径仅 `Arc::try_unwrap` 成功时执行，失败（多引用）直接释放。

## Out of Scope

- 磁盘持久化 fallback 队列（另立变更）。
- worker 批处理调度合并（block_on 摊销）——若实现中确认风险高于收益，记录于 converge 摘要。
- ChannelBufferedFileSink 接入 manager 主路径。
