// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! High-performance file sink using crossbeam channels.

use super::LogSink;
use super::file::{process_rotated, resolve_rotation_target};
use crate::FileSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::LogTemplate;
use crate::support::audit_chain::ArchiveChain;
use crate::support::io::sink::encryption::{
    KeyMaterial, resolve_key_material, validate_key_entropy,
};
use crate::validation::PathValidatorConfig;
use async_trait::async_trait;
use crossbeam_channel;
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration as StdDuration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressureStrategy {
    #[default]
    Block,
    DropOldest,
    DropNewest,
}

/// 轮转参数（size/time 任一触发即轮转）。
///
/// **时间语义为滚动间隔近似**：自上次轮转起算（`last_rotation.elapsed()`），
/// 与 FileSink 的日历对齐不同（FileSink daily = 次日零点、monthly = 次月
/// 同日零点，见 `calculate_next_rotation_time`）。能力矩阵与文档按
/// 「滚动近似」如实标注，不宣称一致。
#[derive(Debug, Clone, Copy)]
struct RotationParams {
    /// size 触发阈值（字节）；None = 不按大小轮转
    max_bytes: Option<u64>,
    /// time 触发间隔；None = 不按时间轮转
    interval: Option<StdDuration>,
}

/// 轮转触发判定（纯函数）：size 达阈值或 time 距上次轮转超间隔。
fn rotation_due(
    max_bytes: Option<u64>,
    current: u64,
    interval: Option<StdDuration>,
    last_rotation: Instant,
) -> bool {
    if max_bytes.is_some_and(|max| current >= max) {
        return true;
    }
    interval.is_some_and(|interval| last_rotation.elapsed() >= interval)
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
    cleanup_thread: Option<thread::JoinHandle<()>>,
}

pub struct ChannelBufferedFileSink {
    config: ChannelBufferedConfig,
    template: LogTemplate,
    sender: crossbeam_channel::Sender<String>,
    receiver: crossbeam_channel::Receiver<String>,
    // Option wrapper is kept to allow future replacement of the writer
    // during runtime (e.g., for log rotation), even though it is not
    // currently exercised. Removing it would require significant refactoring.
    file: Arc<Mutex<Option<BufWriter<File>>>>,
    inner: Mutex<Inner>,
    shutdown_flag: Arc<AtomicBool>,
    bytes_written: Arc<AtomicUsize>,
    flush_count: Arc<AtomicUsize>,
    dropped_count: Arc<AtomicUsize>,
    write_error_count: Arc<AtomicUsize>,
    /// None = base_config.masking_enabled=false（落盘原样）
    masker: Option<crate::DataMasker>,
    /// 当前活动文件字节量（轮转 size 判定；轮转后归零）。
    ///
    /// 已知失真（有意保留，与 FileSink 的 `current_size` 读文件元数据语义
    /// 不同）：进程重启后从 0 起算，未含重启前活动文件已有体积——实际文件
    /// 可增长到 `max_bytes + 既有体积` 才触发 size 轮转。如需严格阈值，
    /// 改为构造期用文件元数据初始化（行为变更需评审）。
    current_file_bytes: Arc<AtomicU64>,
    /// 轮转参数（size/time，R-inklog-003）
    rotation: RotationParams,
    /// 上次轮转时刻（io 线程判定 time 触发；测试可注入合成时刻）
    last_rotation: Arc<Mutex<Instant>>,
    /// 归档审计链（audit_chain_enabled 时启用；轮转成功后 append 并写穿
    /// manifest，与 FileSink 同一范式）
    audit_chain: Option<Arc<Mutex<ArchiveChain>>>,
    /// 轮转产物加密密钥材料（encrypt 时构造期已显性解析；None = 不加密）
    key_material: Option<KeyMaterial>,
}

impl ChannelBufferedFileSink {
    /// 注入自定义 masker（覆盖 base_config.masking_enabled 的默认构造）。
    pub fn with_masker(mut self, masker: crate::DataMasker) -> Self {
        self.masker = Some(masker);
        self
    }

