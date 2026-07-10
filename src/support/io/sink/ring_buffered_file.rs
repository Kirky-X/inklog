// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! High-performance file sink using crossbeam channels.

use crate::FileSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::LogTemplate;
use crate::support::io::sink::LogSink;
use async_trait::async_trait;
use crossbeam_channel;
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration as StdDuration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressureStrategy {
    #[default]
    Block,
    DropOldest,
    DropNewest,
}

#[derive(Debug, Clone)]
pub struct ChannelBufferedConfig {
    pub base_config: FileSinkConfig,
    pub channel_capacity: usize,
    pub backpressure_strategy: BackpressureStrategy,
    pub flush_batch_size: usize,
    pub flush_interval_ms: u64,
}

impl Default for ChannelBufferedConfig {
    fn default() -> Self {
        Self {
            base_config: FileSinkConfig::default(),
            channel_capacity: 10_000,
            backpressure_strategy: BackpressureStrategy::default(),
            flush_batch_size: 1000,
            flush_interval_ms: 100,
        }
    }
}

/// ChannelBufferedFileSink 的可变状态
struct Inner {
    io_thread: Option<thread::JoinHandle<()>>,
    flush_thread: Option<thread::JoinHandle<()>>,
}

pub struct ChannelBufferedFileSink {
    #[allow(dead_code)]
    config: ChannelBufferedConfig,
    template: LogTemplate,
    sender: crossbeam_channel::Sender<String>,
    receiver: crossbeam_channel::Receiver<String>,
    file: Arc<Mutex<Option<BufWriter<File>>>>,
    #[allow(dead_code)]
    file_path: PathBuf,
    inner: Mutex<Inner>,
    shutdown_flag: Arc<AtomicBool>,
    bytes_written: Arc<AtomicUsize>,
    flush_count: Arc<AtomicUsize>,
    dropped_count: Arc<AtomicUsize>,
    #[allow(dead_code)]
    last_flush: Instant,
}

impl ChannelBufferedFileSink {
    pub fn new(config: ChannelBufferedConfig, template: LogTemplate) -> Result<Self, InklogError> {
        let (sender, receiver) = crossbeam_channel::bounded(config.channel_capacity);
        let file_path = config.base_config.path.clone();
        let file = Self::open_file(&file_path)?;
        let file = Arc::new(Mutex::new(Some(BufWriter::new(file))));

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let bytes_written = Arc::new(AtomicUsize::new(0));
        let flush_count = Arc::new(AtomicUsize::new(0));
        let dropped_count = Arc::new(AtomicUsize::new(0));
        let last_flush = Instant::now();

        let sink = Self {
            config,
            template,
            sender,
            receiver,
            file,
            file_path,
            inner: Mutex::new(Inner {
                io_thread: None,
                flush_thread: None,
            }),
            shutdown_flag,
            bytes_written,
            flush_count,
            dropped_count,
            last_flush,
        };

        sink.start_io_thread();
        sink.start_flush_thread();

        Ok(sink)
    }

