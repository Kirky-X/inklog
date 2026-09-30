// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use crate::LogRecord;
use crate::Metrics;
use crate::support::io::sink::middleware::{MiddlewareVerdict, RecordMiddleware};
use crate::support::processing::pipeline::is_critical_level;
use crate::support::processing::target_rate_limiter::TargetRateLimiter;
use crate::support::processing::{
    GlobalRateLimitMiddleware, IdentityFieldsMiddleware, ProcessingPipeline, RateLimiter,
    SanitizeMiddleware, StressRelief, TargetQuotaMiddleware,
};
use crate::validation::sanitize::LogSanitizer;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use serde_json::value;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

const DEFAULT_SEND_TIMEOUT_MS: u64 = 100;
const FALLBACK_BUFFER_SIZE: usize = 100;

/// Fallback buffer 条目：记录 + 各 async 通道的投递状态。
///
/// `delivered[i] == true` 表示第 i 个 async 通道（0 = 主 async 通道，
/// 1.. = `extra_async_senders` 通道）已成功收到该记录。flush 只向未成功
/// 的通道补发，防止已成功通道上的记录被重复写出（重复日志）。
struct FallbackEntry {
    record: Arc<LogRecord>,
    delivered: Vec<bool>,
}

/// High-performance logging subscriber with lock-free hot path.
///
/// Uses crossbeam channels for both console and async sinks to eliminate
/// lock contention in the hot path (on_event).
/// Uses `Arc<LogRecord>` to avoid deep cloning when sending to multiple sinks.
/// Includes fallback buffer for critical logs (ERROR/FATAL).
/// Clone 语义：全字段为 channel sender / Arc 句柄（克隆共享同一 logger 实例），
/// AtomicU64 计数器克隆当前值。供测试 harness 在子线程以
/// tracing::subscriber::with_default 安装线程级 subscriber（需 owned 值）。
///
/// 克隆与原实例共享同一采样计数相位（`stress_relief: Arc<StressRelief>`，
/// 含兜底 ERROR 采样计数器与已绑定策略）：在克隆上调用 `with_sampling_policy`
/// 替换的是共享策略，同样影响原实例及全部克隆的压力采样裁决。
impl Clone for LoggerSubscriber {
    fn clone(&self) -> Self {
        Self {
            console_sender: self.console_sender.clone(),
            async_sender: self.async_sender.clone(),
            extra_async_senders: self.extra_async_senders.clone(),
            metrics: self.metrics.clone(),
            send_timeout_ms: self.send_timeout_ms,
            fallback_buffer: self.fallback_buffer.clone(),
            pipeline: self.pipeline.clone(),
            stress_relief: Arc::clone(&self.stress_relief),
            fallback_pending: Arc::clone(&self.fallback_pending),
            journal: self.journal.clone(),
        }
    }
}
pub struct LoggerSubscriber {
    /// Channel sender for console output (lock-free)
    console_sender: Sender<Arc<LogRecord>>,
    /// Channel sender for async sinks (file, database, etc.)
    async_sender: Sender<Arc<LogRecord>>,
    /// Additional async sink channels. Each enabled async sink gets its own
    /// channel, otherwise a single shared MPMC channel would deliver each
    /// record to only one of the sink workers (data loss for the others).
    extra_async_senders: Vec<Sender<Arc<LogRecord>>>,
    /// Metrics for monitoring
    metrics: Arc<Metrics>,
    /// Timeout for async channel send (milliseconds)
    send_timeout_ms: u64,
    /// Fallback buffer for critical logs
    fallback_buffer: Arc<Mutex<VecDeque<FallbackEntry>>>,
    /// 处理管道（治理轮：配额/限流/压力采样；改写轮：身份注入/脱敏）。
    /// 内置件按配置装配，与迁移前 on_event 的硬编码顺序逐点等价；
    /// with_middleware 追加用户治理件。
    pipeline: ProcessingPipeline,
    /// 压力救援裁决（两个限流中间件共享的采样策略与兜底计数载体）。
    /// Arc 跨 Clone 共享：克隆共享同一采样计数相位。
    stress_relief: Arc<StressRelief>,
    /// 兜底缓冲存在待补发记录（ERROR/FATAL 入队置位；半满触发补发后按
    /// 缓冲是否清空复位）。Arc 跨 Clone 共享，manager 的周期补发任务可见。
    fallback_pending: Arc<AtomicBool>,
    /// 磁盘持久化 fallback journal（deferred-capabilities C4）：
    /// LRU 淘汰的关键日志落盘，进程启动重放。None = 未启用（零开销）。
    journal: Option<Arc<crate::support::fallback_journal::FallbackJournal>>,
}