    /// 渲染前按 base_config.masking_enabled 对记录做 PII 掩码。
    ///
    /// 返回的行以 `\n` 结尾：LogTemplate 是纯渲染契约（不含换行），逐行可
    /// grep 的行分隔在 sink 写入层补齐，取舍与 FileSink 的 `writeln!` 语义一致。
    fn render_masked(&self, record: &LogRecord) -> String {
        let mut rendered = match self.masker.as_ref() {
            Some(masker) => {
                let mut fields = record.fields.clone();
                masker.mask_hashmap(&mut fields);
                let masked = LogRecord {
                    message: masker.mask(&record.message),
                    fields,
                    ..record.clone()
                };
                self.template.render(&masked)
            }
            None => self.template.render(record),
        };
        rendered.push('\n');
        rendered
    }
    pub fn new(config: ChannelBufferedConfig, template: LogTemplate) -> Result<Self, InklogError> {
        // vuln-0002 对齐：与 FileSink.open_file_inner 相同的路径校验语义，
        // 在 create_dir_all / open 之前拒绝路径遍历与敏感组件。
        // 注意：deny_components 需与 src/support/io/sink/file.rs 保持一致。
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
        let validation_result = validator.validate(&config.base_config.path);
        if !validation_result.valid {
            let reason = validation_result
                .error
                .unwrap_or_else(|| "unknown".to_string());
            let mut args = crate::i18n::MsgArgs::new();
            args.set("path", config.base_config.path.display().to_string());
            args.set("reason", reason.clone());
            tracing::warn!("{}", crate::i18n::tr_args("sink-file_reject_path", args));
            let mut err_args = crate::i18n::MsgArgs::new();
            err_args.set("reason", reason);
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-unsafe_path_rejected",
                err_args,
            )));
        }

        // R-inklog-003：加密密钥在构造期显性解析——显式启用加密而 key
        // 缺失/无效在此返回 Err（禁止静默明文落盘）。key 来源 env 优先、
        // 其次配置文件（encryption_key_file，unix 权限 0600），与 fallback
        // journal / FileSink 共享同一解析范式；默认 env 名与 FileSink 一致。
        let key_material = if config.base_config.encrypt {
            let default_env = "LOG_ENCRYPTION_KEY".to_string();
            let env_name = config
                .base_config
                .encryption_key_env
                .as_ref()
                .unwrap_or(&default_env);
            let material = resolve_key_material(
                env_name,
                config
                    .base_config
                    .encryption_key_file
                    .as_deref()
                    .map(std::path::Path::new),
            )?;
            // 纵深防御：直接用作密钥的原始字节须过熵校验（拒绝全零等弱密钥）
            if let KeyMaterial::Raw(key) = &material {
                validate_key_entropy(key.as_slice())?;
            }
            Some(material)
        } else {
            None
        };

        // 归档审计链：与 FileSink 同范式——INKLOG_AUDIT_KEY 为链密钥；
        // 缺失则禁用（随机密钥会让 manifest 事后不可验，宁缺毋滥）
        let audit_chain = if config.base_config.audit_chain_enabled {
            match std::env::var("INKLOG_AUDIT_KEY") {
                Ok(key) if !key.is_empty() => {
                    Some(Arc::new(Mutex::new(ArchiveChain::new(key.as_bytes()))))
                }
                _ => {
                    tracing::warn!("{}", crate::i18n::tr("audit-chain-key-missing"));
                    None
                }
            }
        } else {
            None
        };

        let rotation = Self::rotation_params(&config.base_config);
        let masker = config
            .base_config
            .masking_enabled
            .then(crate::DataMasker::new);
        let (sender, receiver) = crossbeam_channel::bounded(config.channel_capacity);
        let file_path = config.base_config.path.clone();
        let file = Self::open_file(&file_path)?;
        let file = Arc::new(Mutex::new(Some(BufWriter::new(file))));

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let bytes_written = Arc::new(AtomicUsize::new(0));
        let flush_count = Arc::new(AtomicUsize::new(0));
        let dropped_count = Arc::new(AtomicUsize::new(0));
        let write_error_count = Arc::new(AtomicUsize::new(0));

        let sink = Self {
            config,
            template,
            sender,
            receiver,
            file,
            inner: Mutex::new(Inner {
                io_thread: None,
                flush_thread: None,
                cleanup_thread: None,
            }),
            shutdown_flag,
            bytes_written,
            flush_count,
            dropped_count,
            write_error_count,
            masker,
            current_file_bytes: Arc::new(AtomicU64::new(0)),
            rotation,
            last_rotation: Arc::new(Mutex::new(Instant::now())),
            audit_chain,
            key_material,
        };

        sink.start_io_thread();
        sink.start_flush_thread();
        sink.start_cleanup_timer();

        Ok(sink)
    }

    /// 轮转参数解析：`max_size` 走共享 size 解析（不可解析/0 视为不按大小
    /// 轮转）；`rotation_time` 为滚动间隔近似（hourly/daily/weekly/monthly →
    /// 固定秒数，自上次轮转起算）——FileSink 是日历对齐（daily=次日零点等），
    /// 二者语义差异见 [`RotationParams`] 文档。
    fn rotation_params(cfg: &FileSinkConfig) -> RotationParams {
        let max_bytes = super::rotation::parse_size(&cfg.max_size)
            .ok()
            .filter(|&bytes| bytes > 0);
        let interval = match cfg.rotation_time.as_str() {
            "hourly" => Some(StdDuration::from_secs(3600)),
            "daily" => Some(StdDuration::from_secs(86400)),
            "weekly" => Some(StdDuration::from_secs(604800)),
            "monthly" => Some(StdDuration::from_secs(2592000)),
            _ => Some(StdDuration::from_secs(86400)),
        };
        RotationParams {
            max_bytes,
            interval,
        }
    }

    /// 活动文件打开：与 FileSink.open_file_inner 相同的三处审计级加固——
    /// 新建父目录 0700、活动文件 0600、O_NOFOLLOW（vuln-0004 TOCTOU 修复）。
    /// 回落判定收窄后默认配置（含 encrypt/compress）全部走本路径，
    /// 权限不得比 FileSink 退化。
    fn open_file(path: &PathBuf) -> Result<File, InklogError> {
        let parent_newly_created = path
            .parent()
            .is_some_and(|p| !p.as_os_str().is_empty() && !p.exists());
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(InklogError::IoError)?;
        }
        // 新建的日志目录收敛为 0700（已存在目录不回改用户权限）
        #[cfg(unix)]
        if parent_newly_created && let Some(parent) = path.parent() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
        // O_NOFOLLOW + 0600：内核层拒绝末段符号链接，日志可能含 PII/敏感上下文
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
                .open(path)
                .map_err(InklogError::IoError)
        }
        #[cfg(not(unix))]
        {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(InklogError::IoError)
        }
    }

    fn start_io_thread(&self) {
        let receiver = self.receiver.clone();
        let file = self.file.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let bytes_written = self.bytes_written.clone();
        let write_error_count = self.write_error_count.clone();
        let current_file_bytes = self.current_file_bytes.clone();
        let last_rotation = self.last_rotation.clone();
        let audit_chain = self.audit_chain.clone();
        let key_material = self.key_material.clone();
        let base_path = self.config.base_config.path.clone();
        let base_config = self.config.base_config.clone();
        let rotation = self.rotation;
        let batch_size = self.config.flush_batch_size;

        let handle = thread::spawn(move || {
            let mut batch = Vec::with_capacity(batch_size);

            loop {
                if shutdown_flag.load(Ordering::Acquire) {
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
                            tracing::error!(
                                kind = %e.kind(),
                                "ChannelBufferedFileSink: Write error: {}",
                                e
                            );
                            write_error_count.fetch_add(1, Ordering::Relaxed);
                        } else {
                            bytes_written.fetch_add(entry.len(), Ordering::Relaxed);
                            current_file_bytes.fetch_add(entry.len() as u64, Ordering::Relaxed);
                        }
                    }
                    if let Err(e) = writer.flush() {
                        tracing::error!(
                            kind = %e.kind(),
                            "ChannelBufferedFileSink: Flush error: {}",
                            e
                        );
                        write_error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }

                // R-inklog-003：批次落盘后判定轮转（size/time 任一触发）
                let due = {
                    let last = last_rotation.lock();
                    rotation_due(
                        rotation.max_bytes,
                        current_file_bytes.load(Ordering::Relaxed),
                        rotation.interval,
                        *last,
                    )
                };
                if due {
                    Self::rotate_locked(
                        &mut file_guard,
                        &base_path,
                        &current_file_bytes,
                        &last_rotation,
                        &audit_chain,
                        &base_config,
                        key_material.as_ref(),
                    );
                }
            }

            // Drain remaining messages from the channel before exiting.
            // Use try_recv() to avoid blocking after shutdown flag is set.
            // R-inklog-003：drain 与主循环同一轮转判定与 current_file_bytes
            // 记账——此前 drain 绕过 rotation_due，shutdown 时通道积压可无限
            // 越过 max_size（size 轮转断言偶发失败、活动文件超限的根因）。
            loop {
                let mut batch: Vec<String> = Vec::with_capacity(batch_size);
                while batch.len() < batch_size {
                    match receiver.try_recv() {
                        Ok(entry) => batch.push(entry),
                        Err(_) => break,
                    }
                }
                if batch.is_empty() {
                    break;
                }
                let mut file_guard = file.lock();
                if let Some(writer) = file_guard.as_mut() {
                    for entry in &batch {
                        match writer.write_all(entry.as_bytes()) {
                            Ok(()) => {
                                bytes_written.fetch_add(entry.len(), Ordering::Relaxed);
                                current_file_bytes.fetch_add(entry.len() as u64, Ordering::Relaxed);
                            }
                            Err(e) => {
                                tracing::error!(
                                    kind = %e.kind(),
                                    "ChannelBufferedFileSink: Write error during drain: {}",
                                    e
                                );
                                write_error_count.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                // drain 期间同样执行轮转判定（size/time 任一触发）
                let due = {
                    let last = last_rotation.lock();
                    rotation_due(
                        rotation.max_bytes,
                        current_file_bytes.load(Ordering::Relaxed),
                        rotation.interval,
                        *last,
                    )
                };
                if due {
                    Self::rotate_locked(
                        &mut file_guard,
                        &base_path,
                        &current_file_bytes,
                        &last_rotation,
                        &audit_chain,
                        &base_config,
                        key_material.as_ref(),
                    );
                }
            }

            // Final flush
            let mut file_guard = file.lock();
            if let Some(writer) = file_guard.as_mut()
                && let Err(e) = writer.flush()
            {
                tracing::error!(
                    kind = %e.kind(),
                    "ChannelBufferedFileSink: Final flush error: {}",
                    e
                );
                write_error_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        self.inner.lock().io_thread = Some(handle);
    }

    fn start_flush_thread(&self) {
        let file = self.file.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let interval_ms = self.config.flush_interval_ms;
        let flush_count = self.flush_count.clone();
        let write_error_count = self.write_error_count.clone();

        let handle = thread::spawn(move || {
            loop {
                if shutdown_flag.load(Ordering::Acquire) {
                    break;
                }
                thread::sleep(StdDuration::from_millis(interval_ms));
                if shutdown_flag.load(Ordering::Acquire) {
                    break;
                }
                let mut file_guard = file.lock();
                if let Some(writer) = file_guard.as_mut() {
                    if let Err(e) = writer.flush() {
                        tracing::error!(
                            kind = %e.kind(),
                            "ChannelBufferedFileSink: Periodic flush error: {}",
                            e
                        );
                        write_error_count.fetch_add(1, Ordering::Relaxed);
                    }
                    flush_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        self.inner.lock().flush_thread = Some(handle);
    }

    /// 保留清理定时器（与 FileSink 同节奏：cleanup_interval_minutes 间隔，
    /// 复用其 perform_cleanup 单一事实源；shutdown_flag 响应式分段睡眠）。
    fn start_cleanup_timer(&self) {
        let interval = StdDuration::from_secs(
            self.config
                .base_config
                .cleanup_interval_minutes
                .saturating_mul(60)
                .max(1),
        );
        let handle = self.spawn_cleanup_thread(interval);
        self.inner.lock().cleanup_thread = Some(handle);
    }

    fn spawn_cleanup_thread(&self, check_interval: StdDuration) -> thread::JoinHandle<()> {
        let shutdown_flag = self.shutdown_flag.clone();
        let config = self.config.base_config.clone();
        let path = self.config.base_config.path.clone();

        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                loop {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    // 拆分长 sleep 为 100ms 段，每段检查 shutdown_flag
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
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Err(e) =
                        crate::support::io::sink::file::FileSink::perform_cleanup(&config, &path)
                    {
                        let mut args = crate::i18n::MsgArgs::new();
                        args.set("err", &e);
                        tracing::error!(
                            "{}",
                            crate::i18n::tr_args("sink-channel_buffered_cleanup_failed", args)
                        );
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
                args.set("msg", &msg);
                tracing::error!(
                    "{}",
                    crate::i18n::tr_args("sink-channel_buffered_cleanup_panic", args)
                );
            }
        })
    }

    /// 执行轮转（须持文件锁）：flush → 改名（冲突安全）→ 重开 → 重置计数
    /// → 审计链接记 → 后台压缩/加密。命名与归档产物范式与 FileSink 一致
    ///（复用其 resolve_rotation_target / sha256_file / process_rotated）。
    /// 加密用构造期解析的 `key_material`（`None` = 未启用加密）。
    fn rotate_locked(
        file: &mut Option<BufWriter<File>>,
        base_path: &PathBuf,
        current_file_bytes: &AtomicU64,
        last_rotation: &Mutex<Instant>,
        audit_chain: &Option<Arc<Mutex<ArchiveChain>>>,
        base_config: &FileSinkConfig,
        key_material: Option<&KeyMaterial>,
    ) {
        if let Some(writer) = file.as_mut() {
            let _ = writer.flush();
        }
        let _ = file.take();

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let rotated_path = resolve_rotation_target(base_path, &timestamp);
        if base_path.exists() && std::fs::rename(base_path, &rotated_path).is_err() {
            // 改名失败（跨设备等）：复制后删除兜底，与 FileSink 一致
            if std::fs::copy(base_path, &rotated_path).is_ok() {
                let _ = std::fs::remove_file(base_path);
            } else {
                // rename+copy 双失败：轮转产物缺位且源文件被续写——持续失败
                // 环境轮转永久失效，必须显性上报（对齐 process_rotated 失败路径）
                // 日志面走 i18n；ops_event 的 payload 保留英文原始描述（机器可读面，
                // 与 file.rs / 本文件 archive 失败路径的既有约定一致）
                let detail = format!(
                    "rotation rename and copy both failed for {}",
                    base_path.display()
                );
                let mut args = crate::i18n::MsgArgs::new();
                args.set("path", base_path.display());
                tracing::error!(
                    "{}",
                    crate::i18n::tr_args("sink-rotate_rename_copy_both_failed", args)
                );
                crate::support::ops_event::publish_internal(
                    "sink_degraded",
                    Some("file"),
                    serde_json::json!({ "op": "rotation", "error": detail }),
                );
            }
        }

        // 重开活动文件；失败则句柄置空（后续写走 file_guard None 分支，
        // 记录经 channel 计数不丢——与写错误路径语义一致）
        match Self::open_file(base_path) {
            Ok(new_file) => {
                *file = Some(BufWriter::new(new_file));
            }
            Err(e) => tracing::error!(
                path = %base_path.display(),
                "ChannelBufferedFileSink: reopen after rotation failed: {}",
                e
            ),
        }
        current_file_bytes.store(0, Ordering::Relaxed);
        *last_rotation.lock() = Instant::now();

        // 归档审计链登记与压缩/加密后处理都在后台线程执行（sha256 全文件读 +
        // manifest 全量重写是 O(条目数) 工作，不得占用 io 线程），
        // 失败可观测不反压写路径
        if audit_chain.is_some() || base_config.compress || base_config.encrypt {
            let config = base_config.clone();
            let archive_path = rotated_path.clone();
            let audit_chain = audit_chain.clone();
            let base_path = base_path.clone();
            let key_material = key_material.cloned();
            let _ = thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    if let Some(chain) = audit_chain.as_ref() {
                        crate::support::io::sink::file::register_rotated_archive(
                            chain,
                            &base_path,
                            &archive_path,
                        );
                    }
                    if (config.compress || config.encrypt)
                        && let Err(e) =
                            process_rotated(&config, &archive_path, key_material.as_ref())
                    {
                        let mut args = crate::i18n::MsgArgs::new();
                        args.set("err", &e);
                        tracing::error!(
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
                    tracing::error!(
                        "{}",
                        crate::i18n::tr_args("sink-rotate_postprocess_panicked", args)
                    );
                }
            });
        }
    }

    fn try_write(&self, record: &LogRecord) -> bool {
        let entry = self.render_masked(record);
        match self.config.backpressure_strategy {
            // Block 的重试等待由 async write 路径的 write_blocking 处理，
            // 这里只做单次非阻塞尝试。
            BackpressureStrategy::Block => match self.sender.try_send(entry) {
                Ok(()) => true,
                Err(crossbeam_channel::TrySendError::Full(_)) => false,
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    // Channel is dead; don't count as "dropped" since sink is shutting down
                    false
                }
            },
            BackpressureStrategy::DropNewest => match self.sender.try_send(entry) {
                Ok(()) => true,
                Err(crossbeam_channel::TrySendError::Full(_)) => {
                    self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    // Channel is dead; don't count as "dropped" since sink is shutting down
                    false
                }
            },
            BackpressureStrategy::DropOldest => match self.sender.try_send(entry) {
                Ok(()) => true,
                Err(crossbeam_channel::TrySendError::Full(entry)) => {
                    // Try to evict the oldest entry to make room.
                    // The evicted entry is consumed by this thread and
                    // discarded, so it counts as one drop.
                    let evicted = self.receiver.try_recv().is_ok();
                    if evicted {
                        self.dropped_count.fetch_add(1, Ordering::Relaxed);
                    }
                    match self.sender.try_send(entry) {
                        Ok(()) => true,
                        Err(_) => {
                            // Retry also failed — the new entry is dropped too.
                            self.dropped_count.fetch_add(1, Ordering::Relaxed);
                            false
                        }
                    }
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    // Channel is dead; don't count as "dropped" since sink is shutting down
                    false
                }
            },
        }
    }

    /// Block 策略的 async 写入路径。
    ///
    /// 用 `try_send` + 短退避重试代替阻塞式 `sender.send()`：channel 满时
    /// 让出线程休眠 1ms 后重试，避免卡死 tokio worker 线程。channel 断开
    /// （sink 关闭中）时不计为丢弃，返回 false。
    async fn write_blocking(&self, record: &LogRecord) -> bool {
        let mut entry = self.render_masked(record);
        loop {
            match self.sender.try_send(entry) {
                Ok(()) => return true,
                Err(crossbeam_channel::TrySendError::Full(returned)) => {
                    entry = returned;
                    tokio::time::sleep(StdDuration::from_millis(1)).await;
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    // Channel is dead; don't count as "dropped" since sink is shutting down
                    return false;
                }
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
            write_error_count: self.write_error_count.load(Ordering::Relaxed),
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
    pub write_error_count: usize,
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
        self.shutdown_flag.store(true, Ordering::Release);

        // Join threads to ensure all pending writes are completed
        {
            let mut inner = self.inner.lock();
            if let Some(handle) = inner.io_thread.take() {
                let _ = handle.join();
            }
            if let Some(handle) = inner.flush_thread.take() {
                let _ = handle.join();
            }
            if let Some(handle) = inner.cleanup_thread.take() {
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
        let sent = match self.config.backpressure_strategy {
            BackpressureStrategy::Block => self.write_blocking(record).await,
            BackpressureStrategy::DropNewest | BackpressureStrategy::DropOldest => {
                self.try_write(record)
            }
        };
        if !sent {
            let mut args = crate::i18n::MsgArgs::new();
            args.set(
                "count",
                self.dropped_count
                    .load(std::sync::atomic::Ordering::Relaxed),
            );
            return Err(InklogError::ChannelError(crate::i18n::tr_args(
                "config-ring_buffer_dropped",
                args,
            )));
        }
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
        // Ensure shutdown is called, but don't double-join threads.
        // Use catch_unwind to prevent nested panic if shutdown_inner panics
        // (e.g., from a poisoned mutex or a panicking thread join).
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = self.shutdown_inner();
        }));
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
    async fn test_records_are_newline_separated() {
        // 落盘记录必须逐行分隔（可 grep）：模板不含换行，sink 写入层补换行
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("newline_sep.log");

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
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        sink.write(&make_record("first-record")).await.unwrap();
        sink.write(&make_record("second-record")).await.unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(
            data.ends_with('\n'),
            "every record must end with a newline, got: {data:?}"
        );
        let lines: Vec<&str> = data.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "two records must produce exactly two lines, got: {data:?}"
        );
        assert!(
            lines[0].contains("first-record") && !lines[0].contains("second-record"),
            "line 1 must be the first record intact, got: {:?}",
            lines[0]
        );
        assert!(
            lines[1].contains("second-record") && !lines[1].contains("first-record"),
            "line 2 must be the second record intact, got: {:?}",
            lines[1]
        );
    }

    #[tokio::test]
    async fn test_render_masks_pii_when_enabled() {
        // 出口掩码：默认（masking_enabled=true）落盘内容不得含明文手机号
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring-masked.log");

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
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        let mut rec = make_record("user phone 13812345678 login");
        rec.fields.insert(
            "contact".to_string(),
            serde_json::Value::String("13912345678".to_string()),
        );
        sink.write(&rec).await.unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(
            !data.contains("13812345678") && !data.contains("13912345678"),
            "rendered file must not contain plaintext PII: {data}"
        );
    }

    #[tokio::test]
    async fn test_render_passes_through_when_masking_disabled() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring-raw.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: path.clone(),
                masking_enabled: false,
                ..Default::default()
            },
            channel_capacity: 64,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 16,
            flush_interval_ms: 50,
        };
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        sink.write(&make_record("raw phone 13812345678"))
            .await
            .unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(
            data.contains("13812345678"),
            "masking off must pass through verbatim: {data}"
        );
    }

    #[cfg(feature = "secret-scan")]
    #[tokio::test]
    async fn test_outbound_secret_scan_gate_masks_bare_secret() {
        // 出站层挂点：secret-scan 门经 with_masker 挂到 sink 写出前的掩码点，
        // 无键名上下文的裸 secret 在落盘前被替换，且门计数对调用方可见
        use crate::support::processing::{SecretPatternRegistry, SecretScanGate};

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring-secret-scan.log");

        let gate = SecretScanGate::new(SecretPatternRegistry::with_builtins());
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
        let masker = crate::DataMasker::builder()
            .with_secret_scan(gate.clone())
            .build();
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default())
            .unwrap()
            .with_masker(masker);

        sink.write(&make_record(
            "connected with sk-proj-abcdefghijklmnopqrstuvwxyz123456",
        ))
        .await
        .unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(
            !data.contains("sk-proj-abcdefghijklmnopqrstuvwxyz123456"),
            "bare secret must not reach disk: {data}"
        );
        assert!(
            data.contains("***REDACTED_API_KEY***"),
            "gate marker must be on disk: {data}"
        );
        assert_eq!(
            gate.hit_count("openai_sk"),
            Some(1),
            "outbound gate must attribute the hit"
        );
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
            // `flush_batch_size = 0` keeps the IO thread from draining the
            // channel, so overflow (and thus DropNewest drops) is deterministic
            // instead of racing the IO thread's drain speed.
            flush_batch_size: 0,
            flush_interval_ms: 1000,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        // Write far more messages than the channel can buffer: with the IO
        // thread not draining, every write beyond capacity 2 is dropped and
        // counted, regardless of scheduler timing.
        for i in 0..10_000 {
            let rec = make_record(&format!("drop-newest-{i}"));
            // DropNewest strategy returns error when record is dropped
            let _ = sink.write(&rec).await;
        }

        let m = sink.metrics();
        assert!(
            m.dropped_count > 0,
            "DropNewest should have dropped some records"
        );
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
            // Same rationale as test_backpressure_drop_newest: the IO thread
            // never drains, so every write beyond capacity evicts the oldest
            // entry deterministically.
            flush_batch_size: 0,
            flush_interval_ms: 1000,
        };
        let tmpl = LogTemplate::default();
        let sink = ChannelBufferedFileSink::new(cfg, tmpl).unwrap();

        // With no draining, each write beyond capacity evicts one record and
        // increments dropped_count deterministically.
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
    #[cfg(unix)]
    async fn test_open_file_hardening_matches_filesink() {
        // 审计级加固对齐 FileSink：活动文件 0600、新建父目录 0700、
        // O_NOFOLLOW（符号链接目标打开须失败）
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("created-parent").join("ring_perm.log");
        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: nested.clone(),
                ..Default::default()
            },
            ..Default::default()
        };
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();
        sink.write(&make_record("perm-probe")).await.unwrap();
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();

        let file_mode = std::fs::metadata(&nested).unwrap().permissions().mode();
        assert_eq!(
            file_mode & 0o777,
            0o600,
            "CBFS active file must be 0600 (FileSink parity), got {:o}",
            file_mode & 0o777
        );
        let dir_mode = std::fs::metadata(dir.path().join("created-parent"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            dir_mode & 0o777,
            0o700,
            "newly created log parent dir must be 0700, got {:o}",
            dir_mode & 0o777
        );

        // O_NOFOLLOW：日志路径被符号链接替换时打开必须失败（TOCTOU 关闭）
        let link_dir = TempDir::new().unwrap();
        let target = link_dir.path().join("real.log");
        std::fs::write(&target, "victim").unwrap();
        let link = link_dir.path().join("link.log");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let open_result = ChannelBufferedFileSink::open_file(&link);
        assert!(
            open_result.is_err(),
            "O_NOFOLLOW must reject symlinked log path"
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "victim",
            "symlink target must not be touched"
        );
    }

    #[test]
    fn test_new_rejects_path_traversal_components() {
        // vuln-0002 对齐：含 `..` 组件的路径必须在构造期被拒绝
        // （与 FileSink 的路径校验加固保持一致），返回构造错误而非静默打开
        let dir = TempDir::new().unwrap();
        let traversal_path = dir.path().join("..").join("escaped.log");

        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: traversal_path,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = ChannelBufferedFileSink::new(cfg, LogTemplate::default());
        assert!(
            result.is_err(),
            "path with '..' component must be rejected at construction"
        );

        // 敏感组件（deny list）同样被拒绝
        let sensitive_path = dir.path().join(".ssh").join("leak.log");
        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: sensitive_path,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(
            ChannelBufferedFileSink::new(cfg, LogTemplate::default()).is_err(),
            "path with denied component '.ssh' must be rejected at construction"
        );
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
    // try_write 错误分支覆盖
    // Disconnected（channel 断开）在三种策略下都不计入 dropped_count，
    // 写入返回 false；DropOldest retry 失败仍计入丢弃。
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
        // Block 策略下 sender 断开：写入返回 false，不计 dropped_count
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
        assert_eq!(
            m.dropped_count, 0,
            "dropped_count should NOT be incremented for disconnected (channel is dead, not 'dropped')"
        );
        // async write 路径同样返回 Err 且不累计丢弃
        assert!(sink.write(&record).await.is_err());
        assert_eq!(sink.metrics().dropped_count, 0);
    }

    #[tokio::test]
    async fn test_try_write_drop_newest_strategy_disconnected() {
        // DropNewest 策略下 channel 断开：写入返回 false，不计 dropped_count
        // （与 DropOldest 的 Disconnected 语义对齐）
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
        assert_eq!(
            m.dropped_count, 0,
            "dropped_count should NOT be incremented for disconnected (channel is dead, not 'dropped')"
        );
        // async write 路径同样返回 Err 且不累计丢弃
        assert!(sink.write(&record).await.is_err());
        assert_eq!(sink.metrics().dropped_count, 0);
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
        assert_eq!(
            m.dropped_count, 0,
            "dropped_count should NOT be incremented for disconnected (channel is dead, not 'dropped')"
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

    #[tokio::test]
    async fn test_async_write_block_strategy_completes_when_channel_full() {
        // Block 策略下 channel 满：async write 应通过退避重试完成，
        // 而不是阻塞 OS 线程或返回错误
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("block_full.log");
        let mut sink = make_shutdown_sink(BackpressureStrategy::Block, path).await;

        // 替换为容量 1 且已满的 channel；50ms 后由测试线程腾出空间
        let (tx, rx) = crossbeam_channel::bounded::<String>(1);
        tx.send("filler".to_string()).expect("Failed to fill");
        let old_sender = std::mem::replace(&mut sink.sender, tx);
        let old_receiver = std::mem::replace(&mut sink.receiver, rx.clone());
        drop(old_sender);
        drop(old_receiver);

        let drainer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            // 只取走 filler，为新记录腾出空间
            let _ = rx.try_recv();
        });

        let start = std::time::Instant::now();
        let record = make_record("block-full-retry");
        sink.write(&record)
            .await
            .expect("async write should complete on a full channel via backoff retry");
        assert!(
            start.elapsed() >= std::time::Duration::from_millis(40),
            "write should have waited for channel space through backoff retries"
        );
        drainer.join().unwrap();

        // 新记录应已入队（filler 已被取走）
        assert_eq!(sink.sender.len(), 1);
    }

    // 测试故意持有文件锁迫使 IO 线程阻塞、记录走 drain 路径；
    // 持锁跨 await 只阻塞 IO 线程，异步写入走的是 channel，无死锁风险
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn test_drain_loop_counts_bytes_written() {
        // shutdown 触发的 drain 循环中成功写入也应累计 bytes_written
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("drain_bytes.log");
        let tmpl = LogTemplate::default();

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
        let sink = ChannelBufferedFileSink::new(cfg, tmpl.clone()).unwrap();

        // 先占住文件锁，使 IO 线程在写出批次时阻塞，
        // 随后置 shutdown 标志再补发记录，迫使这部分记录走 drain 路径
        let file_guard = sink.file.lock();
        for i in 0..5 {
            let rec = make_record(&format!("drain-first-{i}"));
            sink.write(&rec).await.unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
        sink.shutdown_flag.store(true, Ordering::Release);
        let mut expected_bytes = 0usize;
        for i in 0..5 {
            let rec = make_record(&format!("drain-second-{i}"));
            expected_bytes += tmpl.render(&rec).len();
            sink.write(&rec).await.unwrap();
        }
        drop(file_guard);

        sink.shutdown().await.unwrap();

        let m = sink.metrics();
        assert!(
            m.bytes_written >= expected_bytes,
            "drained writes should count towards bytes_written, got {} < {}",
            m.bytes_written,
            expected_bytes
        );
        let data = std::fs::read_to_string(&path).unwrap();
        assert!(data.contains("drain-second-4"));
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
        let sink = match ChannelBufferedFileSink::new(cfg, tmpl) {
            Ok(sink) => sink,
            Err(_) => {
                eprintln!("Skipping: /dev/full not accessible in this environment");
                return;
            }
        };

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

    // ========================================================================
    // R-inklog-003：CBFS 高级能力（轮转 size/time、压缩、加密、审计链、
    // 保留清理）。manager 回落判定相应收窄后，此处为唯一文件写路径行为
    // 基准（FileSink 仅保留 fsync/JSON 出站/磁盘空间守卫）。
    // ========================================================================

    /// 隔离保留清理干扰的测试配置：retention 拉满（3650 天 / 100 个 /
    /// 30 天周期），轮转产物不会被清理线程误删。
    fn rotation_cfg(
        path: std::path::PathBuf,
        max_size: &str,
        rotation_time: &str,
    ) -> ChannelBufferedConfig {
        ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path,
                max_size: max_size.to_string(),
                rotation_time: rotation_time.to_string(),
                compress: false,
                keep_files: 100,
                retention_days: 3650,
                cleanup_interval_minutes: 60 * 24 * 30,
                ..Default::default()
            },
            channel_capacity: 64,
            backpressure_strategy: BackpressureStrategy::Block,
            flush_batch_size: 16,
            flush_interval_ms: 50,
        }
    }

    fn rotated_files(dir: &std::path::Path, stem: &str) -> Vec<std::path::PathBuf> {
        let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&format!("{stem}_")))
            })
            .collect();
        found.sort();
        found
    }

    /// 轮转触发判定（纯函数，size/time 任一满足即轮转）。
    #[test]
    fn test_rotation_due_decision() {
        let now = std::time::Instant::now();
        // size 未达/已达
        assert!(!rotation_due(Some(1000), 999, None, now));
        assert!(rotation_due(Some(1000), 1000, None, now));
        // time 未到/已过（hourly = 3600s；合成 7200s 前的 last_rotation）
        let stale = now - std::time::Duration::from_secs(7200);
        assert!(!rotation_due(
            None,
            0,
            Some(std::time::Duration::from_secs(3600)),
            now
        ));
        assert!(rotation_due(
            None,
            0,
            Some(std::time::Duration::from_secs(3600)),
            stale
        ));
        // 双条件：任一触发
        assert!(rotation_due(
            Some(1000),
            500,
            Some(std::time::Duration::from_secs(3600)),
            stale
        ));
        // 未配置任何条件：永不轮转
        assert!(!rotation_due(None, u64::MAX, None, stale));
    }

    #[tokio::test]
    async fn test_rotation_by_size_creates_rotated_files() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_size.log");
        let sink = ChannelBufferedFileSink::new(
            rotation_cfg(path.clone(), "1KB", "daily"),
            LogTemplate::default(),
        )
        .unwrap();

        for i in 0..60 {
            sink.write(&make_record(&format!("size-rot-{i}-{}", "x".repeat(30))))
                .await
                .unwrap();
        }
        sink.shutdown().await.unwrap();

        let rotated = rotated_files(dir.path(), "ring_size");
        assert!(
            !rotated.is_empty(),
            "size rotation must produce rotated files, dir: {:?}",
            dir.path().read_dir().unwrap().collect::<Vec<_>>()
        );
        // 全部记录跨轮转产物保留（顺序文件级不乱、总量守恒）
        let mut all = String::new();
        for f in rotated_files(dir.path(), "ring_size")
            .iter()
            .chain(std::iter::once(&path))
        {
            all.push_str(&std::fs::read_to_string(f).unwrap_or_default());
        }
        for i in 0..60 {
            assert!(
                all.contains(&format!("size-rot-{i}")),
                "record {i} must survive rotation, got {all:?}"
            );
        }
        // 轮转后活动文件重置（不再超限）
        let active_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        assert!(
            active_len <= 2048,
            "active file must be reset after rotation, got {active_len}"
        );
    }

    #[tokio::test]
    async fn test_rotation_by_time_after_interval() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_time.log");
        let sink = ChannelBufferedFileSink::new(
            rotation_cfg(path.clone(), "1GB", "hourly"),
            LogTemplate::default(),
        )
        .unwrap();

        // 合成时间触发：把 last_rotation 拨回 7200s 前（hourly 间隔已过）
        *sink.last_rotation.lock() =
            std::time::Instant::now() - std::time::Duration::from_secs(7200);

        sink.write(&make_record("time-rot-marker")).await.unwrap();
        sink.shutdown().await.unwrap();

        let rotated = rotated_files(dir.path(), "ring_time");
        assert!(
            !rotated.is_empty(),
            "time-triggered rotation must produce a rotated file"
        );
        let rotated_content = std::fs::read_to_string(&rotated[0]).unwrap();
        assert!(
            rotated_content.contains("time-rot-marker"),
            "written record must land in the rotated file, got: {rotated_content:?}"
        );
        // 轮转后新活动文件存在且为空（本轮转批次之后无写入）
        let active = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !active.contains("time-rot-marker"),
            "marker must not appear in the new active file"
        );
    }

    #[tokio::test]
    async fn test_encrypt_without_key_fails_explicitly() {
        // 复用 journal 加密（R-inklog-002）的 key 管理范式：显式启用加密而
        // key 缺失 → 构造期显性失败
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_enc_nokey.log");
        unsafe {
            std::env::remove_var("INKLOG_TEST_CBFS_ENC_MISSING");
        }
        let mut cfg = rotation_cfg(path.clone(), "1KB", "daily");
        cfg.base_config.encrypt = true;
        cfg.base_config.encryption_key_env = Some("INKLOG_TEST_CBFS_ENC_MISSING".to_string());

        let result = ChannelBufferedFileSink::new(cfg, LogTemplate::default());
        let Err(err) = result else {
            panic!("encrypt without key must fail construction explicitly");
        };
        assert!(
            err.to_string().contains("INKLOG_TEST_CBFS_ENC_MISSING"),
            "error must name the key source, got: {err}"
        );
        assert!(
            !path.exists(),
            "no plaintext file may be created when key is missing"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_encrypted_rotation_roundtrip() {
        use aes_gcm::KeyInit as _;
        use aes_gcm::aead::Aead as _;
        use base64::{Engine as _, engine::general_purpose};

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_enc.log");
        let key: [u8; 32] = core::array::from_fn(|i| (i as u8).wrapping_mul(41).wrapping_add(7));
        unsafe {
            std::env::set_var(
                "INKLOG_TEST_CBFS_ENC",
                general_purpose::STANDARD.encode(key).as_str(),
            );
        }
        let mut cfg = rotation_cfg(path.clone(), "1KB", "daily");
        cfg.base_config.encrypt = true;
        cfg.base_config.encryption_key_env = Some("INKLOG_TEST_CBFS_ENC".to_string());
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        for i in 0..40 {
            sink.write(&make_record(&format!("enc-rot-{i}-{}", "y".repeat(30))))
                .await
                .unwrap();
        }
        sink.shutdown().await.unwrap();

        // 轮转产物被加密为 .enc（v2 头：ENCLOG1\0 + version + algo + salt + nonce + ct）
        // 轮询等待后台归档线程完成加密（shutdown 不 join 后台归档线程）
        let mut all_plain;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut enc_files: Vec<std::path::PathBuf> = std::fs::read_dir(dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "enc"))
                .collect();
            enc_files.sort();
            assert!(
                !enc_files.is_empty(),
                "rotated files must be encrypted to .enc"
            );
            all_plain = String::new();
            for enc in &enc_files {
                let data = std::fs::read(enc).unwrap();
                assert!(data.starts_with(b"ENCLOG1\0"), "v2 magic required");
                let salt = &data[12..28];
                let nonce = &data[28..40];
                let ciphertext = &data[40..];
                // 原始密钥分支与盐无关（与 FileSink v2 语义一致）
                let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).unwrap();
                let plain = cipher
                    .decrypt(&aes_gcm::Nonce::try_from(nonce).unwrap(), ciphertext)
                    .expect("decryption with the same env key must succeed");
                let _ = salt;
                all_plain.push_str(&String::from_utf8_lossy(&plain));
            }
            // 尾部记录在最后一次轮转阈值前落入活动文件（活动文件不加密，
            // 与 FileSink 只加密轮转归档的语义一致）
            all_plain.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            let complete = (0..40).all(|i| all_plain.contains(&format!("enc-rot-{i}")));

            if complete || std::time::Instant::now() >= deadline {
                for i in 0..40 {
                    assert!(
                        all_plain.contains(&format!("enc-rot-{i}")),
                        "record {i} must survive encrypted rotation"
                    );
                }
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        unsafe {
            std::env::remove_var("INKLOG_TEST_CBFS_ENC");
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_audit_chain_records_rotations() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_chain.log");
        unsafe {
            std::env::set_var("INKLOG_AUDIT_KEY", "cbfs-audit-chain-key-01");
        }
        let mut cfg = rotation_cfg(path.clone(), "1KB", "daily");
        cfg.base_config.audit_chain_enabled = true;
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        for i in 0..40 {
            sink.write(&make_record(&format!("chain-rot-{i}-{}", "z".repeat(30))))
                .await
                .unwrap();
        }
        sink.shutdown().await.unwrap();

        // manifest 与 FileSink 同范式：`<stem>.chain.jsonl`，条目含 path+sha256+ts。
        // 登记在轮转后台线程执行（sha256 全文件读 + manifest 全量重写不占
        // io 线程）；shutdown 不 join 后台线程 → 轮询等待
        let manifest = dir.path().join("ring_chain.chain.jsonl");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let body = loop {
            let body = std::fs::read_to_string(&manifest).unwrap_or_default();
            if !body.is_empty() || std::time::Instant::now() >= deadline {
                break body;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        };
        assert!(!body.is_empty(), "audit chain manifest must have entries");

        let entries: Vec<crate::support::audit_chain::ArchiveChainEntry> = body
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .expect("manifest must be valid JSONL");
        assert!(!entries.is_empty());
        assert!(
            crate::support::audit_chain::ArchiveChain::verify_entries(
                &entries,
                b"cbfs-audit-chain-key-01"
            ),
            "chain must verify with the audit key"
        );
        // 条目 sha256 与轮转产物内容一致
        let first = &entries[0];
        let rotated_path = std::path::PathBuf::from(
            first
                .event
                .split_once("\"path\":\"")
                .and_then(|(_, rest)| rest.split('"').next())
                .expect("entry must carry path"),
        );
        let expected = crate::support::io::sink::file::FileSink::sha256_file(&rotated_path);
        assert!(
            first.event.contains(&expected.unwrap_or_default()),
            "chain entry sha256 must match the rotated file"
        );

        unsafe {
            std::env::remove_var("INKLOG_AUDIT_KEY");
        }
    }

    #[cfg(any(feature = "zstd", feature = "gzip"))]
    #[tokio::test]
    async fn test_compressed_rotation_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_comp.log");
        let mut cfg = rotation_cfg(path.clone(), "1KB", "daily");
        cfg.base_config.compress = true;
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        for i in 0..40 {
            sink.write(&make_record(&format!("comp-rot-{i}-{}", "c".repeat(30))))
                .await
                .unwrap();
        }
        sink.shutdown().await.unwrap();

        // 轮询等待后台归档线程完成压缩（shutdown 不 join 后台归档线程）。
        // 活动文件不参与归档压缩（与 FileSink 只压缩轮转产物的语义一致），
        // 尾部记录留在活动文件中，一并纳入断言。
        let mut all;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let mut artifacts: Vec<std::path::PathBuf> = std::fs::read_dir(dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "zst" || e == "gz"))
                .collect();
            artifacts.sort();
            assert!(
                !artifacts.is_empty(),
                "compressed rotation must produce .zst/.gz artifacts"
            );
            all = String::new();
            for artifact in &artifacts {
                let data = std::fs::read(artifact).unwrap();
                // 分支级 feature 门控：单后端组合（仅 zstd / 仅 gzip）下
                // 另一后端的解码代码不得参与编译（E0433）
                if artifact.extension().is_some_and(|e| e == "zst") {
                    #[cfg(feature = "zstd")]
                    {
                        let plain = zstd::stream::decode_all(&data[..]).unwrap();
                        all.push_str(&String::from_utf8_lossy(&plain));
                    }
                } else {
                    #[cfg(feature = "gzip")]
                    {
                        use std::io::Read as _;
                        let mut decoder = flate2::read::GzDecoder::new(&data[..]);
                        let mut plain = String::new();
                        decoder.read_to_string(&mut plain).unwrap();
                        all.push_str(&plain);
                    }
                }
            }
            all.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            let complete = (0..40).all(|i| all.contains(&format!("comp-rot-{i}")));
            if complete || std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        for i in 0..40 {
            assert!(
                all.contains(&format!("comp-rot-{i}")),
                "record {i} must survive compressed rotation"
            );
        }
    }

    #[tokio::test]
    async fn test_cleanup_timer_enforces_retention() {
        use filetime::{FileTime, set_file_mtime};

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ring_keep.log");
        let mut cfg = rotation_cfg(path.clone(), "1GB", "daily");
        cfg.base_config.keep_files = 1;
        cfg.base_config.retention_days = 0;
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default()).unwrap();

        // 伪造 3 个"轮转产物"（旧 mtime，触发 keep_files=1 + retention=0 清理）
        let old = FileTime::from_unix_time(0, 0);
        for i in 0..3 {
            let f = dir.path().join(format!("ring_keep_old{i}.log"));
            std::fs::write(&f, "stale\n").unwrap();
            set_file_mtime(&f, old).unwrap();
        }

        // 测试直接注入短周期清理线程（共享实例 shutdown_flag，shutdown 时退出）
        let handle = sink.spawn_cleanup_thread(std::time::Duration::from_millis(50));
        sink.inner.lock().cleanup_thread = Some(handle);

        // 轮询等待清理：只保留最新 1 个，其余删除
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut remaining = 3usize;
        while std::time::Instant::now() < deadline {
            remaining = (0..3)
                .filter(|i| dir.path().join(format!("ring_keep_old{i}")).exists())
                .count();
            if remaining <= 1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(
            remaining <= 1,
            "retention cleanup must enforce keep_files, {remaining} stale files left"
        );
        sink.shutdown().await.unwrap();
    }

    // ========================================================================
    // 新旧路径行为对拍（R-inklog-003）：同一批记录经 FileSink 与 CBFS
    // 落盘后（level, target, message) 三元组集合一致。
    // 注：两侧时间戳渲染格式不同（FileSink rfc3339 / 模板毫秒 UTC），
    // 对拍剥离时间戳；格式语义差异由模板文档承载。
    // ========================================================================

    #[tokio::test]
    async fn test_parity_file_sink_vs_cbfs() {
        let dir = TempDir::new().unwrap();
        let file_sink_path = dir.path().join("parity_file.log");
        let cbfs_path = dir.path().join("parity_cbfs.log");

        let file_cfg = FileSinkConfig {
            path: file_sink_path.clone(),
            compress: false,
            ..Default::default()
        };
        let file_sink = crate::support::io::FileSink::new(file_cfg).unwrap();
        let cbfs = ChannelBufferedFileSink::new(
            rotation_cfg(cbfs_path.clone(), "1GB", "daily"),
            LogTemplate::default(),
        )
        .unwrap();

        for i in 0..20 {
            let rec = make_record(&format!("parity-{i}"));
            file_sink.write(&rec).await.unwrap();
            cbfs.write(&rec).await.unwrap();
        }
        file_sink.flush().await.unwrap();
        file_sink.shutdown().await.unwrap();
        cbfs.flush().await.unwrap();
        cbfs.shutdown().await.unwrap();

        let parse = |data: &str| -> Vec<String> {
            // 行形如 "ts [LEVEL] target - message"：剥离时间戳取语义三元组
            let mut keys = Vec::new();
            for line in data.lines() {
                let Some((_, rest)) = line.split_once("] ") else {
                    continue;
                };
                let Some((head, msg)) = rest.rsplit_once(" - ") else {
                    continue;
                };
                keys.push(format!("{head}|{msg}"));
            }
            keys.sort();
            keys
        };
        let file_keys = parse(&std::fs::read_to_string(&file_sink_path).unwrap());
        let cbfs_keys = parse(&std::fs::read_to_string(&cbfs_path).unwrap());
        assert_eq!(file_keys.len(), 20, "FileSink must persist all records");
        assert_eq!(cbfs_keys.len(), 20, "CBFS must persist all records");
        assert_eq!(
            file_keys, cbfs_keys,
            "both paths must persist identical (level, target, message) sets"
        );
    }

    #[test]
    fn test_with_masker_overrides_default() {
        let dir = TempDir::new().unwrap();
        let cfg = ChannelBufferedConfig {
            base_config: FileSinkConfig {
                path: dir.path().join("masker.log"),
                ..Default::default()
            },
            ..Default::default()
        };
        let sink = ChannelBufferedFileSink::new(cfg, LogTemplate::default())
            .unwrap()
            .with_masker(crate::DataMasker::new());
        assert!(
            sink.masker.is_some(),
            "with_masker must store the injected masker"
        );
    }

    #[test]
    fn test_rotation_params_maps_named_intervals() {
        // 命名间隔 → 固定秒数（滚动近似）；未知值回退 daily
        let interval_of = |rotation_time: &str| {
            let cfg = FileSinkConfig {
                rotation_time: rotation_time.to_string(),
                ..Default::default()
            };
            ChannelBufferedFileSink::rotation_params(&cfg).interval
        };
        assert_eq!(interval_of("hourly"), Some(StdDuration::from_secs(3600)));
        assert_eq!(interval_of("daily"), Some(StdDuration::from_secs(86400)));
        assert_eq!(interval_of("weekly"), Some(StdDuration::from_secs(604800)));
        assert_eq!(
            interval_of("monthly"),
            Some(StdDuration::from_secs(2592000))
        );
        assert_eq!(
            interval_of("nonsense"),
            Some(StdDuration::from_secs(86400)),
            "unknown rotation_time must fall back to daily"
        );
    }
}
