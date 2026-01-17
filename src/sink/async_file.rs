// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! High-performance async file sink with batch I/O.

use crate::config::FileSinkConfig;
use crate::error::InklogError;
use crate::log_record::LogRecord;
use crate::sink::{compression, LogSink};
use crate::template::LogTemplate;
use bytes::Bytes;
use crossbeam_channel;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionStrategy {
    None,
    Single,
    Batch,
}

impl Default for CompressionStrategy {
    fn default() -> Self {
        CompressionStrategy::Batch
    }
}

#[derive(Debug)]
pub struct AsyncFileSink {
    config: AsyncFileConfig,
    template: LogTemplate,
    sender: crossbeam_channel::Sender<Bytes>,
    receiver: crossbeam_channel::Receiver<Bytes>,
    file: Arc<Mutex<Option<File>>>,
    file_path: PathBuf,
    io_thread: Option<thread::JoinHandle<()>>,
    flush_thread: Option<thread::JoinHandle<()>>,
    shutdown_flag: Arc<AtomicBool>,
    bytes_written: Arc<AtomicU64>,
    flush_count: Arc<AtomicUsize>,
    dropped_count: Arc<AtomicUsize>,
    batch_count: Arc<AtomicUsize>,
}

#[derive(Debug, Clone)]
pub struct AsyncFileConfig {
    pub base_config: FileSinkConfig,
    pub channel_capacity: usize,
    pub flush_batch_size: usize,
    pub flush_interval_ms: u64,
    pub compression_strategy: CompressionStrategy,
    pub compression_level: i32,
    pub runtime_threads: usize,
}

impl Default for AsyncFileConfig {
    fn default() -> Self {
        Self {
            base_config: FileSinkConfig::default(),
            channel_capacity: 10_000,
            flush_batch_size: 1000,
            flush_interval_ms: 50,
            compression_strategy: CompressionStrategy::default(),
            compression_level: 3,
            runtime_threads: 2,
        }
    }
}

impl AsyncFileSink {
    pub fn new(config: AsyncFileConfig, template: LogTemplate) -> Result<Self, InklogError> {
        let (sender, receiver) = crossbeam_channel::bounded(config.channel_capacity);
        let file_path = config.base_config.path.clone();

        let runtime = Runtime::new().map_err(|e| InklogError::ConfigError(e.to_string()))?;
        let file = runtime.block_on(async {
            let f = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .await
                .map_err(|e| InklogError::IoError(e.into()))?;
            Ok::<File, InklogError>(f)
        })?;

        let file = Arc::new(Mutex::new(Some(file)));

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let bytes_written = Arc::new(AtomicU64::new(0));
        let flush_count = Arc::new(AtomicUsize::new(0));
        let dropped_count = Arc::new(AtomicUsize::new(0));
        let batch_count = Arc::new(AtomicUsize::new(0));

        let mut sink = Self {
            config,
            template,
            sender,
            receiver,
            file,
            file_path,
            io_thread: None,
            flush_thread: None,
            shutdown_flag,
            bytes_written,
            flush_count,
            dropped_count,
            batch_count,
        };

        sink.start_io_thread(runtime);
        sink.start_flush_thread();

        Ok(sink)
    }

    fn start_io_thread(&mut self, runtime: Runtime) {
        let receiver = self.receiver.clone();
        let file = self.file.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let bytes_written = self.bytes_written.clone();
        let dropped_count = self.dropped_count.clone();
        let batch_count = self.batch_count.clone();
        let batch_size = self.config.flush_batch_size;
        let compression = self.config.compression_strategy;
        let compression_level = self.config.compression_level;

        let handle = thread::spawn(move || {
            let rt = runtime;
            let mut batch = Vec::with_capacity(batch_size);

            rt.block_on(async move {
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

                    if recv_count < batch_size {
                        dropped_count.fetch_add(batch_size - recv_count, Ordering::Relaxed);
                        batch.truncate(recv_count);
                    }

                    let write_result =
                        if compression == CompressionStrategy::Batch && batch.len() > 1 {
                            Self::batch_compress_and_write(
                                &batch,
                                compression_level,
                                &file,
                                &bytes_written,
                            )
                            .await
                        } else {
                            Self::write_batch(&batch, &file, &bytes_written).await
                        };

                    if write_result.is_ok() {
                        batch_count.fetch_add(1, Ordering::Relaxed);
                    }
                }

                loop {
                    match receiver.recv_timeout(StdDuration::from_millis(100)) {
                        Ok(entry) => {
                            let _ = Self::write_single(&entry, &file, &bytes_written).await;
                        }
                        Err(_) => break,
                    }
                }

                if let Ok(mut file_guard) = file.lock() {
                    if let Some(f) = file_guard.as_mut() {
                        let _ = f.flush().await;
                    }
                }
            });
        });