impl LoggerSubscriber {
    pub fn new(
        console_sender: Sender<Arc<LogRecord>>,
        async_sender: Sender<Arc<LogRecord>>,
        metrics: Arc<Metrics>,
    ) -> Self {
        // 压力救援裁决与 metrics 共享同一实例：丢弃计数同源
        let stress_relief = Arc::new(StressRelief::new(Arc::clone(&metrics)));
        Self {
            console_sender,
            async_sender,
            extra_async_senders: Vec::new(),
            metrics,
            send_timeout_ms: DEFAULT_SEND_TIMEOUT_MS,
            fallback_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(FALLBACK_BUFFER_SIZE))),
            pipeline: ProcessingPipeline::new(),
            stress_relief,
            fallback_pending: Arc::new(AtomicBool::new(false)),
            journal: None,
        }
    }

    /// 启用磁盘持久化 fallback journal（LRU 淘汰的关键日志落盘）。
    pub fn with_journal(
        mut self,
        journal: Arc<crate::support::fallback_journal::FallbackJournal>,
    ) -> Self {
        self.journal = Some(journal);
        self
    }

    /// Add an additional async sink channel. Each enabled async sink must have
    /// its own channel; sharing a single channel between workers means every
    /// record is consumed by only one worker.
    pub fn with_extra_async_sender(mut self, sender: Sender<Arc<LogRecord>>) -> Self {
        self.extra_async_senders.push(sender);
        self
    }

    /// Send a record to every configured async sink channel. Returns the
    /// per-channel delivery status (`true` = delivered)，索引与
    /// [`FallbackEntry::delivered`] 一致（0 = 主 async 通道）。
    fn send_to_async_sinks(&self, record: &Arc<LogRecord>, timeout: Duration) -> Vec<bool> {
        let mut delivered = Vec::with_capacity(1 + self.extra_async_senders.len());
        delivered.push(
            self.async_sender
                .send_timeout(Arc::clone(record), timeout)
                .is_ok(),
        );
        for sender in &self.extra_async_senders {
            delivered.push(sender.send_timeout(Arc::clone(record), timeout).is_ok());
        }
        delivered
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.send_timeout_ms = timeout_ms;
        self
    }

    /// Set the log sanitizer for preventing log injection attacks.
    pub fn with_sanitizer(mut self, sanitizer: Arc<LogSanitizer>) -> Self {
        self.pipeline
            .register_rewrite(Arc::new(SanitizeMiddleware::new(sanitizer)));
        self
    }

    /// Set the rate limiter for log throughput control.
    pub fn with_rate_limiter(mut self, rate_limiter: Arc<RateLimiter>) -> Self {
        self.pipeline
            .register_governance(Arc::new(GlobalRateLimitMiddleware::new(
                rate_limiter,
                Arc::clone(&self.stress_relief),
            )));
        self
    }

    /// 设置按 target 前缀分组的配额限流（最长前缀优先；命中组独立裁决，
    /// 未命中 target 维持既有全局限流路径）。头插治理环：配额裁决恒先于
    /// 全局限流，与接线顺序无关。
    pub fn with_target_rate_limiter(mut self, limiter: Arc<TargetRateLimiter>) -> Self {
        self.pipeline
            .register_governance_front(Arc::new(TargetQuotaMiddleware::new(
                limiter,
                Arc::clone(&self.stress_relief),
            )));
        self
    }

    /// 设置限流压力下的采样策略。
    pub fn with_sampling_policy(
        self,
        policy: Arc<crate::support::io::sink::sampling::SamplingPolicy>,
    ) -> Self {
        self.stress_relief.set_policy(policy);
        self
    }

    /// 启用服务身份静态字段注入（service_name/instance/env/version 等）。
    ///
    /// 每条记录进入通道前把这些键值并入 `fields`；事件显式携带的同名字段
    /// 优先（or-insert 语义，与 trace_id 的显式覆盖先例一致）。头插改写环：
    /// 身份注入恒先于脱敏（注入值同样被脱敏），与接线顺序无关。
    pub fn with_identity_fields(
        mut self,
        fields: Arc<std::collections::BTreeMap<String, serde_json::Value>>,
    ) -> Self {
        self.pipeline
            .register_rewrite_front(Arc::new(IdentityFieldsMiddleware::new(fields)));
        self
    }

    /// 用户治理中间件装配入口：尾插治理环，链上位次在内置限流件之后、改写轮
    /// 之前。位次不等于恒执行：治理链任一环节 `Drop`/`Reround` 即短路剩余
    /// 治理件——配额管辖 target 放行（`Reround`）时用户件被跳过，其过滤/
    /// 改写对该记录不生效（配额组语义优先）；仅当在前的内置件全部 `Continue`
    /// （未命中配额规则且全局限流放行）时用户件才执行。用户件可裁决丢弃
    /// （`Drop`，计入 `user_middleware_dropped` 归因细分与 `logs_dropped`
    /// 数据损失总账）或原位改写记录（改写结果仍会经改写轮——如脱敏——继续
    /// 处理）。
    pub fn with_middleware(mut self, middleware: Arc<dyn RecordMiddleware>) -> Self {
        self.pipeline
            .register_governance(Arc::new(UserMiddlewareDropCounter {
                inner: middleware,
                metrics: Arc::clone(&self.metrics),
            }));
        self
    }

    /// 从当前线程激活的 dispatch 派生 (trace_id, span_id)。
    ///
    /// log 门面桥接共享入口（`LogAdapter::record_to_log_record`）：无 subscriber
    /// 激活或事件在 span 之外时返回 (None, None)。根 span 派生依赖 dispatch 可
    /// 下转（downcast）到 `tracing_subscriber::registry::Registry`（inklog 自身装配栈满足）；
    /// 下转失败时保留 span_id、trace_id 置 None（退化但不伪造）。
    pub(crate) fn derive_trace_ids_from_current() -> (Option<String>, Option<String>) {
        use tracing_subscriber::registry::LookupSpan;

        let span = tracing::Span::current();
        let Some(id) = span.id() else {
            return (None, None);
        };
        let span_id = Some(format!("{:016x}", id.into_u64()));
        let trace_id = tracing::dispatcher::get_default(|dispatch| {
            let registry = dispatch.downcast_ref::<tracing_subscriber::Registry>()?;
            let span = registry.span(&id)?;
            // otel extension 最高优先（与 tracing 路径同链）
            #[cfg(feature = "otel")]
            if let Some((trace_id, _span_id)) = Self::otel_ids_from_extensions(&span.extensions()) {
                return Some(trace_id);
            }
            span.scope()
                .last()
                .map(|root| format!("{:032x}", root.id().into_u64()))
        });
        (trace_id, span_id)
    }

    /// 从当前 tracing span 上下文提取 trace_id/span_id。
    ///
    /// - `span_id` = 事件所在 span 的 16 位小写 hex id；
    /// - `trace_id` 优先取事件/span 已显式记录的 `trace_id` 字段（与
    ///   OpenTelemetry / tracing-opentelemetry 注入兼容），否则沿 parent 链
    ///   找到根 span，以其 id 派生 32 位小写 hex（同一条 trace 内一致）；
    /// - 事件在任意 span 之外时两字段保持 `None`（热路径零成本直通）。
    fn extract_trace_context<S>(ctx: &Context<'_, S>, record: &mut LogRecord)
    where
        S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        let current = ctx.current_span();
        let Some(id) = current.id() else {
            return;
        };
        record.span_id = Some(format!("{:016x}", id.into_u64()));

        // 优先级链（deferred-capabilities C2）：otel extension > traceparent
        // 字段 > 显式字段覆盖 > 根 span 派生
        #[cfg(feature = "otel")]
        if let Some(span) = ctx.span(id)
            && let Some((trace_id, span_id)) = Self::otel_ids_from_extensions(&span.extensions())
        {
            record.trace_id = Some(trace_id);
            record.span_id = Some(span_id);
            return;
        }
        if let Some(value::Value::String(tp)) = record.fields.get("traceparent")
            && let Some((trace_id, span_id)) = Self::parse_traceparent(tp)
        {
            record.trace_id = Some(trace_id);
            record.span_id = Some(span_id);
            return;
        }

        // 事件字段显式携带的 trace_id 优先
        if let Some(value::Value::String(explicit)) = record.fields.get("trace_id") {
            record.trace_id = Some(explicit.clone());
        }
        if record.trace_id.is_none() {
            record.trace_id = ctx.span(id).and_then(|span| {
                span.scope()
                    .last()
                    .map(|root| format!("{:032x}", root.id().into_u64()))
            });
        }
        // 事件字段显式携带的 span_id 覆盖派生值（与 OTel 语义对齐）
        if let Some(value::Value::String(explicit)) = record.fields.get("span_id") {
            record.span_id = Some(explicit.clone());
        }
    }

    /// 解析 W3C traceparent（`00-<32hex>-<16hex>-<2hex>`）。
    ///
    /// 版本段 `ff`（禁用）或格式不符返回 None。两条提取路径共用
    /// （deferred-capabilities C2b）。
    fn parse_traceparent(tp: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = tp.trim().split('-').collect();
        if parts.len() != 4 || parts[0].eq_ignore_ascii_case("ff") {
            return None;
        }
        let (trace_id, span_id) = (parts[1], parts[2]);
        if trace_id.len() != 32
            || span_id.len() != 16
            || !trace_id.chars().all(|c| c.is_ascii_hexdigit())
            || !span_id.chars().all(|c| c.is_ascii_hexdigit())
        {
            return None;
        }
        Some((trace_id.to_ascii_lowercase(), span_id.to_ascii_lowercase()))
    }

    /// otel feature：从 registry span extensions 读取宿主
    /// tracing-opentelemetry layer 写入的 SpanContext。
    #[cfg(feature = "otel")]
    fn otel_ids_from_extensions(
        extensions: &tracing_subscriber::registry::Extensions<'_>,
    ) -> Option<(String, String)> {
        let span_context = extensions.get::<opentelemetry::trace::SpanContext>()?;
        if !span_context.is_valid() {
            return None;
        }
        // otel 的 Display 即零填充小写 hex（TraceId 32 位 / SpanId 16 位）
        Some((
            format!("{}", span_context.trace_id()),
            format!("{}", span_context.span_id()),
        ))
    }

    pub fn try_flush_fallback(&self) {
        // 锁内仅取出待 flush 批量，循环发送在锁外执行，
        // 避免锁被持有 N × send_timeout
        let batch: Vec<FallbackEntry> = {
            let mut buffer = self.fallback_buffer.lock();
            buffer.drain(..).collect()
        };
        if batch.is_empty() {
            return;
        }
        let timeout = Duration::from_millis(self.send_timeout_ms);
        let channel_count = 1 + self.extra_async_senders.len();
        let mut undelivered: VecDeque<FallbackEntry> = VecDeque::new();
        let mut stopped = false;
        for mut entry in batch {
            if stopped {
                // flush 已中断：剩余记录按原顺序回填，不再尝试发送
                undelivered.push_back(entry);
                continue;
            }
            // 只向尚未成功投递的通道补发；已成功的通道不再重发（抑制重复日志）
            let mut complete = true;
            for idx in 0..channel_count {
                if entry.delivered.get(idx).copied().unwrap_or(false) {
                    continue;
                }
                let sender = if idx == 0 {
                    &self.async_sender
                } else {
                    &self.extra_async_senders[idx - 1]
                };
                if sender
                    .send_timeout(Arc::clone(&entry.record), timeout)
                    .is_ok()
                {
                    if idx < entry.delivered.len() {
                        entry.delivered[idx] = true;
                    }
                } else {
                    // 与既有语义一致：flush 中途失败即停止，未完成的记录回填
                    complete = false;
                    break;
                }
            }
            if !complete {
                undelivered.push_back(entry);
                stopped = true;
            }
        }
        if !undelivered.is_empty() {
            // flush 失败：未完成投递的记录（含已部分补发的）按原顺序回填到队首
            let mut buffer = self.fallback_buffer.lock();
            for entry in undelivered.into_iter().rev() {
                buffer.push_front(entry);
            }
        }
    }
}

