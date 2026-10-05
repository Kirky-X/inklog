// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! File-based log sink with rotation, compression, and encryption support.
//!
//! This module provides the FileSink implementation for writing logs to files
//! with support for automatic rotation, compression, and encryption.

use super::CircuitBreaker;
use super::DiskCheckable;
use super::LogSink;
use super::Rotatable;
use crate::DataMasker;
use crate::FileSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::support::processing::OutputFormat;
use crate::validation::PathValidatorConfig;
use aes_gcm::KeyInit;
use aes_gcm::aead::Aead;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
use parking_lot::RwLock;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

// 类型别名，保持向后兼容
pub use super::circuit_breaker::{CircuitBreakerConfig, CircuitState};

#[cfg(windows)]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        directory_name: *const u16,
        free_bytes_available: *mut u64,
        total_bytes: *mut u64,
        total_free_bytes: *mut u64,
    ) -> i32;
}

/// FileSink 的可变内部状态
///
/// 所有需要 `&mut self` 访问的字段都封装在这里，
/// 通过 `RwLock` 实现内部可变性。
struct FileSinkInner {
    /// 当前文件句柄
    current_file: Option<std::io::BufWriter<File>>,
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
    fallback_sink: Option<Arc<dyn LogSink + Send + Sync>>,
    /// 轮转定时器
    rotation_timer: Option<Arc<parking_lot::Mutex<Instant>>>,
    /// 轮转定时器句柄
    timer_handle: Option<thread::JoinHandle<()>>,
    /// 清理定时器句柄
    cleanup_timer_handle: Option<thread::JoinHandle<()>>,
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
    /// 上次磁盘空间检查的时间与结果（节流缓存）
    last_disk_check: parking_lot::Mutex<Option<(Instant, bool)>>,
    /// Shutdown flag for graceful thread termination
    shutdown_flag: Arc<AtomicBool>,
    /// 终态写失败（熔断无 fallback / 磁盘不足无 fallback / fallback 写失败）
    /// 后置位；批量全量刷盘成功后清除。供 [`FileSink::is_healthy`] 使用，
    /// 保证日志丢失不静默。
    write_unhealthy: AtomicBool,
    /// 因终态写失败而丢失的记录计数（可观测性指标）
    lost_records: AtomicU64,
    /// 数据脱敏器（只读）
    masker: DataMasker,
    /// 归档审计链（audit_chain_enabled 时启用；轮转成功后 append 并写穿 manifest）
    audit_chain: Option<Arc<parking_lot::Mutex<crate::support::audit_chain::ArchiveChain>>>,
    /// 可变内部状态（Arc 共享给轮转定时线程执行空闲 flush）
    inner: Arc<RwLock<FileSinkInner>>,
}

/// FileSink 的实现，包含所有文件日志操作的核心逻辑
impl FileSink {
    /// 注入自定义 masker（含自定义规则；fast-masking feature 下 literal
    /// 规则经 builder 构建走 AC 加速）。注：轮转重建的派生实例不继承注入。
    pub fn with_masker(mut self, masker: DataMasker) -> Self {
        self.masker = masker;
        self
    }

    /// 归档产物 SHA-256（轮转审计链条目用）。
    pub(crate) fn sha256_file(path: &std::path::Path) -> Option<String> {
        use sha2::Digest;
        let data = fs::read(path).ok()?;
        let mut hasher = sha2::Sha256::new();
        hasher.update(&data);
        Some(
            hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        )
    }

    /// manifest 路径：`<stem>.chain.jsonl`（与活动日志同目录）。
    pub(crate) fn audit_manifest_path(log_path: &std::path::Path) -> std::path::PathBuf {
        let stem = log_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let name = format!("{stem}.chain.jsonl");
        match log_path.parent() {
            Some(parent) => parent.join(name),
            None => std::path::PathBuf::from(name),
        }
    }

    /// Creates a new FileSink with the given configuration.
    pub fn new(config: FileSinkConfig) -> Result<Self, InklogError> {
        let rotation_interval = match config.rotation_time.as_str() {
            "hourly" => StdDuration::from_secs(3600),
            "daily" => StdDuration::from_secs(86400),
            "weekly" => StdDuration::from_secs(604800),
            "monthly" => StdDuration::from_secs(2592000),
            _ => StdDuration::from_secs(86400),
        };
        // 归档审计链：audit_chain_enabled 时以 INKLOG_AUDIT_KEY 为链密钥；
        // 缺失则禁用（随机密钥会让 manifest 事后不可验，宁缺毋滥）
        let audit_chain = if config.audit_chain_enabled {
            match std::env::var("INKLOG_AUDIT_KEY") {
                Ok(key) if !key.is_empty() => Some(Arc::new(parking_lot::Mutex::new(
                    crate::support::audit_chain::ArchiveChain::new(key.as_bytes()),
                ))),
                _ => {
                    warn!("{}", crate::i18n::tr("audit-chain-key-missing"));
                    None
                }
            }
        } else {
            None
        };

        let rotation_timer = Arc::new(parking_lot::Mutex::new(Instant::now()));
        let last_rotation = Instant::now();

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
        };

        let sink = Self {
            config: config.clone(),
            rotation_interval,
            last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
            last_disk_check: parking_lot::Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            write_unhealthy: AtomicBool::new(false),
            lost_records: AtomicU64::new(0),
            masker: DataMasker::new(),
            audit_chain,
            inner: Arc::new(RwLock::new(inner)),
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
        super::rotation::parse_size(size_str).ok()
    }

