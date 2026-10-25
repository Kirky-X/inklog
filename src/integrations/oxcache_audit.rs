// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! oxcache `AuditEventPublisher` 端口的 inklog 实现（feature `oxcache-audit`）。
//!
//! 实现下层 oxcache 定义的 [`AuditEventPublisher`] 端口（对象安全，
//! `Arc<dyn AuditEventPublisher>` 注入 cache builder）：把缓存审计事件转换为
//! 结构化 [`LogRecord`]（`target = "oxcache::audit"`），经有界 channel 交由
//! 单一 writer task 顺序写入注入的 inklog [`LogSink`]。
//!
//! 端口契约是**非阻塞** `fn publish(&self, event: AuditEvent)` 且发布失败不得
//! 影响缓存主操作语义，而 [`LogSink::write`] 是 async——桥接经 mpsc 通道 +
//! 惰性 writer task（首个事件进入 tokio runtime 上下文时启动一次，随其
//! runtime 关停而取消）：通道满、通道关闭或无 runtime 上下文时按丢弃计数，
//! 绝不回压缓存路径；sink 写失败单独计数，均为 fail-open 观测面。
//!
//! 级别映射为显式常量表：状态变更（Set/Delete/Clear）→ `INFO`，读探测与
//! 容量/过期清理（Hit/Miss/Evict/Expired）→ `DEBUG`。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once};

use oxcache::features::audit::{AuditAction, AuditEvent, AuditEventPublisher};
use tokio::sync::mpsc;

use crate::LogRecord;
use crate::support::io::sink::LogSink;

/// 发布器内部事件通道容量（事件数）：`publish` 非阻塞 `try_send`，通道写满
/// 即过载丢弃——过载以显性丢弃计数呈现，绝不反压缓存操作。
pub const BRIDGE_CHANNEL_CAPACITY: usize = 1024;

/// 审计事件的日志消息文案（本地静态字面量，不依赖 oxcache 的 i18n 资源）。
const AUDIT_EVENT_MESSAGE: &str = "cache audit event";

/// oxcache 审计事件 → inklog 结构化日志发布器。
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
/// use inklog::integrations::OxcacheAuditPublisher;
///
/// let publisher = Arc::new(OxcacheAuditPublisher::new(sink));
/// let cache = oxcache::Cache::<String, String>::builder()
///     .audit_publisher(publisher)
///     .build()
///     .await?;
/// ```
#[derive(Clone)]
pub struct OxcacheAuditPublisher {
    inner: Arc<BridgeInner>,
}

struct BridgeInner {
    sink: Arc<dyn LogSink>,
    tx: mpsc::Sender<LogRecord>,
    /// writer task 的接收端：首个 runtime 上下文 publish 时取出生成 writer。
    rx: Mutex<Option<mpsc::Receiver<LogRecord>>>,
    /// writer task 只启动一次（首个 runtime 上下文的 publish 触发）。
    writer_started: Once,
    /// writer task 是否已成功派生。未就绪时不得入队（否则记录滞留通道，
    /// writer 复活后被捞出，与已计丢弃数双计失真）。
    writer_active: AtomicBool,
    /// 显性丢弃计数：无 runtime 上下文 / channel 过载 / channel 已关闭 /
    /// runtime 关停时通道内滞留事件（接收端守卫清算）。
    dropped: Arc<AtomicU64>,
    /// sink 写入失败次数。
    write_failures: Arc<AtomicU64>,
}

/// writer task 接收端守卫：writer 所属 runtime 关停会取消 task（future
/// 连同接收端一并丢弃），Drop 中把通道内滞留事件逐条计入 dropped——
/// runtime 关停窗口的事件丢失显性化，不留静默黑洞。
struct WriterRxGuard {
    rx: mpsc::Receiver<LogRecord>,
    dropped: Arc<AtomicU64>,
}

