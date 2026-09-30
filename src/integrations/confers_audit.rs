// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! confers `AuditSink` 端口的 inklog 实现（feature `confers-audit`）。
//!
//! 实现下层 confers 定义的 [`AuditSink`] 端口（对象安全，`Arc<dyn AuditSink>`
//! 注入 [`confers::audit::AuditWriter`]）：把 confers 审计事件批量转换为结构化
//! [`LogRecord`]（`target = "confers::audit"`、`message` = 事件 JSON、变体载荷
//! 平铺进 `fields`），写入注入的 inklog [`LogSink`]——与 console/file/db sink
//! 走同一条落盘管道。
//!
//! 端口契约是**同步** `fn write(&self, events: &[AuditEvent])` 且「不得无限
//! 阻塞、失败自理」，而 [`LogSink::write`] 是 async——桥接经 mpsc 通道 +
//! 惰性 writer task（首个事件进入 tokio runtime 上下文时启动一次，随其
//! runtime 关停而取消）：通道满或 writer 未就绪（无 runtime 上下文 /
//! spawn 失败）时按丢弃计数，绝不回压 confers 的审计写入路径；writer 启动
//! 后无 runtime 上下文的调用仍可正常入队（接收端在位）。
//!
//! 级别映射循 confers 自身的 [`AuditLevel`] 分类：Durable（密钥访问/轮转/
//! 解密）→ `WARN`，BestEffort（加载成功/重载触发）→ `INFO`。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once};

use confers::audit::{AuditEvent, AuditLevel, AuditSink};
use tokio::sync::mpsc;

use crate::LogRecord;
use crate::support::io::sink::LogSink;

/// 通道默认容量：突发批量写入的缓冲上限，超出即丢弃计数。
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// confers 审计事件 → inklog 结构化日志桥。
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
/// use confers::audit::AuditWriter;
/// use inklog::integrations::ConfersAuditSink;
///
/// let bridge = ConfersAuditSink::new(my_inklog_sink);
/// let mut writer = AuditWriter::builder().enabled(true).build();
/// writer.add_sink(Arc::new(bridge));
/// ```
pub struct ConfersAuditSink {
    inner: Arc<BridgeInner>,
}

struct BridgeInner {
    sink: Arc<dyn LogSink>,
    tx: mpsc::Sender<LogRecord>,
    /// writer task 只在首个 runtime 上下文 write 时启动一次；启动前接收端在位。
    rx: Mutex<Option<mpsc::Receiver<LogRecord>>>,
    writer_started: Once,
    /// writer task 是否已成功派生。未就绪时同步端口不得入队（否则记录滞留
    /// 通道，writer 复活后被捞出，与已计丢弃数双计失真）；就绪后无 runtime
    /// 上下文的调用仍可正常入队（接收端在位）。
    writer_active: AtomicBool,
    /// 计数器走 Arc 共享：writer task 与同步端口两侧各自持有克隆。
    dropped: Arc<AtomicU64>,
    write_failures: Arc<AtomicU64>,
    accepted: Arc<AtomicU64>,
}

/// writer task 的接收端守护：无论任务正常结束、随 runtime 关停被取消，
/// 还是 runtime 已关停导致 future 未被调度即丢弃，Drop 都会把通道残留
/// 记录逐条计入 dropped（显性化，不静默消失）。
struct GuardedReceiver {
    rx: mpsc::Receiver<LogRecord>,
    dropped: Arc<AtomicU64>,
}

impl GuardedReceiver {
    async fn recv(&mut self) -> Option<LogRecord> {
        self.rx.recv().await
    }
}