    fn open_file(path: &PathBuf) -> Result<File, InklogError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(InklogError::IoError)?;
            }
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(InklogError::IoError)
    }

    fn start_io_thread(&self) {
        let receiver = self.receiver.clone();
        let file = self.file.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let bytes_written = self.bytes_written.clone();
        let batch_size = self.config.flush_batch_size;

        let handle = thread::spawn(move || {
            #[allow(clippy::await_holding_lock)]
            let mut batch = Vec::with_capacity(batch_size);

            loop {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }

                batch.clear();
                let mut recv_count = 0;

                for _ in 0..batch_size {
                    match receiver.recv_timeout(StdDuration::from_millis(10)) {
                        Ok(entry) => {
                            batch.push(entry);
                            recv_count += 1;
                        }
                        Err(_) => break,
                    }
                }

                if recv_count == 0 {
                    continue;
                }

                let mut file_guard = file.lock();
                if let Some(writer) = file_guard.as_mut() {
                    for entry in &batch {
                        if let Err(e) = writer.write_all(entry.as_bytes()) {
                            eprintln!("ChannelBufferedFileSink: Write error: {}", e);
                        } else {
                            bytes_written.fetch_add(entry.len(), Ordering::Relaxed);
                        }
                    }
                    let _ = writer.flush();
                }
            }

            // Drain remaining messages
            while let Ok(entry) = receiver.recv_timeout(StdDuration::from_millis(100)) {
                let mut file_guard = file.lock();
                if let Some(writer) = file_guard.as_mut() {
                    let _ = writer.write_all(entry.as_bytes());
                }
            }

            // Final flush
            let mut file_guard = file.lock();
            if let Some(writer) = file_guard.as_mut() {
                let _ = writer.flush();
            }
        });

        self.inner.lock().io_thread = Some(handle);
    }

    fn start_flush_thread(&self) {
        let file = self.file.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let interval_ms = self.config.flush_interval_ms;
        let flush_count = self.flush_count.clone();

        let handle = thread::spawn(move || {
            loop {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(StdDuration::from_millis(interval_ms));
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                let mut file_guard = file.lock();
                if let Some(writer) = file_guard.as_mut() {
                    let _ = writer.flush();
                    flush_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        self.inner.lock().flush_thread = Some(handle);
    }

    fn try_write(&self, record: &LogRecord) -> bool {
        let entry = self.template.render(record);
        match self.config.backpressure_strategy {
            BackpressureStrategy::Block => match self.sender.send(entry) {
                Ok(()) => true,
                Err(_) => {
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    false
                }
            },
            BackpressureStrategy::DropNewest => match self.sender.try_send(entry) {
                Ok(()) => true,
                Err(crossbeam_channel::TrySendError::Full(_))
                | Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    false
                }
            },
            BackpressureStrategy::DropOldest => match self.sender.try_send(entry) {
                Ok(()) => true,
                Err(crossbeam_channel::TrySendError::Full(entry)) => {
                    if self.receiver.try_recv().is_ok() {
                        self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    }
                    match self.sender.try_send(entry) {
                        Ok(()) => true,
                        Err(_) => {
                            self.dropped_count.fetch_add(1, Ordering::Relaxed);
                            false
                        }
                    }
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    false
                }
            },
        }
    }

    pub fn metrics(&self) -> ChannelBufferedMetrics {
        ChannelBufferedMetrics {
            channel_capacity: self.config.channel_capacity,
            channel_len: self.sender.len(),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            flush_count: self.flush_count.load(Ordering::Relaxed),
            dropped_count: self.dropped_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelBufferedMetrics {
    pub channel_capacity: usize,
    pub channel_len: usize,
    pub bytes_written: usize,
    pub flush_count: usize,
    pub dropped_count: usize,
}

impl ChannelBufferedFileSink {
    /// 同步 flush 内部实现（供 async flush 和 Drop 共用，避免 Drop 调用 async 方法）
    fn flush_sync(&self) -> Result<(), InklogError> {
        let mut file_guard = self.file.lock();
        if let Some(writer) = file_guard.as_mut() {
            writer.flush()?;
        }
        self.flush_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 同步 shutdown 内部实现（供 async shutdown 和 Drop 共用，避免 Drop 调用 async 方法）
    fn shutdown_inner(&self) -> Result<(), InklogError> {
        // Signal threads to stop
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // Join threads to ensure all pending writes are completed
        {
            let mut inner = self.inner.lock();
            if let Some(handle) = inner.io_thread.take() {
                let _ = handle.join();
            }
            if let Some(handle) = inner.flush_thread.take() {
                let _ = handle.join();
            }
        }

        // Final flush
        self.flush_sync()
    }
}

#[async_trait]
impl LogSink for ChannelBufferedFileSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        self.try_write(record);
        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
        self.flush_sync()
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        self.shutdown_inner()
    }
}

impl Drop for ChannelBufferedFileSink {
    fn drop(&mut self) {
        // Ensure shutdown is called, but don't double-join threads
        // shutdown_inner() will handle joining threads if they haven't been joined yet
        let _ = self.shutdown_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_record(msg: &str) -> crate::LogRecord {
        crate::LogRecord::new(
            tracing::Level::INFO,
            "ring_test".to_string(),
            msg.to_string(),
        )
    }

    #[tokio::test]
    async fn test_write_flush_shutdown_flow() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 64,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 16,
            flush_interval_ms: 50,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        for i in 0..20 {
            let msg = format!("hello-{i}");
            let rec = make_record(&msg);
            sink.write(&rec).await.unwrap();
        }

        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(data.contains("hello-0"));
        assert!(data.contains("hello-19"));
    }

    #[tokio::test]
    async fn test_metrics_updated() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_metrics.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 8,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 4,
            flush_interval_ms: 10,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        for i in 0..6 {
            let rec = make_record(&format!("m-{i}"));
            sink.write(&rec).await.unwrap();
        }

        sink.flush().await.unwrap();

        let start = std::time::Instant::now();
        let mut m = sink.metrics();
        while m.bytes_written == 0 && start.elapsed() < std::time::Duration::from_millis(300) {
            std::thread::sleep(std::time::Duration::from_millis(10));
            m = sink.metrics();
        }

        assert!(m.bytes_written >= 1);
        assert!(m.flush_count >= 1);

        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_backpressure_drop_newest() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_drop_newest.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 2,
            backpressure_strategy: BackpressureStrategy::DropNewest,
            flush_batch_size: 2,
            flush_interval_ms: 1000,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        // Write far more messages than the channel can buffer. The IO thread
        // drains in batches of `flush_batch_size` (2) with a `flush()` syscall
        // per batch; 10_000 synchronous `try_send` calls complete in
        // microseconds and overwhelm the IO thread, guaranteeing the channel
        // fills and DropNewest kicks in.
        for i in 0..10_000 {
            let rec = make_record(&format!("drop-newest-{i}"));
            sink.write(&rec).await.unwrap();
        }

        let m = sink.metrics();
        assert!(m.dropped_count > 0);
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_backpressure_drop_oldest() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_drop_oldest.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 2,
            backpressure_strategy: BackpressureStrategy::DropOldest,
            flush_batch_size: 2,
            flush_interval_ms: 1000,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        // Same rationale as test_backpressure_drop_newest: 10_000 writes
        // overwhelm the IO thread's drain rate, guaranteeing channel overflow.
        for i in 0..10_000 {
            let rec = make_record(&format!("drop-oldest-{i}"));
            sink.write(&rec).await.unwrap();
        }

        let m = sink.metrics();
        assert!(m.dropped_count > 0);
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();
    }

    #[test]
    fn test_channel_buffered_config_default() {
        let config = ChannelBufferedConfig::default();
        assert_eq!(config.channel_capacity, 10_000);
        assert_eq!(config.flush_batch_size, 1000);
        assert_eq!(config.flush_interval_ms, 100);
        assert_eq!(config.backpressure_strategy, BackpressureStrategy::Block);
    }

    #[test]
    fn test_backpressure_strategy_default() {
        assert_eq!(BackpressureStrategy::default(), BackpressureStrategy::Block);
    }

    #[test]
    fn test_channel_buffered_metrics_default() {
        let metrics = ChannelBufferedMetrics::default();
        assert_eq!(metrics.channel_capacity, 0);
        assert_eq!(metrics.channel_len, 0);
        assert_eq!(metrics.bytes_written, 0);
        assert_eq!(metrics.flush_count, 0);
        assert_eq!(metrics.dropped_count, 0);
    }

    #[tokio::test]
    async fn test_open_file_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        // Use a nested path that doesn't exist yet
        let nested_path = dir.path().join("nested").join("subdir").join("test.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: nested_path.clone(),
                ..Default::default()
            },
            channel_capacity: 8,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 4,
            flush_interval_ms: 50,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        // Write a record to verify the file was created
        let rec = make_record("nested-dir-test");
        sink.write(&rec).await.unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        // Verify the file exists
        assert!(nested_path.exists());
    }

    #[tokio::test]
    async fn test_write_with_block_strategy_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("block_test.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 64,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 8,
            flush_interval_ms: 10,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        // Write a single record
        let rec = make_record("block-strategy-test");
        sink.write(&rec).await.unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(data.contains("block-strategy-test"));
    }

    #[tokio::test]
    async fn test_metrics_reflects_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("metrics_config.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 16,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 4,
            flush_interval_ms: 10,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        let m = sink.metrics();
        assert_eq!(m.channel_capacity, 16);

        sink.shutdown().await.unwrap();
    }

    // ========================================================================
    // try_write 错误分支覆盖（额外任务）
    // 覆盖行 225-228（Block Disconnected）、247-248（DropOldest retry 失败）、
    // 253-254（DropOldest Disconnected）、232-236（DropNewest Disconnected）
    // ========================================================================

    /// 辅助函数：创建 sink 并 shutdown，返回可变的 sink 以便替换内部字段
    async fn make_shutdown_sink(
        strategy: BackpressureStrategy,
        path: std::path::PathBuf,
    ) -> ChannelBufferedFileSink {
        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path,
                ..Default::default()
            },
            channel_capacity: 8,
            backpressure_strategy: strategy,
            flush_batch_size: 4,
            flush_interval_ms: 1000,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).expect("Failed to create sink");
        sink.shutdown().await.expect("Failed to shutdown");
        sink
    }

    #[tokio::test]
    async fn test_try_write_block_strategy_disconnected() {
        // 覆盖 Block 策略下 sender.send() 返回 Err 的分支（行 225-228）
        // send() 仅在所有 receiver 被 drop 时返回 Err
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("block_disconnected.log");
        let mut sink = make_shutdown_sink(BackpressureStrategy::Block, path).await;

        // shutdown 已 drop IO 线程的 receiver clone
        // 现在替换 sink.receiver 为另一个 channel 的 receiver，原 channel 断开
        let (_other_tx, other_rx) = crossbeam_channel::bounded::<String>(1);
        let old_receiver = std::mem::replace(&mut sink.receiver, other_rx);
        drop(old_receiver); // 显式 drop，断开原 channel

        let record = make_record("disconnected-block");
        let result = sink.try_write(&record);
        assert!(
            !result,
            "Block strategy should return false when sender is disconnected"
        );
        let m = sink.metrics();
        assert!(
            m.dropped_count > 0,
            "dropped_count should be incremented for disconnected Block, got: {}",
            m.dropped_count
        );
    }

    #[tokio::test]
    async fn test_try_write_drop_newest_strategy_disconnected() {
        // 覆盖 DropNewest 策略下 try_send 返回 Disconnected 的分支（行 232-236）
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("drop_newest_disconnected.log");
        let mut sink = make_shutdown_sink(BackpressureStrategy::DropNewest, path).await;

        let (_other_tx, other_rx) = crossbeam_channel::bounded::<String>(1);
        let old_receiver = std::mem::replace(&mut sink.receiver, other_rx);
        drop(old_receiver);

        let record = make_record("disconnected-drop-newest");
        let result = sink.try_write(&record);
        assert!(
            !result,
            "DropNewest strategy should return false when sender is disconnected"
        );
        let m = sink.metrics();
        assert!(
            m.dropped_count > 0,
            "dropped_count should be incremented for disconnected DropNewest, got: {}",
            m.dropped_count
        );
    }

    #[tokio::test]
    async fn test_try_write_drop_oldest_strategy_disconnected() {
        // 覆盖 DropOldest 策略下 try_send 返回 Disconnected 的分支（行 253-254）
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("drop_oldest_disconnected.log");
        let mut sink = make_shutdown_sink(BackpressureStrategy::DropOldest, path).await;

        let (_other_tx, other_rx) = crossbeam_channel::bounded::<String>(1);
        let old_receiver = std::mem::replace(&mut sink.receiver, other_rx);
        drop(old_receiver);

        let record = make_record("disconnected-drop-oldest");
        let result = sink.try_write(&record);
        assert!(
            !result,
            "DropOldest strategy should return false when sender is disconnected"
        );
        let m = sink.metrics();
        assert!(
            m.dropped_count > 0,
            "dropped_count should be incremented for disconnected DropOldest, got: {}",
            m.dropped_count
        );
    }

    #[tokio::test]
    async fn test_try_write_drop_oldest_retry_failure() {
        // 覆盖 DropOldest 策略下重试 try_send 失败的分支（行 247-248）
        // 使用 capacity=0 的 rendezvous channel：
        //   - try_send 总是返回 Full（无缓冲区）
        //   - try_recv 总是返回 Empty（无数据可收）
        // 因此 try_recv.is_ok() 为 false，不增加 dropped_count
        // 重试 try_send 仍返回 Full，命中 Err(_) 分支
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("drop_oldest_retry.log");
        let mut sink = make_shutdown_sink(BackpressureStrategy::DropOldest, path).await;

        // 替换为 capacity=0 的 channel
        let (new_tx, new_rx) = crossbeam_channel::bounded::<String>(0);
        let old_sender = std::mem::replace(&mut sink.sender, new_tx);
        let old_receiver = std::mem::replace(&mut sink.receiver, new_rx);
        drop(old_sender);
        drop(old_receiver);

        let record = make_record("retry-failure");
        let result = sink.try_write(&record);
        assert!(
            !result,
            "DropOldest should return false when retry try_send fails"
        );
        let m = sink.metrics();
        assert!(
            m.dropped_count > 0,
            "dropped_count should be incremented for retry failure, got: {}",
            m.dropped_count
        );
    }

    #[tokio::test]
    async fn test_try_write_drop_oldest_eviction_then_success() {
        // 覆盖 DropOldest 策略下：try_send 返回 Full → try_recv 成功 → 重试 try_send 成功
        // 即 DropOldest 的正常驱逐路径（行 240-243, 245）
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("drop_oldest_evict.log");
        let mut sink = make_shutdown_sink(BackpressureStrategy::DropOldest, path).await;

        // 替换为 capacity=1 的 channel，并预填充一条消息
        let (new_tx, new_rx) = crossbeam_channel::bounded::<String>(1);
        new_tx.send("filler".to_string()).expect("Failed to fill");
        let old_sender = std::mem::replace(&mut sink.sender, new_tx);
        let old_receiver = std::mem::replace(&mut sink.receiver, new_rx);
        drop(old_sender);
        drop(old_receiver);

        let record = make_record("after-eviction");
        let result = sink.try_write(&record);
        assert!(
            result,
            "DropOldest should succeed after evicting one item from full channel"
        );
        let m = sink.metrics();
        // try_recv 成功，dropped_count 应 +1（驱逐了一条）
        assert!(
            m.dropped_count >= 1,
            "dropped_count should be incremented for evicted item, got: {}",
            m.dropped_count
        );
    }

    #[tokio::test]
    async fn test_try_write_block_strategy_succeeds_when_connected() {
        // 对照测试：Block 策略在 channel 连接时应成功
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("block_success.log");
        let sink = make_shutdown_sink(BackpressureStrategy::Block, path).await;

        let record = make_record("block-success");
        let result = sink.try_write(&record);
        assert!(
            result,
            "Block strategy should succeed when channel is connected"
        );
        let m = sink.metrics();
        assert_eq!(m.dropped_count, 0, "no drops expected for connected Block");
    }

    // ========================================================================
    // IO 线程 write_all 错误路径（行 169）- 仅 Linux，使用 /dev/full
    // ========================================================================

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_io_thread_write_error_to_dev_full() {
        // 覆盖 IO 线程中 writer.write_all 失败的分支（行 169）
        // /dev/full 在写入时始终返回 ENOSPC
        // 需要写入 > 8KB（BufWriter 默认缓冲区大小）以触发实际 I/O
        let path = PathBuf::from("/dev/full");
        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                ..Default::default()
            },
            channel_capacity: 8,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 4,
            flush_interval_ms: 50,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).expect("Failed to create sink");

        // 写入 > 8KB 的消息，迫使 BufWriter 刷新到底层 /dev/full，触发 write_all 失败
        let large_msg = "x".repeat(10_000);
        let record = make_record(&large_msg);
        sink.write(&record).await.expect("write should not error");

        // 等待 IO 线程处理
        std::thread::sleep(std::time::Duration::from_millis(300));

        let m = sink.metrics();
        // write_all 失败，bytes_written 不应增加
        assert_eq!(
            m.bytes_written, 0,
            "write to /dev/full should fail, bytes_written should be 0, got: {}",
            m.bytes_written
        );

        // shutdown 可能因 flush 失败而返回错误，忽略
        let _ = sink.shutdown().await;
    }
}
