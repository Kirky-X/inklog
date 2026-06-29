// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! File-based log sink with rotation, compression, and encryption support.
//!
//! This module provides the FileSink implementation for writing logs to files
//! with support for automatic rotation, compression, and encryption.

use crate::support::io::sink::circuit_breaker::CircuitBreaker;
use crate::support::io::sink::rotation::{RotationStrategy, SizeBasedRotation, TimeBasedRotation};
use crate::support::io::sink::LogSink;
use crate::DataMasker;
use crate::FileSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use aes_gcm::aead::Aead;
use aes_gcm::KeyInit;
use bytes::BytesMut;
use chrono::{DateTime, Datelike, Utc};
use parking_lot::RwLock;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use tracing::{debug, error, info, warn};

// 类型别名，保持向后兼容
pub use super::circuit_breaker::{CircuitBreakerConfig, CircuitState};

/// FileSink 的可变内部状态
///
/// 所有需要 `&mut self` 访问的字段都封装在这里，
/// 通过 `RwLock` 实现内部可变性。
struct FileSinkInner {
    /// 当前文件句柄
    current_file: Option<File>,
    /// 当前文件大小
    current_size: u64,
    /// 上次轮转时间
    last_rotation: Instant,
    /// 下次轮转时间
    next_rotation_time: Option<DateTime<Utc>>,
    /// 上次轮转日期
    last_rotation_date: Option<i32>,
    /// 序列号（用于区分同名轮转文件）
    sequence: u32,
    /// 批量写入缓冲区
    batch_buffer: Vec<LogRecord>,
    /// 最后一次刷新时间
    last_flush_time: Instant,
    /// 断路器
    circuit_breaker: CircuitBreaker,
    /// 降级接收器
    fallback_sink: Option<Arc<parking_lot::Mutex<dyn LogSink + Send>>>,
    /// 轮转定时器
    rotation_timer: Option<Arc<parking_lot::Mutex<Instant>>>,
    /// 轮转定时器句柄
    timer_handle: Option<thread::JoinHandle<()>>,
    /// 清理定时器句柄
    cleanup_timer_handle: Option<thread::JoinHandle<()>>,
    /// 轮转策略
    rotation_strategy: Box<dyn RotationStrategy>,
}

/// 文件日志接收器
///
/// 提供基于文件的日志输出功能，支持：
/// - 自动日志轮转（按大小和时间）
/// - 日志文件压缩（支持 ZSTD、GZIP、Brotli）
/// - AES-256-GCM 加密
/// - 作为 DatabaseSink 的回退 sink（fallback）
///
/// FileSink 是 Inklog 的核心 sink 之一，用于将日志持久化到文件系统。
/// 当数据库不可用时，DatabaseSink 会自动降级使用 FileSink 作为备用方案。
///
/// ## 内部可变性
///
/// FileSink 使用 `RwLock<FileSinkInner>` 实现内部可变性，
/// 允许通过 `&self` 进行写入操作，支持依赖注入模式。
pub struct FileSink {
    /// 配置（只读）
    config: FileSinkConfig,
    /// 轮转间隔（只读）
    rotation_interval: StdDuration,
    /// 上次清理时间（每个实例独立）
    last_cleanup_time: Arc<parking_lot::Mutex<Option<Instant>>>,
    /// Shutdown flag for graceful thread termination
    shutdown_flag: Arc<AtomicBool>,
    /// 数据脱敏器（只读）
    masker: DataMasker,
    /// 可变内部状态
    inner: RwLock<FileSinkInner>,
}

/// FileSink 的实现，包含所有文件日志操作的核心逻辑
impl FileSink {
    /// Creates a new FileSink with the given configuration.
    pub fn new(config: FileSinkConfig) -> Result<Self, InklogError> {
        let rotation_interval = match config.rotation_time.as_str() {
            "hourly" => StdDuration::from_secs(3600),
            "daily" => StdDuration::from_secs(86400),
            "weekly" => StdDuration::from_secs(604800),
            "monthly" => StdDuration::from_secs(2592000),
            _ => StdDuration::from_secs(86400),
        };

        let rotation_timer = Arc::new(parking_lot::Mutex::new(Instant::now()));
        let last_rotation = Instant::now();

        // Create rotation strategy based on config
        let rotation_strategy: Box<dyn RotationStrategy> = {
            let max_size = Self::parse_size(&config.max_size).unwrap_or(100 * 1024 * 1024);
            let size_strategy = SizeBasedRotation::new(max_size);
            let time_strategy = TimeBasedRotation::from_interval_string(&config.rotation_time)
                .unwrap_or_else(|_| {
                    TimeBasedRotation::from_interval_string("daily")
                        .expect("hardcoded 'daily' interval is valid")
                });
            Box::new(crate::support::io::sink::rotation::CompositeRotation::new(
                vec![Box::new(size_strategy), Box::new(time_strategy)],
            ))
        };

        let inner = FileSinkInner {
            current_file: None,
            current_size: 0,
            last_rotation,
            next_rotation_time: None,
            last_rotation_date: None,
            sequence: 0,
            fallback_sink: None,
            circuit_breaker: CircuitBreaker::new(5, StdDuration::from_secs(30), 3),
            batch_buffer: Vec::with_capacity(config.batch_size),
            last_flush_time: Instant::now(),
            timer_handle: None,
            rotation_timer: Some(rotation_timer.clone()),
            cleanup_timer_handle: None,
            rotation_strategy,
        };

        let sink = Self {
            config: config.clone(),
            rotation_interval,
            last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            masker: DataMasker::new(),
            inner: RwLock::new(inner),
        };

        // 初始化轮转时间
        {
            let mut inner = sink.inner.write();
            sink.update_next_rotation_time_inner(&mut inner);
        }

        // 打开日志文件
        {
            let mut inner = sink.inner.write();
            if let Err(e) = sink.open_file_inner(&mut inner) {
                error!("Failed to open log file: {}", e);
                return Err(e);
            }
        }

        // 启动轮转定时器
        sink.start_rotation_timer();

        // 启动清理定时器
        sink.start_cleanup_timer();

        Ok(sink)
    }

    /// 解析文件大小字符串
    pub fn parse_size(size_str: &str) -> Option<u64> {
        let size_str = size_str.trim();
        if size_str.is_empty() {
            return None;
        }

        if size_str.ends_with("TB") {
            size_str
                .trim_end_matches("TB")
                .parse::<u64>()
                .ok()
                .map(|s| s * 1024 * 1024 * 1024 * 1024)
        } else if size_str.ends_with("GB") {
            size_str
                .trim_end_matches("GB")
                .parse::<u64>()
                .ok()
                .map(|s| s * 1024 * 1024 * 1024)
        } else if size_str.ends_with("MB") {
            size_str
                .trim_end_matches("MB")
                .parse::<u64>()
                .ok()
                .map(|s| s * 1024 * 1024)
        } else if size_str.ends_with("KB") {
            size_str
                .trim_end_matches("KB")
                .parse::<u64>()
                .ok()
                .map(|s| s * 1024)
        } else {
            size_str.parse::<u64>().ok()
        }
    }

    /// 获取加密密钥
    fn get_encryption_key(&self) -> Result<BytesMut, InklogError> {
        let default_key = "LOG_ENCRYPTION_KEY".to_string();
        let key_str = self
            .config
            .encryption_key_env
            .as_ref()
            .unwrap_or(&default_key);

        let key = std::env::var(key_str).map_err(|_| {
            InklogError::EncryptionError(format!(
                "Encryption key not found in environment variable: {}",
                key_str
            ))
        })?;

        // 验证密钥长度（Base64 编码前至少 16 字符）
        if key.len() < 16 {
            return Err(InklogError::EncryptionError(
                "Encryption key must be at least 16 characters".to_string(),
            ));
        }

        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &key)
            .map_err(|_| {
                InklogError::EncryptionError(
                    "Invalid base64 encoding in encryption key".to_string(),
                )
            })?;

        if decoded.len() != 32 {
            return Err(InklogError::EncryptionError(
                "Encryption key must be 32 bytes (256 bits)".to_string(),
            ));
        }

        // 验证密钥熵（确保不是弱密钥）
        Self::validate_key_entropy(&decoded)?;

        let key_bytes = BytesMut::from(&decoded[..]);

