// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! High-performance file sink using crossbeam channels.

use crate::config::FileSinkConfig;
use crate::error::InklogError;
use crate::log_record::LogRecord;
use crate::sink::LogSink;
use crate::template::LogTemplate;
use crossbeam_channel;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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

pub struct ChannelBufferedFileSink {
    #[allow(dead_code)]
    config: ChannelBufferedConfig,
    template: LogTemplate,
    sender: crossbeam_channel::Sender<String>,
    receiver: crossbeam_channel::Receiver<String>,
    file: Arc<Mutex<Option<BufWriter<File>>>>,
    #[allow(dead_code)]
    file_path: PathBuf,
    io_thread: Option<thread::JoinHandle<()>>,
    flush_thread: Option<thread::JoinHandle<()>>,
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

    fn start_io_thread(&mut self) {
        let receiver = self.receiver.clone();
        let file = self.file.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let bytes_written = self.bytes_written.clone();
        let dropped_count = self.dropped_count.clone();
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

                if recv_count < batch_size {
                    dropped_count.fetch_add(batch_size - recv_count, Ordering::Relaxed);
                    batch.truncate(recv_count);
                }

                if let Ok(mut file_guard) = file.lock() {
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
            }

            // Drain remaining messages
            while let Ok(entry) = receiver.recv_timeout(StdDuration::from_millis(100)) {
                if let Ok(mut file_guard) = file.lock() {
                    if let Some(writer) = file_guard.as_mut() {
                        let _ = writer.write_all(entry.as_bytes());
                    }
                }
            }

            // Final flush
            if let Ok(mut file_guard) = file.lock() {
                if let Some(writer) = file_guard.as_mut() {
                    let _ = writer.flush();
                }
            }
        });

        self.io_thread = Some(handle);
    }

    fn start_flush_thread(&mut self) {
        let file = self.file.clone();
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
            if let Ok(mut file_guard) = file.lock() {
                if let Some(writer) = file_guard.as_mut() {
                    let _ = writer.flush();
                    flush_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        self.flush_thread = Some(handle);
    }

    fn try_write(&self, record: &LogRecord) -> bool {
        let entry = self.template.render(record);
        match self.sender.send(entry) {
            Ok(()) => true,
            Err(_) => {
                self.dropped_count.fetch_add(1, Ordering::Relaxed);
                false
            }
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

impl LogSink for ChannelBufferedFileSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError> {
        self.try_write(record);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), InklogError> {
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(writer) = file_guard.as_mut() {
                writer.flush()?;
            }
        }
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
        self.flush()?;
        Ok(())
    }
}

impl Drop for ChannelBufferedFileSink {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
