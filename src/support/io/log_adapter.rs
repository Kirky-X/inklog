// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `log` crate 原生支持适配器
//!
//! 此模块实现 `log::Log` trait，使得 inklog 可以直接接收来自 `log` crate 的日志，
//! 无需 `tracing_log` 适配器。

use crate::LogRecord;
use crate::Metrics;
use crate::validation::LogSanitizer;
use chrono::Utc;
use crossbeam_channel::Sender;
use log::{Level, LevelFilter, Metadata, Record};
use std::sync::Arc;

/// `log` crate 适配器，实现 `log::Log` trait
///
/// 此适配器将 `log` crate 的日志转换为 inklog 的 `LogRecord` 格式，
/// 并分发到配置的 channels（console + async workers）。
///
/// 使用 channel 实现无锁热路径，避免锁竞争。
/// 使用 `Arc<LogRecord>` 避免深拷贝。
///
/// # 脱敏
///
/// 转换时对 `message` 与 `target` 应用与原生 tracing 路径一致的
/// [`LogSanitizer`] 脱敏（diting 修复：外部 log 入口此前绕过脱敏，
/// 构成与原生路径不一致的日志注入面）。`fields` 在本路径恒为空——
/// `log` crate 记录不携带结构化字段——故无需字段级脱敏。
///
/// # 结构性限制：database sink 不可达
///
/// 经本适配器转换的记录只会进入 console 与 file 两条通道，**永远进不了
/// database sink**：database worker 消费的是 manager 专用的
/// `db_sender`/`db_receiver` 独立通道，仅与 tracing subscriber 桥接
/// （`subscriber.with_extra_async_sender`），而本适配器只持有 console 与
/// file 的发送端，且 `log` crate 记录不含 database sink 所需的结构化
/// fields 路由信息。宿主应用如需 log 记录入库，应使用原生 tracing API。
pub struct LogAdapter {
    /// Channel sender for console output (lock-free)
    console_sender: Sender<Arc<LogRecord>>,
    /// Channel sender for async sinks (file, etc.; see struct docs for the
    /// database sink limitation)
    async_sender: Sender<Arc<LogRecord>>,
    /// Metrics for monitoring
    metrics: Arc<Metrics>,
    /// 与主路径一致的脱敏器（ANSI 剥离、换行/控制字符转义、敏感信息打码）
    sanitizer: LogSanitizer,
}

impl LogAdapter {
    /// 创建新的 LogAdapter
    ///
    /// # Arguments
    /// * `console_sender` - 控制台 channel 发送端，用于无锁快速输出
    /// * `async_sender` - 异步 channel 发送端，用于后台处理
    /// * `metrics` - 指标收集器
    pub fn new(
        console_sender: Sender<Arc<LogRecord>>,
        async_sender: Sender<Arc<LogRecord>>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            console_sender,
            async_sender,
            metrics,
            sanitizer: LogSanitizer::new(),
        }
    }

    /// 以自定义脱敏器创建 LogAdapter（其余同 [`LogAdapter::new`]）
    pub fn with_sanitizer(mut self, sanitizer: LogSanitizer) -> Self {
        self.sanitizer = sanitizer;
        self
    }

    /// 将 `log::Level` 转换为字符串
    fn level_to_string(level: Level) -> &'static str {
        match level {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// 将 `log::Record` 转换为 `LogRecord`
    ///
    /// `message` 与 `target` 经 [`LogSanitizer`] 脱敏（剥离 ANSI 转义、
    /// 转义换行/控制字符、打码敏感信息），与原生 tracing 路径保持一致，
    /// 堵住经外部 log crate 注入伪造日志行的入口。
    /// `fields` 恒为空（`log` crate 无结构化字段），见结构体文档。
    fn record_to_log_record(&self, record: &Record) -> LogRecord {
        LogRecord {
            timestamp: Utc::now(),
            level: Self::level_to_string(record.level()).to_string(),
            target: self.sanitizer.sanitize(record.target()),
            message: self.sanitizer.sanitize(&record.args().to_string()),
            file: record.file().map(|s| s.to_string()),
            line: record.line(),
            thread_id: format!("{:?}", std::thread::current().id()),
            trace_id: None,
            span_id: None,
            fields: Default::default(),
        }
    }
}