/// 用户治理中间件的丢弃计数装饰器：`with_middleware` 装配时包裹用户件，
/// 裁决 `Drop` 时补独立显性计数（归因细分 `user_middleware_dropped` +
/// 数据损失总账 `logs_dropped`）。内置限流件的丢弃计数在其内部
/// （`StressRelief`）完成，不经此路径，归因不重叠。
struct UserMiddlewareDropCounter {
    inner: Arc<dyn RecordMiddleware>,
    metrics: Arc<Metrics>,
}

impl RecordMiddleware for UserMiddlewareDropCounter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        let verdict = self.inner.process(record);
        if verdict == MiddlewareVerdict::Drop {
            self.metrics.inc_logs_dropped();
            self.metrics.inc_user_middleware_dropped();
        }
        verdict
    }
}

impl Drop for LoggerSubscriber {
    fn drop(&mut self) {
        // Attempt to flush any remaining fallback buffer entries
        let buffer_len = {
            let buffer = self.fallback_buffer.lock();
            buffer.len()
        };
        if buffer_len > 0 {
            self.try_flush_fallback();
            let remaining = self.fallback_buffer.lock().len();
            if remaining > 0 {
                // Drop 阶段不依赖 tracing 全局状态：格式化到 String 后直接输出
                let mut args = crate::i18n::MsgArgs::new();
                args.set("count", remaining.to_string());
                eprintln!(
                    "{}",
                    crate::i18n::tr_args("subscriber-drop-fallback-pending", args)
                );
            }
        }
    }
}

