// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! `log` crate 原生支持适配器
//!
//! 此模块实现 `log::Log` trait，使得 inklog 可以直接接收来自 `log` crate 的日志，
//! 无需 `tracing_log` 适配器。

use crate::LogRecord;
use crate::Metrics;
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
pub struct LogAdapter {
    /// Channel sender for console output (lock-free)
    console_sender: Sender<Arc<LogRecord>>,
    /// Channel sender for async sinks (file, database, etc.)
    async_sender: Sender<Arc<LogRecord>>,
    /// Metrics for monitoring
    metrics: Arc<Metrics>,
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
        }
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
    fn record_to_log_record(&self, record: &Record) -> LogRecord {
        LogRecord {
            timestamp: Utc::now(),
            level: Self::level_to_string(record.level()).to_string(),
            target: record.target().to_string(),
            message: record.args().to_string(),
            file: record.file().map(|s| s.to_string()),
            line: record.line(),
            thread_id: std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string(),
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
        match self.console_sender.try_send(Arc::clone(&log_record)) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                self.metrics.inc_channel_blocked();
                self.metrics.inc_logs_dropped();
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }

        // Slow path: Async sinks (file, database, etc.) - drop on full to avoid blocking
        match self.async_sender.try_send(log_record) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                self.metrics.inc_channel_blocked();
                self.metrics.inc_logs_dropped();
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
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

        // Channels of capacity 1 can hold at most 1 item; 4 items dropped (2 per channel)
        assert_eq!(metrics.logs_dropped(), 8);
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

        // Both channels are disconnected, so 2 drops (one per channel)
        assert_eq!(metrics.logs_dropped(), 2);
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