impl log::Log for LogAdapter {
    /// 检查给定级别的日志是否启用
    fn enabled(&self, metadata: &Metadata) -> bool {
        // 允许所有级别的日志，由全局 LevelFilter 过滤
        metadata.level() <= log::max_level()
    }

    /// 处理日志记录
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let log_record = Arc::new(self.record_to_log_record(record));

        // Fast path: Console - lock-free try_send, drop on full to avoid blocking
        let console_sent = self.console_sender.try_send(Arc::clone(&log_record));

        // Slow path: Async sinks (file, database, etc.) - drop on full to avoid blocking
        let async_sent = self.async_sender.try_send(log_record);

        // 每条记录最多计一次丢弃，即使两个通道同时拒绝它；
        // channel_blocked 仍按通道计数
        if console_sent.is_err() || async_sent.is_err() {
            self.metrics.inc_logs_dropped();
        }
        if let Err(crossbeam_channel::TrySendError::Full(_)) = console_sent {
            self.metrics.inc_channel_blocked();
        }
        if let Err(crossbeam_channel::TrySendError::Full(_)) = async_sent {
            self.metrics.inc_channel_blocked();
        }
    }

    /// 刷新缓冲区（no-op，因为使用 channel）
    fn flush(&self) {
        // Channel 是自动刷新的，这里不需要做任何事
        // 如果需要确保所有日志都已写入，可以等待 channel 排空
    }
}

/// 全局 logger 安装器
///
/// 将 `LogAdapter` 安装为 `log` crate 的全局 logger。
pub struct LogLogger {
    adapter: LogAdapter,
    max_level: LevelFilter,
}

impl LogLogger {
    /// 创建新的 LogLogger
    pub fn new(adapter: LogAdapter, max_level: LevelFilter) -> Self {
        Self { adapter, max_level }
    }

    /// 安装为全局 logger
    ///
    /// 此方法会调用 `log::set_boxed_logger` 和 `log::set_max_level`。
    /// 只能调用一次，多次调用会返回错误。
    ///
    /// # Returns
    /// `Ok(())` 如果安装成功，`Err(...)` 如果已经安装过 logger。
    pub fn install(self) -> Result<(), log::SetLoggerError> {
        let max_level = self.max_level;
        log::set_boxed_logger(Box::new(self))?;
        log::set_max_level(max_level);
        Ok(())
    }
}

impl log::Log for LogLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.adapter.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.adapter.log(record)
    }

    fn flush(&self) {
        self.adapter.flush()
    }
}