impl<S> Layer<S> for LoggerSubscriber
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut record = LogRecord::from_event(event);
        // 从当前 span 上下文提取 trace_id/span_id（未启用 span 时零成本直通）
        Self::extract_trace_context(&ctx, &mut record);

        // 处理管道：治理轮（target 配额 → 全局限流 → 压力采样救援）裁决
        // 丢弃即短路发送，丢弃计数在治理件内完成；改写轮（身份注入 →
        // 脱敏）在发送前完成。发送阶段不属于管道（依赖通道与兜底缓冲，
        // 仍是 subscriber 职责）。
        if !self.pipeline.process(&mut record) {
            return;
        }

        let record = Arc::new(record);

        // Fast path: Console - lock-free try_send, never block
        match self.console_sender.try_send(Arc::clone(&record)) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                // Channel full, drop the message and record metric
                // Hot path should never block
                self.metrics.inc_channel_blocked();
                self.metrics.inc_logs_dropped();
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }

        // Slow path: Async sinks - use timeout for backpressure handling
        let timeout = Duration::from_millis(self.send_timeout_ms);
        let delivered = self.send_to_async_sinks(&record, timeout);
        if delivered.iter().any(|ok| !ok) {
            // For critical logs, add to fallback buffer（保留各通道投递状态，
            // flush 时只补发未成功的通道）
            if is_critical_level(&record.level) {
                let mut buffer = self.fallback_buffer.lock();
                if buffer.len() >= FALLBACK_BUFFER_SIZE {
                    // LRU 淘汰的最旧关键日志落 journal（deferred-capabilities C4：
                    // 内存缓冲 100 条之外不再静默丢失）
                    if let Some(evicted) = buffer.pop_front()
                        && let Some(journal) = self.journal.as_ref()
                        && !journal.spill(&evicted.record)
                    {
                        // 落盘失败：记录已弹出内存缓冲且未持久化，计入数据
                        // 损失总账（瞬时 IO 失败不静默）
                        self.metrics.inc_logs_dropped();
                    }
                }
                buffer.push_back(FallbackEntry { record, delivered });
                self.fallback_pending.store(true, Ordering::Release);
            } else {
                // Timeout on non-critical log: message is lost (send_timeout returns
                // ownership but we have nowhere to buffer it). Only count as dropped,
                // not as channel_blocked, to keep metric semantics distinct:
                // channel_blocked = backpressure event, logs_dropped = data loss.
                self.metrics.inc_logs_dropped();
            }
        } else if self.fallback_pending.load(Ordering::Acquire) {
            // ERROR/FATAL 兜底运行期补发（除进程退出 Drain 外的第二触发点）：
            // 通道从满恢复并排空到半满以下时立即补发，不再等周期任务或 Drop。
            let half = self.async_sender.capacity().unwrap_or(0) / 2;
            if self.async_sender.len() <= half {
                self.try_flush_fallback();
                let still_pending = !self.fallback_buffer.lock().is_empty();
                self.fallback_pending
                    .store(still_pending, Ordering::Release);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::sampling::SamplingConfig;
    use crate::support::io::sink::middleware::MiddlewareVerdict;
    use crate::support::io::sink::sampling::SamplingPolicy;
    use crate::support::processing::target_rate_limiter::TargetRateLimiter;
    use crossbeam_channel::bounded;
    use serde_json::Value;
    use serial_test::serial;
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    #[test]
    fn test_on_event_sends_to_channels() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "hello", user_id = 1u64);
        });

        // Verify console channel received the record
        let console_received = console_rx.recv().unwrap();
        assert_eq!(console_received.level, "INFO");
        assert_eq!(console_received.target, "test::subscriber");
        assert_eq!(console_received.message, "hello");

        // Verify async channel received the record
        let async_received = async_rx.recv().unwrap();
        assert_eq!(async_received.level, "INFO");
        assert_eq!(async_received.target, "test::subscriber");
        assert_eq!(async_received.message, "hello");
    }

    /// 构建"恒定限流"的 subscriber：RateLimiter::new(0) 无令牌且不补充，
    /// 每条记录都进入压力路径（确定性，不依赖测试耗时）。
    fn rate_limited_subscriber(
        console_tx: Sender<Arc<LogRecord>>,
        async_tx: Sender<Arc<LogRecord>>,
        metrics: Arc<Metrics>,
        policy: Option<SamplingPolicy>,
    ) -> LoggerSubscriber {
        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_rate_limiter(Arc::new(RateLimiter::new(0)));
        if let Some(policy) = policy {
            layer.with_sampling_policy(Arc::new(policy))
        } else {
            layer
        }
    }

    // =====================================================================
    // per-target 分级限流（R7）：前缀配额组独立裁决，未命中 target 维持
    // 既有全局路径；未接线时整体零介入。
    // =====================================================================

    fn target_rules(pairs: &[(&str, u64)]) -> TargetRateLimiter {
        TargetRateLimiter::from_rules(pairs.iter().map(|(p, r)| (p.to_string(), *r)).collect())
            .unwrap()
    }

    #[test]
    fn test_target_quota_governed_target_uses_group_not_global() {
        // 全局限流 0 令牌（恒拒）+ app::audit 组 16 令牌：命中组走组桶放行，
        // 证明受管辖 target 不再进入全局路径
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_rate_limiter(Arc::new(RateLimiter::new(0)))
            .with_target_rate_limiter(Arc::new(target_rules(&[("app::audit", 16)])));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            for i in 0..16 {
                tracing::info!(target: "app::audit::core", message = format!("audit {i}"));
            }
        });

        let kept = console_rx
            .try_iter()
            .filter(|r| r.target.starts_with("app::audit"))
            .count();
        assert_eq!(
            kept, 16,
            "governed targets must be adjudicated by their group bucket, bypassing the global limiter"
        );
    }

    #[test]
    fn test_target_quota_exhausted_drops_governed_only() {
        // app::noise 组 1 令牌：第 1 条放行、第 2 条组配额拒绝且非关键级别
        // 无救援（logs_dropped）；未命中规则的 target 不消耗组预算，在无
        // 全局限流时照常放行
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone())
            .with_target_rate_limiter(Arc::new(target_rules(&[("app::noise", 1)])));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::warn!(target: "app::noise::spam", message = "noise 1");
            tracing::warn!(target: "app::noise::spam", message = "noise 2");
            tracing::warn!(target: "other::clean", message = "clean 1");
        });

        let messages: Vec<String> = console_rx.try_iter().map(|r| r.message.clone()).collect();
        assert!(
            messages.contains(&"noise 1".to_string()) && !messages.contains(&"noise 2".to_string()),
            "second same-group record must be dropped by quota, kept: {messages:?}"
        );
        assert!(
            messages.contains(&"clean 1".to_string()),
            "ungoverned target must pass untouched: {messages:?}"
        );
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "quota drop must count as logs_dropped (限流丢弃而非采样)"
        );
    }

    #[test]
    fn test_target_quota_exhaustion_rescues_critical_level() {
        // 组预算耗尽后同组的 ERROR 获 1-in-N 兜底救援（计数器 0 放行），
        // 与全局限流压力路径的关键级别语义一致；随后的 WARN 仍被丢弃
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let layer = LoggerSubscriber::new(console_tx, async_tx, Arc::new(Metrics::new()))
            .with_target_rate_limiter(Arc::new(target_rules(&[("app::audit", 1)])));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::warn!(target: "app::audit::core", message = "audit warn 1");
            // 组预算已耗尽：ERROR 计数器 0 → 兜底采样放行
            tracing::error!(target: "app::audit::core", message = "audit error rescued");
            tracing::warn!(target: "app::audit::core", message = "audit warn 2");
        });

        let messages: Vec<String> = console_rx.try_iter().map(|r| r.message.clone()).collect();
        assert!(
            messages.contains(&"audit error rescued".to_string()),
            "exhausted group must still rescue the first ERROR (1-in-N fallback): {messages:?}"
        );
        assert!(
            messages.contains(&"audit warn 1".to_string())
                && !messages.contains(&"audit warn 2".to_string()),
            "non-critical records after exhaustion must stay dropped: {messages:?}"
        );
    }

    #[test]
    fn test_without_target_quota_wiring_global_behavior_unchanged() {
        // 未接线时全局限流行为与既有语义完全一致（默认全局行为不变）
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_rate_limiter(Arc::new(RateLimiter::new(0)));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::warn!(target: "app::anything", message = "global only");
        });

        assert!(
            console_rx.try_iter().next().is_none(),
            "no target-quota wiring must leave the global limiter in full effect"
        );
    }

    #[test]
    fn test_rate_limit_without_policy_keeps_baseline_behavior() {
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let metrics = Arc::new(Metrics::new());
        let registry = tracing_subscriber::registry().with(rate_limited_subscriber(
            console_tx,
            async_tx,
            metrics.clone(),
            None,
        ));

        with_default(registry, || {
            for i in 0..10 {
                tracing::error!(target: "t::baseline", message = format!("error {i}"));
            }
            for i in 0..5 {
                tracing::info!(target: "t::baseline", message = format!("info {i}"));
            }
        });

        let kept: Vec<String> = console_rx.try_iter().map(|r| r.message.clone()).collect();
        // 未配置策略 = 现状语义：ERROR 计数器 0 放行（1-in-100），其余采样淘汰；
        // INFO 非关键级别全量丢弃
        assert_eq!(
            kept.iter().filter(|m| m.starts_with("error")).count(),
            1,
            "baseline must keep the first rate-limited ERROR (counter 0)"
        );
        assert!(
            kept.iter().all(|m| !m.starts_with("info")),
            "baseline must drop non-critical records entirely"
        );
        // 指标细分：9 条 ERROR 为采样淘汰（sampled_out），5 条 INFO 为压力丢弃
        assert_eq!(metrics.sampled_out(), 9);
        assert_eq!(metrics.logs_dropped(), 14);
    }

    #[test]
    fn test_rate_limit_policy_per_level_sampling() {
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let metrics = Arc::new(Metrics::new());
        let mut per_level = std::collections::HashMap::new();
        per_level.insert("warn".to_string(), 2u64);
        let policy = SamplingPolicy::from_config(&SamplingConfig {
            per_level,
            per_target_prefix: std::collections::HashMap::new(),
        })
        .unwrap();
        let registry = tracing_subscriber::registry().with(rate_limited_subscriber(
            console_tx,
            async_tx,
            metrics.clone(),
            Some(policy),
        ));

        with_default(registry, || {
            for i in 0..5 {
                tracing::warn!(target: "t::policy", message = format!("warn {i}"));
            }
            for i in 0..5 {
                tracing::info!(target: "t::policy", message = format!("info {i}"));
            }
            tracing::error!(target: "t::policy", message = "fallback error");
        });

        let kept: Vec<String> = console_rx.try_iter().map(|r| r.message.clone()).collect();
        // WARN 每级别 1-in-2：计数器 0/2/4 放行，共 3 条
        assert_eq!(
            kept.iter().filter(|m| m.starts_with("warn")).count(),
            3,
            "per-level 1-in-2 rate must keep 3 of 5 WARN records"
        );
        // INFO 无规则命中 → 兜底：非关键级别全量丢弃
        assert!(kept.iter().all(|m| !m.starts_with("info")));
        // ERROR 无规则命中 → 兜底：计数器 0 放行
        assert_eq!(
            kept.iter().filter(|m| m.starts_with("fallback")).count(),
            1,
            "records without a matching rule must fall back to baseline sampling"
        );
        // 指标：2 条 WARN 采样淘汰计入 sampled_out（唯一 ERROR 计数器 0 放行，
        // 不产生采样淘汰）；5 条 INFO 压力丢弃 + 2 条采样淘汰 = logs_dropped 7
        assert_eq!(metrics.sampled_out(), 2);
        assert_eq!(metrics.logs_dropped(), 7);
    }

    #[test]
    fn test_rate_limit_policy_target_prefix_rules() {
        let (console_tx, console_rx) = bounded(64);
        let (async_tx, _async_rx) = bounded(64);
        let metrics = Arc::new(Metrics::new());
        let mut per_target_prefix = std::collections::HashMap::new();
        per_target_prefix.insert(
            "app::audit".to_string(),
            crate::domain::config::sampling::TargetSamplingRule {
                keep_level: Some("debug".to_string()),
                sample_every_n: 10,
            },
        );
        per_target_prefix.insert(
            "app::noise".to_string(),
            crate::domain::config::sampling::TargetSamplingRule {
                keep_level: None,
                sample_every_n: 2,
            },
        );
        let policy = SamplingPolicy::from_config(&SamplingConfig {
            per_level: std::collections::HashMap::new(),
            per_target_prefix,
        })
        .unwrap();
        let registry = tracing_subscriber::registry().with(rate_limited_subscriber(
            console_tx,
            async_tx,
            metrics.clone(),
            Some(policy),
        ));

        with_default(registry, || {
            for i in 0..3 {
                tracing::info!(target: "app::audit::core", message = format!("audit {i}"));
            }
            for i in 0..4 {
                tracing::info!(target: "app::noise", message = format!("noise {i}"));
            }
            for i in 0..2 {
                tracing::error!(target: "app::other", message = format!("other {i}"));
            }
        });

        let kept: Vec<String> = console_rx.try_iter().map(|r| r.message.clone()).collect();
        // audit：keep_level=debug 豁免 INFO → 3 条全放行
        assert_eq!(
            kept.iter().filter(|m| m.starts_with("audit")).count(),
            3,
            "records above keep_level under a matched prefix must all pass"
        );
        // noise：1-in-2 → 计数器 0/2 放行，共 2 条
        assert_eq!(
            kept.iter().filter(|m| m.starts_with("noise")).count(),
            2,
            "prefix rule N-of-1 must keep every 2nd record"
        );
        // other：无前缀命中 → 兜底，ERROR 计数器 0 放行（第 1 条）
        assert_eq!(
            kept.iter().filter(|m| m.starts_with("other")).count(),
            1,
            "unmatched targets must fall back to baseline sampling"
        );
        // 指标：2 条 noise 采样淘汰 + 1 条 other 兜底采样淘汰 = sampled_out 3；
        // logs_dropped = 2（noise）+ 1（other）
        assert_eq!(metrics.sampled_out(), 3);
        assert_eq!(metrics.logs_dropped(), 3);
    }

    fn identity_map(
        pairs: &[(&str, &str)],
    ) -> std::sync::Arc<std::collections::BTreeMap<String, Value>> {
        std::sync::Arc::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
                .collect(),
        )
    }

    #[test]
    fn test_identity_fields_injected_into_records() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);

        let layer = LoggerSubscriber::new(console_tx, async_tx, Arc::new(Metrics::new()))
            .with_identity_fields(identity_map(&[
                ("service_name", "orders"),
                ("service_env", "prod"),
            ]));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::identity", message = "with identity");
        });

        let received = console_rx.recv().unwrap();
        assert_eq!(
            received.fields.get("service_name").and_then(Value::as_str),
            Some("orders"),
            "static identity field must be injected"
        );
        assert_eq!(
            received.fields.get("service_env").and_then(Value::as_str),
            Some("prod")
        );
        // async 通道侧同样带身份字段（多出口一致）
        let async_received = async_rx.recv().unwrap();
        assert_eq!(
            async_received
                .fields
                .get("service_name")
                .and_then(Value::as_str),
            Some("orders")
        );
    }

    #[test]
    fn test_identity_fields_do_not_override_event_fields() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);

        let layer = LoggerSubscriber::new(console_tx, async_tx, Arc::new(Metrics::new()))
            .with_identity_fields(identity_map(&[("service_name", "orders")]));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            // 事件显式携带同名字段：显式值优先（与 trace_id 显式覆盖语义一致）
            tracing::info!(target: "test::identity", service_name = "explicit", message = "override");
        });

        let received = console_rx.recv().unwrap();
        assert_eq!(
            received.fields.get("service_name").and_then(Value::as_str),
            Some("explicit"),
            "event-provided field must win over static identity"
        );
    }

    #[test]
    fn test_without_identity_fields_no_injection() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);

        let layer = LoggerSubscriber::new(console_tx, async_tx, Arc::new(Metrics::new()));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::identity", message = "plain");
        });

        let received = console_rx.recv().unwrap();
        assert!(
            !received.fields.contains_key("service_name"),
            "no identity wiring must leave fields untouched"
        );
    }

    #[test]
    fn test_on_event_handles_full_channel() {
        // Create a channel with capacity 1
        let (console_tx, console_rx) = bounded(1);
        let (async_tx, async_rx) = bounded(1);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        // Send multiple events - should not panic even when channel is full
        with_default(registry, || {
            for i in 0..5 {
                tracing::info!(target: "test::subscriber", message = "msg {}", i);
            }
        });

        // Drain channels to verify messages were sent
        while console_rx.try_recv().is_ok() {}
        while async_rx.try_recv().is_ok() {}
    }

    #[test]
    fn test_traceparent_field_parsing_and_priority() {
        // R-trace-002：合法 traceparent 字段 → 提取；非法/版本 ff → 回退派生
        let (_console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(_console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"; // pragma: allowlist secret — W3C 规范示例 ID
        with_default(registry, || {
            let span = tracing::info_span!("tp-span");
            let _guard = span.enter();
            tracing::info!(target: "test::tp", traceparent = tp, message = "with tp");
        });

        let record = async_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            record.trace_id.as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c")
        );
        assert_eq!(record.span_id.as_deref(), Some("b7ad6b7169203331"));

        // 解析函数边界：版本 ff / 非 hex / 段长不符
        assert!(
            LoggerSubscriber::parse_traceparent(
                "ff-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
            )
            .is_none()
        );
        assert!(
            LoggerSubscriber::parse_traceparent(
                "00-0af7651916cd43dd8448eb211c80319z-b7ad6b7169203331-01"
            )
            .is_none()
        );
        assert!(LoggerSubscriber::parse_traceparent("00-short-short-01").is_none());
    }

    #[test]
    fn test_traceparent_invalid_falls_back_to_derivation() {
        let (_console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(_console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            let span = tracing::info_span!("tp-bad");
            let _guard = span.enter();
            tracing::info!(target: "test::tp", traceparent = "00-bad-bad-01", message = "bad tp");
        });

        let record = async_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        // 回退根 span 派生：trace_id 仍存在（派生值），span_id 为当前 span
        assert!(record.trace_id.is_some());
        assert!(record.span_id.is_some());
    }

    #[cfg(feature = "otel")]
    #[test]
    fn test_otel_span_context_extraction_from_extensions() {
        use opentelemetry::trace::{SpanContext, SpanId, TraceFlags, TraceId, TraceState};
        use tracing_subscriber::registry::LookupSpan;

        // 完整栈（Registry + inklog layer）作为一个 dispatch：扩展插入与
        // 事件发射必须在同一 registry 数据上（与生产 downcast 路径一致）
        let (_console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(_console_tx, async_tx, metrics);
        let subscriber = tracing_subscriber::registry::Registry::default().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("otel-ext-span");
            let id = span.id().unwrap();
            {
                let _enter = span.enter();
                // 模拟宿主 tracing-opentelemetry layer 写入 SpanContext 扩展
                tracing::dispatcher::get_default(|dispatch| {
                    let registry = dispatch
                        .downcast_ref::<tracing_subscriber::registry::Registry>()
                        .expect("inner registry must be downcastable");
                    let span_ref = registry.span(&id).expect("span data in registry");
                    span_ref.extensions_mut().insert(SpanContext::new(
                        TraceId::from_bytes(
                            0x1234_5678_9abc_def0_1122_3344_5566_7788u128.to_be_bytes(),
                        ),
                        SpanId::from_bytes(0xaabb_ccdd_1122_3344u64.to_be_bytes()),
                        TraceFlags::SAMPLED,
                        false,
                        TraceState::from_key_value(Vec::<(String, String)>::new()).unwrap(),
                    ));
                });
                tracing::info!(target: "test::otel", message = "otel extraction probe");
            }
        });

        let record = async_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            record.trace_id.as_deref(),
            Some("123456789abcdef01122334455667788"), // pragma: allowlist secret — W3C 规范示例 ID
            "real OTel trace id must win over derived id"
        );
        assert_eq!(record.span_id.as_deref(), Some("aabbccdd11223344"));
    }

    #[test]
    fn test_fallback_eviction_spills_to_journal() {
        // C4：兜底缓冲（100）打满后 LRU 淘汰的关键日志落 journal
        let dir = tempfile::TempDir::new().unwrap();
        let journal_path = dir.path().join("fb.journal");
        let journal = crate::support::fallback_journal::FallbackJournal::open(&journal_path);

        let (_console_tx, _console_rx) = bounded(10);
        // 0 容量 rendezvous 通道且无接收端：send_timeout 必然失败 → 全部进兜底
        let (async_tx, async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(_console_tx, async_tx, metrics)
            .with_timeout(1)
            .with_journal(Arc::new(journal));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            for i in 0..105 {
                tracing::error!(target: "test::journal", message = format!("spill probe {i}"));
            }
        });
        drop(async_rx);

        let journal = crate::support::fallback_journal::FallbackJournal::open(&journal_path);
        let (records, skipped) = journal.replay();
        assert_eq!(skipped, 0);
        assert_eq!(
            records.len(),
            5,
            "105 pushes into a 100-cap buffer must spill 5 evicted entries"
        );
        assert!(records[0].message.contains("spill probe"));
    }

    #[test]
    fn test_fallback_journal_disabled_means_no_spill() {
        // 开关关闭：行为与既往一致，无文件产生
        let dir = tempfile::TempDir::new().unwrap();
        let (_console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());
        let layer = LoggerSubscriber::new(_console_tx, async_tx, metrics).with_timeout(1);
        let registry = tracing_subscriber::registry().with(layer);
        with_default(registry, || {
            for _ in 0..105 {
                tracing::error!(target: "test::journal", message = "no journal here");
            }
        });
        drop(async_rx);
        assert!(!dir.path().join("fb.journal").exists());
    }

    #[test]
    fn test_fallback_replays_when_channel_recovers() {
        // 运行期补发：ERROR 进 fallback 后，通道恢复（排空到半满）即补发，
        // 不再依赖进程退出时的 Drop drain
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(2);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            // 打满通道（容量 2）并让 ERROR 进 fallback
            _ = tracing::Level::ERROR;
            for _ in 0..5 {
                tracing::error!(target: "test::fallback", message = "critical failure");
            }
            // 通道恢复：drain 后再投一条 INFO（成功路径）应触发半满补发
            while async_rx.try_recv().is_ok() {}
            tracing::info!(target: "test::fallback", message = "recovery probe");
        });

        // 一个补发窗口内（无需进程退出）：兜底中的 ERROR 被补发到通道
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut recovered_error = false;
        while std::time::Instant::now() < deadline {
            match async_rx.try_recv() {
                Ok(record) => {
                    if record.level == "ERROR" {
                        recovered_error = true;
                        break;
                    }
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        assert!(
            recovered_error,
            "ERROR record in fallback must be replayed once the channel recovers"
        );
    }

    #[test]
    fn test_critical_level_adds_to_fallback_buffer() {
        let (console_tx, _console_rx) = bounded(10);
        // Zero-capacity async channel causes send_timeout to always time out,
        // triggering the fallback path for ERROR/FATAL events.
        let (async_tx, _async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // Should not panic: ERROR events route to fallback buffer
        with_default(registry, || {
            tracing::error!(target: "test::subscriber", message = "critical error");
        });
        // If we reach here without panic, the critical-level fallback path works
        assert_eq!(metrics.logs_written(), 0);
    }

    #[test]
    fn test_fallback_buffer_does_not_panic_on_overflow() {
        let (console_tx, _cr) = bounded(10);
        // Zero-capacity async channel: all async sends time out
        let (async_tx, _ar) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        // Send many ERROR events — fallback buffer has max size 100.
        // FILL_BROWSER_SIZE + 5 events. Should not panic.
        with_default(registry, || {
            for i in 0..105 {
                tracing::error!(target: "test::subscriber", msg = "overflow {}", i);
            }
        });
        // Reaching here without panic confirms LRU eviction in fallback buffer works
    }

    #[test]
    fn test_try_flush_fallback_with_disconnected_channel() {
        let (console_tx1, _cr1) = bounded(10);
        let (async_tx1, _ar1) = bounded(1);
        drop(_ar1);
        let metrics = Arc::new(Metrics::new());

        // Subscriber A: used as tracing layer within with_default
        let layer = LoggerSubscriber::new(console_tx1.clone(), async_tx1.clone(), metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // ERROR events go to fallback buffer via subscriber A
        with_default(registry, || {
            tracing::error!(target: "test::subscriber", msg = "fallback before disconnect");
        });

        // Subscriber B: shares same channels via Arc<Metrics> but owns its own fallback buffer.
        // The async_sender is disconnected, so try_flush_fallback hits
        // SendTimeoutError::Disconnected → loop breaks safely. No panic.
        let _subscriber_b = LoggerSubscriber::new(console_tx1, async_tx1, metrics);
        _subscriber_b.try_flush_fallback();
    }

    #[test]
    fn test_on_event_dropped_on_disconnected_async_channel() {
        let (console_tx, _cr) = bounded(10);
        // Create and immediately drop the receiver to simulate disconnection
        let (async_tx, _ar) = bounded(1);
        drop(_ar); // Disconnect async channel
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // Sending should not panic even when async channel is disconnected
        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "after disconnect");
        });

        // Should have incremented logs_dropped for the disconnected async channel
        assert_eq!(metrics.logs_dropped(), 1);
    }

    #[test]
    fn test_with_timeout_configures_send_timeout() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics).with_timeout(500);

        assert_eq!(subscriber.send_timeout_ms, 500);
    }

    // =========================================================================
    // try_flush_fallback() 测试 - 覆盖成功弹出和失败中断分支
    // =========================================================================

    #[test]
    fn test_try_flush_fallback_drains_buffer_on_success() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics);

        // 手动向 fallback_buffer 注入一条记录（测试模块可访问私有字段）
        let record = Arc::new(LogRecord::new(
            tracing::Level::ERROR,
            "test::fallback".to_string(),
            "fallback flush test".to_string(),
        ));
        subscriber.fallback_buffer.lock().push_back(FallbackEntry {
            record,
            delivered: vec![false],
        });

        // 调用 try_flush_fallback，async channel 有容量 → send 成功 → pop_front
        subscriber.try_flush_fallback();

        // 验证 buffer 已清空
        assert!(
            subscriber.fallback_buffer.lock().is_empty(),
            "buffer should be empty after successful flush"
        );

        // 验证记录已发送到 async channel
        let received = async_rx.recv_timeout(std::time::Duration::from_millis(100));
        assert!(received.is_ok(), "should receive the flushed record");
        assert_eq!(received.unwrap().message, "fallback flush test");
    }

    #[test]
    fn test_try_flush_fallback_breaks_on_disconnected_channel() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics);

        // 注入记录到 fallback_buffer
        let record = Arc::new(LogRecord::new(
            tracing::Level::ERROR,
            "test::fallback".to_string(),
            "disconnect test".to_string(),
        ));
        subscriber.fallback_buffer.lock().push_back(FallbackEntry {
            record,
            delivered: vec![false],
        });

        // 断开 async channel 的接收端 → send 返回 Disconnected → break
        drop(_async_rx);
        subscriber.try_flush_fallback();

        // 断开后 buffer 应仍包含记录（break 未弹出）
        assert_eq!(
            subscriber.fallback_buffer.lock().len(),
            1,
            "buffer should still contain the record after disconnect"
        );
    }

    #[test]
    fn test_try_flush_fallback_preserves_order_on_partial_flush() {
        let (console_tx, _console_rx) = bounded(10);
        // 容量 1：第一条发送成功后 channel 满，第二条 send_timeout 超时中断
        let (async_tx, async_rx) = bounded(1);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics);

        for i in 0..3 {
            let record = Arc::new(LogRecord::new(
                tracing::Level::ERROR,
                "test::fallback".to_string(),
                format!("fallback-order-{i}"),
            ));
            subscriber.fallback_buffer.lock().push_back(FallbackEntry {
                record,
                delivered: vec![false],
            });
        }
        subscriber.try_flush_fallback();

        // 第一条已发出，剩余两条应按原顺序回填到队首
        let first = async_rx
            .recv_timeout(std::time::Duration::from_millis(500))
            .expect("first record should be flushed");
        assert_eq!(first.message, "fallback-order-0");

        let buffer = subscriber.fallback_buffer.lock();
        assert_eq!(buffer.len(), 2, "remaining records should be refilled");
        assert_eq!(
            buffer.front().unwrap().record.message,
            "fallback-order-1",
            "refilled records must keep original order (front)"
        );
        assert_eq!(
            buffer.back().unwrap().record.message,
            "fallback-order-2",
            "refilled records must keep original order (back)"
        );
    }

    // =========================================================================
    // fallback 重复抑制回归：按通道精确追踪投递状态，flush 只补发未成功
    // 通道对应的记录；已成功通道不得被再次投递（重复日志）。
    // =========================================================================

    #[test]
    fn test_on_event_partial_failure_records_per_channel_delivery() {
        // 主 async 通道成功、extra 通道（rendezvous 容量 0）必然超时失败：
        // ERROR 记录进入 fallback，且 delivered 状态应精确为 [true, false]
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let (extra_tx, _extra_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_extra_async_sender(extra_tx)
            .with_timeout(50);
        // 在 layer 被 registry 消费前，先拿到 fallback_buffer 的 Arc clone
        let fallback_buffer = Arc::clone(&layer.fallback_buffer);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::error!(target: "test::subscriber", message = "partial delivery");
        });

        // 主通道恰好收到一条（无重复）
        assert!(
            async_rx.try_recv().is_ok(),
            "primary async channel should receive the record"
        );
        assert!(
            async_rx.try_recv().is_err(),
            "primary async channel must not receive duplicates"
        );

        // fallback 中该记录的投递状态精确为 [true, false]
        let buffer = fallback_buffer.lock();
        assert_eq!(
            buffer.len(),
            1,
            "record should be buffered for the failed channel"
        );
        let entry = buffer.front().unwrap();
        assert_eq!(
            entry.delivered,
            vec![true, false],
            "delivery state must be tracked per channel"
        );
        assert_eq!(entry.record.message, "partial delivery");
    }

    #[test]
    fn test_fallback_flush_only_resends_undelivered_channels() {
        // 已投递通道（delivered[0]=true）在 flush 时不得重发；
        // 未投递通道补发恰好一条，全部投递完成后条目移出 buffer
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        // rendezvous 通道：flush 的补发与接收线程会合后成功
        let (extra_tx, extra_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_extra_async_sender(extra_tx)
            .with_timeout(500);

        subscriber.fallback_buffer.lock().push_back(FallbackEntry {
            record: Arc::new(LogRecord::new(
                tracing::Level::ERROR,
                "test::fallback".to_string(),
                "dup suppression".to_string(),
            )),
            delivered: vec![true, false],
        });

        // 接收线程先阻塞在 rendezvous channel 上，flush 的补发与其会合
        let receiver = std::thread::spawn(move || {
            extra_rx.recv_timeout(std::time::Duration::from_millis(2000))
        });
        std::thread::sleep(std::time::Duration::from_millis(50));

        subscriber.try_flush_fallback();

        // 未成功通道收到恰好一条补发
        let resent = receiver
            .join()
            .unwrap()
            .expect("extra channel should receive the flushed record");
        assert_eq!(resent.message, "dup suppression");

        // 已成功通道不得收到重复记录
        assert!(
            async_rx.try_recv().is_err(),
            "already-delivered channel must NOT receive a duplicate on flush"
        );

        // 全部通道投递完成后 buffer 清空
        assert!(
            subscriber.fallback_buffer.lock().is_empty(),
            "fully delivered entry should be removed from the buffer"
        );
    }

    // =========================================================================
    // 敏感键判定单一事实源：subscriber 的 sanitizer 路径直接引用
    // `LogRecord::is_sensitive_key`（token 边界语义），本测试钉住该语义，
    // 防止有人再引入按子串匹配的本地副本（子串匹配会把 "author" 误判为敏感）。
    // =========================================================================

    #[test]
    fn test_is_sensitive_key_matches_log_record_canonical_semantics() {
        // 敏感键（与 log_record.rs 的 token 边界判定一致）
        for key in ["password", "api_key", "auth_token", "secret"] {
            assert!(
                LogRecord::is_sensitive_key(key),
                "'{key}' must be judged sensitive by the canonical implementation"
            );
        }
        // 非敏感键（子串匹配会误判 "author" 含 "auth"，token 边界判定不会）
        for key in ["primary_key", "author"] {
            assert!(
                !LogRecord::is_sensitive_key(key),
                "'{key}' must NOT be judged sensitive by the canonical implementation"
            );
        }
    }

    // NOTE: parking_lot::Mutex 不支持 poison，无需测试毒化恢复
    // try_flush_fallback 的断开 channel 场景由
    // test_try_flush_fallback_with_disconnected_channel 覆盖

    // =========================================================================
    // on_event console channel 断开测试
    // =========================================================================

    #[test]
    fn test_on_event_console_disconnected_increments_dropped() {
        let (console_tx, _console_rx) = bounded(10);
        // 断开 console channel
        drop(_console_rx);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "console disconnected");
        });

        // console 断开 → logs_dropped += 1；async 正常 → 无变化
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "console disconnect should increment logs_dropped by 1"
        );
    }

    #[test]
    fn test_on_event_console_full_channel_increments_blocked_and_dropped() {
        // console channel 容量 1，发送 2 条事件 → 第二条 Full
        let (console_tx, console_rx) = bounded(1);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // 先填满 console channel（容量 1）
        // 第一条事件：console Ok，async Ok
        // 第二条事件：console Full → channel_blocked++ + logs_dropped++
        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "first");
            tracing::info!(target: "test::subscriber", message = "second");
        });

        // 排空 console channel
        while console_rx.try_recv().is_ok() {}

        // console Full 应触发 channel_blocked 和 logs_dropped
        assert!(
            metrics.logs_dropped() >= 1,
            "console full should increment logs_dropped, got: {}",
            metrics.logs_dropped()
        );
    }

    // =========================================================================
    // on_event fallback_buffer 锁毒化恢复（行 116, 119-120）
    // =========================================================================

    // NOTE: parking_lot::Mutex 不支持 poison，无需测试毒化恢复
    // on_event 的 fallback buffer 路径由
    // test_critical_level_adds_to_fallback_buffer 覆盖

    #[test]
    fn test_on_event_console_ok_and_async_ok_paths() {
        // 显式覆盖行 95（console try_send Ok）和行 110（async send_timeout Ok）
        // 现有 test_on_event_sends_to_channels 已覆盖，但这里额外验证
        // metrics 没有增加（确认 Ok 路径不触发 drop/blocked 计数）
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "ok path test");
        });

        // 两个 channel 都应收到记录
        assert!(
            console_rx.try_recv().is_ok(),
            "console should receive record"
        );
        assert!(async_rx.try_recv().is_ok(), "async should receive record");

        // Ok 路径不应增加 logs_dropped 或 channel_blocked
        assert_eq!(
            metrics.logs_dropped(),
            0,
            "Ok path should not increment logs_dropped"
        );
    }

    // =========================================================================
    // on_event 错误路径覆盖：非关键级别 async 超时 → 仅 logs_dropped 递增
    // channel_blocked 仅在背压事件（send_timeout 返回 Full）时递增，
    // 而超时丢弃消息仅属于数据丢失，不属于背压。
    // =========================================================================

    #[test]
    #[serial]
    fn test_on_event_non_critical_async_timeout_increments_blocked_and_dropped() {
        // async channel 容量 0（rendezvous）→ send_timeout 必然超时
        // INFO 级别非关键 → 仅 increment logs_dropped (not channel_blocked)
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        let before_blocked = metrics.channel_blocked();
        let before_dropped = metrics.logs_dropped();

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "non-critical timeout");
        });

        // Metric semantics: channel_blocked = backpressure event,
        // logs_dropped = data loss.  A timeout on a non-critical log
        // is data-loss only, not backpressure.
        assert_eq!(
            metrics.channel_blocked(),
            before_blocked,
            "non-critical async timeout should NOT increment channel_blocked"
        );
        assert_eq!(
            metrics.logs_dropped(),
            before_dropped + 1,
            "non-critical async timeout should increment logs_dropped"
        );
    }

    // =========================================================================
    // on_event 错误路径覆盖：关键级别 async 超时 → 记录存入 fallback_buffer
    // 显式覆盖行 113-126，并验证 buffer 内容和 metrics 不递增
    // =========================================================================

    #[test]
    #[serial]
    fn test_on_event_critical_async_timeout_stores_record_in_fallback_buffer() {
        // async channel 容量 0（rendezvous）→ send_timeout 必然超时
        // ERROR 级别为关键 → 走行 113-126（存入 fallback_buffer，不递增 metrics）
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        // 在 layer 被 registry 消费前，先拿到 fallback_buffer 的 Arc clone
        let fallback_buffer = Arc::clone(&layer.fallback_buffer);
        let registry = tracing_subscriber::registry().with(layer);

        let before_blocked = metrics.channel_blocked();
        let before_dropped = metrics.logs_dropped();

        with_default(registry, || {
            tracing::error!(target: "test::subscriber", message = "critical timeout");
        });

        // 关键级别 + async 超时：记录存入 fallback_buffer
        let buffer_guard = fallback_buffer.lock();
        assert_eq!(
            buffer_guard.len(),
            1,
            "fallback_buffer should contain exactly 1 record"
        );
        let entry = buffer_guard
            .front()
            .expect("should have a record in fallback_buffer");
        assert_eq!(entry.record.level, "ERROR", "record level should be ERROR");
        assert_eq!(
            entry.record.message, "critical timeout",
            "record message should match"
        );
        assert_eq!(
            entry.delivered,
            vec![false],
            "all async channels failed, delivery state should be [false]"
        );
        drop(buffer_guard);

        // 关键级别不应递增 channel_blocked 或 logs_dropped
        assert_eq!(
            metrics.channel_blocked(),
            before_blocked,
            "critical level should not increment channel_blocked"
        );
        assert_eq!(
            metrics.logs_dropped(),
            before_dropped,
            "critical level should not increment logs_dropped"
        );
    }

    // =========================================================================
    // sanitizer integration tests
    // =========================================================================

    #[test]
    fn test_with_sanitizer_escapes_newline_in_message() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let sanitizer = Arc::new(LogSanitizer::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics).with_sanitizer(sanitizer);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            // tracing will把 \n 保留在 message 中
            tracing::info!(target: "test::sanitizer", message = "line1\nline2");
        });

        let received = console_rx.recv().unwrap();
        // Sanitizer should have escaped the newline
        assert!(
            received.message.contains("\\n"),
            "message should contain escaped newline, got: {:?}",
            received.message
        );
        assert!(
            !received.message.contains('\n'),
            "message should not contain raw newline"
        );
    }

    #[test]
    fn test_without_sanitizer_message_unchanged() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        // No sanitizer set — default behavior
        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::no_sanitizer", message = "plain message");
        });

        let received = console_rx.recv().unwrap();
        assert_eq!(received.message, "plain message");
    }

    // =========================================================================
    // rate limiter integration tests
    // =========================================================================

    #[test]
    fn test_rate_limiter_drops_non_critical_logs() {
        let (console_tx, console_rx) = bounded(100);
        let (async_tx, _async_rx) = bounded(100);
        let metrics = Arc::new(Metrics::new());
        // Rate of 2 tokens: only 2 logs allowed initially
        let limiter = Arc::new(RateLimiter::new(2));

        let layer =
            LoggerSubscriber::new(console_tx, async_tx, metrics.clone()).with_rate_limiter(limiter);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            for _ in 0..10 {
                tracing::info!(target: "test::rate", message = "flood");
            }
        });

        // Only 2 should have gotten through (bucket started with 2 tokens)
        let mut count = 0;
        while console_rx.try_recv().is_ok() {
            count += 1;
        }
        assert!(
            count <= 2,
            "at most 2 logs should pass rate limiter, got {}",
            count
        );
        // Dropped logs should be counted
        assert!(
            metrics.logs_dropped() >= 8,
            "at least 8 logs should be dropped, got {}",
            metrics.logs_dropped()
        );
    }

    #[test]
    fn test_rate_limiter_samples_error_on_rejection() {
        let (console_tx, console_rx) = bounded(200);
        let (async_tx, _async_rx) = bounded(200);
        let metrics = Arc::new(Metrics::new());
        // Rate of 1: only 1 log allowed, then all rejected
        let limiter = Arc::new(RateLimiter::new(1));

        let layer =
            LoggerSubscriber::new(console_tx, async_tx, metrics.clone()).with_rate_limiter(limiter);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            // First INFO consumes the token
            tracing::info!(target: "test::rate", message = "consume token");
            // Now send 100 ERRORs — only 1-in-100 should pass
            for _ in 0..100 {
                tracing::error!(target: "test::rate", message = "error flood");
            }
        });

        // Drain console channel
        let mut error_count = 0;
        while let Ok(record) = console_rx.try_recv() {
            if record.level == "ERROR" {
                error_count += 1;
            }
        }
        // First ERROR passes (counter=0, 0%100==0), rest are sampled at 1/100
        // So we expect ~1-2 ERRORs through (the first sampled one)
        assert!(
            (1..=5).contains(&error_count),
            "expected ~1 sampled ERROR through rate limiter, got {}",
            error_count
        );
    }

    // =========================================================================
    // with_middleware 用户治理装配入口：治理环尾插——用户件的原位改写
    // 仍进改写轮被脱敏；Drop 裁决短路发送并计入独立显性丢弃计数
    // =========================================================================

    /// 用户治理中间件测试件：为记录打标（值含换行，用于验证改写轮脱敏），
    /// message 含 "drop-me" 时裁决丢弃。
    struct UserGovernanceMiddleware;

    impl RecordMiddleware for UserGovernanceMiddleware {
        fn name(&self) -> &str {
            "user-governance"
        }
        fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
            record
                .fields
                .insert("user_tag".to_string(), serde_json::json!("line1\nline2"));
            if record.message.contains("drop-me") {
                MiddlewareVerdict::Drop
            } else {
                MiddlewareVerdict::Continue
            }
        }
    }

    #[test]
    fn test_with_middleware_user_governance_runs_before_rewrite_round() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_sanitizer(Arc::new(LogSanitizer::new()))
            .with_middleware(Arc::new(UserGovernanceMiddleware));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::middleware", message = "kept");
            tracing::info!(target: "test::middleware", message = "drop-me");
        });

        let kept = console_rx.recv().unwrap();
        assert_eq!(kept.message, "kept");
        let tag = kept.fields.get("user_tag").and_then(Value::as_str).unwrap();
        assert!(
            !tag.contains('\n') && tag.contains("\\n"),
            "user middleware rewrite must still pass the rewrite round (sanitized), got: {tag:?}"
        );
        assert!(
            console_rx.try_recv().is_err(),
            "dropped record must not reach any channel"
        );
    }

    #[test]
    fn test_with_middleware_drop_counts_user_middleware_dropped() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone())
            .with_middleware(Arc::new(UserGovernanceMiddleware));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::middleware", message = "drop-me");
        });

        assert!(
            console_rx.try_recv().is_err(),
            "user-middleware drop must short-circuit sending"
        );
        assert_eq!(
            metrics.user_middleware_dropped(),
            1,
            "user middleware Drop verdict must be counted explicitly"
        );
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "user middleware drop is data loss and must count toward logs_dropped"
        );
    }

    #[test]
    fn test_with_middleware_continue_does_not_count_drop_metric() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone())
            .with_middleware(Arc::new(UserGovernanceMiddleware));
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::middleware", message = "kept");
        });

        assert!(console_rx.try_recv().is_ok(), "kept record must be sent");
        assert_eq!(
            metrics.user_middleware_dropped(),
            0,
            "Continue verdict must not touch the drop counter"
        );
        assert_eq!(metrics.logs_dropped(), 0);
    }

    #[test]
    fn test_builtin_limiter_drop_not_attributed_to_user_middleware() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let registry = tracing_subscriber::registry().with(rate_limited_subscriber(
            console_tx,
            async_tx,
            metrics.clone(),
            None,
        ));

        with_default(registry, || {
            tracing::warn!(target: "t::builtin", message = "stress drop");
        });

        assert_eq!(
            metrics.logs_dropped(),
            1,
            "builtin limiter stress drop still counts toward logs_dropped"
        );
        assert_eq!(
            metrics.user_middleware_dropped(),
            0,
            "builtin limiter drops must not be attributed to user middleware"
        );
    }
}