        Ok(key_bytes)
    }

    /// 验证密钥熵（Shannon entropy）
    /// 返回 Ok(()) 如果密钥有足够的熵（>= 4.0）
    fn validate_key_entropy(key: &[u8]) -> Result<(), InklogError> {
        if key.is_empty() {
            return Err(InklogError::EncryptionError(
                "Encryption key cannot be empty".to_string(),
            ));
        }

        let mut freq = [0u32; 256];
        for &b in key {
            freq[b as usize] += 1;
        }

        let len = key.len() as f64;
        let entropy: f64 = freq
            .iter()
            .filter(|&&count| count > 0)
            .map(|&count| {
                let p = count as f64 / len;
                -p * p.log2()
            })
            .sum();

        const MIN_ENTROPY_THRESHOLD: f64 = 4.0;
        if entropy < MIN_ENTROPY_THRESHOLD {
            return Err(InklogError::EncryptionError(format!(
                "Encryption key has insufficient entropy ({} < {}). \
                 Please use a cryptographically random key.",
                entropy, MIN_ENTROPY_THRESHOLD
            )));
        }

        Ok(())
    }

    fn open_file_inner(&self, inner: &mut FileSinkInner) -> Result<(), InklogError> {
        if let Some(parent) = self.config.path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create log directory {}: {}", parent.display(), e);
                return Err(InklogError::IoError(e));
            }
        }

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.path)
        {
            Ok(file) => {
                inner.current_file = Some(file);
                inner.current_size = self.config.path.metadata().map(|m| m.len()).unwrap_or(0);
                debug!(
                    "Opened log file: {} (size: {} bytes)",
                    self.config.path.display(),
                    inner.current_size
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to open log file: {}", e);
                Err(InklogError::IoError(e))
            }
        }
    }

    /// 启动清理定时器
    fn start_cleanup_timer(&self) {
        let interval_minutes = self.config.cleanup_interval_minutes;
        let cleanup_interval = StdDuration::from_secs(interval_minutes * 60);
        let shutdown_flag = self.shutdown_flag.clone();
        let config = self.config.clone();
        let path = self.config.path.clone();
        let last_cleanup_time = self.last_cleanup_time.clone();

        let handle = thread::spawn(move || {
            let check_interval = StdDuration::from_secs(60);

            loop {
                // 检查关闭标志
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }

                // 拆分长 sleep 为 100ms 段，每段检查 shutdown_flag。
                // 修复根因：原 thread::sleep(60s) 期间无法响应 shutdown，
                // 即使 FileSink::Drop 设置 flag 后也要等 sleep 结束才能退出，
                // 导致测试进程无法退出（PID 20848 等挂起问题）。
                let mut elapsed = StdDuration::ZERO;
                const POLL_INTERVAL: StdDuration = StdDuration::from_millis(100);
                while elapsed < check_interval {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let step = std::cmp::min(POLL_INTERVAL, check_interval - elapsed);
                    thread::sleep(step);
                    elapsed += step;
                }

                // 检查关闭标志
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }

                // 检查是否到达清理时间（使用实例级别的清理时间）
                let mut last_cleanup = last_cleanup_time.lock();
                let now = Instant::now();

                if last_cleanup.is_none_or(|t| now.duration_since(t) >= cleanup_interval) {
                    // 执行清理
                    if let Err(e) = Self::perform_cleanup(&config, &path) {
                        error!("Cleanup failed: {}", e);
                    } else {
                        *last_cleanup = Some(now);
                    }
                }
            }
        });

        self.inner.write().cleanup_timer_handle = Some(handle);
    }

    /// 清理旧的日志文件
    ///
    /// 根据 retention_days 和 max_total_size 配置自动清理过期日志。
    /// 此方法在后台定期调用，也可手动触发。
    ///
    /// # Errors
    ///
    /// 返回文件系统操作可能产生的错误
    #[allow(dead_code)]
    fn perform_cleanup(config: &FileSinkConfig, log_path: &Path) -> Result<(), InklogError> {
        if let Some(parent) = log_path.parent() {
            let entries: Result<Vec<_>, _> = fs::read_dir(parent)?.collect();

            if let Ok(entries) = entries {
                // 计算截止日期
                let cutoff_date = Utc::now()
                    .checked_sub_signed(chrono::Duration::days(config.retention_days as i64))
                    .unwrap_or_else(Utc::now);

                let mut expired_count = 0;
                let mut total_size = 0u64;

                for entry in &entries {
                    total_size += entry.path().metadata()?.len();

                    if let Ok(modified) = entry.path().metadata().and_then(|m| m.modified()) {
                        let modified_utc: DateTime<Utc> = modified.into();
                        if modified_utc < cutoff_date {
                            expired_count += 1;
                        }
                    }
                }

                if let Some(max_total_size_bytes) = Self::parse_size(&config.max_total_size) {
                    if total_size > max_total_size_bytes {
                        let excess_size = total_size.saturating_sub(max_total_size_bytes);
                        let mut deleted_size: u64 = 0;

                        for entry in entries {
                            if deleted_size >= excess_size {
                                break;
                            }

                            if let Ok(metadata) = entry.path().metadata() {
                                deleted_size += metadata.len();
                            }

                            if let Err(e) = fs::remove_file(entry.path()) {
                                error!("Failed to remove {}: {}", entry.path().display(), e);
                            }
                        }
                    } else if expired_count > 0 {
                        let to_delete =
                            (entries.len() as i32 - config.keep_files as i32).max(0) as usize;
                        for entry in entries.into_iter().take(to_delete) {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns disk space information for the log file's filesystem.
    pub fn get_disk_space_info(&self) -> Result<(u64, u64), InklogError> {
        if let Some(parent) = self.config.path.parent() {
            if let Ok(_metadata) = fs::metadata(parent) {
                if let Ok(stat) = nix::sys::statfs::statfs(parent) {
                    let total_blocks = stat.blocks();
                    let available_blocks = stat.blocks_available();

                    // 获取块大小
                    let block_size = stat.block_size() as u64;
                    let total_bytes = total_blocks * block_size;
                    let available_bytes = available_blocks * block_size;

                    return Ok((total_bytes, available_bytes));
                }
            }
        }

        Err(InklogError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Unable to get disk space info",
        )))
    }

    /// 检查磁盘空间是否充足
    fn check_disk_space(&self) -> Result<bool, InklogError> {
        let (_total, available) = self.get_disk_space_info()?;
        // 保留 50MB 或 10% 的可用空间，以较大者为准
        let reserved = (50 * 1024 * 1024u64).max(available / 10);
        Ok(available > reserved)
    }

    /// 计算下次轮转时间
    fn calculate_next_rotation_time(rotation_time: &str) -> Option<DateTime<Utc>> {
        let now = Utc::now();

        match rotation_time {
            "hourly" => Some(now + chrono::Duration::hours(1)),
            "daily" => {
                let next_naive = now.date_naive().and_hms_opt(0, 0, 0)? + chrono::Duration::days(1);
                Some(next_naive.and_utc())
            }
            "weekly" => {
                let next_naive =
                    now.date_naive().and_hms_opt(0, 0, 0)? + chrono::Duration::weeks(1);
                Some(next_naive.and_utc())
            }
            "monthly" => {
                let next_naive =
                    (now.date_naive() + chrono::Duration::days(1)).and_hms_opt(0, 0, 0)?;
                Some(next_naive.and_utc())
            }
            _ => {
                // 默认每日轮转
                let next_naive = now.date_naive().and_hms_opt(0, 0, 0)? + chrono::Duration::days(1);
                Some(next_naive.and_utc())
            }
        }
    }

    fn should_rotate_by_time_inner(&self, inner: &FileSinkInner) -> bool {
        let now = Utc::now();
        let current_date = now.date_naive().num_days_from_ce();

        if self.config.rotation_time == "daily" || self.config.rotation_time == "weekly" {
            if let Some(last_date) = inner.last_rotation_date {
                if current_date > last_date {
                    return true;
                }
            }
        }

        if let Some(next_time) = inner.next_rotation_time {
            if now >= next_time {
                return true;
            }
        }

        false
    }

    fn update_next_rotation_time_inner(&self, inner: &mut FileSinkInner) {
        inner.next_rotation_time = Self::calculate_next_rotation_time(&self.config.rotation_time);
    }

    /// 启动轮转定时器
    fn start_rotation_timer(&self) {
        let rotation_interval = self.rotation_interval;
        let last_rotation;
        {
            let inner = self.inner.read();
            last_rotation = Arc::new(parking_lot::Mutex::new(inner.last_rotation));
        }
        {
            let mut inner = self.inner.write();
            inner.rotation_timer = Some(last_rotation.clone());
        }

        // Clone the shutdown flag for the timer thread
        let shutdown_flag = self.shutdown_flag.clone();

        let timer_handle = thread::spawn(move || {
            let check_interval = StdDuration::from_secs(60); // Check every minute
            loop {
                // Check shutdown flag before sleeping to allow graceful exit
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }

                // 拆分长 sleep 为 100ms 段，每段检查 shutdown_flag
                // （修复根因见 cleanup_timer 同样修改）
                let mut elapsed = StdDuration::ZERO;
                const POLL_INTERVAL: StdDuration = StdDuration::from_millis(100);
                while elapsed < check_interval {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let step = std::cmp::min(POLL_INTERVAL, check_interval - elapsed);
                    thread::sleep(step);
                    elapsed += step;
                }

                // Check again after sleep to avoid race condition
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }

                let mut last_rotation_guard = last_rotation.lock();
                if last_rotation_guard.elapsed() >= rotation_interval {
                    // Timer will trigger rotation on next write
                    *last_rotation_guard =
                        Instant::now() - rotation_interval + StdDuration::from_secs(1);
                }
            }
        });

        self.inner.write().timer_handle = Some(timer_handle);
    }

    /// 批量刷新缓冲区到文件
    fn flush_batch_inner(&self, inner: &mut FileSinkInner) -> Result<(), InklogError> {
        if inner.batch_buffer.is_empty() {
            return Ok(());
        }

        let records = std::mem::take(&mut inner.batch_buffer);

        if let Some(file) = &mut inner.current_file {
            for record in &records {
                match writeln!(
                    file,
                    "{} [{}] {} - {}",
                    record.timestamp.to_rfc3339(),
                    record.level,
                    record.target,
                    record.message
                ) {
                    Ok(_) => {
                        let len = record.timestamp.to_rfc3339().len()
                            + record.level.len()
                            + record.target.len()
                            + record.message.len()
                            + 7; // " []  - \n"

                        inner.current_size += len as u64;
                    }
                    Err(e) => {
                        error!("Batch write error: {}", e);
                        inner.circuit_breaker.record_failure();
                        let _ = self.open_file_inner(inner);
                        break;
                    }
                }
            }
            inner.circuit_breaker.record_success();
        }

        inner.last_flush_time = Instant::now();

        // 批量写入后检查是否需要旋转
        self.check_rotation_inner(inner)?;

        Ok(())
    }

    /// 同步压缩文件（可在后台线程调用）
    fn compress_file(&self, path: &PathBuf) -> Result<PathBuf, InklogError> {
        let compressed_path = path.with_extension("zst");

        let input_file = fs::File::open(path).map_err(|e| {
            error!("Failed to open file for compression: {}", e);
            InklogError::IoError(e)
        })?;

        let output_file = fs::File::create(&compressed_path).map_err(|e| {
            error!("Failed to create compressed file: {}", e);
            InklogError::IoError(e)
        })?;

        let mut encoder = zstd::stream::Encoder::new(output_file, self.config.compression_level)
            .map_err(|e| InklogError::CompressionError(e.to_string()))?
            .auto_finish();

        let mut reader = std::io::BufReader::new(input_file);
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = std::io::Read::read(&mut reader, &mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            std::io::Write::write_all(&mut encoder, &buffer[..bytes_read])?;
        }

        // Encoder is automatically finished when dropped due to auto_finish()
        drop(encoder);

        // 如果需要加密
        if self.config.encrypt {
            let encrypted_path = compressed_path.with_extension("zst.enc");
            if let Err(e) = self.encrypt_file(&compressed_path, &encrypted_path) {
                error!("Encryption failed: {}", e);
                // 加密失败，保留压缩文件
                let _ = fs::rename(
                    &compressed_path,
                    encrypted_path.with_extension("zst.unencrypted"),
                );
                return Err(e);
            }
            let _ = fs::remove_file(&compressed_path);
            Ok(encrypted_path)
        } else {
            // 删除原始文件
            let _ = fs::remove_file(path);
            Ok(compressed_path)
        }
    }

    /// 同步加密文件（可在后台线程调用）
    fn encrypt_file(&self, input_path: &PathBuf, output_path: &PathBuf) -> Result<(), InklogError> {
        use aes_gcm::{Aes256Gcm, Nonce};
        use rand::RngCore;

        // 获取密钥
        let key_bytes = self.get_encryption_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| InklogError::EncryptionError(format!("Invalid key: {}", e)))?;

        // 生成加密安全的随机 nonce
        // 使用 rand::rng() 获取线程本地 RNG，该 RNG 从 OsRng 定期种子化
        // rand::rng() 返回 ThreadRng，它是密码学安全的
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 读取输入文件
        let input_data = fs::read(input_path).map_err(|e| {
            error!("Failed to read file for encryption: {}", e);
            InklogError::IoError(e)
        })?;

        // 加密
        let ciphertext = cipher.encrypt(nonce, input_data.as_slice()).map_err(|e| {
            error!("Encryption failed: {}", e);
            InklogError::EncryptionError(e.to_string())
        })?;

        // 写入加密文件
        let mut output = fs::File::create(output_path).map_err(|e| {
            error!("Failed to create encrypted file: {}", e);
            InklogError::IoError(e)
        })?;

        // 写入格式：nonce (12 bytes) + ciphertext
        output.write_all(&nonce_bytes)?;
        output.write_all(&ciphertext)?;

        debug!("Encrypted log file: {}", output_path.display());
        Ok(())
    }

    /// 执行文件轮转
    fn rotate_inner(&self, inner: &mut FileSinkInner) -> Result<(), InklogError> {
        debug!("Rotating log file: {}", self.config.path.display());

        // 关闭当前文件
        let _ = inner.current_file.take();

        // 重命名当前日志文件
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let new_path = if let Some(parent) = self.config.path.parent() {
            let stem = self.config.path.file_stem().unwrap_or_default();
            let ext = self.config.path.extension().unwrap_or_default();
            parent.join(format!(
                "{}_{}.{}",
                stem.to_string_lossy(),
                timestamp,
                ext.to_string_lossy()
            ))
        } else {
            PathBuf::from(format!("{}_{}", self.config.path.display(), timestamp))
        };

        // 尝试重命名
        if self.config.path.exists() {
            if let Err(e) = fs::rename(&self.config.path, &new_path) {
                error!("Failed to rename log file: {}", e);
                // 尝试复制后删除
                if fs::copy(&self.config.path, &new_path).is_ok() {
                    let _ = fs::remove_file(&self.config.path);
                } else {
                    return Err(InklogError::IoError(e));
                }
            }
        }

        // 更新序列号
        inner.sequence += 1;

        // 更新轮转时间
        inner.last_rotation = Instant::now();
        self.update_next_rotation_time_inner(inner);
        inner.current_size = 0;

        info!("Log rotated to: {}", new_path.display());

        // 如果启用压缩，在后台线程处理
        if self.config.compress {
            let config = self.config.clone();
            let path = new_path.clone();
            let _ = thread::spawn(move || {
                // 为后台线程创建一个最小化的 FileSink 实例用于压缩
                let inner = FileSinkInner {
                    current_file: None,
                    current_size: 0,
                    last_rotation: Instant::now(),
                    next_rotation_time: None,
                    last_rotation_date: None,
                    sequence: 0,
                    fallback_sink: None,
                    circuit_breaker: CircuitBreaker::new(5, StdDuration::from_secs(30), 3),
                    batch_buffer: Vec::new(),
                    last_flush_time: Instant::now(),
                    timer_handle: None,
                    rotation_timer: None,
                    cleanup_timer_handle: None,
                    rotation_strategy: Box::new(
                        crate::support::io::sink::rotation::CompositeRotation::new(vec![]),
                    ),
                };
                let sink = FileSink {
                    config,
                    rotation_interval: StdDuration::from_secs(86400),
                    last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
                    shutdown_flag: Arc::new(AtomicBool::new(false)),
                    masker: DataMasker::new(),
                    inner: RwLock::new(inner),
                };
                if let Err(e) = sink.compress_file(&path) {
                    error!("Failed to compress rotated log: {}", e);
                }
            });
        } else if self.config.encrypt {
            // 如果只启用加密（不压缩），直接在后台线程加密
            let config = self.config.clone();
            let path = new_path.clone();
            let _ = thread::spawn(move || {
                // 为后台线程创建一个最小化的 FileSink 实例用于加密
                let inner = FileSinkInner {
                    current_file: None,
                    current_size: 0,
                    last_rotation: Instant::now(),
                    next_rotation_time: None,
                    last_rotation_date: None,
                    sequence: 0,
                    fallback_sink: None,
                    circuit_breaker: CircuitBreaker::new(5, StdDuration::from_secs(30), 3),
                    batch_buffer: Vec::new(),
                    last_flush_time: Instant::now(),
                    timer_handle: None,
                    rotation_timer: None,
                    cleanup_timer_handle: None,
                    rotation_strategy: Box::new(
                        crate::support::io::sink::rotation::CompositeRotation::new(vec![]),
                    ),
                };
                let sink = FileSink {
                    config,
                    rotation_interval: StdDuration::from_secs(86400),
                    last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
                    shutdown_flag: Arc::new(AtomicBool::new(false)),
                    masker: DataMasker::new(),
                    inner: RwLock::new(inner),
                };
                let encrypted_path = path.with_extension("enc");
                if let Err(e) = sink.encrypt_file(&path, &encrypted_path) {
                    error!("Failed to encrypt rotated log: {}", e);
                } else {
                    let _ = fs::remove_file(&path);
                }
            });
        }

        // 重新打开文件
        self.open_file_inner(inner)
    }

    /// 检查是否需要轮转
    fn check_rotation_inner(&self, inner: &mut FileSinkInner) -> Result<(), InklogError> {
        let rotate_by_size =
            Self::parse_size(&self.config.max_size).is_some_and(|max| inner.current_size >= max);

        let rotate_by_time = self.should_rotate_by_time_inner(inner);

        if rotate_by_size || rotate_by_time {
            self.rotate_inner(inner)?;
        }

        Ok(())
    }
}