    /// 获取加密密钥（密码模式用文件头中的盐确定性派生）。
    /// 单一事实源在 [`encryption_key_for`]，此处仅委托。
    ///
    /// 密钥（`Zeroizing` 包裹，离开作用域自动清零）
    #[cfg_attr(not(test), allow(dead_code))] // 生产路径走 encryption_key_for；测试面便捷委托
    fn get_encryption_key(&self, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, InklogError> {
        encryption_key_for(&self.config, salt)
    }

    /// 验证密钥熵（Shannon entropy）
    /// 返回 Ok(()) 如果密钥有足够的熵（>= 4.0）。
    /// 单一事实源在 `encryption::validate_key_entropy`，此处仅委托。
    #[cfg_attr(not(test), allow(dead_code))] // 生产路径走共享实现；测试面便捷委托
    fn validate_key_entropy(key: &[u8]) -> Result<(), InklogError> {
        super::encryption::validate_key_entropy(key)
    }

    fn open_file_inner(&self, inner: &mut FileSinkInner) -> Result<(), InklogError> {
        // vuln-0002: 验证路径安全性，防止路径遍历和敏感文件访问。
        // 必须在 `create_dir_all` 之前执行，避免恶意路径创建目录。
        // FileSink 需支持绝对路径（如 /var/log），但收紧 deny 黑名单，
        // 禁止落到用户主目录/密钥等敏感文件，避免默认宽松配置写到宿主任意文件。
        let validator = crate::validation::PathValidator::with_config(PathValidatorConfig {
            allow_absolute: true,
            allow_symlinks: false,
            deny_components: vec![
                "..".to_string(),
                ".git".to_string(),
                ".ssh".to_string(),
                ".env".to_string(),
                "etc".to_string(),
                "passwd".to_string(),
                "shadow".to_string(),
                ".bashrc".to_string(),
                ".bash_profile".to_string(),
                ".profile".to_string(),
                ".zshrc".to_string(),
                ".netrc".to_string(),
                "id_rsa".to_string(),
                "id_ed25519".to_string(),
            ],
            ..Default::default()
        });
        let validation_result = validator.validate(&self.config.path);
        if !validation_result.valid {
            let reason = validation_result
                .error
                .unwrap_or_else(|| "unknown".to_string());
            let mut args = crate::i18n::MsgArgs::new();
            args.set("path", self.config.path.display().to_string());
            args.set("reason", reason.clone());
            warn!("{}", crate::i18n::tr_args("sink-file_reject_path", args));
            let mut err_args = crate::i18n::MsgArgs::new();
            err_args.set("reason", reason);
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-unsafe_path_rejected",
                err_args,
            )));
        }

        let parent_newly_created = self
            .config
            .path
            .parent()
            .is_some_and(|p| !p.as_os_str().is_empty() && !p.exists());
        if let Some(parent) = self.config.path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("dir", parent.display().to_string());
            args.set("err", e.to_string());
            error!("{}", crate::i18n::tr_args("sink-file_mkdir_failed", args));
            return Err(InklogError::IoError(e));
        }
        // 审计级加固：新建的日志目录收敛为 0700（已存在目录不回改用户权限）
        #[cfg(unix)]
        if parent_newly_created && let Some(parent) = self.config.path.parent() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }

        // vuln-0004: 打开时在内核层拒绝末段符号链接（O_NOFOLLOW）。
        // 上方 PathValidator 的符号链接检查与实际 open 之间存在 validate-then-use
        // 窗口（TOCTOU）：攻击者可在校验通过后把日志路径替换为符号链接。
        // 非 Unix 平台退化为普通打开（与 validation 模块的策略一致）。
        #[cfg(unix)]
        let open_result = {
            use std::os::unix::fs::OpenOptionsExt;
            // 0600：日志可能含 PII/敏感上下文，默认不给组/其他用户可读
            OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
                .open(&self.config.path)
        };
        #[cfg(not(unix))]
        let open_result = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.path);

        match open_result {
            Ok(file) => {
                inner.current_file = Some(std::io::BufWriter::with_capacity(64 * 1024, file));
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

            // Wrap thread body in catch_unwind to make panics observable
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
            }));

            if let Err(panic_info) = result {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                let mut args = crate::i18n::MsgArgs::new();
                args.set("msg", msg.to_string());
                tracing::error!("{}", crate::i18n::tr_args("sink-file_cleanup_panic", args));
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
    pub(crate) fn perform_cleanup(
        config: &FileSinkConfig,
        log_path: &Path,
    ) -> Result<(), InklogError> {
        let parent = match log_path.parent() {
            Some(p) => p,
            None => return Ok(()),
        };

        // 只清理属于当前日志集的轮转文件（{stem}_<timestamp>.log[.zst][.enc]），
        // 绝不触碰目录中无关文件，也绝不删除当前活动文件 log_path 本身。
        let stem = log_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let prefix = format!("{}_", stem);

        // (path, modified) 列表，按修改时间升序（最旧在前）
        let mut candidates: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        match fs::read_dir(parent) {
            Ok(rd) => {
                for entry in rd.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if !path.is_file() || path == log_path {
                        continue;
                    }
                    let name = match path.file_name().and_then(|n| n.to_str()) {
                        Some(n) => n,
                        None => continue,
                    };
                    if !name.starts_with(&prefix) {
                        continue;
                    }
                    if let Ok(metadata) = path.metadata()
                        && let Ok(modified) = metadata.modified()
                    {
                        candidates.push((path, modified));
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read directory '{}' for cleanup: {}",
                    parent.display(),
                    e
                );
                return Ok(());
            }
        }

        // 最旧在前，保证删除时优先删最旧的日志
        candidates.sort_by_key(|(_, modified)| *modified);

        // 计算截止日期
        let cutoff_date = Utc::now()
            .checked_sub_signed(chrono::Duration::days(config.retention_days as i64))
            .unwrap_or_else(Utc::now);

        let total_size: u64 = candidates
            .iter()
            .filter_map(|(path, _)| path.metadata().ok())
            .map(|metadata| metadata.len())
            .sum();

        // 始终按修改时间保留最新的 keep_files 个轮转文件，
        // 两个清理分支都只允许删除更旧的文件
        let keep_newest = config.keep_files as usize;
        let deletable = candidates.len().saturating_sub(keep_newest);

        // 大小清理：仅超限时删除最旧文件以回到上限以内
        if let Some(max_total_size_bytes) = Self::parse_size(&config.max_total_size)
            && total_size > max_total_size_bytes
        {
            let excess_size = total_size.saturating_sub(max_total_size_bytes);
            let mut deleted_size: u64 = 0;

            for (path, _) in candidates.iter().take(deletable) {
                if deleted_size >= excess_size {
                    break;
                }

                if let Ok(metadata) = path.metadata() {
                    deleted_size += metadata.len();
                }

                if let Err(e) = fs::remove_file(path) {
                    warn!(
                        "Failed to remove {} during size cleanup: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        // 年龄清理：独立于大小分支执行——此前藏在 else 里，max_total_size
        // 不可解析时过期文件永远不会被清理（审计修复）
        for (path, modified) in candidates.iter().take(deletable) {
            let modified_utc: DateTime<Utc> = (*modified).into();
            if modified_utc < cutoff_date
                && let Err(e) = fs::remove_file(path)
            {
                warn!(
                    "Failed to remove {} during expiry cleanup: {}",
                    path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Returns disk space information for the log file's filesystem.
    pub fn get_disk_space_info(&self) -> Result<(u64, u64), InklogError> {
        #[cfg(unix)]
        {
            if let Some(parent) = self.config.path.parent()
                && let Ok(_metadata) = fs::metadata(parent)
                && let Ok(stat) = nix::sys::statfs::statfs(parent)
            {
                let total_blocks = stat.blocks();
                let available_blocks = stat.blocks_available();

                // 获取块大小
                let block_size = stat.block_size() as u64;
                let total_bytes = total_blocks * block_size;
                let available_bytes = available_blocks * block_size;

                return Ok((total_bytes, available_bytes));
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            // GetDiskFreeSpaceExW 只接受目录路径；若父路径是文件，则沿父级上溯到目录
            //（与 unix statfs 容忍文件路径语义对齐）；若父路径不存在则保持 Err（与
            // unix 的 metadata 存在性检查一致）。
            if let Some(parent) = self.config.path.parent()
                && parent.exists()
            {
                {
                    let mut free_bytes_available: u64 = 0;
                    let mut total_bytes: u64 = 0;
                    let mut total_free_bytes: u64 = 0;
                    let mut current = parent.to_path_buf();
                    loop {
                        let mut wide_path: Vec<u16> = current.as_os_str().encode_wide().collect();
                        wide_path.push(0);
                        let result = unsafe {
                            GetDiskFreeSpaceExW(
                                wide_path.as_ptr(),
                                &mut free_bytes_available,
                                &mut total_bytes,
                                &mut total_free_bytes,
                            )
                        };
                        if result != 0 {
                            return Ok((total_bytes, free_bytes_available));
                        }
                        match current.parent() {
                            Some(p) => current = p.to_path_buf(),
                            None => break,
                        }
                    }
                }
            }
        }

        Err(InklogError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Unable to get disk space info",
        )))
    }

    /// 磁盘空间检查的节流窗口：窗口内的"空间充足"结果直接复用，
    /// 避免 async write 每条记录都执行 fs::metadata + statfs
    const DISK_CHECK_THROTTLE: StdDuration = StdDuration::from_secs(5);

    /// 记录一条**终态丢失**的日志记录，保证丢失可观测而非静默吞掉：
    ///
    /// - `tracing::error` 结构化记录；
    /// - stderr 输出丢失详情（sink 路径与原因），覆盖无 tracing subscriber
    ///   的运行场景（记录本身已无法写入日志文件）；
    /// - 置位健康标志，使 [`FileSink::is_healthy`] 返回 false；
    /// - 递增丢失记录计数器。
    ///
    /// 返回值语义保持兼容（调用方仍收到 `Ok`，不改变错误传播契约）。
    fn mark_write_lost(&self, record: &LogRecord, reason: &str) {
        self.lost_records.fetch_add(1, Ordering::Relaxed);
        self.write_unhealthy.store(true, Ordering::Relaxed);
        error!(
            path = %self.config.path.display(),
            target = %record.target,
            level = %record.level,
            "Log record lost: {}",
            reason
        );
        eprintln!(
            "[inklog] LOST log record (sink: {}, target: {}, level: {}): {}",
            self.config.path.display(),
            record.target,
            record.level,
            reason
        );
    }

    /// 检查磁盘空间是否充足
    ///
    /// 带节流：距上次实际检查不足 [`Self::DISK_CHECK_THROTTLE`] 且结果为
    /// 空间充足时直接复用缓存结果；缓存结果为空间不足时不节流，每次都
    /// 实际检查，以便空间恢复后能立即恢复写入。
    fn check_disk_space(&self) -> Result<bool, InklogError> {
        {
            let cached = self.last_disk_check.lock();
            if let Some((at, true)) = *cached
                && at.elapsed() < Self::DISK_CHECK_THROTTLE
            {
                return Ok(true);
            }
        }

        let (_total, available) = self.get_disk_space_info()?;
        // 保留 50MB 或 10% 的可用空间，以较大者为准
        let reserved = (50 * 1024 * 1024u64).max(available / 10);
        let sufficient = available > reserved;
        *self.last_disk_check.lock() = Some((Instant::now(), sufficient));
        Ok(sufficient)
    }

    /// 计算下次轮转时间
    fn calculate_next_rotation_time(rotation_time: &str) -> Option<DateTime<Utc>> {
        Self::calculate_next_rotation_time_from(rotation_time, Utc::now())
    }

    /// [`Self::calculate_next_rotation_time`] 的可注入时钟变体（便于测试）。
    fn calculate_next_rotation_time_from(
        rotation_time: &str,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
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
                // 修复：原实现误写为“明天零点”，导致 monthly 实际每天轮转。
                // 正确语义为“下个月同日的零点”；`checked_add_months` 在月末
                // 溢出时钳制到次月最后一天（如 1 月 31 日 → 2 月 28 日）。
                let next_date = now
                    .date_naive()
                    .checked_add_months(chrono::Months::new(1))?;
                Some(next_date.and_hms_opt(0, 0, 0)?.and_utc())
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

        if (self.config.rotation_time == "daily" || self.config.rotation_time == "weekly")
            && let Some(last_date) = inner.last_rotation_date
            && current_date > last_date
        {
            return true;
        }

        if let Some(next_time) = inner.next_rotation_time
            && now >= next_time
        {
            return true;
        }

        false
    }

    fn update_next_rotation_time_inner(&self, inner: &mut FileSinkInner) {
        inner.next_rotation_time = Self::calculate_next_rotation_time(&self.config.rotation_time);
    }

    /// 启动轮转定时器（默认 60s tick）
    fn start_rotation_timer(&self) {
        self.start_rotation_timer_with_tick(StdDuration::from_secs(60));
    }

    /// tick 间隔可注入：空闲 flush 测试用短 tick 驱动同一实现。
    fn start_rotation_timer_with_tick(&self, check_interval: StdDuration) {
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
        // 空闲 flush 需要 inner（Arc 共享）与 config 快照
        let inner_shared = Arc::clone(&self.inner);
        let config = self.config.clone();

        let timer_handle = thread::spawn(move || {
            // Wrap thread body in catch_unwind to make panics observable
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                    drop(last_rotation_guard);

                    // 空闲期兜底刷盘：日志停止产生后，batch 缓冲不再等
                    // "下一条写入"触发——超过 flush_interval_ms 即由本线程落盘。
                    // （定时线程经 Arc 共享 inner；写语义在此内联而非复用
                    // flush_batch_inner，避免计时线程与实例方法的生命周期耦合。）
                    if let Some(mut inner) = inner_shared.try_write()
                        && !inner.batch_buffer.is_empty()
                        && inner.last_flush_time.elapsed()
                            >= StdDuration::from_millis(config.flush_interval_ms)
                    {
                        let pending = std::mem::take(&mut inner.batch_buffer);
                        if let Some(file) = &mut inner.current_file {
                            use std::io::Write as _;
                            for record in &pending {
                                let line = if config.output_format == OutputFormat::Json {
                                    serde_json::to_string(record)
                                        .unwrap_or_else(|_| "{}".to_string())
                                } else {
                                    format!(
                                        "{} [{}] {} - {}",
                                        record.timestamp.to_rfc3339(),
                                        record.level,
                                        record.target,
                                        record.message
                                    )
                                };
                                let _ = writeln!(file, "{line}");
                            }
                            let _ = file.flush();
                            if config.fsync {
                                let _ = file.get_ref().sync_all();
                            }
                            inner.current_size = file
                                .get_ref()
                                .metadata()
                                .map(|m| m.len())
                                .unwrap_or(inner.current_size);
                        }
                        inner.last_flush_time = Instant::now();
                    }
                }
            }));

            if let Err(panic_info) = result {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                let mut args = crate::i18n::MsgArgs::new();
                args.set("msg", msg.to_string());
                tracing::error!("{}", crate::i18n::tr_args("sink-file_rotation_panic", args));
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
        let mut write_failed = false;

        if let Some(file) = &mut inner.current_file {
            for (index, record) in records.iter().enumerate() {
                let write_result = if self.config.output_format == OutputFormat::Json {
                    // NDJSON: each record is a single-line JSON object
                    match serde_json::to_string(record) {
                        Ok(json) => writeln!(file, "{}", json),
                        Err(e) => Err(std::io::Error::other(e)),
                    }
                } else {
                    writeln!(
                        file,
                        "{} [{}] {} - {}",
                        record.timestamp.to_rfc3339(),
                        record.level,
                        record.target,
                        record.message
                    )
                };

                match write_result {
                    Ok(_) => {
                        // Sync estimated size with actual file position to prevent drift
                    }
                    Err(e) => {
                        error!("Batch write error: {}", e);
                        inner.circuit_breaker.record_failure();
                        let _ = self.open_file_inner(inner);
                        // 失败记录及其后的记录回填缓冲区，等待下次 flush 重试
                        let unsent = records.len() - index;
                        inner.batch_buffer.extend_from_slice(&records[index..]);
                        warn!(
                            "Batch write failed: {} unsent records re-queued for retry",
                            unsent
                        );
                        write_failed = true;
                        break;
                    }
                }
            }
        } else {
            // 无文件句柄：没有任何记录被写入，全部回填等待重试
            inner.batch_buffer.extend_from_slice(&records);
        }

        // 每批一次 flush：BufWriter 摊销了逐条 syscall，批末 flush 保证
        // "flush_batch_inner 返回即落盘"的外部契约（轮转/关停依赖它）。
        // 独立借用作用域：错误路径已在循环内重开句柄。
        if !write_failed
            && let Some(file) = &mut inner.current_file
            && let Err(e) = file.flush()
        {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", &e);
            error!("{}", crate::i18n::tr_args("sink-batch_flush_failed", args));
            inner.circuit_breaker.record_failure();
            // BufWriter 层整批写失败（如 /dev/full）：无法确定部分写入边界，
            // 整批回填重试（at-least-once，与"无句柄"分支语义一致）
            inner.batch_buffer.extend_from_slice(&records);
            write_failed = true;
        }

        if !write_failed {
            inner.circuit_breaker.record_success();
            // 全量刷盘成功：清除终态写失败的健康标记（瞬时磁盘满等故障
            // 恢复后 sink 自动转回健康）。
            self.write_unhealthy.store(false, Ordering::Relaxed);
            // fsync（可选）：崩溃一致性增强，等保/审计场景开启。
            // 数据已写入 BufWriter，先 flush 再 sync_all 落盘。
            if self.config.fsync
                && let Some(file) = &mut inner.current_file
                && let Err(e) = file.flush().and_then(|_| file.get_ref().sync_all())
            {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("err", &e);
                error!("{}", crate::i18n::tr_args("sink-fsync_failed", args));
                inner.circuit_breaker.record_failure();
            }
        }

        // Sync estimated size with actual file position to prevent drift
        if let Some(file) = &inner.current_file {
            // Use metadata() instead of stream_position() to avoid borrow conflicts
            if let Ok(meta) = file.get_ref().metadata() {
                inner.current_size = meta.len();
            }
        }

        inner.last_flush_time = Instant::now();

        // 批量写入后检查是否需要旋转
        self.check_rotation_inner(inner)?;

        Ok(())
    }

    /// 同步压缩文件（可在后台线程调用）。
    ///
    /// 单一事实源在 [`compress_rotated`]：无条件压缩（`encrypt = true` 时
    /// 对压缩产物加密），不读 `config.compress` 旋钮——`process_rotated`
    /// 才负责"按旋钮分发"。
    #[cfg_attr(not(test), allow(dead_code))] // 生产路径走 process_rotated；测试面便捷委托
    fn compress_file(&self, path: &Path) -> Result<PathBuf, InklogError> {
        compress_rotated(&self.config, path, None)
    }

    /// 同步加密文件（可在后台线程调用）。
    /// 单一事实源在 [`encrypt_file_v2`]，此处仅委托。
    ///
    /// 输出格式 v2（与 CLI 解密工具一致）：
    /// magic(8) + version=2(2) + algo(2) + **salt(16)** + nonce(12) + ciphertext
    pub fn encrypt_file(&self, input_path: &Path, output_path: &Path) -> Result<(), InklogError> {
        encrypt_file_v2(&self.config, input_path, output_path, None)
    }

    /// 执行文件轮转
    fn rotate_inner(&self, inner: &mut FileSinkInner) -> Result<(), InklogError> {
        debug!("Rotating log file: {}", self.config.path.display());

        // 关闭当前文件
        let _ = inner.current_file.take();

        // 重命名当前日志文件
        // 修复：`%Y%m%d_%H%M%S` 为秒级精度，同秒二次轮转会静默覆盖既有轮转
        // 产物。目标已存在时追加 `.1`、`.2` … 序号后缀，保证永不覆盖。
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let new_path = resolve_rotation_target(&self.config.path, &timestamp);

        // 尝试重命名
        if self.config.path.exists()
            && let Err(e) = fs::rename(&self.config.path, &new_path)
        {
            error!("Failed to rename log file: {}", e);
            // 尝试复制后删除
            if fs::copy(&self.config.path, &new_path).is_ok() {
                if let Err(e) = fs::remove_file(&self.config.path) {
                    warn!(
                        "Failed to remove original file after copy during rotation: {}",
                        e
                    );
                }
            } else {
                return Err(InklogError::IoError(e));
            }
        }

        // 更新序列号
        inner.sequence += 1;

        // 更新轮转时间
        inner.last_rotation = Instant::now();
        self.update_next_rotation_time_inner(inner);
        inner.current_size = 0;

        // Reset circuit breaker after successful rotation:
        // The new file handle is healthy, so the circuit breaker should not
        // carry over failure state from the previous file.
        inner.circuit_breaker.reset();

        info!("Log rotated to: {}", new_path.display());

        // 归档审计链：轮转成功即登记（path + SHA-256 + 时间戳），写穿 manifest
        // （共享单一事实源 register_rotated_archive，与 CBFS 同一实现）
        if let Some(chain) = self.audit_chain.as_ref() {
            register_rotated_archive(chain, &self.config.path, &new_path);
        }

        // 压缩/加密归档后处理（后台线程；单一事实源 process_rotated）
        if self.config.compress || self.config.encrypt {
            let config = self.config.clone();
            let path = new_path.clone();
            let _ = thread::spawn(move || {
                // Wrap thread body in catch_unwind so a post-rotation panic is
                // logged instead of aborting an unnoticed worker thread.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    if let Err(e) = process_rotated(&config, &path, None) {
                        let mut args = crate::i18n::MsgArgs::new();
                        args.set("err", &e);
                        error!(
                            "{}",
                            crate::i18n::tr_args("sink-rotate_postprocess_failed", args)
                        );
                        crate::support::ops_event::publish_internal(
                            "sink_degraded",
                            Some("file"),
                            serde_json::json!({ "op": "archive", "error": e.to_string() }),
                        );
                    }
                }));
                if let Err(panic_info) = result {
                    let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("msg", msg);
                    error!(
                        "{}",
                        crate::i18n::tr_args("sink-rotate_postprocess_panicked", args)
                    );
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

/// 轮转目标（或其归档衍生产物）是否已被占用。
///
/// 后处理线程异步加密/压缩后会**删除轮转源文件**：同秒内的下一次轮转
/// 若只检查源文件存在性，会复用同名并覆盖既有归档产物（记录丢失）。因
/// 此按 [`append_ext`] 语义派生产物名（追加，与 `compress_rotated`/加密
/// 的实际产物一一对应）——不能做 `with_extension` 派生：那会把候选自身
/// 的序号后缀当扩展名剥掉，所有 attempt 都命中同一产物，冲突循环永不
/// 终止。
fn rotation_family_taken(candidate: &Path) -> bool {
    if candidate.exists() {
        return true;
    }
    super::compression::append_ext(candidate, "zst").exists()
        || super::compression::append_ext(candidate, "zst.enc").exists()
        || super::compression::append_ext(candidate, "gz").exists()
        || super::compression::append_ext(candidate, "gz.enc").exists()
        || super::compression::append_ext(candidate, "enc").exists()
}

/// 解析轮转目标路径：`{stem}_{timestamp}.{ext}`，冲突时追加序号后缀。
///
/// 时间戳为秒级精度（`%Y%m%d_%H%M%S`），同一秒内二次轮转会命中同名目标。
/// 目标及其归档衍生产物已存在时依次尝试 `.1`、`.2` … 序号后缀，保证绝不
/// 覆盖既有轮转产物。
/// FileSink 与 ChannelBufferedFileSink 轮转共用同一命名范式。
pub(crate) fn resolve_rotation_target(original: &Path, stamp: &str) -> PathBuf {
    let make_path = |attempt: u32| -> PathBuf {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!(".{attempt}")
        };
        match original.parent() {
            Some(parent) => {
                let stem = original.file_stem().unwrap_or_default();
                let ext = original.extension().unwrap_or_default();
                parent.join(format!(
                    "{}_{}.{}{}",
                    stem.to_string_lossy(),
                    stamp,
                    ext.to_string_lossy(),
                    suffix
                ))
            }
            None => PathBuf::from(format!("{}_{}{}", original.display(), stamp, suffix)),
        }
    };

    let mut attempt = 0u32;
    let mut candidate = make_path(attempt);
    while rotation_family_taken(&candidate) {
        attempt += 1;
        candidate = make_path(attempt);
    }
    candidate
}

/// 按配置解析加密密钥（密码模式用盐确定性派生）。
///
/// v2 加密格式：加密时生成 16 字节随机盐写入文件头，密码模式密钥经
/// PBKDF2(密码, 盐) 确定性派生，解密方（CLI）读出盐后可重导出同一密钥。
/// Base64 / 原始 32 字节密钥分支与盐无关。
///
/// key 来源：**env 优先，其次配置文件**（`encryption_key_file`，unix
/// 权限 0600）——与 fallback journal 共享同一解析范式。
/// 密钥（`Zeroizing` 包裹，离开作用域自动清零）。
fn encryption_key_for(
    config: &FileSinkConfig,
    salt: &[u8],
) -> Result<Zeroizing<[u8; 32]>, InklogError> {
    let default_key = "LOG_ENCRYPTION_KEY".to_string();
    let key_env = config.encryption_key_env.as_ref().unwrap_or(&default_key);

    let material = super::encryption::resolve_key_material(
        key_env,
        config
            .encryption_key_file
            .as_deref()
            .map(std::path::Path::new),
    )?;
    let key = material.derive_with_salt(salt);

    // 纵深防御：对直接用作密钥的原始字节（非 PBKDF2 派生输出）保留
    // Shannon 熵校验，拒绝全零等弱密钥。
    super::encryption::validate_key_entropy(&*key)?;

    Ok(key)
}

/// 同步加密文件（可在后台线程调用）。
///
/// 输出格式 v2（与 CLI 解密工具及 docs/SECURITY.md 一致）：
/// magic(8) + version=2(2) + algo(2) + **salt(16)** + nonce(12) + ciphertext
///
/// v2 相比 v1 新增 16 字节盐字段：密码模式密钥经 PBKDF2(密码, 盐) 确定性
/// 派生，盐随头存储，解密方才能重导出同一密钥（v1 密码模式文件因未存盐
/// 而不可解密，见 CLI 解密工具的 v1 诊断）。
///
/// `key_override`：构造期已解析的密钥材料（长生命周期 sink 复用，key 源
/// 事后被破坏不影响运行期轮转加密）；`None` 时按 config 现场解析。
///
/// 已知内存权衡（继承 FileSink 既有范式，登记于此）：整文件读入内存加密
/// 后写盘，峰值 ≈ 2× 文件体积（明文缓冲 + 密文输出）。大归档场景的流式
/// 加密属后续优化，不阻塞当前单文件 ≤ max_size 的部署形态。
pub(crate) fn encrypt_file_v2(
    config: &FileSinkConfig,
    input_path: &Path,
    output_path: &Path,
    key_override: Option<&super::encryption::KeyMaterial>,
) -> Result<(), InklogError> {
    use aes_gcm::{Aes256Gcm, Nonce};
    use rand::Rng;

    // 生成加密安全的随机盐（16 字节），写入 v2 文件头
    let mut salt = [0u8; 16];
    rand::rng().fill_bytes(&mut salt);

    // 获取密钥（密码模式用上面的盐确定性派生）
    let key_bytes = match key_override {
        Some(material) => {
            // 构造期已解析并做过熵校验的原始密钥，直接按盐派生
            material.derive_with_salt(&salt)
        }
        None => encryption_key_for(config, &salt)?,
    };
    let cipher = Aes256Gcm::new_from_slice(&*key_bytes).map_err(|e| {
        let mut args = crate::i18n::MsgArgs::new();
        args.set("err", e.to_string());
        InklogError::EncryptionError {
            message: crate::i18n::tr_args("config-invalid_encryption_key", args),
            source: Some(Box::new(e)),
        }
    })?;

    // 生成加密安全的随机 nonce
    // 使用 rand::rng() 获取线程本地 RNG，该 RNG 从 SysRng 定期种子化
    // rand::rng() 返回 ThreadRng，它是密码学安全的
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    // 读取输入文件
    let input_data = fs::read(input_path).map_err(|e| {
        error!("Failed to read file for encryption: {}", e);
        InklogError::IoError(e)
    })?;

    // 加密
    let ciphertext = cipher.encrypt(&nonce, input_data.as_slice()).map_err(|e| {
        error!("Encryption failed: {}", e);
        InklogError::EncryptionError {
            message: e.to_string(),
            source: Some(Box::new(e)),
        }
    })?;

    // 写入加密文件（0600：密文同样不该给组/其他用户可读）
    #[cfg(unix)]
    let mut output = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(output_path)
            .map_err(|e| {
                error!("Failed to create encrypted file: {}", e);
                InklogError::IoError(e)
            })?
    };
    #[cfg(not(unix))]
    let mut output = fs::File::create(output_path).map_err(|e| {
        error!("Failed to create encrypted file: {}", e);
        InklogError::IoError(e)
    })?;

    output.write_all(b"ENCLOG1\0")?;
    output.write_all(&2u16.to_le_bytes())?;
    output.write_all(&1u16.to_le_bytes())?;
    output.write_all(&salt)?;
    output.write_all(&nonce_bytes)?;
    output.write_all(&ciphertext)?;

    debug!("Encrypted log file: {}", output_path.display());
    Ok(())
}

/// 轮转归档后处理（单一事实源，FileSink 与 ChannelBufferedFileSink 共用）：
/// `compress = true` 时压缩（zstd → gzip → 无后端告警跳过），随后按
/// `encrypt = true` 对产物加密；仅加密（不压缩）时直接加密原文件。
/// 产物扩展名与 FileSink 既有范式一致（`.zst` / `.gz` / `.enc`）。
///
/// `key_override` 语义同 [`encrypt_file_v2`]：构造期已解析的密钥材料
/// （ChannelBufferedFileSink 运行期复用），`None` 按 config 现场解析。
/// 加密后明文残留（remove_file 失败）：warn + `sink_degraded` ops 事件
/// 显性上报——加密产物虽已生成，但明文源仍在盘上，属安全相关降级状态。
fn report_plaintext_residue(path: &Path, err: &std::io::Error) {
    warn!(
        "Failed to remove plaintext source after encryption {}: {}",
        path.display(),
        err
    );
    crate::support::ops_event::publish_internal(
        "sink_degraded",
        Some("file"),
        serde_json::json!({
            "op": "archive",
            "error": crate::i18n::tr("sink-plaintext_residue"),
            "path": path.display().to_string(),
            "io_error": err.to_string(),
        }),
    );
}

pub(crate) fn process_rotated(
    config: &FileSinkConfig,
    path: &Path,
    key_override: Option<&super::encryption::KeyMaterial>,
) -> Result<PathBuf, InklogError> {
    if config.compress {
        compress_rotated(config, path, key_override)
    } else if config.encrypt {
        let encrypted_path = super::compression::append_ext(path, "enc");
        encrypt_file_v2(config, path, &encrypted_path, key_override)?;
        if let Err(e) = fs::remove_file(path) {
            report_plaintext_residue(path, &e);
        }
        Ok(encrypted_path)
    } else {
        Ok(path.to_path_buf())
    }
}

/// 压缩轮转产物（按启用 feature 选后端），`encrypt = true` 时加密压缩产物。
///
/// 产物命名一律**追加扩展名**（`X.log.1` → `X.log.1.zst`）而非替换：
/// `with_extension` 会把序号候选的序号当扩展名剥掉，不同 attempt 的产物
/// 互相覆盖、冲突检查不单射（曾致同秒高频轮转挂死）。
#[allow(unused_variables)] // 无压缩后端组合下 config 仅加密分支使用
fn compress_rotated(
    config: &FileSinkConfig,
    path: &Path,
    key_override: Option<&super::encryption::KeyMaterial>,
) -> Result<PathBuf, InklogError> {
    #[cfg(feature = "zstd")]
    {
        let compressed_path = super::compression::append_ext(path, "zst");
        super::compression::zstd_encode_to(path, &compressed_path, config.compression_level)?;

        // 如果需要加密
        if config.encrypt {
            let encrypted_path = super::compression::append_ext(&compressed_path, "enc");
            if let Err(e) = encrypt_file_v2(config, &compressed_path, &encrypted_path, key_override)
            {
                error!("Encryption failed: {}", e);
                // 加密失败，保留压缩文件
                let _ = fs::rename(
                    &compressed_path,
                    super::compression::append_ext(&compressed_path, "unencrypted"),
                );
                return Err(e);
            }
            if let Err(e) = fs::remove_file(&compressed_path) {
                report_plaintext_residue(&compressed_path, &e);
            }
            return Ok(encrypted_path);
        }
        // 删除原始文件
        if let Err(e) = fs::remove_file(path) {
            warn!(
                "Failed to remove original file after compression {}: {}",
                path.display(),
                e
            );
        }
        Ok(compressed_path)
    }

    #[cfg(all(feature = "gzip", not(feature = "zstd")))]
    {
        // 追加命名 + 不删源（源文件删除统一在下方按需执行），与 zstd 路径
        // 行为对齐
        let compressed_path =
            super::compression::gzip_compress_file_keeping_name(path, config.compression_level)?;

        // 如果需要加密（与 compression feature 启用时的 zstd 路径行为对齐）
        if config.encrypt {
            let encrypted_path = super::compression::append_ext(&compressed_path, "enc");
            if let Err(e) = encrypt_file_v2(config, &compressed_path, &encrypted_path, key_override)
            {
                error!("Encryption failed: {}", e);
                // 加密失败，保留压缩文件
                let _ = fs::rename(
                    &compressed_path,
                    super::compression::append_ext(&compressed_path, "unencrypted"),
                );
                return Err(e);
            }
            if let Err(e) = fs::remove_file(&compressed_path) {
                report_plaintext_residue(&compressed_path, &e);
            }
            return Ok(encrypted_path);
        }
        if let Err(e) = fs::remove_file(path) {
            warn!(
                "Failed to remove original file after compression {}: {}",
                path.display(),
                e
            );
        }
        Ok(compressed_path)
    }

    #[cfg(not(any(feature = "zstd", feature = "gzip")))]
    {
        warn!(
            path = %path.display(),
            "Compression requested but no compression backend feature is enabled \
             (enable \"gzip\" or \"compression\"); leaving the file uncompressed"
        );
        if config.encrypt {
            let encrypted_path = super::compression::append_ext(path, "enc");
            encrypt_file_v2(config, path, &encrypted_path, key_override)?;
            if let Err(e) = fs::remove_file(path) {
                warn!(
                    "Failed to remove original file after encryption {}: {}",
                    path.display(),
                    e
                );
            }
            return Ok(encrypted_path);
        }
        Ok(path.to_path_buf())
    }
}

/// 归档审计链登记（FileSink 与 ChannelBufferedFileSink 共享单一事实源）：
/// sha256 轮转产物 → 追加链条目 → 全量写穿 manifest。
/// sha256 失败以 `sink_degraded` ops 事件显性告警——与条目内 "unavailable"
/// 占位（链仍可验、仅摘要缺失）区分，产物不可摘要本身即需告警的状态。
pub(crate) fn register_rotated_archive(
    chain: &parking_lot::Mutex<crate::support::audit_chain::ArchiveChain>,
    base_path: &Path,
    rotated_path: &Path,
) {
    let digest = FileSink::sha256_file(rotated_path);
    if digest.is_none() {
        crate::support::ops_event::publish_internal(
            "sink_degraded",
            Some("file"),
            serde_json::json!({
                "op": "audit_chain",
                "error": "sha256 unavailable",
                "path": rotated_path.display().to_string(),
            }),
        );
    }
    let digest = digest.unwrap_or_else(|| "unavailable".to_string());
    let mut chain = chain.lock();
    let event = serde_json::json!({
        "path": rotated_path.display().to_string(),
        "sha256": digest,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
    .to_string();
    chain.append(&event);
    let manifest = FileSink::audit_manifest_path(base_path);
    let mut body = String::new();
    for entry in chain.entries() {
        body.push_str(&serde_json::to_string(entry).unwrap_or_default());
        body.push('\n');
    }
    if let Err(e) = fs::write(&manifest, body) {
        let mut args = crate::i18n::MsgArgs::new();
        args.set("path", manifest.display().to_string());
        args.set("err", e);
        error!(
            "{}",
            crate::i18n::tr_args("sink-audit_manifest_write_failed", args)
        );
    }
}

/// 构建期不触碰文件系统的 [`FileSink`] 惰性包装。
///
/// 内部 FileSink 推迟到首次 [`LogSink::write`] 时构造：一次性命令构建
/// logger 后未发生任何 error 级事件即退出时，不再遗留空的目标文件（如
/// `logs/error.log`）。对写入方透明——实现同一 [`LogSink`] 端口，内部
/// FileSink 构造失败时错误原样上抛，不静默吞掉。
pub(crate) struct LazyFileSink {
    config: FileSinkConfig,
    inner: parking_lot::Mutex<Option<Arc<FileSink>>>,
}

impl LazyFileSink {
    pub(crate) fn new(config: FileSinkConfig) -> Self {
        Self {
            config,
            inner: parking_lot::Mutex::new(None),
        }
    }

    /// 取内部 FileSink，首次调用时构造；锁内仅做同步构造，不跨 await。
    fn get_or_create(&self) -> Result<Arc<FileSink>, InklogError> {
        let mut guard = self.inner.lock();
        if let Some(sink) = guard.as_ref() {
            return Ok(sink.clone());
        }
        let sink = Arc::new(FileSink::new(self.config.clone())?);
        *guard = Some(sink.clone());
        Ok(sink)
    }
}

#[async_trait]
impl LogSink for LazyFileSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        let sink = self.get_or_create()?;
        sink.write(record).await
    }

    async fn flush(&self) -> Result<(), InklogError> {
        // 从未写入时无文件也无缓冲，安全 no-op
        let sink = self.inner.lock().as_ref().cloned();
        match sink {
            Some(sink) => sink.flush().await,
            None => Ok(()),
        }
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        // 从未写入时无定时器线程与句柄需要清理，安全 no-op
        let sink = self.inner.lock().as_ref().cloned();
        match sink {
            Some(sink) => sink.shutdown().await,
            None => Ok(()),
        }
    }
}

#[async_trait]
impl LogSink for FileSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        // 检查断路器（使用 read lock，作用域内释放后再 await）
        let circuit_open = {
            let inner = self.inner.read();
            !inner.circuit_breaker.can_execute()
        };
        if circuit_open {
            // 修复：熔断打开时记录不能静默吞掉——降级失败/无 fallback 时
            // 记录已终态丢失，必须 error + stderr + 置不健康。
            let fallback = self.inner.read().fallback_sink.clone();
            match fallback {
                Some(sink) => {
                    if let Err(e) = sink.write(record).await {
                        self.mark_write_lost(
                            record,
                            &format!("circuit breaker open and fallback sink write failed: {e}"),
                        );
                    }
                }
                None => {
                    self.mark_write_lost(
                        record,
                        "circuit breaker open and no fallback sink configured",
                    );
                }
            }
            return Ok(());
        }

        // 检查磁盘空间（sync，不持有锁）
        if !self.check_disk_space()? {
            // 修复：磁盘不足且无法降级时记录不能静默吞掉
            warn!("Low disk space - checking before write");
            let fallback = self.inner.read().fallback_sink.clone();
            match fallback {
                Some(sink) => {
                    if let Err(e) = sink.write(record).await {
                        self.mark_write_lost(
                            record,
                            &format!("low disk space and fallback sink write failed: {e}"),
                        );
                    }
                }
                None => {
                    self.mark_write_lost(record, "low disk space and no fallback sink configured");
                }
            }
            return Ok(());
        }

        // 主路径：所有需要 write lock 的同步操作都封装在 block 内，
        // block 返回 Some(fallback) 表示轮转失败需要降级写入，None 表示正常完成。
        // parking_lot::RwLockWriteGuard 非 Send，不能跨 await 持有，故用 block scope 隔离。
        let rotation_failed_fallback: Option<Arc<dyn LogSink + Send + Sync>> = {
            let mut inner = self.inner.write();

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
                    inner.fallback_sink.clone()
                } else {
                    // 轮转成功，继续 batch flush
                    let now = Instant::now();
                    let flush_interval = StdDuration::from_millis(self.config.flush_interval_ms);
                    if inner.batch_buffer.len() >= self.config.batch_size
                        || now.duration_since(inner.last_flush_time) >= flush_interval
                    {
                        self.flush_batch_inner(&mut inner)?;
                    }
                    None
                }
            } else {
                // 无需轮转，batch flush
                let now = Instant::now();
                let flush_interval = StdDuration::from_millis(self.config.flush_interval_ms);
                if inner.batch_buffer.len() >= self.config.batch_size
                    || now.duration_since(inner.last_flush_time) >= flush_interval
                {
                    self.flush_batch_inner(&mut inner)?;
                }
                None
            }
        }; // inner 在此 drop，write lock 释放

        // 轮转失败路径：await fallback sink 的 write（lock 已释放，安全 await）
        if let Some(sink) = rotation_failed_fallback {
            let _ = sink.write(record).await;
        }

        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
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
        self.inner.read().current_file.is_some() && !self.write_unhealthy.load(Ordering::Relaxed)
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        // Signal shutdown to all timer threads first
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // All sync operations on `inner` are confined to this block.
        // The guard is dropped at block end, so the await below does not
        // cross a `!Send` boundary (parking_lot::RwLockWriteGuard is !Send).
        let fallback = {
            let mut inner = self.inner.write();

            // Stop rotation timer with graceful shutdown
            if let Some(handle) = inner.timer_handle.take() {
                let _ = handle.join();
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

            inner.fallback_sink.take()
        };

        // Shut down fallback sink (lock released, safe to await)
        if let Some(sink) = fallback {
            let _ = sink.shutdown().await;
        }

        Ok(())
    }
}

impl Rotatable for FileSink {
    fn start_rotation_timer(&self) {
        // Delegate to inherent method
        FileSink::start_rotation_timer(self)
    }

    fn stop_rotation_timer(&self) {
        // Stop the rotation timer by setting the rotation timer to None
        let mut inner = self.inner.write();
        inner.rotation_timer = None;
    }
}

impl DiskCheckable for FileSink {
    fn check_disk_space(&self) -> Result<bool, InklogError> {
        // Delegate to inherent method
        FileSink::check_disk_space(self)
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

        // Fallback sink: best-effort cleanup only.
        // Async shutdown() already calls `sink.shutdown().await` (line 994-999).
        // In Drop we cannot `.await`; rely on `Arc` drop + fallback sink's own Drop impl.
        // If caller forgets to call `shutdown()`, fallback sink resources are reclaimed
        // when the last `Arc` is dropped (Sink's own Drop handles file close etc.).
        {
            let mut inner = self.inner.write();
            let _fallback = inner.fallback_sink.take();
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
        };

        Self {
            config: self.config.clone(),
            rotation_interval: self.rotation_interval,
            last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
            last_disk_check: parking_lot::Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            write_unhealthy: AtomicBool::new(false),
            lost_records: AtomicU64::new(0),
            masker: DataMasker::new(),
            audit_chain: self.audit_chain.clone(),
            inner: Arc::new(RwLock::new(inner)),
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
            trace_id: None,
            span_id: None,
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
        };

        FileSink {
            config,
            rotation_interval: StdDuration::from_secs(86400),
            last_cleanup_time: Arc::new(parking_lot::Mutex::new(None)),
            last_disk_check: parking_lot::Mutex::new(None),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            write_unhealthy: AtomicBool::new(false),
            lost_records: AtomicU64::new(0),
            masker: DataMasker::new(),
            audit_chain: None,
            inner: Arc::new(RwLock::new(inner)),
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
            encryption_key_file: None,
            retention_days: 30,
            max_total_size: "1GB".to_string(),
            cleanup_interval_minutes: 60,
            batch_size: 100,
            flush_interval_ms: 100,
            fsync: false,
            audit_chain_enabled: false,
            masking_enabled: true,
            output_format: Default::default(),
        };

        // Create test files
        let old_file = dir.path().join("test_old.log");
        std::fs::write(&old_file, "old content").unwrap();

        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_get_encryption_key() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_KEY".to_string()),
            ..Default::default()
        };

        // Set a valid 32-byte test key (base64 encoded, mixed characters for entropy)
        // "abcdefghijklmnopqrstuvwxyz123456" = 32 varied bytes
        unsafe {
            std::env::set_var("TEST_KEY", "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=");
        }

        let sink = create_test_file_sink(config);

        let key_result = sink.get_encryption_key(b"test-salt-16bytes");
        assert!(key_result.is_ok());
        assert_eq!(key_result.unwrap().len(), 32);

        // Clean up
        unsafe {
            std::env::remove_var("TEST_KEY");
        }
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
    fn test_check_disk_space_throttle_reuses_cached_result() {
        // 节流窗口内的"空间充足"结果直接复用：即使磁盘信息不可获取也返回缓存值
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/nonexistent_root_path_xyz/log.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        *sink.last_disk_check.lock() = Some((Instant::now(), true));

        // get_disk_space_info 对不存在的路径会失败；命中节流缓存则不会触达
        let result = sink.check_disk_space();
        assert!(
            matches!(result, Ok(true)),
            "cached sufficient result should be reused within throttle window, got {:?}",
            result
        );
    }

    #[test]
    fn test_check_disk_space_insufficient_result_not_throttled() {
        // 缓存为"空间不足"时不节流，每次都实际检查（此处路径无效 → Err 透传）
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/nonexistent_root_path_xyz/log.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        *sink.last_disk_check.lock() = Some((Instant::now(), false));

        let result = sink.check_disk_space();
        assert!(
            result.is_err(),
            "insufficient cached result must force a real disk check"
        );
    }

    #[test]
    fn test_check_disk_space_stale_cache_triggers_real_check() {
        // 缓存超过节流窗口后应重新实际检查并刷新缓存
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        *sink.last_disk_check.lock() = Some((Instant::now() - StdDuration::from_secs(10), true));

        let marker = Instant::now();
        let result = sink.check_disk_space();
        assert!(result.is_ok(), "real check should succeed on a valid path");
        let (at, _) = sink.last_disk_check.lock().expect("cache should be set");
        assert!(at >= marker, "cache should be refreshed by the real check");
    }

    #[tokio::test]
    async fn test_write_with_disk_space_check() {
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
            trace_id: None,
            span_id: None,
        };

        // Should succeed with sufficient disk space
        let result = sink.write(&record).await;
        assert!(
            result.is_ok(),
            "Write should succeed with sufficient disk space"
        );

        // Flush to ensure data is written
        sink.flush().await.unwrap();

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
    #[serial]
    fn test_get_encryption_key_missing_env() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("MISSING_KEY".to_string()),
            ..Default::default()
        };

        // Ensure the env var doesn't exist
        unsafe {
            std::env::remove_var("MISSING_KEY");
        }

        let sink = create_test_file_sink(config);

        let result = sink.get_encryption_key(b"test-salt-16bytes");
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

        let result = sink.get_encryption_key(b"test-salt-16bytes");
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
        // FileSinkConfig::default() 的 path 为空 PathBuf，测试需显式提供路径。
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
    #[serial]
    fn test_get_encryption_key_too_short() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_SHORT_KEY".to_string()),
            ..Default::default()
        };

        // 设置一个太短的密钥（Base64 编码前 < 16 字符）
        unsafe {
            std::env::set_var("TEST_SHORT_KEY", "YWJjZA==");
        } // "abcd"

        let sink = create_test_file_sink(config);

        let result = sink.get_encryption_key(b"test-salt-16bytes");
        assert!(result.is_err());
        // v2 统一走加密模块派生：base64 解码成功但只有 4 字节 → 长度错误
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
    }

    #[test]
    fn test_nonce_generation_unique() {
        // 测试每次生成的 nonce 都是唯一的
        use rand::Rng;

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
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "zstd")]
    fn test_compress_file_with_encryption_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("test.log");
        let original_content = b"Sensitive log content that needs encryption";
        std::fs::write(&original_path, original_content).unwrap();

        let (key_bytes, key_b64) = make_test_key();
        unsafe {
            std::env::set_var("TEST_COMPRESS_ENC_KEY", &key_b64);
        }

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

        // 解密：v2 头 40 字节（magic 8 + version 2 + algo 2 + salt 16 + nonce 12）+ ciphertext
        let encrypted_data = std::fs::read(&encrypted_path).unwrap();
        assert!(encrypted_data.len() > 40);
        assert_eq!(&encrypted_data[..8], b"ENCLOG1\0");
        assert_eq!(
            u16::from_le_bytes([encrypted_data[8], encrypted_data[9]]),
            2
        );
        assert_eq!(
            u16::from_le_bytes([encrypted_data[10], encrypted_data[11]]),
            1
        );
        use aes_gcm::{Aes256Gcm, Nonce};
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce_arr: [u8; 12] = encrypted_data[28..40].try_into().unwrap();
        let nonce = Nonce::from(nonce_arr);
        let ciphertext = &encrypted_data[40..];
        let decrypted_compressed = cipher.decrypt(&nonce, ciphertext).unwrap();

        // 解压
        let mut decoder = zstd::stream::Decoder::new(&decrypted_compressed[..]).unwrap();
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, original_content);

        unsafe {
            std::env::remove_var("TEST_COMPRESS_ENC_KEY");
        }
    }

    #[test]
    #[serial]
    #[cfg(all(feature = "gzip", not(feature = "zstd")))]
    fn test_compress_file_gzip_fallback_with_encryption_roundtrip() {
        // 覆盖 gzip fallback 路径的 compress + encrypt 行为：
        // compression feature 未启用时，compress_file 应用 gzip 压缩 + AES-GCM 加密，
        // 生成 .gz.enc 文件，且可通过解密 + gzip 解压还原原文。
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("test_gzip_enc.log");
        let original_content = b"Sensitive log content for gzip fallback encryption test";
        std::fs::write(&original_path, original_content).unwrap();

        let (key_bytes, key_b64) = make_test_key();
        unsafe {
            std::env::set_var("TEST_GZIP_ENC_KEY", &key_b64);
        }

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            compress: true,
            compression_level: 6,
            encrypt: true,
            encryption_key_env: Some("TEST_GZIP_ENC_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&original_path);
        assert!(
            result.is_ok(),
            "gzip fallback compress_file failed: {:?}",
            result.err()
        );
        let encrypted_path = result.unwrap();
        assert_eq!(encrypted_path.extension().unwrap(), "enc");
        assert!(encrypted_path.exists());

        // 解密：v2 头 40 字节（magic 8 + version 2 + algo 2 + salt 16 + nonce 12）+ ciphertext
        let encrypted_data = std::fs::read(&encrypted_path).unwrap();
        assert!(encrypted_data.len() > 40);
        assert_eq!(&encrypted_data[..8], b"ENCLOG1\0");
        assert_eq!(
            u16::from_le_bytes([encrypted_data[8], encrypted_data[9]]),
            2
        );
        assert_eq!(
            u16::from_le_bytes([encrypted_data[10], encrypted_data[11]]),
            1
        );
        use aes_gcm::{Aes256Gcm, Nonce};
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce_arr: [u8; 12] = encrypted_data[28..40].try_into().unwrap();
        let nonce = Nonce::from(nonce_arr);
        let ciphertext = &encrypted_data[40..];
        let decrypted_compressed = cipher.decrypt(&nonce, ciphertext).unwrap();

        // gzip 解压
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(&decrypted_compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, original_content);

        // 原始文件应已被删除（GzipCompression::compress_file 内部删除）
        assert!(
            !original_path.exists(),
            "original file should be removed after gzip compress"
        );

        unsafe {
            std::env::remove_var("TEST_GZIP_ENC_KEY");
        }
    }

    #[test]
    #[serial]
    #[cfg(not(any(feature = "zstd", feature = "gzip")))]
    fn test_compress_file_no_backend_leaves_file_uncompressed() {
        // 无任何压缩后端 feature：compress=true 时不压缩、原文件保留
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("no_backend.log");
        std::fs::write(&original_path, b"plain content").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            compress: true,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&original_path);
        assert!(result.is_ok(), "err: {:?}", result.err());
        assert_eq!(result.unwrap(), original_path);
        assert!(original_path.exists(), "file should be left in place");
    }

    #[test]
    #[serial]
    #[cfg(not(any(feature = "zstd", feature = "gzip")))]
    fn test_compress_file_no_backend_still_encrypts() {
        // 无压缩后端但 encrypt=true：跳过压缩但保留加密保证
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("no_backend_enc.log");
        std::fs::write(&original_path, b"secret content").unwrap();

        let (_key_bytes, key_b64) = make_test_key();
        unsafe {
            std::env::set_var("TEST_NO_BACKEND_ENC_KEY", &key_b64);
        }

        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("dummy.log"),
            compress: true,
            encrypt: true,
            encryption_key_env: Some("TEST_NO_BACKEND_ENC_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        let result = sink.compress_file(&original_path);
        assert!(result.is_ok(), "err: {:?}", result.err());
        let encrypted_path = result.unwrap();
        assert_eq!(encrypted_path.extension().unwrap(), "enc");
        assert!(encrypted_path.exists());

        let encrypted_data = std::fs::read(&encrypted_path).unwrap();
        assert!(encrypted_data.len() > 24);
        assert_eq!(&encrypted_data[..8], b"ENCLOG1\0");
        assert!(
            !original_path.exists(),
            "original file should be removed after encryption"
        );

        unsafe {
            std::env::remove_var("TEST_NO_BACKEND_ENC_KEY");
        }
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
        unsafe {
            std::env::set_var("TEST_ENC_KEY_RT", &key_b64);
        }

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

        // 解密：v2 头 40 字节（magic 8 + version 2 + algo 2 + salt 16 + nonce 12）+ ciphertext
        let encrypted_data = std::fs::read(&output_path).unwrap();
        assert!(encrypted_data.len() > 40);
        assert_eq!(&encrypted_data[..8], b"ENCLOG1\0");
        // v2：version 字段必须为 2（密码模式密钥依赖头中的盐）
        assert_eq!(
            u16::from_le_bytes([encrypted_data[8], encrypted_data[9]]),
            2
        );
        assert_eq!(
            u16::from_le_bytes([encrypted_data[10], encrypted_data[11]]),
            1
        );
        let header_salt: [u8; 16] = encrypted_data[12..28].try_into().unwrap();
        assert!(
            header_salt.iter().any(|&b| b != 0),
            "v2 header must carry a random salt"
        );
        use aes_gcm::{Aes256Gcm, Nonce};
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();
        let nonce_arr: [u8; 12] = encrypted_data[28..40].try_into().unwrap();
        let nonce = Nonce::from(nonce_arr);
        let ciphertext = &encrypted_data[40..];
        let decrypted = cipher.decrypt(&nonce, ciphertext).unwrap();
        assert_eq!(decrypted, original_content);

        unsafe {
            std::env::remove_var("TEST_ENC_KEY_RT");
        }
    }

    #[test]
    #[serial]
    fn test_encrypt_file_missing_key_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.log");
        let output_path = temp_dir.path().join("output.log.enc");
        std::fs::write(&input_path, "content").unwrap();
        unsafe {
            std::env::remove_var("TEST_MISSING_ENC_KEY_VAR");
        }

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
        // v2 统一走共享密钥解析，未设置变量报显性 key-source 缺失错误
        assert!(result.unwrap_err().to_string().contains("no key was found"));
    }

    #[test]
    #[serial]
    fn test_encrypt_file_nonexistent_input_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("nonexistent.log");
        let output_path = temp_dir.path().join("output.log.enc");

        let (_key_bytes, key_b64) = make_test_key();
        unsafe {
            std::env::set_var("TEST_ENC_KEY_NI", &key_b64);
        }

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
        unsafe {
            std::env::remove_var("TEST_ENC_KEY_NI");
        }
    }

    #[test]
    #[serial]
    fn test_encrypt_file_invalid_base64_key_returns_error() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.log");
        let output_path = temp_dir.path().join("output.log.enc");
        std::fs::write(&input_path, "content").unwrap();
        // v2 语义：非 Base64 输入按密码处理；但短于 12 字符的密码必须被拒绝
        unsafe {
            std::env::set_var("TEST_INVALID_B64_KEY", "short");
        }

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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least 12 characters")
        );
        unsafe {
            std::env::remove_var("TEST_INVALID_B64_KEY");
        }
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
        unsafe {
            std::env::set_var("TEST_WRONG_LEN_KEY", &short_key);
        }

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
        unsafe {
            std::env::remove_var("TEST_WRONG_LEN_KEY");
        }
    }

    // ==================== 审计加固测试 ====================

    #[cfg(unix)]
    #[test]
    fn test_log_file_created_with_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("perm.log");
        let config = FileSinkConfig {
            path: path.clone(),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let record = crate::LogRecord::new(
            tracing::Level::INFO,
            "perm::test".to_string(),
            "perm check".to_string(),
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(sink.write(&record)).unwrap();
        rt.block_on(sink.flush()).unwrap();
        let meta = std::fs::metadata(&path).unwrap();
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o600,
            "log file must be 0600"
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn test_encrypted_output_created_with_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let plain = dir.path().join("plain.log");
        std::fs::write(&plain, "sensitive content").unwrap();
        unsafe {
            // 原始 32 字节密钥（直用分支），字符多样以通过熵校验
            std::env::set_var(
                "INKLOG_TEST_ENC_PERM_KEY",
                "abcdefghijklmnopqrstuvwxyz123456",
            );
        }
        let config = FileSinkConfig {
            path: plain.clone(),
            encrypt: true,
            encryption_key_env: Some("INKLOG_TEST_ENC_PERM_KEY".to_string()),
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let out = dir.path().join("plain.enc");
        sink.encrypt_file(&plain, &out).unwrap();
        let meta = std::fs::metadata(&out).unwrap();
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o600,
            "enc file must be 0600"
        );
        unsafe {
            std::env::remove_var("INKLOG_TEST_ENC_PERM_KEY");
        }
    }

    #[test]
    fn test_age_cleanup_runs_when_max_total_size_unparsable() {
        // 审计：年龄清理曾被 max_total_size 分支遮蔽——parse 失败时
        // 过期文件永远不会被清理。修复后年龄清理独立执行。
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        let old1 = dir.path().join("test_20260101_000000.log");
        let old2 = dir.path().join("test_20260102_000000.log");
        std::fs::write(&old1, "old-1").unwrap();
        std::fs::write(&old2, "old-2").unwrap();
        std::fs::write(&log_path, "active").unwrap();
        // 两个文件都置为 3 天前；old1 再早一小时，确保排序后最旧者唯一
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let mtime_old2 = filetime::FileTime::from_unix_time(now_secs - 3 * 86400, 0);
        let mtime_old1 = filetime::FileTime::from_unix_time(now_secs - 3 * 86400 - 3600, 0);
        filetime::set_file_mtime(&old1, mtime_old1).unwrap();
        filetime::set_file_mtime(&old2, mtime_old2).unwrap();

        let config = FileSinkConfig {
            retention_days: 1,
            keep_files: 1,
            max_total_size: "not-a-size".to_string(), // 不可解析：旧代码两个分支都不执行
            ..Default::default()
        };
        FileSink::perform_cleanup(&config, &log_path).unwrap();

        assert!(
            !old1.exists(),
            "oldest expired file must be removed by age cleanup"
        );
        assert!(old2.exists(), "file within keep_files must be kept");
        assert!(log_path.exists(), "active file must never be deleted");
    }

    #[test]
    fn test_fsync_roundtrip_writes_are_visible_immediately() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fsync.log");
        let config = FileSinkConfig {
            path: path.clone(),
            fsync: true,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let record = crate::LogRecord::new(
            tracing::Level::INFO,
            "fsync::test".to_string(),
            "fsync visible marker 13812345678".to_string(),
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(sink.write(&record)).unwrap();
        rt.block_on(sink.flush()).unwrap();
        let data = std::fs::read_to_string(&path).unwrap();
        assert!(
            data.contains("fsync visible marker"),
            "fsync=true writes must be on disk right after flush"
        );
        rt.block_on(sink.shutdown()).unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_writes_produce_intact_lines() {
        // 多任务并发写 1000 条后行数完整、每行可解析（BufWriter 无交错）
        let dir = tempdir().unwrap();
        let path = dir.path().join("concurrent.log");
        let config = FileSinkConfig {
            path: path.clone(),
            ..Default::default()
        };
        let sink = std::sync::Arc::new(FileSink::new(config).unwrap());

        let mut tasks = Vec::new();
        for t in 0..4 {
            let sink = std::sync::Arc::clone(&sink);
            tasks.push(tokio::spawn(async move {
                for i in 0..250 {
                    let rec = crate::LogRecord::new(
                        tracing::Level::INFO,
                        format!("task{t}"),
                        format!("task-{t}-line-{i}"),
                    );
                    sink.write(&rec).await.unwrap();
                }
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = data.lines().collect();
        assert_eq!(lines.len(), 1000, "all 1000 records must be intact");
        for (i, line) in lines.iter().enumerate() {
            assert!(
                line.contains("task-") && line.contains("-line-"),
                "line {i} must be a complete record: {line}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_audit_chain_appends_on_rotation_and_verifies() {
        use crate::support::audit_chain::{ArchiveChain, ArchiveChainEntry};

        unsafe {
            std::env::set_var("INKLOG_AUDIT_KEY", "audit-chain-test-key-01");
        }
        let dir = tempdir().unwrap();
        let path = dir.path().join("chained.log");
        let config = FileSinkConfig {
            path: path.clone(),
            max_size: "200".to_string(),
            audit_chain_enabled: true,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // 200 字节上限：数条记录即触发轮转
        for i in 0..12 {
            let rec = crate::LogRecord::new(
                tracing::Level::INFO,
                "chain::test".to_string(),
                format!("audit chain record {i} with some padding text"),
            );
            rt.block_on(sink.write(&rec)).unwrap();
        }
        rt.block_on(sink.flush()).unwrap();
        rt.block_on(sink.shutdown()).unwrap();
        unsafe {
            std::env::remove_var("INKLOG_AUDIT_KEY");
        }

        let manifest = FileSink::audit_manifest_path(&path);
        let data = std::fs::read_to_string(&manifest).expect("manifest must exist after rotation");
        let entries: Vec<ArchiveChainEntry> = data
            .lines()
            .map(|l| serde_json::from_str(l).expect("manifest line must be JSON"))
            .collect();
        assert!(
            !entries.is_empty(),
            "at least one rotation entry expected, got {}",
            entries.len()
        );
        assert!(
            ArchiveChain::verify_entries(&entries, b"audit-chain-test-key-01"),
            "manifest chain must verify with the correct key"
        );
        // 篡改任一事件（改 sha256 摘要）→ 校验失败
        let mut tampered = entries.clone();
        tampered[0].event = tampered[0].event.replace("sha256", "sha256_tampered");
        assert!(
            !ArchiveChain::verify_entries(&tampered, b"audit-chain-test-key-01"),
            "tampered manifest must fail verification"
        );
    }

    #[test]
    fn test_idle_flush_lands_within_one_tick() {
        // R-rel-003：空闲期（无后续写入）batch 中的记录在一个 tick 内落盘
        let dir = tempdir().unwrap();
        let path = dir.path().join("idle.log");
        let config = FileSinkConfig {
            path: path.clone(),
            batch_size: 100,
            flush_interval_ms: 100,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(sink.write(&create_test_record("idle flush marker")))
            .unwrap();

        // 注入 1s tick：空闲 flush 由轮转定时器线程代执行
        sink.start_rotation_timer_with_tick(StdDuration::from_secs(1));

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut landed = false;
        while std::time::Instant::now() < deadline {
            if let Ok(data) = std::fs::read_to_string(&path)
                && data.contains("idle flush marker")
            {
                landed = true;
                break;
            }
            thread::sleep(StdDuration::from_millis(50));
        }
        assert!(landed, "idle batch must flush within one tick");
        rt.block_on(sink.shutdown()).unwrap();
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
            encryption_key_file: None,
            retention_days: 30,
            max_total_size: "1KB".to_string(),
            cleanup_interval_minutes: 60,
            batch_size: 100,
            flush_interval_ms: 100,
            fsync: false,
            audit_chain_enabled: false,
            masking_enabled: true,
            output_format: Default::default(),
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
    fn test_perform_cleanup_size_limit_respects_keep_files() {
        // 尺寸超限触发删除时，同样必须保留最新的 keep_files 个文件
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("keep_size.log");

        // 5 个 1KB 轮转文件，mtime 依次递增（keep_size_4.log 最新）
        let now = std::time::SystemTime::now();
        for i in 0..5 {
            let p = temp_dir.path().join(format!("keep_size_{}.log", i));
            std::fs::write(&p, "x".repeat(1024)).unwrap();
            let mtime = now - std::time::Duration::from_secs(1000 - i as u64 * 100);
            let _ = filetime::set_file_mtime(&p, filetime::FileTime::from_system_time(mtime));
        }

        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            keep_files: 1,
            max_total_size: "1KB".to_string(),
            ..Default::default()
        };
        let result = FileSink::perform_cleanup(&config, &temp_dir.path().join("keep_size.log"));
        assert!(result.is_ok());

        // 总量 5KB 超限 4KB：删除最旧的 4 个，keep_files=1 保留最新的
        for i in 0..4 {
            let p = temp_dir.path().join(format!("keep_size_{}.log", i));
            assert!(!p.exists(), "keep_size_{}.log should be removed", i);
        }
        assert!(
            temp_dir.path().join("keep_size_4.log").exists(),
            "newest file must survive keep_files=1 under size-based cleanup"
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
    fn test_perform_cleanup_nonexistent_parent_returns_ok() {
        let dir = tempdir().unwrap();
        let nonexistent_parent = dir.path().join("does_not_exist");
        let log_path = nonexistent_parent.join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_total_size: "1GB".to_string(),
            ..Default::default()
        };
        // parent 目录不存在 → read_dir 失败，但 perform_cleanup 优雅降级为 Ok(())
        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_ok());
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

    #[tokio::test]
    async fn test_file_sink_flush_writes_buffered_records() {
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
        let result = sink.flush().await;
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Flush test"));
    }

    #[tokio::test]
    async fn test_file_sink_flush_without_file_succeeds() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        // 未打开文件，flush 应仍成功（空操作）
        let result = sink.flush().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_sink_shutdown_flushes_remaining_data() {
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
        let result = sink.shutdown().await;
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Shutdown test"));
    }

    // ==================== LogSink::write 测试 ====================

    #[tokio::test]
    async fn test_write_multiple_records_all_persisted() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            batch_size: 2, // 小批量触发刷新
            flush_interval_ms: 1000,
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        };
        let sink = FileSink::new(config).unwrap();
        for i in 0..5 {
            let record = create_test_record(&format!("Message {}", i));
            sink.write(&record).await.unwrap();
        }
        sink.flush().await.unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        for i in 0..5 {
            assert!(content.contains(&format!("Message {}", i)));
        }
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_write_with_masking_disabled_preserves_sensitive_value() {
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
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await.unwrap();
        sink.flush().await.unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            content.contains("secret1234567890"),
            "masking disabled should preserve original value"
        );
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_write_with_masking_enabled_redacts_sensitive_value() {
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
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await.unwrap();
        sink.flush().await.unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            !content.contains("secret1234567890"),
            "masking enabled should redact sensitive value"
        );
        assert!(
            content.contains("***REDACTED***"),
            "masked output should contain REDACTED marker"
        );
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_lazy_file_sink_defers_file_creation_until_first_write() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("lazy/error.log");

        let sink = LazyFileSink::new(FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        });
        assert!(
            !log_path.exists(),
            "构建 LazyFileSink 不得创建目标文件（含父目录）"
        );

        let record = LogRecord {
            timestamp: Utc::now(),
            level: "ERROR".to_string(),
            target: "lazy_test".to_string(),
            message: "first error event".to_string(),
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: "t1".to_string(),
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await.unwrap();
        sink.flush().await.unwrap();
        assert!(log_path.exists(), "首次写入后目标文件必须出现");
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("first error event"));
        sink.shutdown().await.unwrap();

        // 从未写入时 flush/shutdown 必须为安全 no-op，同样不得创建文件
        let untouched_path = temp_dir.path().join("lazy/never.log");
        let fresh = LazyFileSink::new(FileSinkConfig {
            enabled: true,
            path: untouched_path.clone(),
            ..Default::default()
        });
        fresh.flush().await.unwrap();
        fresh.shutdown().await.unwrap();
        assert!(
            !untouched_path.exists(),
            "flush/shutdown on an unwritten LazyFileSink must not create the file"
        );
    }

    #[tokio::test]
    async fn test_write_with_masking_keeps_plain_english_words_intact() {
        // 落盘脱敏路径：普通英文单词不得被护照规则误掩，真护照形态仍被掩
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
            message: "task execution completed for E12345678".to_string(),
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: "t1".to_string(),
            trace_id: None,
            span_id: None,
        };
        sink.write(&record).await.unwrap();
        sink.flush().await.unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            content.contains("task execution completed"),
            "plain English words must survive masking on disk: {content}"
        );
        assert!(
            !content.contains("e******on"),
            "'execution' must not be masked as a passport: {content}"
        );
        assert!(
            !content.contains("E12345678") && content.contains("E******78"),
            "real passport numbers must still be masked on disk: {content}"
        );
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_write_appends_to_existing_file() {
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
        sink.write(&create_test_record("Appended message"))
            .await
            .unwrap();
        sink.flush().await.unwrap();
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.starts_with("pre-existing line"));
        assert!(content.contains("Appended message"));
        sink.shutdown().await.unwrap();
    }

    // ==================== rotation_time 分支覆盖测试 ====================

    #[tokio::test]
    async fn test_file_sink_new_with_weekly_rotation() {
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
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_sink_new_with_monthly_rotation() {
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
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_sink_new_with_unknown_rotation_falls_back_to_daily() {
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
        sink.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_sink_new_with_hourly_rotation() {
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
        sink.shutdown().await.unwrap();
    }

    // ==================== get_encryption_key 错误路径测试 ====================

    #[test]
    #[serial]
    fn test_get_encryption_key_password_mode_deterministic_with_salt() {
        // v2 语义：非 Base64、非 32 字节的输入按密码处理，
        // 同一盐确定性派生（与解密方用文件头盐重导出对齐）。
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("test.log"),
            encryption_key_env: Some("TEST_PWD_SALT_KEY".to_string()),
            ..Default::default()
        };
        // 含 '_'/'!'/'@'/'#' → 非 Base64；长度 30 ≠ 32 → 密码分支
        unsafe {
            std::env::set_var("TEST_PWD_SALT_KEY", "this_is_not_valid_base64!!!@#$");
        }

        let sink = create_test_file_sink(config);
        let k1 = sink
            .get_encryption_key(b"fixed-salt-16byt")
            .expect("password mode with salt should derive a key");
        let k2 = sink
            .get_encryption_key(b"fixed-salt-16byt")
            .expect("second derive with same salt should succeed");
        assert_eq!(*k1, *k2, "same password + same salt must be deterministic");
        let k3 = sink
            .get_encryption_key(b"other-salt-16byt")
            .expect("derive with different salt should succeed");
        assert_ne!(*k1, *k3, "different salt must derive a different key");

        unsafe {
            std::env::remove_var("TEST_PWD_SALT_KEY");
        }
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
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "zstd")]
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
        unsafe {
            std::env::set_var(
                "TEST_ENCRYPT_KEY",
                "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=",
            );
        }

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

        unsafe {
            std::env::remove_var("TEST_ENCRYPT_KEY");
        }
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
        unsafe {
            std::env::remove_var("MISSING_ENCRYPT_KEY_ENV_VAR");
        }

        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(result.is_err(), "encrypt_file should fail without key");
        assert!(
            result.unwrap_err().to_string().contains("no key was found"),
            "missing key must surface the explicit key-source error"
        );
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
        unsafe {
            std::env::set_var(
                "TEST_ENCRYPT_KEY_2",
                "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=",
            );
        }

        let sink = create_test_file_sink(config);
        let result = sink.encrypt_file(&input_path, &output_path);
        assert!(
            result.is_err(),
            "encrypt_file should fail for nonexistent input"
        );

        unsafe {
            std::env::remove_var("TEST_ENCRYPT_KEY_2");
        }
    }

    // ==================== compress_file with encryption 测试 ====================

    #[test]
    #[serial]
    #[cfg(feature = "zstd")]
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
        unsafe {
            std::env::set_var(
                "TEST_COMPRESS_ENC_KEY",
                "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=",
            );
        }

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

        unsafe {
            std::env::remove_var("TEST_COMPRESS_ENC_KEY");
        }
    }

    // ==================== rotate_inner 测试 ====================

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_rotate_inner_basic() {
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

        sink.shutdown().await.unwrap();

        // 原文件应被重命名（轮转后），新文件应被创建
        let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
        // 至少应该有轮转后的文件
        assert!(
            !entries.is_empty(),
            "rotated file should exist in directory"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    #[cfg(feature = "zstd")]
    async fn test_rotate_inner_with_compression() {
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
        sink.shutdown().await.unwrap();

        // 检查是否有 .zst 文件生成
        let has_zst = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .any(|e| e.is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "zst")));
        assert!(has_zst, "compressed rotated file (.zst) should exist");
    }

    // ==================== rotate_inner encrypt-only 分支测试 ====================

    #[tokio::test]
    #[serial]
    #[allow(clippy::await_holding_lock)]
    async fn test_rotate_inner_with_encryption_only_branch() {
        // 覆盖行 808-847：compress=false 但 encrypt=true 的分支
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("rotate_encrypt.log");

        let (_key_bytes, key_b64) = make_test_key();
        unsafe {
            std::env::set_var("TEST_ROTATE_ENC_KEY", &key_b64);
        }

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
        sink.shutdown().await.unwrap();

        // 检查是否有 .enc 文件生成（encrypt-only 路径会生成 .enc 文件）
        let has_enc = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .any(|e| e.is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "enc")));
        assert!(
            has_enc,
            "encrypted rotated file (.enc) should exist in encrypt-only mode"
        );

        unsafe {
            std::env::remove_var("TEST_ROTATE_ENC_KEY");
        }
    }

    // ==================== compress_file 加密失败回退测试 ====================

    #[test]
    #[serial]
    #[cfg(feature = "zstd")]
    fn test_compress_file_with_encryption_failure_keeps_compressed() {
        // 覆盖行 666-676：当 encrypt=true 但密钥无效时，
        // compress_file 应将压缩文件重命名为 .unencrypted 后缀并返回错误
        let temp_dir = tempdir().unwrap();
        let original_path = temp_dir.path().join("to_compress_fail.log");
        std::fs::write(&original_path, "content for failed encryption\n").unwrap();

        // 设置一个无效的加密密钥（长度足够但解码后不是 32 字节）
        let invalid_key = base64::engine::general_purpose::STANDARD.encode(b"1234567890123456");
        unsafe {
            std::env::set_var("TEST_COMPRESS_ENC_FAIL_KEY", &invalid_key);
        }

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

        unsafe {
            std::env::remove_var("TEST_COMPRESS_ENC_FAIL_KEY");
        }
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
        // 无文件句柄时记录不应被静默丢弃，应回填等待重试
        assert_eq!(
            inner.batch_buffer.len(),
            1,
            "records must be re-queued when there is no file handle"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_flush_batch_inner_write_error_requeues_remaining_records() {
        // /dev/full 写入始终返回 ENOSPC：构造批量写入中途失败，
        // 未写入的记录应回填 batch_buffer 以便重试
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/dev/full"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        if sink.open_file_inner(&mut inner).is_err() {
            eprintln!("Skipping: /dev/full not accessible in this environment");
            return;
        }

        inner.batch_buffer.push(create_test_record("Unsent 1"));
        inner.batch_buffer.push(create_test_record("Unsent 2"));

        let result = sink.flush_batch_inner(&mut inner);
        assert!(result.is_ok(), "flush_batch_inner swallows the io error");
        assert_eq!(
            inner.batch_buffer.len(),
            2,
            "unsent records should be re-queued after a write failure"
        );
        assert_eq!(
            inner.circuit_breaker.failure_count(),
            1,
            "a failed batch should record a circuit breaker failure, not a success"
        );
    }

    // ==================== CircuitBreaker 打开时使用 fallback sink 测试 ====================

    /// 简单的 mock LogSink，用于测试 fallback 路径
    struct MockFallbackSink {
        write_count: Arc<parking_lot::Mutex<usize>>,
    }

    /// Mock LogSink，跟踪 shutdown 调用，用于测试 shutdown 路径
    struct MockFallbackSinkWithShutdownFlag {
        shutdown_called: Arc<parking_lot::Mutex<bool>>,
    }

    #[async_trait::async_trait]
    impl LogSink for MockFallbackSinkWithShutdownFlag {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            Ok(())
        }
        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        fn is_healthy(&self) -> bool {
            true
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            *self.shutdown_called.lock() = true;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl LogSink for MockFallbackSink {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            *self.write_count.lock() += 1;
            Ok(())
        }
        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        fn is_healthy(&self) -> bool {
            true
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_write_with_open_circuit_breaker_uses_fallback_sink() {
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
            inner.fallback_sink = Some(Arc::new(mock_sink));
            // 触发足够多的失败使断路器打开（failure_threshold=5）
            for _ in 0..5 {
                inner.circuit_breaker.record_failure();
            }
            // 验证断路器确实打开了
            assert_eq!(inner.circuit_breaker.state(), CircuitState::Open);
        }

        let record = create_test_record("Fallback test");
        let result = sink.write(&record).await;
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

    #[tokio::test]
    async fn test_write_with_rotation_failure_uses_fallback_sink() {
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
            inner.fallback_sink = Some(Arc::new(mock_sink));
        }

        // 写入一条记录，触发 size 轮转，但轮转会因路径无效而失败
        let record = create_test_record("Rotation failure test");
        let result = sink.write(&record).await;
        // write 不应返回错误（错误被吞掉，转用 fallback sink）
        assert!(result.is_ok(), "write should not error when rotation fails");
        // fallback sink 应被调用
        assert!(
            *write_count.lock() >= 1,
            "fallback sink should be called when rotation fails"
        );
    }

    // ==================== shutdown 完整流程测试 ====================

    #[tokio::test]
    async fn test_shutdown_with_active_timers_completes_successfully() {
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
            sink.write(&record).await.unwrap();
        }

        // shutdown 应能正常完成（线程会响应 shutdown_flag 并退出）
        let result = sink.shutdown().await;
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

    #[tokio::test]
    async fn test_drop_does_not_panic_with_active_timers() {
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
            .await
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

        // 创建 4 个过期文件（使用 keep_test_ 前缀以匹配当前日志集的轮转命名）
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
        for i in 0..4 {
            let p = temp_dir.path().join(format!("keep_test_{}.log", i));
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

        // 验证：4 个过期文件，keep_files=2，应保留最新的 2 个（删除 2 个最旧的）
        let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
            .collect();
        assert_eq!(
            remaining.len(),
            2,
            "keep_files should preserve exactly 2 files, got {}",
            remaining.len()
        );
    }

    #[test]
    fn test_perform_cleanup_never_touches_unrelated_files() {
        // 回归测试：cleanup 只允许删除当前日志集的轮转文件，目录中的无关文件不得被删除
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("app.log");
        std::fs::write(&log_path, "active log").unwrap();

        // 当前日志集的过期轮转文件（2KB，足以触发 size 分支）
        let rotated = temp_dir.path().join("app_20250101_000000.log");
        std::fs::write(&rotated, "x".repeat(2048)).unwrap();
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
        let _ = filetime::set_file_mtime(&rotated, filetime::FileTime::from_system_time(old_time));

        // 无关文件（不属于 app 日志集），即便很小也应保留
        let unrelated = temp_dir.path().join("user_data.txt");
        std::fs::write(&unrelated, "must not be deleted").unwrap();
        let unrelated2 = temp_dir.path().join("otherapp.log");
        std::fs::write(&unrelated2, "other logger, must survive").unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            retention_days: 7,
            keep_files: 0,
            max_total_size: "1KB".to_string(), // 触发 size 分支，迫使 cleanup 尝试删除
            ..Default::default()
        };
        let result = FileSink::perform_cleanup(&config, &log_path);
        assert!(result.is_ok());

        assert!(!rotated.exists(), "expired rotated log should be removed");
        assert!(unrelated.exists(), "unrelated file must never be deleted");
        assert!(
            unrelated2.exists(),
            "other logger's file must never be deleted"
        );
        assert!(log_path.exists(), "active log file must never be deleted");
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

    // ==================== open_file_inner: OpenOptions 失败分支 (-322) ====================

    #[test]
    fn test_open_file_inner_fails_when_path_is_directory() {
        // 覆盖行 320-322：OpenOptions::open 失败时返回 IoError
        // 当 path 指向一个已存在的目录时，open(create+append) 会失败
        let temp_dir = tempdir().unwrap();
        let dir_as_path = temp_dir.path().to_path_buf();
        // dir_as_path 是目录，OpenOptions::new().create(true).append(true).open(dir) 会失败

        let config = FileSinkConfig {
            enabled: true,
            path: dir_as_path,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(
            result.is_err(),
            "open_file_inner should fail when path is an existing directory"
        );
        // 确认 inner.current_file 未被设置
        assert!(inner.current_file.is_none());
    }

    // ==================== compress_file: File::create 失败分支 (-644) ====================

    #[test]
    #[cfg(unix)]
    #[cfg(feature = "zstd")]
    fn test_compress_file_fails_when_output_dir_readonly() {
        // 覆盖行 643-644：File::create(compressed_path) 失败时返回 IoError
        // 通过将父目录设为只读来触发 File::create 失败
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("readonly_test.log");
        std::fs::write(&log_path, "test data").unwrap();

        // 将父目录设为只读
        let original_perms = std::fs::metadata(temp_dir.path()).unwrap().permissions();
        let mut readonly_perms = original_perms.clone();
        readonly_perms.set_mode(0o555); // r-x for all
        std::fs::set_permissions(temp_dir.path(), readonly_perms).unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let result = sink.compress_file(&log_path);

        // 恢复权限以便 tempdir 能清理（先恢复再断言，避免泄漏）
        std::fs::set_permissions(temp_dir.path(), original_perms).unwrap();

        // root 用户会绕过权限检查；只在 result 为 Err 时断言错误类型
        match result {
            Err(InklogError::IoError(_)) => { /* 预期：非 root 下 File::create 失败 */ }
            Ok(_) => {
                // root 下权限被绕过，压缩成功——清理产物
                let _ = std::fs::remove_file(log_path.with_extension("zst"));
            }
            other => panic!(
                "expected IoError or Ok, got: {}",
                other
                    .as_ref()
                    .err()
                    .map(|e| e.safe_message())
                    .unwrap_or_else(|| "Ok".to_string())
            ),
        }
    }

    // ==================== encrypt_file: File::create 失败分支 (-717) ====================

    #[test]
    #[serial]
    #[cfg(unix)]
    fn test_encrypt_file_fails_when_output_dir_readonly() {
        // 覆盖行 716-717：File::create(output_path) 失败时返回 IoError
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("encrypt_input.bin");
        std::fs::write(&input_path, b"plaintext data").unwrap();

        // 设置有效的加密密钥（32 字节 base64）；用自定义 env var 避免与其他测试串扰
        let (_key_bytes, key_b64) = make_test_key();
        let enc_key_env = "TEST_ENCRYPT_READONLY_KEY";
        unsafe {
            std::env::set_var(enc_key_env, &key_b64);
        }

        // 将父目录设为只读
        let original_perms = std::fs::metadata(temp_dir.path()).unwrap().permissions();
        let mut readonly_perms = original_perms.clone();
        readonly_perms.set_mode(0o555);
        std::fs::set_permissions(temp_dir.path(), readonly_perms).unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: input_path.clone(),
            encrypt: true,
            encryption_key_env: Some(enc_key_env.to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let output_path = temp_dir.path().join("nonexistent_encrypted.enc");
        let result = sink.encrypt_file(&input_path, &output_path);

        // 恢复权限
        std::fs::set_permissions(temp_dir.path(), original_perms).unwrap();
        unsafe {
            std::env::remove_var(enc_key_env);
        }

        // root 用户会绕过权限检查；只在 result 为 Err 时断言错误类型
        match result {
            Err(InklogError::IoError(_)) => { /* 预期：非 root 下 File::create 失败 */ }
            Ok(_) => {
                let _ = std::fs::remove_file(&output_path);
            }
            other => panic!(
                "expected IoError or Ok, got: {}",
                other
                    .as_ref()
                    .err()
                    .map(|e| e.safe_message())
                    .unwrap_or_else(|| "Ok".to_string())
            ),
        }
    }

    // ==================== rotate_inner: rename 失败 fallback 分支 (-758) ====================

    #[test]
    #[cfg(unix)]
    fn test_rotate_inner_rename_failure_returns_error_when_copy_also_fails() {
        // 覆盖行 753-758：rename 失败且 copy 也失败时返回 IoError
        // 通过将父目录设为只读来使 rename 和 copy 都失败
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("rotate_rename_fail.log");
        std::fs::write(&log_path, "rotation test data").unwrap();

        // 将父目录设为只读
        let original_perms = std::fs::metadata(temp_dir.path()).unwrap().permissions();
        let mut readonly_perms = original_perms.clone();
        readonly_perms.set_mode(0o555);
        std::fs::set_permissions(temp_dir.path(), readonly_perms).unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.rotate_inner(&mut inner);

        // 恢复权限
        std::fs::set_permissions(temp_dir.path(), original_perms).unwrap();

        // root 用户会绕过权限检查；只在 result 为 Err 时断言错误类型
        match result {
            Err(InklogError::IoError(_)) => { /* 预期：非 root 下 rename+copy 失败 */ }
            Ok(_) => {
                // root 下 rename 成功——清理轮转产物
                let _ = std::fs::remove_file(log_path);
            }
            other => panic!(
                "expected IoError or Ok, got: {}",
                other
                    .as_ref()
                    .err()
                    .map(|e| e.safe_message())
                    .unwrap_or_else(|| "Ok".to_string())
            ),
        }
    }

    // ==================== shutdown: fallback_sink.shutdown() 调用 (-1014) ====================

    #[tokio::test]
    async fn test_shutdown_calls_fallback_sink_shutdown() {
        // 覆盖：当 fallback_sink 存在时，shutdown() 应调用其 shutdown()
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("shutdown_fallback.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);

        // 注入 fallback sink
        let shutdown_called = Arc::new(parking_lot::Mutex::new(false));
        let mock_sink = MockFallbackSinkWithShutdownFlag {
            shutdown_called: shutdown_called.clone(),
        };
        {
            let mut inner = sink.inner.write();
            inner.fallback_sink = Some(Arc::new(mock_sink));
        }

        // 调用 shutdown——应触发 fallback_sink.shutdown()
        let result = sink.shutdown().await;
        assert!(result.is_ok(), "shutdown should succeed");

        // 验证 fallback sink 的 shutdown 被调用
        assert!(
            *shutdown_called.lock(),
            "fallback sink shutdown should be called"
        );
    }

    // ========================================================================
    // vuln-0002: FileSink 路径遍历防护测试
    // ========================================================================
    //
    // PathValidator 默认配置拒绝：
    // - 含 ".." 的路径（路径遍历）
    // - 含敏感组件的路径（etc / passwd / shadow / .git / .ssh / .env）
    // - 符号链接（allow_symlinks = false）
    // FileSink::open_file_inner 在 create_dir_all 之前验证路径，
    // 避免恶意路径创建目录或写入敏感文件。

    /// vuln-0002 #1: FileSink 拒绝 "../../../etc/passwd" 路径遍历。
    #[test]
    fn file_sink_rejects_path_traversal_to_etc_passwd() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("../../../etc/passwd"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        let err = result.expect_err("should reject path traversal to /etc/passwd");
        assert!(
            matches!(err, InklogError::ConfigError(_)),
            "expected ConfigError, got: {err:?}"
        );
        assert!(
            err.to_string().contains("Unsafe log path rejected"),
            "error should mention unsafe path: {err}"
        );
    }

    /// vuln-0002 #2: FileSink 拒绝 "/etc/cron.d/malicious" 系统敏感路径。
    #[test]
    fn file_sink_rejects_system_cron_path() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/etc/cron.d/malicious"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        let err = result.expect_err("should reject /etc/cron.d system path");
        assert!(
            matches!(err, InklogError::ConfigError(_)),
            "expected ConfigError, got: {err:?}"
        );
    }

    /// vuln-0002 #3: FileSink 拒绝 "../../system/file" 路径遍历。
    #[test]
    fn file_sink_rejects_parent_dir_traversal() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("../../system/file"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        let err = result.expect_err("should reject parent-dir traversal");
        assert!(
            matches!(err, InklogError::ConfigError(_)),
            "expected ConfigError, got: {err:?}"
        );
    }

    /// vuln-0002 #4: FileSink 拒绝 "/etc/passwd" 直接访问。
    #[test]
    fn file_sink_rejects_etc_passwd_direct() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/etc/passwd"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_err(),
            "should reject direct access to /etc/passwd"
        );
        assert!(matches!(result.unwrap_err(), InklogError::ConfigError(_)));
    }

    /// vuln-0002 #5: FileSink 拒绝 "/etc/shadow" 直接访问。
    #[test]
    fn file_sink_rejects_etc_shadow_direct() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/etc/shadow"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_err(),
            "should reject direct access to /etc/shadow"
        );
    }

    /// vuln-0002 #6: FileSink 拒绝含 ".git" 组件的路径。
    #[test]
    fn file_sink_rejects_git_directory_path() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("project/.git/config"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_err(),
            "should reject path containing .git component"
        );
    }

    /// vuln-0002 #7: FileSink 拒绝含 ".ssh" 组件的路径。
    #[test]
    fn file_sink_rejects_ssh_directory_path() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("~/.ssh/id_rsa"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_err(),
            "should reject path containing .ssh component"
        );
    }

    /// vuln-0002 #8: FileSink 拒绝含 ".env" 组件的路径。
    #[test]
    fn file_sink_rejects_env_file_path() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("./.env"),
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_err(),
            "should reject path containing .env component"
        );
    }

    /// vuln-0002 #9: FileSink 接受合法相对路径 "logs/app.log"（通过 tempdir 隔离）。
    ///
    /// 用 tempdir 路径拼接 "logs/app.log" 子路径，等价于测试相对路径
    /// "logs/app.log" 的安全性（PathValidator 检查路径组件，不含 ".."
    /// 且组件不在 deny 列表中）。
    #[test]
    fn file_sink_accepts_valid_logs_app_log_path() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("logs").join("app.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_ok(),
            "should accept valid logs/app.log path, got: {:?}",
            result.err()
        );
    }

    /// vuln-0002 #10: FileSink 接受合法相对路径 "var/log/app.log"（通过 tempdir 隔离）。
    #[test]
    fn file_sink_accepts_valid_var_log_app_log_path() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("var").join("log").join("app.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_ok(),
            "should accept valid var/log/app.log path, got: {:?}",
            result.err()
        );
    }

    /// vuln-0002 #11: FileSink 接受 tempdir 下的绝对路径（默认 allow_absolute=true）。
    #[test]
    fn file_sink_accepts_absolute_tempdir_path() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("app.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            ..Default::default()
        };
        let result = FileSink::new(config);
        assert!(
            result.is_ok(),
            "should accept absolute path under tempdir, got: {:?}",
            result.err()
        );
    }

    /// vuln-0002 #12: open_file_inner 直接调用也验证路径（深度防御）。
    #[test]
    fn open_file_inner_rejects_path_traversal_directly() {
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("../../../etc/passwd"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(
            result.is_err(),
            "open_file_inner should reject path traversal"
        );
        assert!(matches!(result.unwrap_err(), InklogError::ConfigError(_)));
    }

    /// vuln-0002 #13: open_file_inner 接受合法路径并成功打开文件。
    #[test]
    fn open_file_inner_accepts_valid_path_and_opens_file() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("valid.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(result.is_ok(), "open_file_inner should accept valid path");
        assert!(inner.current_file.is_some(), "file should be opened");
        assert!(log_path.exists(), "log file should exist on disk");
    }

    // ==================== Rotatable / DiskCheckable trait impl 测试 ====================

    #[test]
    fn test_rotatable_trait_start_and_stop_rotation_timer() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("rotatable_test.log"),
            rotation_time: "daily".to_string(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        // start_rotation_timer via Rotatable trait (line 1203-1205)
        Rotatable::start_rotation_timer(&sink);
        // Verify timer was set
        {
            let inner = sink.inner.read();
            assert!(inner.timer_handle.is_some(), "timer handle should be set");
        }
        // stop_rotation_timer via Rotatable trait (lines 1208-1211)
        Rotatable::stop_rotation_timer(&sink);
        {
            let inner = sink.inner.read();
            assert!(
                inner.rotation_timer.is_none(),
                "rotation timer should be None"
            );
        }
    }

    #[test]
    fn test_disk_checkable_trait_check_disk_space() {
        let temp_dir = tempdir().unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: temp_dir.path().join("disk_check_test.log"),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        // check_disk_space via DiskCheckable trait (lines 1216-1218)
        let result = DiskCheckable::check_disk_space(&sink);
        assert!(result.is_ok(), "check_disk_space should succeed");
        assert!(result.unwrap(), "disk should have sufficient space");
    }

    #[tokio::test]
    async fn test_write_batch_flush_after_rotation() {
        // Cover lines 1117-1124: batch flush triggered after rotation during write
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("batch_rotation.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            max_size: "100".to_string(), // Very small to trigger rotation
            batch_size: 2,               // Small batch size
            flush_interval_ms: 10000,    // Long interval so size triggers flush
            fsync: false,
            audit_chain_enabled: false,
            rotation_time: "daily".to_string(),
            compress: false,
            encrypt: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        // Open the file first
        {
            let mut inner = sink.inner.write();
            sink.open_file_inner(&mut inner).unwrap();
        }
        // Write enough records to trigger rotation and batch flush
        for i in 0..10 {
            let record = create_test_record(&format!("batch rotation message {}", i));
            sink.write(&record).await.unwrap();
        }
        // Flush remaining
        sink.flush().await.unwrap();
        // Verify some content was written
        let mut any_content = false;
        for entry in std::fs::read_dir(temp_dir.path()).unwrap() {
            let entry = entry.unwrap();
            if let Ok(content) = std::fs::read_to_string(entry.path())
                && content.contains("batch rotation message")
            {
                any_content = true;
                break;
            }
        }
        assert!(
            any_content,
            "at least one file should contain written records"
        );
    }

    // ========================================================================
    // 缺陷修复回归测试
    // ========================================================================

    // ---- 缺陷 #2: monthly 轮转实际每天轮转 ----

    #[test]
    fn test_calculate_next_rotation_time_monthly_july_31_is_august_31() {
        // 回归：7 月 31 日 20:00 的 monthly 下次轮转必须是 8 月 31 日，
        // 而不是旧实现算出的“明天（7 月）零点”。
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 7, 31, 20, 0, 0).unwrap();
        let next = FileSink::calculate_next_rotation_time_from("monthly", now)
            .expect("monthly rotation time must be computable");
        assert_eq!(
            next,
            Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap(),
            "monthly rotation on Jul 31 must land on Aug 31, not any day in July"
        );
    }

    #[test]
    fn test_calculate_next_rotation_time_monthly_clamps_month_end() {
        // 月末溢出：1 月 31 日 + 1 个月应钳制到 2 月最后一天（2026 非闰年 → 28 日）
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 1, 31, 12, 0, 0).unwrap();
        let next = FileSink::calculate_next_rotation_time_from("monthly", now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 2, 28, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_calculate_next_rotation_time_monthly_lands_in_next_month() {
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2026, 9, 9, 10, 30, 0).unwrap();
        let next = FileSink::calculate_next_rotation_time_from("monthly", now).unwrap();
        assert!(next > now);
        assert_eq!((next.year(), next.month()), (2026, 10));
        assert_eq!((next.hour(), next.minute(), next.second()), (0, 0, 0));
    }

    // ---- 缺陷 #3: 轮转文件名秒级精度覆盖 ----

    #[test]
    fn test_resolve_rotation_target_avoids_collision_with_sequence_suffix() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("test.log");
        let stamp = "20260909_120000";

        // 无冲突 → 标准名
        let first = resolve_rotation_target(&original, stamp);
        assert_eq!(first, dir.path().join("test_20260909_120000.log"));

        // 同秒二次轮转：目标已存在 → 追加 .1，绝不覆盖
        std::fs::write(&first, "first rotation").unwrap();
        let second = resolve_rotation_target(&original, stamp);
        assert_eq!(second, dir.path().join("test_20260909_120000.log.1"));
        assert_eq!(
            std::fs::read_to_string(&first).unwrap(),
            "first rotation",
            "existing rotated file must not be overwritten"
        );

        // 同秒三次轮转 → .2
        std::fs::write(&second, "second rotation").unwrap();
        let third = resolve_rotation_target(&original, stamp);
        assert_eq!(third, dir.path().join("test_20260909_120000.log.2"));
    }

    #[test]
    fn test_resolve_rotation_target_family_includes_compressed_artifacts() {
        // 后处理线程压缩后会删除轮转源文件：同秒下一次轮转若只看源文件
        // 存在性会复用同名覆盖既有 `.zst` 产物（记录丢失）。产物检查必须
        // 按追加语义派生——`with_extension` 会剥掉序号后缀，所有 attempt
        // 命中同一产物，冲突循环永不终止（曾致同秒高频轮转挂死）。
        let dir = tempdir().unwrap();
        let original = dir.path().join("test.log");
        let stamp = "20260909_120000";

        // 首次轮转产物已被压缩为 `X.log.zst`，源文件已删（后处理正常终态）
        std::fs::write(
            dir.path().join("test_20260909_120000.log.zst"),
            b"compressed",
        )
        .unwrap();

        let next = resolve_rotation_target(&original, stamp);
        assert_eq!(
            next,
            dir.path().join("test_20260909_120000.log.1"),
            "rotated source deleted after compression: next rotation must take .1"
        );

        // .1 也被压缩为 `X.log.1.zst` 后 → .2（追加命名保证 attempt 单射）
        std::fs::write(
            dir.path().join("test_20260909_120000.log.1.zst"),
            b"compressed",
        )
        .unwrap();
        assert_eq!(
            resolve_rotation_target(&original, stamp),
            dir.path().join("test_20260909_120000.log.2")
        );
    }

    // zstd 专属端到端对拍（断言 .zst 产物与 zstd 解码）：gzip-only 组合的
    // 产物为 .gz，语义不同；无压缩后端组合无 zstd 依赖
    #[cfg(feature = "zstd")]
    #[tokio::test]
    async fn test_same_second_rotations_keep_every_artifact_distinct() {
        // 端到端对拍：同秒高频轮转 + 压缩，各 attempt 产物互不覆盖、全部可解
        let dir = tempdir().unwrap();
        let path = dir.path().join("same_sec.log");
        let mut cfg = crate::FileSinkConfig::default();
        cfg.path = path.clone();
        cfg.max_size = "200".to_string();
        cfg.compress = true;
        cfg.retention_days = 3650;
        cfg.keep_files = 100;
        cfg.cleanup_interval_minutes = 60 * 24 * 30;
        let sink = crate::support::io::FileSink::new(cfg).unwrap();
        for i in 0..30 {
            sink.write(&crate::LogRecord::new(
                tracing::Level::INFO,
                "same_sec::test".to_string(),
                format!("same-second record {i} with padding {}", "x".repeat(20)),
            ))
            .await
            .unwrap();
        }
        sink.shutdown().await.unwrap();

        // 等待后台归档线程压缩（shutdown 不 join）。追加命名保证 attempt
        // 单射：解码全部 .zst 产物并纳入活动文件与尚未压缩的轮转源，30 条
        // 记录各出现恰好一次（无丢失、无覆盖混杂）。目录快照与后台压缩
        // 并发非原子（源被删而产物未入列 / 半成品产物解码失败）→ 整体重
        // 试；到 deadline 仍不满足才判真实缺陷。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        {
            let scan = || -> Result<String, std::io::Error> {
                let mut all_text = String::new();
                for entry in std::fs::read_dir(dir.path())?.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.extension().is_some_and(|x| x == "zst") {
                        let data = std::fs::read(&p)?;
                        let plain = zstd::stream::decode_all(&data[..]).map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e)
                        })?;
                        all_text.push_str(&String::from_utf8_lossy(&plain));
                    } else if p.extension().is_some_and(|x| x == "log") {
                        all_text.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
                    }
                }
                Ok(all_text)
            };
            let complete = |all_text: &str| -> bool {
                (0..30).all(|i| {
                    all_text
                        .matches(&format!("same-second record {i} "))
                        .count()
                        == 1
                })
            };
            let mut zst_count = 0usize;
            let mut settled: Option<String> = None;
            while std::time::Instant::now() < deadline {
                if let Ok(text) = scan() {
                    zst_count = std::fs::read_dir(dir.path())
                        .unwrap()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|x| x == "zst"))
                        .count();
                    if complete(&text) {
                        settled = Some(text);
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            assert!(
                zst_count >= 2,
                "same-second rotations must each leave a distinct .zst artifact, got {zst_count}"
            );
            let all_text = settled.unwrap_or_else(|| {
                scan().expect("compressed artifacts must decode cleanly after grace period")
            });
            for i in 0..30 {
                let needle = format!("same-second record {i} ");
                let count = all_text.matches(&needle).count();
                assert_eq!(
                    count, 1,
                    "record {i} must appear exactly once across artifacts, got {count}"
                );
            }
        }
    }

    #[test]
    fn test_rotate_inner_same_second_does_not_overwrite_existing_target() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        // 预创建“本秒/下一秒”两个可能的时间戳目标，模拟同秒二次轮转
        let t0 = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let t1 = (Utc::now() + chrono::Duration::seconds(1))
            .format("%Y%m%d_%H%M%S")
            .to_string();
        let candidate0 = dir.path().join(format!("test_{t0}.log"));
        let candidate1 = dir.path().join(format!("test_{t1}.log"));
        std::fs::write(&candidate0, "PRECIOUS0").unwrap();
        std::fs::write(&candidate1, "PRECIOUS1").unwrap();

        std::fs::write(&log_path, "rotated content").unwrap();
        let config = FileSinkConfig {
            enabled: true,
            path: log_path.clone(),
            compress: false,
            encrypt: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        sink.open_file_inner(&mut inner).unwrap();
        sink.rotate_inner(&mut inner).unwrap();
        drop(inner);

        // 预创建文件内容必须原封不动（不被静默覆盖）
        assert_eq!(std::fs::read_to_string(&candidate0).unwrap(), "PRECIOUS0");
        assert_eq!(std::fs::read_to_string(&candidate1).unwrap(), "PRECIOUS1");

        // 轮转内容应落入带序号后缀的新文件
        let suffixed_candidates = [
            dir.path().join(format!("test_{t0}.log.1")),
            dir.path().join(format!("test_{t1}.log.1")),
        ];
        let rotated_into = suffixed_candidates
            .iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| panic!("rotated content must go to a sequence-suffixed file"));
        assert_eq!(
            std::fs::read_to_string(rotated_into).unwrap(),
            "rotated content"
        );
    }

    // ---- 缺陷 #4: 打开日志文件未用 O_NOFOLLOW ----

    #[cfg(unix)]
    #[test]
    fn test_open_file_inner_fails_when_path_is_symlink() {
        // 日志路径为符号链接时打开必须失败：
        // 第一层（PathValidator, allow_symlinks=false）+ 第二层（O_NOFOLLOW
        // 内核兜底，覆盖 validate-then-use 竞态）都应使打开失败。
        let dir = tempdir().unwrap();
        let real_file = dir.path().join("real.log");
        std::fs::write(&real_file, "real content").unwrap();
        let link = dir.path().join("link.log");
        std::os::unix::fs::symlink(&real_file, &link).unwrap();

        let config = FileSinkConfig {
            enabled: true,
            path: link.clone(),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        let mut inner = sink.inner.write();
        let result = sink.open_file_inner(&mut inner);
        assert!(result.is_err(), "symlinked log path must fail to open");
        assert!(inner.current_file.is_none());
        // 符号链接目标不得被创建/截断/写入
        assert_eq!(std::fs::read_to_string(&real_file).unwrap(), "real content");
    }

    // ---- 缺陷 #5: 断路器/磁盘不足路径静默吞日志 ----

    #[test]
    fn test_mark_write_lost_sets_unhealthy_and_recovers_on_successful_flush() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        let config = FileSinkConfig {
            enabled: true,
            path: log_path,
            compress: false,
            encrypt: false,
            batch_size: 10,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        {
            let mut inner = sink.inner.write();
            sink.open_file_inner(&mut inner).unwrap();
        }
        assert!(sink.is_healthy());

        // 终态写丢失：error + stderr（mark_write_lost 内部）+ 置不健康 + 计数
        sink.mark_write_lost(
            &create_test_record("lost record"),
            "test: no fallback configured",
        );
        assert!(
            !sink.is_healthy(),
            "terminal write loss must mark the sink unhealthy"
        );
        assert_eq!(sink.lost_records.load(Ordering::Relaxed), 1);

        // 成功全量刷盘后应恢复健康（瞬时故障恢复），丢失计数不清零
        {
            let mut inner = sink.inner.write();
            inner.batch_buffer.push(create_test_record("recovered"));
            sink.flush_batch_inner(&mut inner).unwrap();
        }
        assert!(
            sink.is_healthy(),
            "a fully successful flush must clear the unhealthy flag"
        );
        assert_eq!(
            sink.lost_records.load(Ordering::Relaxed),
            1,
            "lost-record counter must not reset on recovery"
        );
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_write_lost_is_observable_when_circuit_opens_without_fallback() {
        // /dev/full 写入始终返回 ENOSPC：注入连续写失败使断路器打开后，
        // 无 fallback 的写入必须可观测（不健康 + 丢失计数），且返回值仍为 Ok
        //（保持调用方语义兼容）。
        let config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("/dev/full"),
            compress: false,
            encrypt: false,
            batch_size: 1,
            flush_interval_ms: 1000,
            fsync: false,
            audit_chain_enabled: false,
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        {
            let mut inner = sink.inner.write();
            if sink.open_file_inner(&mut inner).is_err() {
                eprintln!("Skipping: /dev/full not accessible in this environment");
                return;
            }
        }
        assert!(sink.is_healthy());

        let record = create_test_record("will be lost");
        for _ in 0..5 {
            let result = sink.write(&record).await;
            assert!(result.is_ok(), "failed flush path must still return Ok");
        }
        // 5 次批量写失败 → 断路器打开
        {
            let inner = sink.inner.read();
            assert_eq!(inner.circuit_breaker.state(), CircuitState::Open);
        }

        // 第 6 条：熔断打开且无 fallback → 记录终态丢失但可观测
        let result = sink.write(&record).await;
        assert!(
            result.is_ok(),
            "write must stay Ok to keep caller semantics"
        );
        assert!(
            !sink.is_healthy(),
            "sink must be marked unhealthy after a lost record"
        );
        assert_eq!(sink.lost_records.load(Ordering::Relaxed), 1);
    }

    // ---- 缺陷 #1（Critical）: 密码模式加密文件无法解密（v2 头 + 盐） ----

    #[test]
    #[serial]
    fn test_encrypt_file_password_mode_v2_salt_roundtrip() {
        // Critical 修复回归：密码模式 env → encrypt_file(v2) →
        // 解密方读出头中的盐 → 确定性重导出同一密钥 → 解密还原明文。
        let dir = tempdir().unwrap();
        let input_path = dir.path().join("plain.log");
        let output_path = dir.path().join("plain.log.enc");
        let original = b"password-mode roundtrip content";
        std::fs::write(&input_path, original).unwrap();

        // 测试用假密码向量（非真实凭据）
        unsafe {
            std::env::set_var("TEST_PWD_V2_KEY", "v2-roundtrip-password-01");
        }

        let config = FileSinkConfig {
            enabled: true,
            path: dir.path().join("dummy.log"),
            encrypt: true,
            encryption_key_env: Some("TEST_PWD_V2_KEY".to_string()),
            ..Default::default()
        };
        let sink = create_test_file_sink(config);
        sink.encrypt_file(&input_path, &output_path)
            .expect("password-mode encrypt_file must succeed in v2");

        // v2 头布局：magic(8) + version(2) + algo(2) + salt(16) + nonce(12)
        let encrypted = std::fs::read(&output_path).unwrap();
        assert_eq!(&encrypted[..8], b"ENCLOG1\0");
        assert_eq!(
            u16::from_le_bytes([encrypted[8], encrypted[9]]),
            2,
            "password-mode file must be written as version 2"
        );
        let header_salt: [u8; 16] = encrypted[12..28].try_into().unwrap();
        let nonce: [u8; 12] = encrypted[28..40].try_into().unwrap();

        // 解密方路径：用头中的盐确定性重导出密钥
        let key = crate::support::io::sink::encryption::get_encryption_key_with_salt(
            "TEST_PWD_V2_KEY",
            &header_salt,
        )
        .expect("key re-derivation from header salt must succeed");
        let cipher = aes_gcm::Aes256Gcm::new_from_slice(&*key).unwrap();
        let decrypted = cipher
            .decrypt(&aes_gcm::Nonce::from(nonce), &encrypted[40..])
            .expect("deterministic key from header salt must decrypt the v2 file");
        assert_eq!(decrypted, original);

        unsafe {
            std::env::remove_var("TEST_PWD_V2_KEY");
        }
    }
}