        self.io_thread = Some(handle);
    }

    async fn write_batch(
        batch: &[Bytes],
        file: &Arc<Mutex<Option<File>>>,
        bytes_written: &Arc<AtomicU64>,
    ) -> Result<(), std::io::Error> {
        if let Ok(mut file_guard) = file.lock() {
            if let Some(f) = file_guard.as_mut() {
                for entry in batch {
                    f.write_all(entry).await?;
                    bytes_written.fetch_add(entry.len() as u64, Ordering::Relaxed);
                }
                f.flush().await?;
            }
        }
        Ok(())
    }

    async fn write_single(
        entry: &Bytes,
        file: &Arc<Mutex<Option<File>>>,
        bytes_written: &Arc<AtomicU64>,
    ) -> Result<(), std::io::Error> {
        if let Ok(mut file_guard) = file.lock() {
            if let Some(f) = file_guard.as_mut() {
                f.write_all(entry).await?;
                bytes_written.fetch_add(entry.len() as u64, Ordering::Relaxed);
                f.flush().await?;
            }
        }
        Ok(())
    }

    async fn batch_compress_and_write(
        batch: &[Bytes],
        level: i32,
        file: &Arc<Mutex<Option<File>>>,
        bytes_written: &Arc<AtomicU64>,
    ) -> Result<(), std::io::Error> {
        let total_size: usize = batch.iter().map(|b| b.len()).sum();
        let mut combined = String::with_capacity(total_size);
        for entry in batch {
            if let Ok(s) = std::str::from_utf8(entry) {
                combined.push_str(s);
                combined.push('\n');
            }
        }

        let compressed = compression::compress_data(combined.as_bytes(), level)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if let Ok(mut file_guard) = file.lock() {
            if let Some(f) = file_guard.as_mut() {
                f.write_all(&compressed).await?;
                bytes_written.fetch_add(compressed.len() as u64, Ordering::Relaxed);
                f.flush().await?;
            }
        }

        Ok(())
    }

    fn start_flush_thread(&mut self) {
        let shutdown_flag = self.shutdown_flag.clone();
        let interval_ms = self.config.flush_interval_ms;
        let flush_count = self.flush_count.clone();

        let handle = thread::spawn(move || loop {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(StdDuration::from_millis(interval_ms));
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            flush_count.fetch_add(1, Ordering::Relaxed);
        });

        self.flush_thread = Some(handle);
    }

    fn try_write(&self, record: &LogRecord) -> bool {
        let rendered = self.template.render(record);
        let bytes = Bytes::from(rendered);

        match self.sender.send(bytes) {
            Ok(()) => true,
            Err(_) => {
                self.dropped_count.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn metrics(&self) -> AsyncFileMetrics {
        AsyncFileMetrics {
            channel_capacity: self.config.channel_capacity,
            channel_len: self.sender.len(),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            flush_count: self.flush_count.load(Ordering::Relaxed),
            dropped_count: self.dropped_count.load(Ordering::Relaxed),
            batch_count: self.batch_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AsyncFileMetrics {
    pub channel_capacity: usize,
    pub channel_len: usize,
    pub bytes_written: u64,
    pub flush_count: usize,
    pub dropped_count: usize,
    pub batch_count: usize,
}

impl LogSink for AsyncFileSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError> {
        self.try_write(record);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), InklogError> {
        self.flush_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), InklogError> {
        self.shutdown_flag.store(true, Ordering::Relaxed);

        if let Some(handle) = self.io_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.flush_thread.take() {
            let _ = handle.join();
        }

        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(f) = file_guard.as_mut() {
                // 使用 tokio 的 spawn_blocking 在同步上下文中执行文件同步
                // 这样可以避免在 Drop 中创建新的 tokio 运行时
                let rt = tokio::runtime::Handle::current();
                let sync_result = rt.block_on(async { f.sync_all().await });
                if let Err(e) = sync_result {
                    tracing::error!("Failed to sync file on drop: {}", e);
                }
            }
        }

        Ok(())
    }
}

impl Drop for AsyncFileSink {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