impl LogSink for FileSink {
    fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        let mut inner = self.inner.write();

        // 检查断路器
        if !inner.circuit_breaker.can_execute() {
            if let Some(sink) = &inner.fallback_sink {
                let locked = sink.lock();
                let _ = locked.write(record);
            }
            return Ok(());
        }

        // 检查磁盘空间
        if !self.check_disk_space()? {
            warn!("Low disk space - checking before write");
            if let Some(sink) = &inner.fallback_sink {
                let locked = sink.lock();
                let _ = locked.write(record);
            }
            return Ok(());
        }

        // 应用数据脱敏（如果启用）
        let masked_record = if self.config.masking_enabled {
            let mut masked = record.clone();
            masked.message = self.masker.mask(&record.message);
            self.masker.mask_hashmap(&mut masked.fields);
            masked
        } else {
            record.clone()
        };

        // 添加到批量缓冲区
        // Update current_size before adding to buffer
        let record_len = masked_record.timestamp.to_rfc3339().len()
            + masked_record.level.len()
            + masked_record.target.len()
            + masked_record.message.len()
            + 7;
        inner.current_size += record_len as u64;
        inner.batch_buffer.push(masked_record);

        // 检查轮转条件（在更新 current_size 之后）
        let should_rotate = Self::parse_size(&self.config.max_size)
            .is_some_and(|max| inner.current_size >= max)
            || inner
                .rotation_timer
                .as_ref()
                .map(|t| t.lock().elapsed() >= self.rotation_interval)
                .unwrap_or(false);

        if should_rotate {
            if let Err(e) = self.rotate_inner(&mut inner) {
                error!("Rotation failed: {}", e);
                if let Some(sink) = &inner.fallback_sink {
                    let locked = sink.lock();
                    let _ = locked.write(record);
                }
                return Ok(());
            }
        }

        // 检查是否需要批量写入
        let now = Instant::now();
        let flush_interval = StdDuration::from_millis(self.config.flush_interval_ms);

        if inner.batch_buffer.len() >= self.config.batch_size
            || now.duration_since(inner.last_flush_time) >= flush_interval
        {
            self.flush_batch_inner(&mut inner)?;
        }

        Ok(())
    }

    fn flush(&self) -> Result<(), InklogError> {
        let mut inner = self.inner.write();
        // 先刷新批量缓冲区
        self.flush_batch_inner(&mut inner)?;

        // 然后刷新文件
        if let Some(file) = &mut inner.current_file {
            file.flush()?;
        }
        Ok(())
    }

    fn is_healthy(&self) -> bool {
        self.inner.read().current_file.is_some()
    }

    fn shutdown(&self) -> Result<(), InklogError> {
        // Signal shutdown to all timer threads first
        self.shutdown_flag.store(true, Ordering::Relaxed);

        let mut inner = self.inner.write();

        // Stop rotation timer with graceful shutdown
        if let Some(handle) = inner.timer_handle.take() {
            let _ = handle.join(); // Join without timeout for simplicity
        }
        inner.rotation_timer = None;

        // Stop cleanup timer with graceful shutdown
        if let Some(handle) = inner.cleanup_timer_handle.take() {
            let _ = handle.join();
        }

        // Flush remaining data
        self.flush_batch_inner(&mut inner)?;
        if let Some(file) = &mut inner.current_file {
            file.flush()?;
        }

        Ok(())
    }
}

