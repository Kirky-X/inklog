// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! `log` crate 原生支持适配器
//!
//! 此模块实现 `log::Log` trait，使得 inklog 可以直接接收来自 `log` crate 的日志，
//! 无需 `tracing_log` 适配器。

use crate::log_record::LogRecord;
use crate::metrics::Metrics;
use crate::object_pool::{LOG_RECORD_POOL, STRING_POOL};
use crate::sink::console::ConsoleSink;
use crate::sink::LogSink;
use chrono::Utc;
use crossbeam_channel::Sender;
use log::{Level, LevelFilter, Metadata, Record};
use std::sync::{Arc, Mutex};

/// `log` crate 适配器，实现 `log::Log` trait
///
/// 此适配器将 `log` crate 的日志转换为 inklog 的 `LogRecord` 格式，
/// 并分发到配置的 sinks（console + async workers）。
pub struct LogAdapter {
    console_sink: Arc<Mutex<ConsoleSink>>,
    async_sender: Sender<LogRecord>,
    metrics: Arc<Metrics>,
}

impl LogAdapter {
    /// 创建新的 LogAdapter
    ///
    /// # Arguments
    /// * `console_sink` - 控制台 sink，用于同步快速输出
    /// * `async_sender` - 异步 channel 发送端，用于后台处理
    /// * `metrics` - 指标收集器
    pub fn new(
        console_sink: Arc<Mutex<ConsoleSink>>,
        async_sender: Sender<LogRecord>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            console_sink,
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
        let mut log_record = LOG_RECORD_POOL.get();
        log_record.reset();

        let mut message = STRING_POOL.get();
        message.clear();
        message.push_str(&record.args().to_string());

        log_record.timestamp = Utc::now();
        log_record.level.clear();
        log_record
            .level
            .push_str(Self::level_to_string(record.level()));
        log_record.target.clear();
        log_record.target.push_str(record.target());
        log_record.message = message;
        log_record.file = record.file().map(|s| s.to_string());
        log_record.line = record.line();
        log_record.thread_id.clear();
        log_record
            .thread_id
            .push_str(std::thread::current().name().unwrap_or("unknown"));

        log_record
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

        let log_record = self.record_to_log_record(record);

        // Fast path: Console (同步写入)
        if let Ok(mut sink) = self.console_sink.lock() {
            if sink.write(&log_record).is_err() {
                self.metrics.inc_sink_error();
            }
        }

        // Slow path: Async (发送到后台 workers)
        match self.async_sender.try_send(log_record.clone()) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(r)) => {
                self.metrics.inc_channel_blocked();
                if self.async_sender.send(r).is_err() {
                    self.metrics.inc_logs_dropped();
                }
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }

        LOG_RECORD_POOL.put(log_record);
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
        let (sender, _) = bounded(100);
        let console_config = crate::config::ConsoleSinkConfig::default();
        let template = crate::template::LogTemplate::new("{timestamp} [{level}] {message}");
        let console_sink = Arc::new(Mutex::new(ConsoleSink::new(console_config, template)));
        let metrics = Arc::new(Metrics::new());

        let adapter = LogAdapter::new(console_sink, sender, metrics);

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
}