impl Drop for WriterRxGuard {
    fn drop(&mut self) {
        while self.rx.try_recv().is_ok() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl OxcacheAuditPublisher {
    /// 以注入的 inklog `LogSink` 创建发布器（默认通道容量）。
    pub fn new(sink: Arc<dyn LogSink>) -> Self {
        Self::with_capacity(sink, BRIDGE_CHANNEL_CAPACITY)
    }

    /// 以指定通道容量创建发布器。
    pub fn with_capacity(sink: Arc<dyn LogSink>, capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity.max(1));
        Self {
            inner: Arc::new(BridgeInner {
                sink,
                tx,
                rx: Mutex::new(Some(rx)),
                writer_started: Once::new(),
                writer_active: AtomicBool::new(false),
                dropped: Arc::new(AtomicU64::new(0)),
                write_failures: Arc::new(AtomicU64::new(0)),
            }),
        }
    }

    /// 显性丢弃的事件数（无 runtime / 通道过载 / 通道关闭 / 关停滞留清算）。
    pub fn dropped_count(&self) -> u64 {
        self.inner.dropped.load(Ordering::Relaxed)
    }

    /// sink 写入失败次数。
    pub fn write_failure_count(&self) -> u64 {
        self.inner.write_failures.load(Ordering::Relaxed)
    }

    /// 审计事件 → inklog 日志记录的纯映射（级别/字段构造，无 IO）。
    pub fn to_log_record(event: &AuditEvent) -> LogRecord {
        let level = match event.action {
            // 状态变更是审计留痕主体
            AuditAction::Set | AuditAction::Delete | AuditAction::Clear => tracing::Level::INFO,
            // 读探测与容量/过期清理属诊断信息
            AuditAction::Hit | AuditAction::Miss | AuditAction::Evict | AuditAction::Expired => {
                tracing::Level::DEBUG
            }
        };
        let mut record = LogRecord::new(
            level,
            "oxcache::audit".to_string(),
            AUDIT_EVENT_MESSAGE.to_string(),
        );
        record.fields.insert(
            "action".to_string(),
            serde_json::Value::from(event.action.as_str()),
        );
        if let Some(key) = &event.key {
            record
                .fields
                .insert("key".to_string(), serde_json::Value::from(key.as_str()));
        }
        if let Some(namespace) = &event.namespace {
            record.fields.insert(
                "namespace".to_string(),
                serde_json::Value::from(namespace.as_str()),
            );
        }
        if let Some(operator) = &event.operator {
            record.fields.insert(
                "operator".to_string(),
                serde_json::Value::from(operator.as_str()),
            );
        }
        record.fields.insert(
            "timestamp_ms".to_string(),
            serde_json::Value::from(event.timestamp_ms),
        );
        for (k, v) in &event.metadata {
            record
                .fields
                .insert(format!("meta.{k}"), serde_json::Value::from(v.as_str()));
        }
        record
    }
}

impl AuditEventPublisher for OxcacheAuditPublisher {
    fn publish(&self, event: AuditEvent) {
        if !self.ensure_writer() {
            self.inner.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        match self.inner.tx.try_send(Self::to_log_record(&event)) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) | Err(mpsc::error::TrySendError::Closed(_)) => {
                self.inner.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

impl OxcacheAuditPublisher {
    /// 惰性启动 writer task；返回 writer 是否就绪（已成功派生）。
    ///
    /// 只有就绪才允许入队：无 runtime 且 writer 未启动时返回 `false`——此时
    /// 入队会让记录滞留通道，writer 之后在别的 runtime 里启动时被捞出，
    /// 与已计丢弃数双计失真；writer 已启动时无 runtime 上下文仍返回 `true`
    /// （接收端在位，writer 会继续捞出）。
    fn ensure_writer(&self) -> bool {
        if self.inner.writer_active.load(Ordering::Acquire) {
            return true;
        }
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return false;
        };
        self.inner.writer_started.call_once(|| {
            let rx = self
                .inner
                .rx
                .lock()
                .unwrap_or_else(|_| {
                    panic!("{}", crate::i18n::tr("audit-writer-rx-lock-not-poisoned"))
                })
                .take()
                .unwrap_or_else(|| {
                    panic!("{}", crate::i18n::tr("audit-writer-rx-present-at-startup"))
                });
            let sink = self.inner.sink.clone();
            let dropped = self.inner.dropped.clone();
            let write_failures = self.inner.write_failures.clone();
            // Handle::spawn 不返回 Result：runtime 已关停时 future 未被调度
            // 即随 WriterRxGuard 一并丢弃，通道残留由 Drop 逐条计入
            // dropped；此后入队走 Closed 路径继续逐条计数
            handle.spawn(async move {
                let mut guard = WriterRxGuard { rx, dropped };
                while let Some(record) = guard.rx.recv().await {
                    if sink.write(&record).await.is_err() {
                        write_failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
            self.inner.writer_active.store(true, Ordering::Release);
        });
        self.inner.writer_active.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InklogError;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;

    /// sink 写入时捕获的字段快照（LogRecord 可 Clone，这里只留断言所需面）。
    #[derive(Debug, Clone)]
    struct Captured {
        target: String,
        level: String,
        message: String,
        fields: HashMap<String, serde_json::Value>,
    }

    /// 收集型测试 sink：写入完成经 channel 通知（确定性等待），可注入写失败。
    struct CollectingSink {
        captured: Arc<Mutex<Vec<Captured>>>,
        done: std::sync::mpsc::Sender<()>,
        fail: AtomicBool,
    }

    #[async_trait::async_trait]
    impl LogSink for CollectingSink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            let fail = self.fail.load(Ordering::Relaxed);
            if !fail {
                self.captured.lock().unwrap().push(Captured {
                    target: record.target.clone(),
                    level: record.level.clone(),
                    message: record.message.clone(),
                    fields: record.fields.clone(),
                });
            }
            let _ = self.done.send(());
            if fail {
                Err(InklogError::RuntimeError("sink write fault".into()))
            } else {
                Ok(())
            }
        }

        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    fn publisher_with_sink(
        fail: bool,
    ) -> (
        OxcacheAuditPublisher,
        Arc<Mutex<Vec<Captured>>>,
        std::sync::mpsc::Receiver<()>,
    ) {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = std::sync::mpsc::channel();
        let sink = Arc::new(CollectingSink {
            captured: captured.clone(),
            done: tx,
            fail: AtomicBool::new(fail),
        });
        (OxcacheAuditPublisher::new(sink), captured, rx)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn status_changes_map_to_info_and_read_probes_to_debug() {
        let (publisher, captured, rx) = publisher_with_sink(false);
        let dyn_publisher: Arc<dyn AuditEventPublisher> = Arc::new(publisher.clone());

        // 7 个动作变体全覆盖：状态变更（Set/Delete/Clear）→ INFO；
        // 读探测与容量/过期清理（Hit/Miss/Evict/Expired）→ DEBUG
        dyn_publisher.publish(AuditEvent::new(AuditAction::Set).with_key("k1"));
        dyn_publisher.publish(AuditEvent::new(AuditAction::Delete).with_key("k2"));
        dyn_publisher.publish(AuditEvent::new(AuditAction::Clear).with_key("k5"));
        dyn_publisher.publish(AuditEvent::new(AuditAction::Hit).with_key("k3"));
        dyn_publisher.publish(AuditEvent::new(AuditAction::Miss).with_key("k4"));
        dyn_publisher.publish(AuditEvent::new(AuditAction::Evict).with_key("k6"));
        dyn_publisher.publish(AuditEvent::new(AuditAction::Expired).with_key("k7"));

        for _ in 0..7 {
            rx.recv_timeout(std::time::Duration::from_secs(5))
                .expect("the write-done signal");
        }
        let snapshot = captured.lock().unwrap();
        assert_eq!(snapshot.len(), 7);
        assert_eq!(snapshot[0].level, "INFO");
        assert_eq!(snapshot[1].level, "INFO");
        assert_eq!(snapshot[2].level, "INFO");
        assert_eq!(snapshot[3].level, "DEBUG");
        assert_eq!(snapshot[4].level, "DEBUG");
        assert_eq!(snapshot[5].level, "DEBUG");
        assert_eq!(snapshot[6].level, "DEBUG");
        assert_eq!(snapshot[0].target, "oxcache::audit");
        assert_eq!(snapshot[0].message, "cache audit event");
        assert_eq!(
            snapshot[0].fields.get("action").and_then(|v| v.as_str()),
            Some("set")
        );
        assert_eq!(
            snapshot[0].fields.get("key").and_then(|v| v.as_str()),
            Some("k1")
        );
        assert!(snapshot[0].fields.contains_key("timestamp_ms"));
        assert_eq!(publisher.dropped_count(), 0);
        assert_eq!(publisher.write_failure_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn publish_without_runtime_after_writer_started_is_delivered() {
        // 口径：writer 启动后，无 runtime 上下文的 publish 仍入队投递，
        // 不计入 dropped（比原实现"恒丢"更宽松：少丢事件、绝不阻塞、绝不上抛）。
        let (publisher, captured, rx) = publisher_with_sink(false);
        let dyn_publisher: Arc<dyn AuditEventPublisher> = Arc::new(publisher.clone());

        // 先在 runtime 内启动 writer
        dyn_publisher.publish(AuditEvent::new(AuditAction::Set).with_key("warmup"));
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .expect("the warm-up write-done signal");

        // 再从无 runtime 上下文的线程 publish
        let p = dyn_publisher.clone();
        let handle = std::thread::spawn(move || {
            p.publish(AuditEvent::new(AuditAction::Delete).with_key("no-runtime"));
        });
        handle.join().expect("the publish thread finished");

        rx.recv_timeout(std::time::Duration::from_secs(5))
            .expect("the no-runtime write-done signal");
        let snapshot = captured.lock().unwrap();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(
            snapshot[1].fields.get("key").and_then(|v| v.as_str()),
            Some("no-runtime")
        );
        assert_eq!(publisher.dropped_count(), 0);
        assert_eq!(publisher.write_failure_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sink_write_failure_is_counted() {
        let (publisher, captured, rx) = publisher_with_sink(true);

        publisher.publish(AuditEvent::new(AuditAction::Set).with_key("k"));

        rx.recv_timeout(std::time::Duration::from_secs(5))
            .expect("the write-attempt done signal");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while publisher.write_failure_count() == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(captured.lock().unwrap().is_empty());
        assert_eq!(
            publisher.write_failure_count(),
            1,
            "sink write failures should be counted explicitly"
        );
    }

    /// 闸门 sink：write 先经 `started` 确认已消费一条事件，再挂起于 `gate`
    /// 闸门前——用于把 writer 精确钉在首条写入上，构造确定的通道过载。
    struct GatedSink {
        started: std::sync::mpsc::Sender<()>,
        gate: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl LogSink for GatedSink {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            let _ = self.started.send(());
            self.gate.notified().await;
            Ok(())
        }

        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn channel_overload_drops_are_counted() {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let gate = Arc::new(tokio::sync::Notify::new());
        let publisher = OxcacheAuditPublisher::new(Arc::new(GatedSink {
            started: started_tx,
            gate: gate.clone(),
        }));

        publisher.publish(AuditEvent::new(AuditAction::Set).with_key("first"));
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the writer should have consumed the first event and be gated");

        for _ in 0..BRIDGE_CHANNEL_CAPACITY {
            publisher.publish(AuditEvent::new(AuditAction::Miss).with_key("fill"));
        }
        let overflow = 7u64;
        for _ in 0..overflow {
            publisher.publish(AuditEvent::new(AuditAction::Miss).with_key("overflow"));
        }

        assert_eq!(
            publisher.dropped_count(),
            overflow,
            "publish after a full channel should count as dropped, not block"
        );
        assert_eq!(publisher.write_failure_count(), 0);

        gate.notify_waiters();
    }
}