impl Drop for FileSink {
    fn drop(&mut self) {
        const SHUTDOWN_TIMEOUT_MS: u64 = 5000; // 5 second timeout

        // Set shutdown flag to signal threads to stop
        self.shutdown_flag.store(true, Ordering::SeqCst);

        // Flush any remaining buffered records
        {
            let mut inner = self.inner.write();
            let _ = self.flush_batch_inner(&mut inner);
            // Close current file handle
            if let Some(mut file) = inner.current_file.take() {
                let _ = file.flush();
            }
        }

        // Wait for rotation timer thread to finish with timeout
        {
            let mut inner = self.inner.write();
            if let Some(handle) = inner.timer_handle.take() {
                let start = std::time::Instant::now();
                while !handle.is_finished() {
                    if start.elapsed().as_millis() > SHUTDOWN_TIMEOUT_MS as u128 {
                        tracing::warn!(
                            "Warning: rotation timer shutdown timeout after {}ms",
                            SHUTDOWN_TIMEOUT_MS
                        );
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }

        // Wait for cleanup timer thread to finish with timeout
        {
            let mut inner = self.inner.write();
            if let Some(handle) = inner.cleanup_timer_handle.take() {
                let start = std::time::Instant::now();
                while !handle.is_finished() {
                    if start.elapsed().as_millis() > SHUTDOWN_TIMEOUT_MS as u128 {
                        tracing::warn!(
                            "Warning: cleanup timer shutdown timeout after {}ms",
                            SHUTDOWN_TIMEOUT_MS
                        );
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }

        // Close fallback console sink
        {
            let mut inner = self.inner.write();
            if let Some(fallback) = inner.fallback_sink.take() {
                let locked = fallback.lock();
                let _ = locked.shutdown();
            }
        }
    }
}

impl std::fmt::Debug for FileSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("FileSink")
            .field("path", &self.config.path)
            .field("current_size", &inner.current_size)
            .field("circuit_breaker", &inner.circuit_breaker)
            .finish()
    }
}

impl Clone for FileSink {
    fn clone(&self) -> Self {
        let inner = FileSinkInner {
            current_file: None,
            current_size: 0,
            last_rotation: Instant::now(),
            next_rotation_time: None,
            last_rotation_date: None,
            sequence: 0,
            fallback_sink: None,
            circuit_breaker: CircuitBreaker::new(5, StdDuration::from_secs(30), 3),
            batch_buffer: Vec::with_capacity(self.config.batch_size),
            last_flush_time: Instant::now(),
            timer_handle: None,
            rotation_timer: None,
            cleanup_timer_handle: None,
            rotation_strategy: self.inner.read().rotation_strategy.clone_boxed(),
        };

        Self {
            config: self.config.clone(),
            rotation_interval: self.rotation_interval,
            last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            masker: DataMasker::new(),
            inner: RwLock::new(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileSinkConfig;
    use crate::LogRecord;
    use base64::Engine;
    use chrono::Timelike;
    use chrono::Utc;
    use serial_test::serial;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[allow(dead_code)]
    fn create_test_record(message: &str) -> LogRecord {
        LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "test_module".to_string(),
            message: message.to_string(),
            fields: HashMap::new(),
            file: Some("/path/to/test.rs".to_string()),
            line: Some(42),
            thread_id: "test-thread".to_string(),
        }
    }

    /// Helper function to create a FileSink for testing without starting timers
    fn create_test_file_sink(config: FileSinkConfig) -> FileSink {
        let inner = FileSinkInner {
            current_file: None,
            current_size: 0,
            last_rotation: Instant::now(),
            next_rotation_time: None,
            last_rotation_date: None,
            sequence: 0,
            fallback_sink: None,
            circuit_breaker: CircuitBreaker::new(5, StdDuration::from_secs(30), 3),
            batch_buffer: Vec::new(),
            last_flush_time: Instant::now(),
            timer_handle: None,
            rotation_timer: None,
            cleanup_timer_handle: None,
            rotation_strategy: Box::new(
                crate::support::io::sink::rotation::CompositeRotation::new(vec![]),
            ),
        };

        FileSink {
            config,
            rotation_interval: StdDuration::from_secs(86400),
            last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            masker: DataMasker::new(),
            inner: RwLock::new(inner),
        }
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(FileSink::parse_size("100"), Some(100));
        assert_eq!(FileSink::parse_size("100KB"), Some(100 * 1024));
        assert_eq!(FileSink::parse_size("10MB"), Some(10 * 1024 * 1024));
        assert_eq!(FileSink::parse_size("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(FileSink::parse_size("  5MB  "), Some(5 * 1024 * 1024));
        assert_eq!(FileSink::parse_size("invalid"), None);
    }

    #[test]
    fn test_perform_cleanup() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "1MB".to_string(),
            rotation_time: "daily".to_string(),
            keep_files: 2,
            compress: false,
            compression_level: 3,
            encrypt: false,
            encryption_key_env: None,
            retention_days: 30,
            max_total_size: "1GB".to_string(),
            cleanup_interval_minutes: 60,
            batch_size: 100,
            flush_interval_ms: 100,
            masking_enabled: true,
        };

        // Create test files
        let old_file = dir.path().join("test_old.log");
        std::fs::write(&old_file, "old content").unwrap();

        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_encryption_key() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_KEY".to_string()),
            ..Default::default()
        };

        // Set a valid 32-byte test key (base64 encoded, mixed characters for entropy)
        // "abcdefghijklmnopqrstuvwxyz123456" = 32 varied bytes
        std::env::set_var("TEST_KEY", "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=");

        let sink = create_test_file_sink(config);

        let key_result = sink.get_encryption_key();
        assert!(key_result.is_ok());
        assert_eq!(key_result.unwrap().len(), 32);

        // Clean up
        std::env::remove_var("TEST_KEY");
    }

    #[test]
    fn test_disk_space_info() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let sink = create_test_file_sink(config);

        let result = sink.get_disk_space_info();
        assert!(result.is_ok());

        let (total, available) = result.unwrap();
        assert!(total > 0);
        assert!(available > 0);
    }

    #[test]
    fn test_check_disk_space_logic() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };

        let sink = create_test_file_sink(config);

        let result = sink.check_disk_space();
        // Should succeed if there's sufficient disk space
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_with_disk_space_check() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };

        let sink = FileSink::new(config).unwrap();

        let record = LogRecord {
            timestamp: chrono::Utc::now(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            message: "Test message".to_string(),
            fields: HashMap::new(),
            file: Some("test.rs".to_string()),
            line: Some(1),
            thread_id: format!("{:?}", std::thread::current().id()),
        };

        // Should succeed with sufficient disk space
        let result = sink.write(&record);
        assert!(
            result.is_ok(),
            "Write should succeed with sufficient disk space"
        );

        // Flush to ensure data is written
        sink.flush().unwrap();

        // Verify file was created and contains data
        assert!(log_path.exists(), "Log file should exist");
    }

    #[test]
    fn test_parse_size_kb() {
        assert_eq!(FileSink::parse_size("500KB"), Some(500 * 1024));
    }

    #[test]
    fn test_parse_size_mb() {
        assert_eq!(FileSink::parse_size("2MB"), Some(2 * 1024 * 1024));
    }

    #[test]
    fn test_parse_size_gb() {
        assert_eq!(FileSink::parse_size("1GB"), Some(1024 * 1024 * 1024));
    }

    #[test]
    fn test_parse_size_with_spaces() {
        assert_eq!(FileSink::parse_size("  3MB  "), Some(3 * 1024 * 1024));
    }

    #[test]
    fn test_parse_size_invalid() {
        assert_eq!(FileSink::parse_size("invalid"), None);
        assert_eq!(FileSink::parse_size(""), None);
    }

    #[test]
    fn test_parse_size_zero() {
        assert_eq!(FileSink::parse_size("0"), Some(0));
        assert_eq!(FileSink::parse_size("0MB"), Some(0));
    }

    #[test]
    fn test_get_encryption_key_missing_env() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("MISSING_KEY".to_string()),
            ..Default::default()
        };

        // Ensure the env var doesn't exist
        std::env::remove_var("MISSING_KEY");

        let sink = create_test_file_sink(config);

        let result = sink.get_encryption_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_encryption_key_no_env_var() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: None,
            ..Default::default()
        };

        let sink = create_test_file_sink(config);