// 以下是为测试提供的辅助函数

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use log::Log;

    #[test]
    fn test_level_to_string() {
        assert_eq!(LogAdapter::level_to_string(Level::Error), "ERROR");
        assert_eq!(LogAdapter::level_to_string(Level::Warn), "WARN");
        assert_eq!(LogAdapter::level_to_string(Level::Info), "INFO");
        assert_eq!(LogAdapter::level_to_string(Level::Debug), "DEBUG");
        assert_eq!(LogAdapter::level_to_string(Level::Trace), "TRACE");
    }

    #[test]
    fn test_record_to_log_record() {
        let (console_tx, _) = bounded(100);
        let (async_tx, _) = bounded(100);
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_tx, async_tx, metrics);

        // 创建一个测试 log::Record
        let metadata = log::Metadata::builder()
            .target("test::module")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("Test message"))
            .file(Some("test.rs"))
            .line(Some(42))
            .build();

        let log_record = adapter.record_to_log_record(&record);

        assert_eq!(log_record.level, "INFO");
        assert_eq!(log_record.target, "test::module");
        assert_eq!(log_record.message, "Test message");
        assert_eq!(log_record.file, Some("test.rs".to_string()));
        assert_eq!(log_record.line, Some(42));
    }

    // ========================================================================
    // diting 修复：log 入口绕过脱敏（注入面）
    // ========================================================================

    #[test]
    fn test_record_to_log_record_sanitizes_message_and_target() {
        let (console_tx, _) = bounded(100);
        let (async_tx, _) = bounded(100);
        let adapter = LogAdapter::new(console_tx, async_tx, Arc::new(Metrics::new()));

        // 消息含 ANSI 控制序列 + CRLF 伪造日志行注入；target 含 ANSI + LF
        let raw_message = "\x1b[31mERR\x1b[0m fake\r\n2026-01-01 INFO injected";
        let raw_target = "tgt\x1b[31m\ninjected";

        let metadata = log::Metadata::builder()
            .target(raw_target)
            .level(Level::Info)
            .build();
        let args = format_args!("{}", raw_message);
        let record = log::Record::builder().metadata(metadata).args(args).build();

        let log_record = adapter.record_to_log_record(&record);

        // 转换前（原始输入）确实携带注入载荷
        assert!(raw_message.contains('\x1b') && raw_message.contains('\n'));
        assert!(raw_target.contains('\x1b') && raw_target.contains('\n'));

        // 转换后：ANSI 被剥离、换行被转义为字面量 "\n"，伪造行注入失效
        assert_eq!(log_record.message, "ERR fake\\n2026-01-01 INFO injected");
        assert_eq!(log_record.target, "tgt\\ninjected");
        assert!(!log_record.message.contains('\x1b'));
        assert!(!log_record.message.contains('\n') && !log_record.message.contains('\r'));
        assert!(!log_record.target.contains('\x1b'));
        assert!(!log_record.target.contains('\n'));

        // fields 路径恒为空（见结构体文档），无注入面
        assert!(log_record.fields.is_empty());
    }

    #[test]
    fn test_log_adapter_log_sanitizes_injected_record_through_channels() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let adapter = LogAdapter::new(console_tx, async_tx, Arc::new(Metrics::new()));

        log::set_max_level(log::LevelFilter::Info);
        // token 后置空格，避免 redaction 的 \S+ 连带吞掉字面量 "\n"
        let raw_message = "line1\r\n\x1b[32mtoken=supersecret injected";
        let metadata = log::Metadata::builder()
            .target("test::adapter::inject")
            .level(Level::Info)
            .build();
        let args = format_args!("{}", raw_message);
        let record = log::Record::builder().metadata(metadata).args(args).build();

        adapter.log(&record);

        // console 与 async 两条通道收到的记录均已脱敏
        for rx in [&console_rx, &async_rx] {
            let received = rx.recv().unwrap();
            assert!(!received.message.contains('\x1b'));
            assert!(!received.message.contains('\n') && !received.message.contains('\r'));
            // 敏感信息打码 + 换行转义 + ANSI 剥离
            assert!(
                received.message.contains("token=[REDACTED]"),
                "sensitive pattern should be redacted: {}",
                received.message
            );
            assert!(
                !received.message.contains("supersecret"),
                "secret must not survive sanitization: {}",
                received.message
            );
            assert!(received.message.contains("\\n"));
        }
    }

    #[test]
    fn test_log_adapter_with_sanitizer_uses_custom_instance() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        // 自定义 sanitizer：额外替换规则生效
        let mut sanitizer = LogSanitizer::new();
        sanitizer.add_replacement("corp-secret".to_string(), "[CUSTOM]".to_string());
        let adapter = LogAdapter::new(console_tx, async_tx, Arc::new(Metrics::new()))
            .with_sanitizer(sanitizer);

        let metadata = log::Metadata::builder()
            .target("test::custom")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("leak corp-secret now"))
            .build();

        let log_record = adapter.record_to_log_record(&record);
        assert_eq!(log_record.message, "leak [CUSTOM] now");
    }

    #[test]
    fn test_log_adapter_log_sends_to_channels() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_tx, async_tx, metrics);

        log::set_max_level(log::LevelFilter::Info);
        let metadata = log::Metadata::builder()
            .target("test::adapter")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("Adapter send"))
            .file(Some("test.rs"))
            .line(Some(7))
            .build();

        adapter.log(&record);

        // Verify console channel received the record
        let console_received = console_rx.recv().unwrap();
        assert_eq!(console_received.level, "INFO");
        assert_eq!(console_received.target, "test::adapter");
        assert_eq!(console_received.message, "Adapter send");

        // Verify async channel received the record
        let async_received = async_rx.recv().unwrap();
        assert_eq!(async_received.level, "INFO");
        assert_eq!(async_received.target, "test::adapter");
        assert_eq!(async_received.message, "Adapter send");
    }

    #[test]
    fn test_record_to_log_record_thread_id_is_real_id() {
        let (console_tx, _) = bounded(100);
        let (async_tx, _) = bounded(100);
        let adapter = LogAdapter::new(console_tx, async_tx, Arc::new(Metrics::new()));

        let metadata = log::Metadata::builder()
            .target("test::thread")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("thread id check"))
            .build();

        let log_record = adapter.record_to_log_record(&record);

        // thread_id must carry the real thread id, not the thread name
        let expected = format!("{:?}", std::thread::current().id());
        assert_eq!(log_record.thread_id, expected);
        assert!(expected.starts_with("ThreadId("));
    }

    #[test]
    fn test_log_adapter_handles_full_channel() {
        // Create channels with capacity 1
        let (console_tx, console_rx) = bounded(1);
        let (async_tx, async_rx) = bounded(1);
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_tx, async_tx, metrics.clone());

        log::set_max_level(log::LevelFilter::Info);

        // Send multiple records - should not panic even when channels are full
        for i in 0..5 {
            let metadata = log::Metadata::builder()
                .target("test::adapter")
                .level(Level::Info)
                .build();
            let msg = format!("Test message {}", i);
            let args = format_args!("{}", msg);
            let record = log::Record::builder().metadata(metadata).args(args).build();
            adapter.log(&record);
        }

        // Drain channels
        while console_rx.try_recv().is_ok() {}
        while async_rx.try_recv().is_ok() {}

        // Channels of capacity 1 hold at most 1 item each; the remaining
        // 4 records fail on both channels but count as dropped only once
        assert_eq!(metrics.logs_dropped(), 4);
        // channel_blocked stays per-channel: 4 rejected records × 2 channels
        assert_eq!(metrics.channel_blocked(), 8);
    }

    #[test]
    fn test_log_adapter_disconnected_channel() {
        let (console_tx, _cr) = bounded(10);
        let (async_tx, _ar) = bounded(1);
        drop(_ar); // Disconnect async channel
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_tx, async_tx, metrics.clone());

        log::set_max_level(log::LevelFilter::Info);
        let metadata = log::Metadata::builder()
            .target("test::adapter")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("after disconnect"))
            .build();

        // Should not panic; disconnected async channel drops log and increments metric
        adapter.log(&record);

        assert_eq!(metrics.logs_dropped(), 1);
    }

    #[test]
    fn test_flush_is_noop() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_tx, async_tx, metrics);
        // flush() is a no-op with channel-based design — just verify it doesn't panic
        adapter.flush();
    }

    #[test]
    fn test_log_adapter_all_levels_mapped() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let adapter = LogAdapter::new(console_tx, async_tx, metrics.clone());

        log::set_max_level(log::LevelFilter::Trace);

        for (level, expected_str) in [
            (Level::Error, "ERROR"),
            (Level::Warn, "WARN"),
            (Level::Info, "INFO"),
            (Level::Debug, "DEBUG"),
            (Level::Trace, "TRACE"),
        ] {
            let metadata = log::Metadata::builder()
                .target("test::levels")
                .level(level)
                .build();
            let args = format_args!("msg for {expected_str}");
            let record = log::Record::builder().metadata(metadata).args(args).build();

            let log_record = adapter.record_to_log_record(&record);
            assert_eq!(
                log_record.level, expected_str,
                "level {level:?} should map to {expected_str}"
            );
        }
    }

    #[test]
    fn test_log_adapter_enabled_respects_max_level() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let adapter = LogAdapter::new(console_tx, async_tx, metrics);

        // Set global max level to Info
        log::set_max_level(log::LevelFilter::Info);

        let info_meta = log::Metadata::builder()
            .target("test")
            .level(Level::Info)
            .build();
        let debug_meta = log::Metadata::builder()
            .target("test")
            .level(Level::Debug)
            .build();
        let trace_meta = log::Metadata::builder()
            .target("test")
            .level(Level::Trace)
            .build();

        assert!(adapter.enabled(&info_meta));
        assert!(!adapter.enabled(&debug_meta));
        assert!(!adapter.enabled(&trace_meta));
    }

    #[test]
    fn test_log_adapter_console_disconnected_channel() {
        // Test the console Disconnected branch (lines 104-106)
        let (console_tx, _cr) = bounded(10);
        let (async_tx, _ar) = bounded(10);
        drop(_cr); // Disconnect console channel
        drop(_ar); // Disconnect async channel
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_tx, async_tx, metrics.clone());

        log::set_max_level(log::LevelFilter::Info);
        let metadata = log::Metadata::builder()
            .target("test::adapter")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("console disconnected"))
            .build();

        // Should not panic; disconnected console channel drops log and increments metric
        adapter.log(&record);

        // Both channels are disconnected, but the record is counted as
        // dropped only once
        assert_eq!(metrics.logs_dropped(), 1);
    }

    #[test]
    fn test_log_logger_new() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let adapter = LogAdapter::new(console_tx, async_tx, metrics);
        let logger = LogLogger::new(adapter, LevelFilter::Info);
        // Just verify creation succeeds - we can't directly access private fields
        // but the fact that it compiles and runs is sufficient
        let _ = logger;
    }

    #[test]
    fn test_log_logger_enabled() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let adapter = LogAdapter::new(console_tx, async_tx, metrics);
        let logger = LogLogger::new(adapter, LevelFilter::Info);

        log::set_max_level(log::LevelFilter::Info);
        let info_meta = log::Metadata::builder()
            .target("test")
            .level(Level::Info)
            .build();
        let debug_meta = log::Metadata::builder()
            .target("test")
            .level(Level::Debug)
            .build();

        assert!(logger.enabled(&info_meta));
        assert!(!logger.enabled(&debug_meta));
    }

    #[test]
    fn test_log_logger_log() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let adapter = LogAdapter::new(console_tx, async_tx, metrics);
        let logger = LogLogger::new(adapter, LevelFilter::Info);

        log::set_max_level(log::LevelFilter::Info);
        let metadata = log::Metadata::builder()
            .target("test::logger")
            .level(Level::Info)
            .build();
        let record = log::Record::builder()
            .metadata(metadata)
            .args(format_args!("via LogLogger"))
            .build();

        logger.log(&record);

        // Verify the record was sent through both channels
        let console_received = console_rx.recv().unwrap();
        assert_eq!(console_received.message, "via LogLogger");

        let async_received = async_rx.recv().unwrap();
        assert_eq!(async_received.message, "via LogLogger");
    }

    #[test]
    fn test_log_logger_flush() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let adapter = LogAdapter::new(console_tx, async_tx, metrics);
        let logger = LogLogger::new(adapter, LevelFilter::Info);

        // flush() is a no-op, just verify it doesn't panic
        logger.flush();
    }

    // ========================================================================
    // LogLogger::install - 覆盖行 153-154（成功路径）与错误路径
    // ========================================================================

    #[test]
    #[serial_test::serial]
    fn test_log_logger_install_second_call_err_after_first() {
        // 全局 logger 只能安装一次。
        // 注意：在并行测试环境中，全局 logger 可能已被其他测试（如 manager.rs 的测试）设置。
        // 本测试验证：第二次 install 必然返回 Err(SetLoggerError)。
        //
        // 如果第一次 install 成功（全局 logger 未被占用），则覆盖行 153-154（set_max_level + Ok(())）。
        // 如果第一次 install 失败（全局 logger 已被占用），则跳过行 153-154 覆盖，但仍验证第二次失败。

        let (console_tx1, _cr1) = bounded(10);
        let (async_tx1, _ar1) = bounded(10);
        let metrics1 = Arc::new(Metrics::new());
        let adapter1 = LogAdapter::new(console_tx1, async_tx1, metrics1);
        let logger1 = LogLogger::new(adapter1, LevelFilter::Info);

        // 第一次 install：可能成功（覆盖行 153-154）或失败（全局 logger 已被占用）
        let result1 = logger1.install();

        // 第二次 install：必须失败（无论第一次是否成功，全局 logger 此时已被占用）
        let (console_tx2, _cr2) = bounded(10);
        let (async_tx2, _ar2) = bounded(10);
        let metrics2 = Arc::new(Metrics::new());
        let adapter2 = LogAdapter::new(console_tx2, async_tx2, metrics2);
        let logger2 = LogLogger::new(adapter2, LevelFilter::Info);

        let result2 = logger2.install();
        assert!(
            result2.is_err(),
            "second install should fail because a global logger is already installed, \
             got: {:?} (first install result: {:?})",
            result2,
            result1
        );
    }
}