impl Drop for GuardedReceiver {
    fn drop(&mut self) {
        while let Ok(record) = self.rx.try_recv() {
            drop(record);
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl ConfersAuditSink {
    /// 以注入的 inklog `LogSink` 创建审计桥（默认通道容量）。
    pub fn new(sink: Arc<dyn LogSink>) -> Self {
        Self::with_capacity(sink, DEFAULT_CHANNEL_CAPACITY)
    }

    /// 以指定通道容量创建审计桥。
    ///
    /// 容量约束的是同步端口与异步落盘之间的突发缓冲；溢出走丢弃计数而非
    /// 阻塞（端口契约「不得无限阻塞」）。
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
                accepted: Arc::new(AtomicU64::new(0)),
            }),
        }
    }

    /// 已丢弃记录数（通道满 / writer 未就绪 / writer 关停残留）。
    pub fn dropped(&self) -> u64 {
        self.inner.dropped.load(Ordering::Relaxed)
    }

    /// sink 写入失败次数（失败自理契约的观测面）。
    pub fn write_failures(&self) -> u64 {
        self.inner.write_failures.load(Ordering::Relaxed)
    }

    /// 已成功写入 sink 的记录数。
    pub fn accepted(&self) -> u64 {
        self.inner.accepted.load(Ordering::Relaxed)
    }

    /// [`AuditEvent`] → [`LogRecord`] 映射：message 携带完整事件 JSON，
    /// 变体载荷平铺进 `fields` 便于 sink 侧按字段检索。
    pub fn to_log_record(event: &AuditEvent) -> LogRecord {
        let level = match AuditLevel::for_event(event) {
            AuditLevel::Durable => tracing::Level::WARN,
            AuditLevel::BestEffort => tracing::Level::INFO,
        };
        let serialized = serde_json::to_string(event);
        debug_assert!(
            serialized.is_ok(),
            "AuditEvent 序列化不应失败：变体载荷均为可序列化基本类型"
        );
        let mut record = LogRecord::new(
            level,
            "confers::audit".to_string(),
            serialized.unwrap_or_else(|_| "{}".to_string()),
        );
        record.timestamp = event.event_timestamp();
        record.fields.insert(
            "event".to_string(),
            serde_json::Value::String(event_name(event).to_string()),
        );
        record.fields.insert(
            "timestamp".to_string(),
            serde_json::Value::String(event.event_timestamp().to_rfc3339()),
        );
        match event {
            AuditEvent::KeyAccess { key, .. } => {
                record
                    .fields
                    .insert("key".to_string(), serde_json::Value::String(key.clone()));
            }
            AuditEvent::KeyRotation {
                old_version,
                new_version,
                ..
            } => {
                record.fields.insert(
                    "old_version".to_string(),
                    serde_json::Value::String(old_version.clone()),
                );
                record.fields.insert(
                    "new_version".to_string(),
                    serde_json::Value::String(new_version.clone()),
                );
            }
            AuditEvent::Decrypt { field, success, .. } => {
                record.fields.insert(
                    "field".to_string(),
                    serde_json::Value::String(field.clone()),
                );
                record
                    .fields
                    .insert("success".to_string(), serde_json::Value::Bool(*success));
            }
            AuditEvent::LoadSuccess { source, .. } | AuditEvent::ReloadTrigger { source, .. } => {
                record.fields.insert(
                    "source".to_string(),
                    serde_json::Value::String(source.clone()),
                );
            }
        }
        record
    }

    /// 单事件入队：writer 未就绪时丢弃计数且不入队（防复活双计）。
    fn enqueue(&self, record: LogRecord) {
        if !self.ensure_writer() {
            self.inner.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        match self.inner.tx.try_send(record) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) | Err(mpsc::error::TrySendError::Closed(_)) => {
                self.inner.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// 惰性启动 writer task；返回 writer 是否就绪（已成功派生）。
    ///
    /// 只有就绪才允许入队：无 runtime 且 writer 未启动时返回 false——此时
    /// 入队会让记录滞留通道，writer 之后在别的 runtime 里启动时被捞出，
    /// 与已计丢弃数双计失真；writer 已启动时无 runtime 上下文仍返回 true
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
                .expect("writer 接收端锁不会中毒（临界区无 panic 点）")
                .take()
                .expect("writer 首次启动时接收端必然在位");
            let sink = self.inner.sink.clone();
            let dropped = self.inner.dropped.clone();
            let write_failures = self.inner.write_failures.clone();
            let accepted = self.inner.accepted.clone();
            // Handle::spawn 不返回 Result：runtime 已关停时 future 未被调度
            // 即随 GuardedReceiver 一并丢弃，通道残留由 Drop 逐条计入
            // dropped；此后入队走 Closed 路径继续逐条计数
            handle.spawn(async move {
                let mut rx = GuardedReceiver { rx, dropped };
                while let Some(record) = rx.recv().await {
                    if sink.write(&record).await.is_ok() {
                        accepted.fetch_add(1, Ordering::Relaxed);
                    } else {
                        write_failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
            self.inner.writer_active.store(true, Ordering::Release);
        });
        self.inner.writer_active.load(Ordering::Acquire)
    }
}

/// 变体判别名（fields.event 取值；message JSON 的顶层键同此名）。
fn event_name(event: &AuditEvent) -> &'static str {
    match event {
        AuditEvent::KeyAccess { .. } => "KeyAccess",
        AuditEvent::KeyRotation { .. } => "KeyRotation",
        AuditEvent::Decrypt { .. } => "Decrypt",
        AuditEvent::LoadSuccess { .. } => "LoadSuccess",
        AuditEvent::ReloadTrigger { .. } => "ReloadTrigger",
    }
}

impl AuditSink for ConfersAuditSink {
    /// 批量投递：逐事件转换入队，全程同步不阻塞（通道满即丢弃计数）。
    fn write(&self, events: &[AuditEvent]) {
        for event in events {
            self.enqueue(Self::to_log_record(event));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InklogError;
    use chrono::Utc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    /// 测试 sink：记录写入的 LogRecord；可注入失败与阻塞。
    struct RecordingSink {
        records: Mutex<Vec<LogRecord>>,
        fail: AtomicBool,
        hold: AtomicBool,
    }

    impl RecordingSink {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                records: Mutex::new(Vec::new()),
                fail: AtomicBool::new(false),
                hold: AtomicBool::new(false),
            })
        }

        fn snapshot(&self) -> Vec<LogRecord> {
            self.records.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl LogSink for RecordingSink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            while self.hold.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err(InklogError::ConfigError("injected failure".to_string()));
            }
            self.records.lock().unwrap().push(record.clone());
            Ok(())
        }

        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    fn sample_events() -> Vec<AuditEvent> {
        let now = Utc::now();
        vec![
            AuditEvent::KeyAccess {
                key: "db.password".to_string(),
                timestamp: now,
            },
            AuditEvent::KeyRotation {
                old_version: "v1".to_string(),
                new_version: "v2".to_string(),
                timestamp: now,
            },
            AuditEvent::Decrypt {
                field: "api_key".to_string(),
                success: false,
                timestamp: now,
            },
            AuditEvent::LoadSuccess {
                source: "app.toml".to_string(),
                timestamp: now,
            },
            AuditEvent::ReloadTrigger {
                source: "watch".to_string(),
                timestamp: now,
            },
        ]
    }

    /// 等待 accepted 达到目标值（writer task 异步落盘，轮询让出调度）。
    async fn wait_accepted(bridge: &ConfersAuditSink, target: u64) {
        for _ in 0..2000 {
            if bridge.accepted() >= target {
                return;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        panic!(
            "accepted did not reach {target} in time (accepted={})",
            bridge.accepted()
        );
    }

    #[tokio::test]
    async fn test_batch_events_reach_structured_sink() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());

        let events = sample_events();
        bridge.write(&events);
        wait_accepted(&bridge, events.len() as u64).await;

        let records = sink.snapshot();
        assert_eq!(
            records.len(),
            5,
            "批量事件的每一条都必须出现在结构化 sink 中"
        );
        assert!(records.iter().all(|r| r.target == "confers::audit"));
    }

    #[tokio::test]
    async fn test_variant_payloads_are_flattened_into_fields() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());

        bridge.write(&sample_events());
        wait_accepted(&bridge, 5).await;

        let records = sink.snapshot();
        let by_event = |name: &str| {
            records
                .iter()
                .find(|r| r.fields.get("event").unwrap() == name)
                .unwrap_or_else(|| panic!("event {name} missing"))
        };

        let key_access = by_event("KeyAccess");
        assert_eq!(key_access.fields.get("key").unwrap(), "db.password");

        let rotation = by_event("KeyRotation");
        assert_eq!(rotation.fields.get("old_version").unwrap(), "v1");
        assert_eq!(rotation.fields.get("new_version").unwrap(), "v2");

        let decrypt = by_event("Decrypt");
        assert_eq!(decrypt.fields.get("field").unwrap(), "api_key");
        assert_eq!(
            decrypt.fields.get("success").unwrap(),
            &serde_json::json!(false)
        );

        let load = by_event("LoadSuccess");
        assert_eq!(load.fields.get("source").unwrap(), "app.toml");

        let reload = by_event("ReloadTrigger");
        assert_eq!(reload.fields.get("source").unwrap(), "watch");
    }

    #[tokio::test]
    async fn test_level_follows_confers_audit_level() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());

        bridge.write(&sample_events());
        wait_accepted(&bridge, 5).await;

        let records = sink.snapshot();
        for record in &records {
            let durable = matches!(
                record.fields.get("event").unwrap().as_str().unwrap(),
                "KeyAccess" | "KeyRotation" | "Decrypt"
            );
            let expected = if durable { "WARN" } else { "INFO" };
            assert_eq!(
                record.level, expected,
                "级别映射必须循 confers AuditLevel 分类"
            );
        }
    }

    #[tokio::test]
    async fn test_message_is_event_json_and_timestamp_passthrough() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());

        let events = sample_events();
        bridge.write(&events);
        wait_accepted(&bridge, events.len() as u64).await;

        let records = sink.snapshot();
        for (record, event) in records.iter().zip(&events) {
            let json: serde_json::Value =
                serde_json::from_str(&record.message).expect("message 必须是可解析的事件 JSON");
            assert!(
                json.get(event_name(event)).is_some(),
                "外部标签变体名在顶层"
            );
            assert_eq!(record.timestamp, event.event_timestamp(), "时间戳透传");
        }
    }

    #[tokio::test]
    async fn test_write_failures_are_counted_not_swallowed() {
        let sink = RecordingSink::new();
        sink.fail.store(true, Ordering::SeqCst);
        let bridge = ConfersAuditSink::new(sink);

        bridge.write(&sample_events());
        for _ in 0..2000 {
            if bridge.write_failures() >= 5 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        assert_eq!(
            bridge.write_failures(),
            5,
            "sink 写入失败必须显性计入 write_failures"
        );
        assert_eq!(bridge.accepted(), 0);
    }

    #[test]
    fn test_no_runtime_context_drops_without_blocking() {
        // 非 tokio 上下文调用同步端口：不 panic、不阻塞、按条计数丢弃
        let bridge = ConfersAuditSink::new(RecordingSink::new());
        bridge.write(&sample_events());
        assert_eq!(bridge.dropped(), 5);
        assert_eq!(bridge.accepted(), 0);
    }

    #[test]
    fn test_dropped_before_writer_start_never_revive_after_start() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());

        // 时序一：writer 启动前、无 runtime——丢弃计数且不得入队
        let early: Vec<AuditEvent> = sample_events().into_iter().take(2).collect();
        bridge.write(&early);
        assert_eq!(bridge.dropped(), 2);
        assert_eq!(bridge.accepted(), 0);

        // 时序二：runtime 内 writer 惰性启动——已计丢弃的记录不得复活
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("测试 runtime 构建不会失败");
        rt.block_on(async {
            let late: Vec<AuditEvent> = sample_events().into_iter().skip(2).collect();
            bridge.write(&late);
            wait_accepted(&bridge, 3).await;
        });
        assert_eq!(bridge.accepted(), 3, "writer 只接受启动后入队的记录");
        assert_eq!(bridge.dropped(), 2, "已计丢弃的记录不得双计");
        assert_eq!(sink.snapshot().len(), 3);
    }

    #[test]
    fn test_no_runtime_after_writer_started_enqueues_normally() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("测试 runtime 构建不会失败");

        // 时序一：runtime 内启动 writer
        rt.block_on(async {
            bridge.write(&sample_events());
            wait_accepted(&bridge, 5).await;
        });

        // 时序二：writer 已启动后无 runtime——接收端在位，正常入队不计数
        let late = vec![AuditEvent::LoadSuccess {
            source: "late".to_string(),
            timestamp: Utc::now(),
        }];
        bridge.write(&late);
        assert_eq!(
            bridge.dropped(),
            0,
            "writer 存活时无 runtime 调用不得计丢弃"
        );

        rt.block_on(async {
            wait_accepted(&bridge, 6).await;
        });
        assert_eq!(bridge.accepted(), 6);
        assert_eq!(sink.snapshot().len(), 6);
    }

    #[test]
    fn test_shutdown_runtime_records_counted_dropped_explicitly() {
        let sink = RecordingSink::new();
        let bridge = ConfersAuditSink::new(sink.clone());

        // runtime 关停上下文：writer task 随关停被丢弃（接收端不在位），
        // 记录必须逐条计入 dropped 而非静默滞留通道
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("测试 runtime 构建不会失败");
        let handle = rt.handle().clone();
        drop(rt);
        let _enter = handle.enter();

        bridge.write(&sample_events());
        assert_eq!(
            bridge.dropped(),
            5,
            "关停 runtime 上启动 writer 后记录必须显性计入 dropped"
        );
        assert_eq!(bridge.accepted(), 0);
        assert!(sink.snapshot().is_empty());

        // writer 永不复活（接收端已消失）：后续无 runtime 调用继续逐条计数
        drop(_enter);
        let more = vec![AuditEvent::LoadSuccess {
            source: "after-shutdown".to_string(),
            timestamp: Utc::now(),
        }];
        bridge.write(&more);
        assert_eq!(bridge.dropped(), 6);
    }

    #[tokio::test]
    async fn test_full_channel_drops_instead_of_blocking() {
        let sink = RecordingSink::new();
        sink.hold.store(true, Ordering::SeqCst);
        // 容量 2：writer 被阻塞期间，同步 write 的 try_send 在任何 await
        // 之前同步完成，前 2 条填满通道，其余全部丢弃
        let bridge = ConfersAuditSink::with_capacity(sink.clone(), 2);

        let events: Vec<AuditEvent> = (0..10)
            .map(|i| AuditEvent::LoadSuccess {
                source: format!("src-{i}"),
                timestamp: Utc::now(),
            })
            .collect();
        bridge.write(&events);

        assert_eq!(
            bridge.dropped(),
            8,
            "通道满必须立即丢弃计数，绝不阻塞 confers 审计路径"
        );

        sink.hold.store(false, Ordering::SeqCst);
        wait_accepted(&bridge, 2).await;
    }
}