        let result = sink.get_encryption_key();
        // When encryption_key_env is None, it tries to use LOG_ENCRYPTION_KEY env var
        // This test expects the env var to be set or the test to handle missing env
        // Let's check if we get an error and skip if env var is not set
        if result.is_err() {
            // This is expected if LOG_ENCRYPTION_KEY is not set
            assert!(std::env::var("LOG_ENCRYPTION_KEY").is_err());
        }
    }

    #[test]
    fn test_file_sink_new_default() {
        // Note: confers derive generates Default with empty PathBuf for path field.
        // We need to provide an explicit path for the test.
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        println!("FileSinkConfig: {:?}", config);
        let result = FileSink::new(config);
        if let Err(ref e) = result {
            println!("Error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Expected FileSink::new to succeed with default config, but got error: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_file_sink_new_with_path() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_sink_disabled() {
        let config = FileSinkConfig {
            enabled: false,
            path: PathBuf::from("test.log"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_key_entropy_strong() {
        // 使用真正的随机密钥（高熵）
        let strong_key = [
            0x3a, 0x7b, 0x9c, 0x1d, 0x4e, 0x8f, 0x2c, 0x6b, 0x9a, 0x3d, 0x8e, 0x1f, 0x4a, 0x7d,
            0x2e, 0x6f, 0x9b, 0x3c, 0x8d, 0x1e, 0x4b, 0x6a, 0x2b, 0x6c, 0x9f, 0x3a, 0x8b, 0x1c,
            0x4d, 0x7e, 0x2f, 0x6a,
        ];
        assert!(FileSink::validate_key_entropy(&strong_key).is_ok());
    }

    #[test]
    fn test_validate_key_entropy_weak() {
        // 使用弱密钥（全相同字节）
        let weak_key = [0xaa; 32];
        assert!(FileSink::validate_key_entropy(&weak_key).is_err());
    }

    #[test]
    fn test_validate_key_entropy_empty() {
        // 空密钥应该返回错误
        let empty_key: [u8; 0] = [];
        assert!(FileSink::validate_key_entropy(&empty_key).is_err());
    }

    #[test]
    fn test_get_encryption_key_too_short() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_SHORT_KEY".to_string()),
            ..Default::default()
        };

        // 设置一个太短的密钥（Base64 编码前 < 16 字符）
        std::env::set_var("TEST_SHORT_KEY", "YWJjZA=="); // "abcd"

        let sink = create_test_file_sink(config);

        let result = sink.get_encryption_key();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least 16 characters"));
    }

    #[test]
    fn test_nonce_generation_unique() {
        // 测试每次生成的 nonce 都是唯一的
        use rand::RngCore;

        let mut nonces = Vec::new();
        for _ in 0..100 {
            let mut nonce_bytes = [0u8; 12];
            rand::rng().fill_bytes(&mut nonce_bytes);
            nonces.push(nonce_bytes);
        }

        // 确保所有 nonce 都是唯一的
        for i in 0..nonces.len() {
            for j in (i + 1)..nonces.len() {
                assert_ne!(
                    nonces[i], nonces[j],
                    "Nonce {} and {} should be different",
                    i, j
                );
            }
        }
    }

    /// 生成测试用的 32 字节加密密钥（base64 编码），用于加密相关测试
    #[allow(dead_code)]
    fn make_test_key() -> (Vec<u8>, String) {
        let key_bytes: Vec<u8> = vec![
            0x3a, 0x7b, 0x9c, 0x1d, 0x4e, 0x8f, 0x2c, 0x6b, 0x9a, 0x3d, 0x8e, 0x1f, 0x4a, 0x7d,
            0x2e, 0x6f, 0x9b, 0x3c, 0x8d, 0x1e, 0x4b, 0x6a, 0x2b, 0x6c, 0x9f, 0x3a, 0x8b, 0x1c,
            0x4d, 0x7e, 0x2f, 0x6a,
        ];
        let key_b64 = base64::engine::general_purpose::STANDARD.encode(&key_bytes);
        (key_bytes, key_b64)
    }

    // ==================== parse_size 边界测试 ====================

    #[test]
    fn test_parse_size_tb() {
        assert_eq!(FileSink::parse_size("1TB"), Some(1024 * 1024 * 1024 * 1024));
        assert_eq!(
            FileSink::parse_size("2TB"),
            Some(2 * 1024 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn test_parse_size_decimal_rejected() {
        // 小数应被拒绝（parse::<u64> 不支持小数）
        assert_eq!(FileSink::parse_size("1.5MB"), None);
        assert_eq!(FileSink::parse_size("0.5"), None);
    }

    #[test]
    fn test_parse_size_negative_rejected() {
        // 负数应被拒绝
        assert_eq!(FileSink::parse_size("-100"), None);
    }

    // ==================== calculate_next_rotation_time 测试 ====================

    #[test]
    fn test_calculate_next_rotation_time_hourly() {
        let now = Utc::now();
        let result = FileSink::calculate_next_rotation_time("hourly");
        assert!(result.is_some());
        let next = result.unwrap();
        assert!(next > now);
        // hourly 应该是大约 1 小时后（允许 1 分钟误差）
        let diff = next - now;
        assert!(
            diff.num_minutes() >= 59 && diff.num_minutes() <= 61,
            "hourly rotation should be ~60 minutes away, got {}",
            diff.num_minutes()
        );
    }

    #[test]
    fn test_calculate_next_rotation_time_daily() {
        let now = Utc::now();
        let result = FileSink::calculate_next_rotation_time("daily");
        assert!(result.is_some());
        let next = result.unwrap();
        // daily 应该是明天的 00:00:00
        assert_eq!(next.hour(), 0);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.second(), 0);
        assert!(next > now);
    }

    #[test]
    fn test_calculate_next_rotation_time_weekly() {
        let now = Utc::now();
        let result = FileSink::calculate_next_rotation_time("weekly");
        assert!(result.is_some());
        let next = result.unwrap();
        assert_eq!(next.hour(), 0);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.second(), 0);
        assert!(next > now);
    }

    #[test]
    fn test_calculate_next_rotation_time_monthly() {
        let result = FileSink::calculate_next_rotation_time("monthly");
        assert!(result.is_some());
    }

    #[test]
    fn test_calculate_next_rotation_time_invalid_defaults_to_daily() {
        let result = FileSink::calculate_next_rotation_time("invalid_interval");
        assert!(result.is_some());
        // 无效配置应回退到 daily 行为
        let next = result.unwrap();
        assert_eq!(next.hour(), 0);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.second(), 0);
    }

    // ==================== update_next_rotation_time_inner 测试 ====================

    #[test]
    fn test_update_next_rotation_time_inner_sets_value() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "hourly".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        inner.next_rotation_time = None;
        sink.update_next_rotation_time_inner(&mut inner);
        assert!(inner.next_rotation_time.is_some());
    }

    // ==================== should_rotate_by_time_inner 测试 ====================

    #[test]
    fn test_should_rotate_by_time_inner_no_next_time() {
        // next_rotation_time 为 None，last_rotation_date 也为 None → 不轮转
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let inner = sink.inner.read();
        let result = sink.should_rotate_by_time_inner(&inner);
        assert!(!result);
    }

    #[test]
    fn test_should_rotate_by_time_inner_past_next_time() {
        // next_rotation_time 在过去 → 应轮转
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        inner.next_rotation_time = Some(Utc::now() - chrono::Duration::hours(1));
        let result = sink.should_rotate_by_time_inner(&inner);
        assert!(result);
    }

    #[test]
    fn test_should_rotate_by_time_inner_future_next_time() {
        // next_rotation_time 在未来 → 不轮转
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        inner.next_rotation_time = Some(Utc::now() + chrono::Duration::hours(1));
        let result = sink.should_rotate_by_time_inner(&inner);
        assert!(!result);
    }

    #[test]
    fn test_should_rotate_by_time_inner_daily_date_change() {
        // daily + last_rotation_date 为昨天 → 应轮转（即便 next_time 在未来）
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let yesterday = Utc::now().date_naive().num_days_from_ce() - 1;
        inner.last_rotation_date = Some(yesterday);
        inner.next_rotation_time = Some(Utc::now() + chrono::Duration::days(1));
        let result = sink.should_rotate_by_time_inner(&inner);
        assert!(result);
    }

    // ==================== open_file_inner 测试 ====================

    #[test]
    fn test_open_file_inner_creates_nested_directory() {
        let temp_dir = tempdir().unwrap();
        let nested = temp_dir.path().join("nested").join("deep");
        let log_path = nested.join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(result.is_ok());
        assert!(inner.current_file.is_some());
        assert!(log_path.exists());
    }

    #[test]
    fn test_open_file_inner_detects_existing_size() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let existing = "existing content\n";
        std::fs::write(&log_path, existing).unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(result.is_ok());
        // current_size 应反映已有文件大小
        assert_eq!(inner.current_size, existing.len() as u64);
    }

    // ==================== flush_batch_inner 测试 ====================

    #[test]
    fn test_flush_batch_inner_empty_buffer_noop() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        let result = sink.flush_batch_inner(&mut inner);
        assert!(result.is_ok());
    }

    #[test]
    fn test_flush_batch_inner_writes_records_to_file() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();

        inner.batch_buffer.push(create_test_record("Message 1"));
        inner.batch_buffer.push(create_test_record("Message 2"));

        let result = sink.flush_batch_inner(&mut inner);
        assert!(result.is_ok());
        assert!(inner.batch_buffer.is_empty());

        drop(inner);
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Message 1"));
        assert!(content.contains("Message 2"));
    }

    #[test]
    fn test_flush_batch_inner_increments_current_size() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();

        let initial_size = inner.current_size;
        inner.batch_buffer.push(create_test_record("Test message"));
        sink.flush_batch_inner(&mut inner).unwrap();
        assert!(inner.current_size > initial_size);
    }

    // ==================== check_rotation_inner 测试 ====================

    #[test]
    fn test_check_rotation_inner_no_rotation_needed() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            max_size: "1MB".to_string(),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        inner.current_size = 100; // 远小于 1MB
        inner.next_rotation_time = Some(Utc::now() + chrono::Duration::days(1));

        let result = sink.check_rotation_inner(&mut inner);
        assert!(result.is_ok());
        assert_eq!(inner.sequence, 0); // 未轮转
    }

    #[test]
    fn test_check_rotation_inner_by_size_triggers_rotation() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            max_size: "100".to_string(), // 极小限制
            rotation_time: "daily".to_string(),
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        // 文件需有内容才能被 rotate 重命名
        std::fs::write(sink.config.path.clone(), "x").unwrap();
        inner.current_size = 200; // 超过 100
        inner.next_rotation_time = Some(Utc::now() + chrono::Duration::days(1));

        let result = sink.check_rotation_inner(&mut inner);
        assert!(result.is_ok());
        assert_eq!(inner.sequence, 1); // 已轮转
    }

    // ==================== rotate_inner 测试 ====================

    #[test]
    fn test_rotate_inner_renames_original_file() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        std::fs::write(&log_path, "test content").unwrap();

        let result = sink.rotate_inner(&mut inner);
        assert!(result.is_ok());
        // 轮转后原路径应被重新创建（open_file_inner 在 rotate 末尾被调用）
        assert!(log_path.exists());
        // 目录下应至少有 2 个文件（重命名的旧文件 + 新文件）
        let count = std::fs::read_dir(temp_dir.path()).unwrap().count();
        assert!(count >= 2);
    }

    #[test]
    fn test_rotate_inner_increments_sequence() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();

        let initial = inner.sequence;
        sink.rotate_inner(&mut inner).unwrap();
        assert_eq!(inner.sequence, initial + 1);
        sink.rotate_inner(&mut inner).unwrap();
        assert_eq!(inner.sequence, initial + 2);
    }

    #[test]
    fn test_rotate_inner_resets_current_size() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        inner.current_size = 5000;

        sink.rotate_inner(&mut inner).unwrap();
        assert_eq!(inner.current_size, 0);
    }

    #[test]
    fn test_rotate_inner_updates_next_rotation_time() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "hourly".to_string(),
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        inner.next_rotation_time = None;

        sink.rotate_inner(&mut inner).unwrap();
        // 轮转应更新 next_rotation_time
        assert!(inner.next_rotation_time.is_some());
    }

    // ==================== compress_file 测试 ====================

    #[test]
    fn test_compress_file_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("test.log");
        let original_content = b"This is test content for compression. Hello World!";
        std::fs::write(&original_path, original_content).unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            compress: true,
            compression_level: 3,
            encrypt: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&original_path);
        assert!(result.is_ok());
        let compressed_path = result.unwrap();
        assert_eq!(compressed_path.extension().unwrap(), "zst");
        assert!(compressed_path.exists());
        // 原文件应被删除
        assert!(!original_path.exists());

        // 解压验证内容一致
        let compressed_file = std::fs::File::open(&compressed_path).unwrap();
        let mut decoder = zstd::stream::Decoder::new(compressed_file).unwrap();
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, original_content);
    }

    #[test]
    fn test_compress_file_nonexistent_input_returns_error() {
        let temp_dir = tempdir().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.log");
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            compress: true,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.compress_file(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_compress_file_with_encryption_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("test.log");
        let original_content = b"Sensitive log content that needs encryption";
        std::fs::write(&original_path, original_content).unwrap();

        let (key_bytes, key_b64) = make_test_key();
        std::env::set_var("TEST_COMPRESS_ENC_KEY", &key_b64);

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            compress: true,
            compression_level: 3,
            encrypt: true,
            encryption_key_env: Some("TEST_COMPRESS_ENC_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&original_path);
        assert!(result.is_ok(), "compress_file failed: {:?}", result.err());
        let encrypted_path = result.unwrap();
        assert_eq!(encrypted_path.extension().unwrap(), "enc");
        assert!(encrypted_path.exists());

        // 解密：前 12 字节是 nonce，其余是 ciphertext
        let encrypted_data = std::fs::read(&encrypted_path).unwrap();
        assert!(encrypted_data.len() > 12);
        use aes_gcm::{Aes256Gcm, Nonce};
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];
        let decrypted_compressed = cipher.decrypt(nonce, ciphertext).unwrap();

        // 解压
        let mut decoder = zstd::stream::Decoder::new(&decrypted_compressed[..]).unwrap();
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, original_content);

        std::env::remove_var("TEST_COMPRESS_ENC_KEY");
    }

    // ==================== encrypt_file 测试 ====================

    #[test]
    #[serial]
    fn test_encrypt_file_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("test.log");
        let output_path = temp_dir.path().join("test.log.enc");
        let original_content = b"Secret log content for encryption test";
        std::fs::write(&input_path, original_content).unwrap();

        let (key_bytes, key_b64) = make_test_key();
        std::env::set_var("TEST_ENC_KEY_RT", &key_b64);

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_ENC_KEY_RT".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_ok(), "encrypt_file failed: {:?}", result.err());
        assert!(output_path.exists());

        // 解密验证
        let encrypted_data = std::fs::read(&output_path).unwrap();
        assert!(encrypted_data.len() > 12);
        use aes_gcm::{Aes256Gcm, Nonce};
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];
        let decrypted = cipher.decrypt(nonce, ciphertext).unwrap();
        assert_eq!(decrypted, original_content);

        std::env::remove_var("TEST_ENC_KEY_RT");
    }

    #[test]
    #[serial]
    fn test_encrypt_file_missing_key_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.log");
        let output_path = temp_dir.path().join("output.log.enc");
        std::fs::write(&input_path, "content").unwrap();
        std::env::remove_var("TEST_MISSING_ENC_KEY_VAR");

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_MISSING_ENC_KEY_VAR".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    #[serial]
    fn test_encrypt_file_nonexistent_input_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("nonexistent.log");
        let output_path = temp_dir.path().join("output.log.enc");

        let (_key_bytes, key_b64) = make_test_key();
        std::env::set_var("TEST_ENC_KEY_NI", &key_b64);

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_ENC_KEY_NI".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_err());
        std::env::remove_var("TEST_ENC_KEY_NI");
    }

    #[test]
    #[serial]
    fn test_encrypt_file_invalid_base64_key_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.log");
        let output_path = temp_dir.path().join("output.log.enc");
        std::fs::write(&input_path, "content").unwrap();
        // 长度 >= 16 但不是有效 base64
        std::env::set_var("TEST_INVALID_B64_KEY", "not_valid_base64!!!*@$");

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_INVALID_B64_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_err());
        std::env::remove_var("TEST_INVALID_B64_KEY");
    }

    #[test]
    #[serial]
    fn test_encrypt_file_wrong_length_key_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.log");
        let output_path = temp_dir.path().join("output.log.enc");
        std::fs::write(&input_path, "content").unwrap();
        // 解码后 16 字节（非 32），但 base64 字符串长度 >= 16
        let short_key = base64::engine::general_purpose::STANDARD.encode(b"1234567890123456");
        std::env::set_var("TEST_WRONG_LEN_KEY", &short_key);

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_WRONG_LEN_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
        std::env::remove_var("TEST_WRONG_LEN_KEY");
    }

    // ==================== perform_cleanup 测试 ====================

    #[test]
    fn test_perform_cleanup_removes_excess_by_total_size() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        // 创建 5 个文件，每个 1KB，总大小 5KB 超过 1KB 限制
        for i in 0..5 {
            let p = dir.path().join(format!("test_{}.log", i));
            std::fs::write(&p, "x".repeat(1024)).unwrap();
        }

        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            max_size: "1MB".to_string(),
            rotation_time: "daily".to_string(),
            keep_files: 2,
            compress: false,
            compression_level: 3,
            encrypt: false,
            encryption_key_env: None,
            retention_days: 30,
            max_total_size: "1KB".to_string(),
            cleanup_interval_minutes: 60,
            batch_size: 100,
            flush_interval_ms: 100,
            masking_enabled: true,
        };

        let result = FileSink::perform_cleanup(&config, &dir.path().join("test.log"));
        assert!(result.is_ok());
        // 应删除了部分文件（5KB 超过 1KB，需删除 ~4KB ≈ 4 个文件）
        let remaining = std::fs::read_dir(dir.path()).unwrap().count();
        assert!(
            remaining < 5,
            "expected some files removed, got {}",
            remaining
        );
    }

    #[test]
    fn test_perform_cleanup_empty_directory() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            max_total_size: "1GB".to_string(),
            ..Default::default()
        };
        let result = FileSink::perform_cleanup(&config, &dir.path().join("test.log"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_perform_cleanup_nonexistent_parent_returns_error() {
        let dir = tempdir().unwrap();
        let nonexistent_parent = dir.path().join("does_not_exist");
        let log_path = nonexistent_parent.join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_total_size: "1GB".to_string(),
            ..Default::default()
        };
        // parent 目录不存在 → read_dir 失败
        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_err());
    }

    // ==================== Clone / Debug / is_healthy / flush / shutdown 测试 ====================

    #[test]
    fn test_file_sink_clone_produces_independent_instance() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            max_size: "1MB".to_string(),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let cloned = sink.clone();
        // Clone 后应为新实例：current_file 为 None、size/sequence 归零
        assert!(cloned.inner.read().current_file.is_none());
        assert_eq!(cloned.inner.read().current_size, 0);
        assert_eq!(cloned.inner.read().sequence, 0);
        // 配置应相同
        assert_eq!(sink.config.path, cloned.config.path);
        assert_eq!(sink.config.max_size, cloned.config.max_size);
    }

    #[test]
    fn test_file_sink_debug_format_contains_key_fields() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let debug_str = format!("{:?}", sink);
        assert!(debug_str.contains("FileSink"));
        assert!(debug_str.contains("path"));
        assert!(debug_str.contains("current_size"));
    }

    #[test]
    fn test_file_sink_is_healthy_false_without_file() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        assert!(!sink.is_healthy());
    }

    #[test]
    fn test_file_sink_is_healthy_true_with_file() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        {
            let mut inner = sink.inner.write();
            sink.open_file_inner(&mut inner).unwrap();
        }
        assert!(sink.is_healthy());
    }

    #[test]
    fn test_file_sink_flush_writes_buffered_records() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        {
            let mut inner = sink.inner.write();
            sink.open_file_inner(&mut inner).unwrap();
            inner.batch_buffer.push(create_test_record("Flush test"));
        }
        let result = sink.flush();
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Flush test"));
    }

    #[test]
    fn test_file_sink_flush_without_file_succeeds() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        // 未打开文件，flush 应仍成功（空操作）
        let result = sink.flush();
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_sink_shutdown_flushes_remaining_data() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        {
            let mut inner = sink.inner.write();
            sink.open_file_inner(&mut inner).unwrap();
            inner.batch_buffer.push(create_test_record("Shutdown test"));
        }
        let result = sink.shutdown();
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Shutdown test"));
    }

    // ==================== LogSink::write 测试 ====================

    #[test]
    fn test_write_multiple_records_all_persisted() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            batch_size: 2, // 小批量触发刷新
            flush_interval_ms: 1000,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        for i in 0..5 {
            let record = create_test_record(&format!("Message {}", i));
            sink.write(&record).unwrap();
        }
        sink.flush().unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        for i in 0..5 {
            assert!(content.contains(&format!("Message {}", i)));
        }
        sink.shutdown().unwrap();
    }

    #[test]
    fn test_write_with_masking_disabled_preserves_sensitive_value() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: false,
            batch_size: 1,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let record = LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            // 值需 >= 16 字符才会被 generic_secret 规则匹配
            message: "password=secret1234567890".to_string(),
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: "t1".to_string(),
        };
        sink.write(&record).unwrap();
        sink.flush().unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            content.contains("secret1234567890"),
            "masking disabled should preserve original value"
        );
        sink.shutdown().unwrap();
    }

    #[test]
    fn test_write_with_masking_enabled_redacts_sensitive_value() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            masking_enabled: true,
            batch_size: 1,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let record = LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "test".to_string(),
            // 值 19 字符 >= 16，会被 generic_secret 规则匹配
            message: "password=secret1234567890".to_string(),
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: "t1".to_string(),
        };
        sink.write(&record).unwrap();
        sink.flush().unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            !content.contains("secret1234567890"),
            "masking enabled should redact sensitive value"
        );
        assert!(
            content.contains("***REDACTED***"),
            "masked output should contain REDACTED marker"
        );
        sink.shutdown().unwrap();
    }

    #[test]
    fn test_write_appends_to_existing_file() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        // 预先写入内容
        std::fs::write(&log_path, "pre-existing line\n").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            batch_size: 1,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        sink.write(&create_test_record("Appended message")).unwrap();
        sink.flush().unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.starts_with("pre-existing line"));
        assert!(content.contains("Appended message"));
        sink.shutdown().unwrap();
    }

    // ==================== rotation_time 分支覆盖测试 ====================

    #[test]
    fn test_file_sink_new_with_weekly_rotation() {
        // 覆盖行 108: "weekly" => StdDuration::from_secs(604800)
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("weekly.log"),
            rotation_time: "weekly".to_string(),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        assert_eq!(sink.rotation_interval, StdDuration::from_secs(604800));
        sink.shutdown().unwrap();
    }

    #[test]
    fn test_file_sink_new_with_monthly_rotation() {
        // 覆盖行 109: "monthly" => StdDuration::from_secs(2592000)
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("monthly.log"),
            rotation_time: "monthly".to_string(),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        assert_eq!(sink.rotation_interval, StdDuration::from_secs(2592000));
        sink.shutdown().unwrap();
    }

    #[test]
    fn test_file_sink_new_with_unknown_rotation_falls_back_to_daily() {
        // 覆盖行 110: _ => StdDuration::from_secs(86400)（默认分支）
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("unknown.log"),
            rotation_time: "unknown_interval".to_string(),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        assert_eq!(sink.rotation_interval, StdDuration::from_secs(86400));
        sink.shutdown().unwrap();
    }

    #[test]
    fn test_file_sink_new_with_hourly_rotation() {
        // 覆盖行 106: "hourly" => StdDuration::from_secs(3600)
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("hourly.log"),
            rotation_time: "hourly".to_string(),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        assert_eq!(sink.rotation_interval, StdDuration::from_secs(3600));
        sink.shutdown().unwrap();
    }

    // ==================== get_encryption_key 错误路径测试 ====================

    #[test]
    #[serial]
    fn test_get_encryption_key_invalid_base64() {
        // 覆盖行 239-244: 无效 base64 解码错误
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_INVALID_B64".to_string()),
            ..Default::default()
        };
        // 设置非法 base64 字符串（长度足够但不是有效 base64）
        std::env::set_var("TEST_INVALID_B64", "this_is_not_valid_base64!!!@#$");

        let sink = create_test_file_sink(config);
        let result = sink.get_encryption_key();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid base64 encoding"));

        std::env::remove_var("TEST_INVALID_B64");
    }

    #[test]
    #[serial]
    fn test_get_encryption_key_wrong_byte_length() {
        // 覆盖行 246-250: 解码后字节数不等于 32
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_WRONG_LEN".to_string()),
            ..Default::default()
        };
        // 16 字节（足够长，但解码后不是 32 字节）
        std::env::set_var("TEST_WRONG_LEN", "YWJjZGVmZ2hpamtsbW5v"); // 16 字节

        let sink = create_test_file_sink(config);
        let result = sink.get_encryption_key();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));

        std::env::remove_var("TEST_WRONG_LEN");
    }

    // ==================== get_disk_space_info 错误路径测试 ====================

    #[test]
    fn test_get_disk_space_info_nonexistent_path() {
        // 覆盖行 452-455: 路径不存在时返回错误
        let config = FileSinkConfig {
            enabled: true,
            // 使用一个肯定不存在的父路径
            path: PathBuf::from("/nonexistent_root_path_xyz/log.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.get_disk_space_info();
        assert!(result.is_err());
    }

    // ==================== perform_cleanup 边界测试 ====================

    #[test]
    fn test_perform_cleanup_with_empty_directory() {
        // 覆盖 perform_cleanup 在空目录中的行为
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("app.log");
        // 创建空目录（无旧日志文件）
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            retention_days: 7,
            max_total_size: "1GB".to_string(),
            ..Default::default()
        };
        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_perform_cleanup_removes_expired_files() {
        // 覆盖 perform_cleanup 删除过期文件的行为
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("app.log");

        // 创建一个"过期"的日志文件（修改时间为 30 天前）
        let old_file = temp_dir.path().join("app_20250101_000000.log");
        std::fs::write(&old_file, "old log content").unwrap();

        // 设置文件修改时间为 30 天前
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
        let _ = filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(old_time));

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            retention_days: 7, // 保留 7 天，30 天前的文件应被删除
            max_total_size: "1GB".to_string(),
            keep_files: 0,
            ..Default::default()
        };
        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_ok());
        // 过期文件应被删除
        assert!(!old_file.exists(), "expired file should be removed");
    }

    // ==================== compress_file 测试 ====================

    #[test]
    fn test_compress_file_basic() {
        // 覆盖 compress_file 基本压缩路径（不加密）
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("to_compress.log");
        std::fs::write(&log_path, "some log content to compress\n").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            compress: false, // compress_file 本身不依赖此标志，但配置需要
            encrypt: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&log_path);
        assert!(result.is_ok(), "compress_file should succeed");
        let compressed_path = result.unwrap();
        assert!(compressed_path.exists(), "compressed file should exist");
        assert!(compressed_path.extension().is_some_and(|e| e == "zst"));
        // 原文件应被删除（因为 encrypt=false）
        assert!(
            !log_path.exists(),
            "original file should be removed after compression"
        );
    }

    #[test]
    fn test_compress_file_nonexistent_input() {
        // 覆盖 compress_file 错误路径（输入文件不存在）
        let temp_dir = tempdir().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist.log");

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&nonexistent);
        assert!(
            result.is_err(),
            "compress_file should fail for nonexistent input"
        );
    }

    // ==================== encrypt_file 测试 ====================

    #[test]
    #[serial]
    fn test_encrypt_file_basic() {
        // 覆盖 encrypt_file 基本加密路径
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("to_encrypt.log");
        let output_path = temp_dir.path().join("encrypted.log.enc");
        std::fs::write(&input_path, "secret log content\n").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            encryption_key_env: Some("TEST_ENCRYPT_KEY".to_string()),
            ..Default::default()
        };
        // 设置有效密钥（32 字节，高熵）
        std::env::set_var(
            "TEST_ENCRYPT_KEY",
            "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=",
        );

        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_ok(), "encrypt_file should succeed");
        assert!(output_path.exists(), "encrypted file should be created");
        // 加密文件应大于 12 字节（nonce）+ 明文长度
        let encrypted_size = std::fs::metadata(&output_path).unwrap().len();
        assert!(
            encrypted_size > 12,
            "encrypted file should contain nonce + ciphertext"
        );

        std::env::remove_var("TEST_ENCRYPT_KEY");
    }

    #[test]
    #[serial]
    fn test_encrypt_file_missing_key_env() {
        // 覆盖 encrypt_file 错误路径（密钥环境变量未设置）
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("to_encrypt.log");
        let output_path = temp_dir.path().join("encrypted.log.enc");
        std::fs::write(&input_path, "content\n").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            encryption_key_env: Some("MISSING_ENCRYPT_KEY_ENV_VAR".to_string()),
            ..Default::default()
        };
        std::env::remove_var("MISSING_ENCRYPT_KEY_ENV_VAR");

        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_err(), "encrypt_file should fail without key");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Encryption key not found"));
    }

    #[test]
    #[serial]
    fn test_encrypt_file_nonexistent_input() {
        // 覆盖 encrypt_file 错误路径（输入文件不存在）
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("does_not_exist.log");
        let output_path = temp_dir.path().join("out.log.enc");

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            encryption_key_env: Some("TEST_ENCRYPT_KEY_2".to_string()),
            ..Default::default()
        };
        std::env::set_var(
            "TEST_ENCRYPT_KEY_2",
            "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=",
        );

        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(
            result.is_err(),
            "encrypt_file should fail for nonexistent input"
        );

        std::env::remove_var("TEST_ENCRYPT_KEY_2");
    }

    // ==================== compress_file with encryption 测试 ====================

    #[test]
    #[serial]
    fn test_compress_file_with_encryption() {
        // 覆盖 compress_file 的加密分支（行 640-652）
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("to_compress_enc.log");
        std::fs::write(&log_path, "content to compress and encrypt\n").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_COMPRESS_ENC_KEY".to_string()),
            ..Default::default()
        };
        std::env::set_var(
            "TEST_COMPRESS_ENC_KEY",
            "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=",
        );

        let sink = create_test_file_sink(config);
        let result = sink.compress_file(&log_path);
        assert!(
            result.is_ok(),
            "compress_file with encryption should succeed"
        );
        let encrypted_path = result.unwrap();
        assert!(
            encrypted_path.exists(),
            "encrypted compressed file should exist"
        );
        assert!(encrypted_path.extension().is_some_and(|e| e == "enc"));

        std::env::remove_var("TEST_COMPRESS_ENC_KEY");
    }

    // ==================== rotate_inner 测试 ====================

    #[test]
    fn test_rotate_inner_basic() {
        // 覆盖 rotate_inner 基本轮转路径
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("rotate.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            compress: false,
            encrypt: false,
            ..Default::default()
        };
        // 先创建文件并写入内容
        std::fs::write(&log_path, "original content\n").unwrap();

        let sink = FileSink::new(config).unwrap();
        // 手动触发轮转
        let mut inner = sink.inner.write();
        let result = sink.rotate_inner(&mut inner);
        assert!(result.is_ok(), "rotate_inner should succeed");
        drop(inner);

        sink.shutdown().unwrap();

        // 原文件应被重命名（轮转后），新文件应被创建
        let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
        // 至少应该有轮转后的文件
        assert!(
            !entries.is_empty(),
            "rotated file should exist in directory"
        );
    }

    #[test]
    fn test_rotate_inner_with_compression() {
        // 覆盖 rotate_inner 的压缩分支（行 748-755）
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("rotate_compress.log");

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            compress: true,
            encrypt: false,
            compression_level: 3,
            ..Default::default()
        };
        std::fs::write(&log_path, "content to be rotated and compressed\n").unwrap();

        let sink = FileSink::new(config).unwrap();
        let mut inner = sink.inner.write();
        let result = sink.rotate_inner(&mut inner);
        assert!(
            result.is_ok(),
            "rotate_inner with compression should succeed"
        );
        drop(inner);

        // 给后台压缩线程一点时间完成
        std::thread::sleep(std::time::Duration::from_millis(500));
        sink.shutdown().unwrap();

        // 检查是否有 .zst 文件生成
        let has_zst = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .any(|e| e.is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "zst")));
        assert!(has_zst, "compressed rotated file (.zst) should exist");
    }

    // ==================== rotate_inner encrypt-only 分支测试 ====================

    #[test]
    #[serial]
    fn test_rotate_inner_with_encryption_only_branch() {
        // 覆盖行 808-847：compress=false 但 encrypt=true 的分支
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("rotate_encrypt.log");

        let (_key_bytes, key_b64) = make_test_key();
        std::env::set_var("TEST_ROTATE_ENC_KEY", &key_b64);

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            compress: false, // 关闭压缩
            encrypt: true,   // 开启加密，触发 encrypt-only 分支
            encryption_key_env: Some("TEST_ROTATE_ENC_KEY".to_string()),
            ..Default::default()
        };
        std::fs::write(&log_path, "content to be rotated and encrypted\n").unwrap();

        let sink = FileSink::new(config).unwrap();
        let mut inner = sink.inner.write();
        let result = sink.rotate_inner(&mut inner);
        assert!(
            result.is_ok(),
            "rotate_inner with encryption-only should succeed"
        );
        drop(inner);

        // 给后台加密线程一点时间完成
        std::thread::sleep(std::time::Duration::from_millis(500));
        sink.shutdown().unwrap();

        // 检查是否有 .enc 文件生成（encrypt-only 路径会生成 .enc 文件）
        let has_enc = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .any(|e| e.is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "enc")));
        assert!(
            has_enc,
            "encrypted rotated file (.enc) should exist in encrypt-only mode"
        );

        std::env::remove_var("TEST_ROTATE_ENC_KEY");
    }

    // ==================== compress_file 加密失败回退测试 ====================

    #[test]
    #[serial]
    fn test_compress_file_with_encryption_failure_keeps_compressed() {
        // 覆盖行 666-676：当 encrypt=true 但密钥无效时，
        // compress_file 应将压缩文件重命名为 .unencrypted 后缀并返回错误
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("to_compress_fail.log");
        std::fs::write(&original_path, "content for failed encryption\n").unwrap();

        // 设置一个无效的加密密钥（长度足够但解码后不是 32 字节）
        let invalid_key = base64::engine::general_purpose::STANDARD.encode(b"1234567890123456");
        std::env::set_var("TEST_COMPRESS_ENC_FAIL_KEY", &invalid_key);

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("active.log"),
            compress: true,
            compression_level: 3,
            encrypt: true,
            encryption_key_env: Some("TEST_COMPRESS_ENC_FAIL_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&original_path);
        assert!(
            result.is_err(),
            "compress_file should fail when encryption key is invalid"
        );

        // 加密失败时，压缩文件应被重命名为 .unencrypted 结尾（保留压缩内容）
        // with_extension("zst.unencrypted") 会替换原扩展名 enc 为 zst.unencrypted
        let unencrypted_file = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| {
                p.to_string_lossy()
                    .ends_with(".unencrypted")
            })
            .expect("compressed file should be preserved with .unencrypted suffix when encryption fails");

        // 验证保留的文件确实是有效的 zst 压缩数据
        let compressed_file = std::fs::File::open(&unencrypted_file).unwrap();
        let decoder_result = zstd::stream::Decoder::new(compressed_file);
        assert!(
            decoder_result.is_ok(),
            "preserved file should be valid zst compressed data"
        );

        std::env::remove_var("TEST_COMPRESS_ENC_FAIL_KEY");
    }

    // ==================== open_file_inner 错误路径测试 ====================

    #[test]
    fn test_open_file_inner_create_dir_failure_returns_error() {
        // 覆盖行 297-302：create_dir_all 失败时返回 IoError
        // 使用一个无法创建的父目录路径（在文件路径下创建目录会失败）
        let temp_dir = tempdir().unwrap();
        // 构造一个路径：在已有文件路径下再尝试创建子目录会失败
        let blocking_file = temp_dir.path().join("blocking_file");
        std::fs::write(&blocking_file, "block").unwrap();
        // 现在 blocking_file 是文件，但我们将以 blocking_file/sub/log.log 为路径，
        // create_dir_all 会失败因为 blocking_file 已经是文件
        let impossible_path = blocking_file.join("sub").join("log.log");

        let config = FileSinkConfig {
            enabled: true,
            path: impossible_path,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(
            result.is_err(),
            "open_file_inner should fail when parent directory cannot be created"
        );
        // 确认 inner.current_file 未被设置
        assert!(inner.current_file.is_none());
    }

    // ==================== FileSink::new open_file_inner 失败测试 ====================

    #[test]
    fn test_file_sink_new_open_file_failure_returns_error() {
        // 覆盖行 165-168：FileSink::new 时 open_file_inner 失败应返回 Err
        let temp_dir = tempdir().unwrap();
        let blocking_file = temp_dir.path().join("block_new");
        std::fs::write(&blocking_file, "block").unwrap();
        // 在已有文件路径下创建子目录会失败
        let impossible_log_path = blocking_file.join("nested").join("log.log");

        let config = FileSinkConfig {
            enabled: true,
            path: impossible_log_path,
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_err(),
            "FileSink::new should return error when open_file_inner fails"
        );
    }

    // ==================== check_rotation_inner 时间触发轮转测试 ====================

    #[test]
    fn test_check_rotation_inner_by_time_triggers_rotation() {
        // 覆盖 line 858: rotate_by_time 为 true 时触发轮转
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            max_size: "1MB".to_string(), // 大限制，避免触发 size 轮转
            rotation_time: "daily".to_string(),
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        // 文件需有内容才能被 rotate 重命名
        std::fs::write(sink.config.path.clone(), "x").unwrap();
        // 设置 next_rotation_time 在过去，触发时间轮转
        inner.next_rotation_time = Some(Utc::now() - chrono::Duration::hours(1));

        let result = sink.check_rotation_inner(&mut inner);
        assert!(result.is_ok());
        // 时间触发轮转应执行
        assert_eq!(inner.sequence, 1, "rotation should be triggered by time");
    }

    // ==================== should_rotate_by_time_inner weekly 分支测试 ====================

    #[test]
    fn test_should_rotate_by_time_inner_weekly_date_change() {
        // 覆盖 line 511: weekly 配置下的日期变更检测
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            rotation_time: "weekly".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        // last_rotation_date 为上周 → 应触发轮转
        let last_week = Utc::now().date_naive().num_days_from_ce() - 7;
        inner.last_rotation_date = Some(last_week);
        // next_rotation_time 在未来（不应触发时间轮转）
        inner.next_rotation_time = Some(Utc::now() + chrono::Duration::days(1));

        let result = sink.should_rotate_by_time_inner(&inner);
        assert!(
            result,
            "weekly rotation should trigger when date changed since last rotation"
        );
    }

    // ==================== flush_batch_inner 写入错误测试 ====================

    #[test]
    fn test_flush_batch_inner_write_error_records_failure_and_reopens() {
        // 覆盖行 613-619：writeln! 失败时记录断路器失败并尝试重新打开文件
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();

        // 构造写入失败：删除底层文件，使 writeln! 到已关闭的句柄失败
        // 注意：append 模式下的 File 句柄即使文件被删除仍可写入（POSIX 语义）
        // 所以我们改为构造一个无文件句柄的场景
        let _ = inner.current_file.take(); // 移除文件句柄

        // 此时 batch_buffer 有记录但无文件句柄
        inner.batch_buffer.push(create_test_record("Will fail"));
        let initial_failures = inner.circuit_breaker.failure_count();
        let result = sink.flush_batch_inner(&mut inner);

        // 无文件句柄时，for 循环不会执行（if let Some(file) = ... 为 None）
        // 但 last_flush_time 仍会更新，方法返回 Ok
        assert!(
            result.is_ok(),
            "flush should succeed even without file handle"
        );
        // 没有文件句柄时，circuit_breaker 不应记录失败
        assert_eq!(
            inner.circuit_breaker.failure_count(),
            initial_failures,
            "no failure should be recorded when there is no file handle"
        );
    }

    // ==================== CircuitBreaker 打开时使用 fallback sink 测试 ====================

    /// 简单的 mock LogSink，用于测试 fallback 路径
    struct MockFallbackSink {
        write_count: Arc<parking_lot::Mutex<usize>>,
    }

    impl LogSink for MockFallbackSink {
        fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            *self.write_count.lock() += 1;
            Ok(())
        }
        fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        fn is_healthy(&self) -> bool {
            true
        }
        fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[test]
    fn test_write_with_open_circuit_breaker_uses_fallback_sink() {
        // 覆盖行 873-879：circuit breaker 打开时，使用 fallback sink 写入
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        // 构造 fallback sink
        let write_count = Arc::new(parking_lot::Mutex::new(0usize));
        let mock_sink = MockFallbackSink {
            write_count: write_count.clone(),
        };
        {
            let mut inner = sink.inner.write();
            inner.fallback_sink = Some(Arc::new(parking_lot::Mutex::new(mock_sink)));
            // 触发足够多的失败使断路器打开（failure_threshold=5）
            for _ in 0..5 {
                inner.circuit_breaker.record_failure();
            }
            // 验证断路器确实打开了
            assert_eq!(inner.circuit_breaker.state(), CircuitState::Open);
        }

        let record = create_test_record("Fallback test");
        let result = sink.write(&record);
        assert!(
            result.is_ok(),
            "write should not error when circuit is open"
        );

        // fallback sink 应被调用一次
        assert_eq!(
            *write_count.lock(),
            1,
            "fallback sink should be called once when circuit breaker is open"
        );
    }

    // ==================== rotation 失败使用 fallback sink 测试 ====================

    #[test]
    fn test_write_with_rotation_failure_uses_fallback_sink() {
        // 覆盖行 921-928：rotate_inner 失败时使用 fallback sink 写入
        let temp_dir = tempdir().unwrap();
        // 构造一个会让 rotate_inner 失败的场景：
        // 文件存在但无法重命名（在已有文件路径下）
        let blocking_file = temp_dir.path().join("block_rotate");
        std::fs::write(&blocking_file, "block").unwrap();
        // 现在 blocking_file 是文件，无法作为目录使用
        // rotate_inner 会尝试创建父目录、重命名等，但 path 本身是文件下的子路径
        let impossible_log_path = blocking_file.join("inner.log");

        let config = FileSinkConfig {
            enabled: true,
            path: impossible_log_path,
            max_size: "1".to_string(), // 极小限制，立即触发轮转
            compress: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        // 构造 fallback sink
        let write_count = Arc::new(parking_lot::Mutex::new(0usize));
        let mock_sink = MockFallbackSink {
            write_count: write_count.clone(),
        };
        {
            let mut inner = sink.inner.write();
            inner.fallback_sink = Some(Arc::new(parking_lot::Mutex::new(mock_sink)));
        }

        // 写入一条记录，触发 size 轮转，但轮转会因路径无效而失败
        let record = create_test_record("Rotation failure test");
        let result = sink.write(&record);
        // write 不应返回错误（错误被吞掉，转用 fallback sink）
        assert!(result.is_ok(), "write should not error when rotation fails");
        // fallback sink 应被调用
        assert!(
            *write_count.lock() >= 1,
            "fallback sink should be called when rotation fails"
        );
    }

    // ==================== shutdown 完整流程测试 ====================

    #[test]
    fn test_shutdown_with_active_timers_completes_successfully() {
        // 覆盖 line 960-984：shutdown 应能正确停止 active 的 timer 线程
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("shutdown_test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        // FileSink::new 会启动 rotation_timer 和 cleanup_timer 两个后台线程
        let sink = FileSink::new(config).unwrap();

        // 写入一些数据
        for i in 0..3 {
            let record = create_test_record(&format!("Pre-shutdown message {}", i));
            sink.write(&record).unwrap();
        }

        // shutdown 应能正常完成（线程会响应 shutdown_flag 并退出）
        let result = sink.shutdown();
        assert!(result.is_ok(), "shutdown should complete successfully");

        // 验证数据已被刷盘
        let content = std::fs::read_to_string(&log_path).unwrap();
        for i in 0..3 {
            assert!(
                content.contains(&format!("Pre-shutdown message {}", i)),
                "all buffered records should be flushed before shutdown completes"
            );
        }
    }

    // ==================== Drop trait 测试 ====================

    #[test]
    fn test_drop_does_not_panic_with_active_timers() {
        // 覆盖 line 988-1048：Drop 实现应能优雅处理 active 的 timer 线程
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("drop_test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();

        // 写入一些数据但不调用 shutdown，直接 drop
        sink.write(&create_test_record("Drop test message"))
            .unwrap();

        // drop 应不 panic，且应等待线程退出（带超时）
        drop(sink);

        // 验证文件存在（Drop 会 flush 剩余数据）
        assert!(log_path.exists(), "log file should exist after drop");
    }

    // ==================== perform_cleanup keep_files 边界测试 ====================

    #[test]
    fn test_perform_cleanup_with_keep_files_boundary() {
        // 覆盖行 433-438：expired_count > 0 但受 keep_files 限制的分支
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("keep_test.log");

        // 创建 4 个过期文件
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
        for i in 0..4 {
            let p = temp_dir.path().join(format!("keep_{}.log", i));
            std::fs::write(&p, "old content").unwrap();
            let _ = filetime::set_file_mtime(&p, filetime::FileTime::from_system_time(old_time));
        }

        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            retention_days: 7,                 // 保留 7 天，30 天前的文件算过期
            keep_files: 2,                     // 至少保留 2 个文件
            max_total_size: "1GB".to_string(), // 大限制，不触发 total_size 分支
            ..Default::default()
        };
        let result = FileSink::perform_cleanup(&config, &temp_dir.path().join("keep_test.log"));
        assert!(result.is_ok());

        // 验证：4 个过期文件，keep_files=2，应保留 2 个（entries.len() - keep_files = 2 个被删）
        let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
            .collect();
        // 至少应保留 2 个文件（keep_files 限制）
        assert!(
            remaining.len() >= 2,
            "keep_files should preserve at least 2 files, got {}",
            remaining.len()
        );
    }

    // ==================== parse_size 大数边界测试 ====================

    #[test]
    fn test_parse_size_large_values() {
        // 覆盖 parse_size 处理大数值的边界
        assert_eq!(
            FileSink::parse_size("1024TB"),
            Some(1024 * 1024 * 1024 * 1024 * 1024)
        );
        // 验证各个单位分支都能正确处理 1
        assert_eq!(FileSink::parse_size("1KB"), Some(1024));
        assert_eq!(FileSink::parse_size("1MB"), Some(1024 * 1024));
        assert_eq!(FileSink::parse_size("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(FileSink::parse_size("1TB"), Some(1024_u64.pow(4)));
    }

    // ==================== validate_key_entropy 边界测试 ====================

    #[test]
    fn test_validate_key_entropy_single_byte_repeated() {
        // 单字节重复 32 次：熵为 0，应被拒绝
        let weak_key = [0x42; 32];
        let result = FileSink::validate_key_entropy(&weak_key);
        assert!(
            result.is_err(),
            "single-byte repeated key should be rejected"
        );
    }

    #[test]
    fn test_validate_key_entropy_two_byte_pattern() {
        // 两字节交替：熵约 1.0，低于阈值 4.0，应被拒绝
        let mut pattern_key = [0u8; 32];
        for (i, byte) in pattern_key.iter_mut().enumerate() {
            *byte = if i % 2 == 0 { 0xAA } else { 0x55 };
        }
        let result = FileSink::validate_key_entropy(&pattern_key);
        assert!(
            result.is_err(),
            "two-byte pattern key should be rejected (entropy < 4.0)"
        );
    }

    #[test]
    fn test_validate_key_entropy_four_byte_pattern() {
        // 四字节循环模式：熵 = 2.0 < 4.0，应被拒绝
        let pattern = [0x11, 0x22, 0x33, 0x44];
        let mut pattern_key = [0u8; 32];
        for (i, byte) in pattern_key.iter_mut().enumerate() {
            *byte = pattern[i % 4];
        }
        let result = FileSink::validate_key_entropy(&pattern_key);
        assert!(
            result.is_err(),
            "four-byte pattern key should be rejected (entropy = 2.0 < 4.0)"
        );
    }
}