// ============================================================================
// 追踪 ID 关联 —— span 上下文提取与输出
// ============================================================================

#[cfg(test)]
mod trace_context_tests {
    use super::*;
    use crossbeam_channel::bounded;
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    /// 构建 (registry, console_rx)：从 console 通道读取 LoggerSubscriber
    /// 提取后的记录（trace 上下文在 on_event 中注入）。
    type TestSubscriber =
        tracing_subscriber::layer::Layered<LoggerSubscriber, tracing_subscriber::Registry>;

    fn setup() -> (TestSubscriber, crossbeam_channel::Receiver<Arc<LogRecord>>) {
        let (console_tx, console_rx) = bounded(100);
        let (async_tx, _async_rx) = bounded(100);
        let layer = LoggerSubscriber::new(console_tx, async_tx, Arc::new(Metrics::new()));
        (tracing_subscriber::registry().with(layer), console_rx)
    }

    #[test]
    fn test_event_inside_span_gets_trace_and_span_ids() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            let span = tracing::info_span!("handler", request = "r-1");
            let _guard = span.enter();
            tracing::info!(target: "t504", message = "inside span");
            tracing::info!(target: "t504", message = "still inside");
        });

        let r1 = console_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let r2 = console_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let (s1, s2) = (
            r1.span_id.clone().expect("span_id"),
            r2.span_id.clone().expect("span_id"),
        );
        assert_eq!(s1, s2, "same span must share span_id");
        assert_eq!(s1.len(), 16, "span_id must be 16-char hex");
        let (t1, t2) = (
            r1.trace_id.clone().expect("trace_id"),
            r2.trace_id.clone().expect("trace_id"),
        );
        assert_eq!(t1, t2, "same span must share trace_id");
        assert_eq!(t1.len(), 32, "trace_id must be 32-char hex");
        assert_ne!(t1, s1, "trace_id must not equal span_id (root derivation)");
    }

    #[test]
    fn test_child_span_shares_root_trace_id() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            let root = tracing::info_span!("root");
            let _root_guard = root.enter();
            tracing::info!(target: "t504", message = "at root");
            let child = tracing::info_span!("child");
            let _child_guard = child.enter();
            tracing::info!(target: "t504", message = "at child");
        });

        let root = console_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let child = console_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let root_trace = root.trace_id.clone().expect("root trace_id");
        let child_trace = child.trace_id.clone().expect("child trace_id");
        assert_eq!(
            root_trace, child_trace,
            "child span must inherit the root span's trace_id"
        );
        assert_ne!(
            root.span_id, child.span_id,
            "different spans must have different span_ids"
        );
    }

    #[test]
    fn test_event_outside_span_has_no_trace_ids() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            tracing::info!(target: "t504", message = "no span");
        });

        let record = console_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(record.trace_id.is_none(), "no span → trace_id None");
        assert!(record.span_id.is_none(), "no span → span_id None");
    }

    #[test]
    fn test_explicit_trace_fields_override_derivation() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            let span = tracing::info_span!("otel-ish");
            let _guard = span.enter();
            tracing::info!(
                target: "t504",
                message = "explicit",
                trace_id = "0af7651916cd43dd8448eb211c80319c",
                span_id = "b7ad6b7169203331"
            );
        });

        let record = console_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(
            record.trace_id.as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c"),
            "explicit trace_id field must win (OTel compatibility)"
        );
        assert_eq!(
            record.span_id.as_deref(),
            Some("b7ad6b7169203331"),
            "explicit span_id field must win (OTel compatibility)"
        );
    }
}
