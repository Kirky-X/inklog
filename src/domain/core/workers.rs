// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Worker thread management for log sinks.

use super::LoggerManager;
use super::recovery::SinkControlMessage;
use crate::InklogConfig;
use crate::Metrics;
use crate::support::io::LogSink;
use crate::{InklogError, LogRecord};
use chrono::Utc;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// DatabaseSink 工厂闭包类型
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
type DbSinkFactory = Box<
    dyn Fn(
            Arc<dyn crate::integrations::Database>,
            Arc<Metrics>,
        ) -> Result<Box<dyn LogSink>, InklogError>
        + Send
        + Sync,
>;

/// Parameters for worker threads
pub(crate) struct WorkerParams {
    pub(crate) config: InklogConfig,
    pub(crate) receiver: Receiver<Arc<LogRecord>>,
    pub(crate) console_receiver: Receiver<Arc<LogRecord>>,
    pub(crate) control_rx: Receiver<SinkControlMessage>,
    pub(crate) control_tx: Sender<SinkControlMessage>,
    pub(crate) metrics: Arc<Metrics>,
    /// Mutex 保护的是 sink 句柄（`Arc<dyn LogSink>`）而非 sink 本体：
    /// 锁内只做句柄克隆，异步写必须在锁外执行（MutexGuard 不得横跨 block_on）
    pub(crate) console_sink: Arc<Mutex<Arc<dyn LogSink>>>,
    /// 同上：锁内仅克隆 `Option<Arc<dyn LogSink>>`，异步写在锁外执行
    pub(crate) error_sink: Arc<Mutex<Option<Arc<dyn LogSink>>>>,
    pub(crate) effective_capacity: Arc<AtomicUsize>,
    /// FileSink 工厂闭包（用于初始创建和恢复，打破具体类型依赖）
    pub(crate) file_sink_factory:
        Box<dyn Fn() -> Result<Box<dyn LogSink>, InklogError> + Send + Sync>,
    /// DatabaseSink 工厂闭包（用于初始创建和恢复，内部处理 set_metrics）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub(crate) db_sink_factory: DbSinkFactory,
    /// 注入的数据库依赖（DI 模式）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub(crate) database: Option<Arc<dyn crate::integrations::Database>>,
    /// 数据库 sink 专用数据 channel 接收端（独立于 file worker 的 receiver）
    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    pub(crate) db_receiver: Option<Receiver<Arc<LogRecord>>>,
}

/// `start_workers` 返回值类型别名，避免 clippy `type_complexity` 警告。
/// 第一项为 worker 线程句柄，第二项为每个 worker 对应的 shutdown 信号 sender。
pub(crate) type WorkerStartResult =
    Result<(Vec<tokio::task::JoinHandle<()>>, Vec<Sender<()>>), InklogError>;

// ============================================================================
// Extracted pure functions (testable without runtime/threads)
// ============================================================================

/// Check whether auto-recovery should be attempted based on consecutive
/// failure count and elapsed time since the last failure.
pub(crate) fn should_auto_recover(
    consecutive_failures: u32,
    last_failure_time: Option<Instant>,
) -> bool {
    consecutive_failures > 5
        && last_failure_time
            .map(|t| t.elapsed() > Duration::from_secs(60))
            .unwrap_or(false)
}

/// Check whether a recovery attempt should be made, respecting a cooldown
/// period between attempts.
pub(crate) fn should_attempt_recovery(last_attempt: Option<&Instant>, cooldown: Duration) -> bool {
    match last_attempt {
        None => true,
        Some(inst) => inst.elapsed() > cooldown,
    }
}

/// Result of classifying a [`SinkControlMessage`] for a specific target sink.
pub(crate) enum ControlAction {
    /// Attempt to recover the target sink.
    Recover,
    /// Report status (GetStatus received).
    Status,
    /// Message is for a different sink; ignore.
    Ignore,
}

/// Classify a control message relative to a target sink name.
pub(crate) fn classify_control_message(
    msg: &SinkControlMessage,
    target_sink: &str,
) -> ControlAction {
    match msg {
        SinkControlMessage::RecoverSink(name) if name == target_sink => ControlAction::Recover,
        SinkControlMessage::GetStatus => ControlAction::Status,
        _ => ControlAction::Ignore,
    }
}

/// Compute the new adaptive channel capacity given current usage.
///
/// Returns the updated capacity value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_adaptive_capacity(
    current_eff: usize,
    channel_len: usize,
    min_capacity: usize,
    max_capacity: usize,
    expand_threshold_percent: u8,
    shrink_threshold_percent: u8,
    shrink_wait: Duration,
    low_usage_since: &mut Option<Instant>,
) -> usize {
    let usage = if current_eff > 0 {
        channel_len as f64 / current_eff as f64
    } else {
        0.0
    };
    let usage_percent = (usage * 100.0).round() as u8;

    if usage_percent >= expand_threshold_percent && current_eff < max_capacity {
        let grow_to = (current_eff + current_eff / 2).min(max_capacity);
        *low_usage_since = None;
        grow_to
    } else if usage_percent <= shrink_threshold_percent && current_eff > min_capacity {
        match low_usage_since {
            None => {
                *low_usage_since = Some(Instant::now());
                current_eff
            }
            Some(inst) => {
                if inst.elapsed() >= shrink_wait {
                    let shrink_to = (current_eff.saturating_mul(70) / 100).max(min_capacity);
                    *low_usage_since = None;
                    shrink_to
                } else {
                    current_eff
                }
            }
        }
    } else {
        *low_usage_since = None;
        current_eff
    }
}

/// sink 工厂失败重试的初始退避：1s 起，每轮翻倍，上限 30s。
/// 持续重试而非放弃，下游存储恢复后 worker 自动恢复写入。
const FACTORY_RETRY_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const FACTORY_RETRY_MAX_BACKOFF: Duration = Duration::from_secs(30);

/// 测试钩子：非零时覆盖工厂重试的初始退避（毫秒），避免测试等待真实的秒级退避。
/// 用完全限定路径声明，避免非测试构建出现未使用的导入。
#[cfg(test)]
static FACTORY_RETRY_INITIAL_BACKOFF_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// 当前生效的工厂重试初始退避（测试可注入更短值）。
fn factory_retry_initial_backoff() -> Duration {
    #[cfg(test)]
    {
        let ms = FACTORY_RETRY_INITIAL_BACKOFF_MS.load(Ordering::Relaxed);
        if ms > 0 {
            return Duration::from_millis(ms);
        }
    }
    FACTORY_RETRY_INITIAL_BACKOFF
}

/// sink 工厂持续失败（worker 无 sink 可用）期间到达记录的降级处理：
/// 尽力写入 error sink 保留内容，并递增 failed/dropped 指标与 sink 健康状态，
/// 绝不无声丢弃。锁内仅取句柄，异步写在锁外执行（与热路径约定一致）。
fn record_sink_unavailable(
    runtime_handle: &tokio::runtime::Handle,
    error_sink: &Arc<Mutex<Option<Arc<dyn LogSink>>>>,
    metrics: &Metrics,
    record: &Arc<LogRecord>,
    sink_name: &str,
) {
    metrics.inc_sink_error();
    metrics.inc_logs_dropped();
    metrics.update_sink_health(
        sink_name,
        false,
        Some("sink unavailable: factory keeps failing".to_string()),
    );
    let error_sink_handle = error_sink.lock().ok().and_then(|guard| guard.clone());
    if let Some(error_sink) = error_sink_handle {
        let _ = runtime_handle.block_on(async { error_sink.write(record).await });
    }
}

impl LoggerManager {
    pub(crate) fn start_workers(params: WorkerParams) -> WorkerStartResult {
        let runtime_handle = tokio::runtime::Handle::current();
        let WorkerParams {
            config,
            receiver,
            console_receiver,
            control_rx,
            control_tx,
            metrics,
            console_sink,
            error_sink,
            effective_capacity,
            file_sink_factory,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            db_sink_factory,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            database,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            db_receiver,
        } = params;
        let file_config = config.file_sink.clone();
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let db_config = config.database_sink.clone();

        // database 依赖由调用方（build_detached，async 上下文）保证有效：DI 注入优先，
        // 未注入且 db sink 启用时由 build_detached 在当前 runtime 上创建默认 DbNexusAdapter。
        // 核正：原先在此处经 Handle::current().block_on 同步创建——start_workers 在
        // runtime 线程上被调用时必然 panic（runtime-in-runtime），故上移至 async 层。

        // Thread 0: Console Sink (dedicated for lock-free hot path)
        // 每个 worker 拥有独立的 shutdown channel，确保广播信号能被每个 worker 接收
        // （MPMC channel 的 send() 只能被一个 receiver 消费，共享 channel 会导致
        // 只有首个 worker 收到信号、其余 worker 死循环）
        let (shutdown_tx_console, shutdown_console) = bounded(1);
        let metrics_console = metrics.clone();
        let console_sink_console = console_sink.clone();
        let handle_console = {
            let runtime_handle = runtime_handle.clone();
            tokio::task::spawn_blocking(move || {
                metrics_console.active_workers.inc();
                loop {
                    // Check for shutdown
                    if shutdown_console.try_recv().is_ok() {
                        // Drain with 5s timeout (console is fast)
                        let deadline = Instant::now() + Duration::from_secs(5);
                        while let Ok(record) = console_receiver.try_recv() {
                            let latency = Utc::now()
                                .signed_duration_since(record.timestamp)
                                .to_std()
                                .unwrap_or(Duration::ZERO);
                            metrics_console.record_latency(latency);

                            // Hot path: use try_lock to avoid blocking.
                            // 锁内仅克隆 sink 句柄，异步写在锁外执行，
                            // 避免MutexGuard 横跨 block_on
                            let sink = console_sink_console
                                .try_lock()
                                .ok()
                                .map(|guard| Arc::clone(&*guard));
                            match sink {
                                Some(sink) => {
                                    if runtime_handle
                                        .block_on(async { sink.write(&record).await })
                                        .is_err()
                                    {
                                        metrics_console.inc_sink_error();
                                    }
                                }
                                None => {
                                    // Lock contention detected, increment metric and skip
                                    metrics_console.inc_lock_contention();
                                }
                            }

                            if Instant::now() > deadline {
                                break;
                            }
                        }
                        break;
                    }

                    // Process console logs with timeout
                    match console_receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok(record) => {
                            let latency = Utc::now()
                                .signed_duration_since(record.timestamp)
                                .to_std()
                                .unwrap_or(Duration::ZERO);
                            metrics_console.record_latency(latency);

                            // Hot path: use try_lock to avoid blocking.
                            // 锁内仅克隆 sink 句柄，异步写在锁外执行，
                            // 避免MutexGuard 横跨 block_on
                            let sink = console_sink_console
                                .try_lock()
                                .ok()
                                .map(|guard| Arc::clone(&*guard));
                            match sink {
                                Some(sink) => {
                                    if runtime_handle
                                        .block_on(async { sink.write(&record).await })
                                        .is_err()
                                    {
                                        metrics_console.inc_sink_error();
                                        metrics_console.update_sink_health(
                                            "console",
                                            false,
                                            Some("Write error".to_string()),
                                        );
                                    } else {
                                        metrics_console.inc_logs_written();
                                        metrics_console.update_sink_health("console", true, None);
                                    }
                                }
                                None => {
                                    // Lock contention detected, increment metric and skip
                                    metrics_console.inc_lock_contention();
                                }
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            // Timeout, continue loop
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
                metrics_console.active_workers.dec();
            })
        };

        // Thread 1: File Sink
        let rx_file = receiver.clone();
        let (shutdown_tx_file, shutdown_file) = bounded(1);
        let metrics_file = metrics.clone();
        let console_sink_file = console_sink.clone();
        let error_sink_file = error_sink.clone();
        let control_rx_file = control_rx.clone();
        let handle_file = {
            let runtime_handle = runtime_handle.clone();
            tokio::task::spawn_blocking(move || {
                metrics_file.active_workers.inc();
                if let Some(cfg) = file_config
                    && cfg.enabled
                {
                    // 工厂启动失败不再让 worker 直接退出（否则主体无 sink 可写、
                    // 上游通道填满后记录被静默丢弃）：进入降级模式——error 日志
                    // + 指数退避持续重试工厂，期间到达的记录写入 error sink 并
                    // 计为 failed（见循环内的 record_sink_unavailable 与重试块）。
                    let mut sink: Option<Box<dyn LogSink>> = match file_sink_factory() {
                        Ok(sink) => Some(sink),
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "File sink factory failed on startup; entering degraded retry mode (exponential backoff: 1s doubling up to 30s, retrying indefinitely); records arriving during retry are forwarded to the error sink and counted as failed"
                            );
                            metrics_file.update_sink_health("file", false, Some(e.to_string()));
                            None
                        }
                    };
                    let mut consecutive_failures = 0;
                    #[allow(unused_assignments)]
                    let mut last_failure_time = None::<Instant>;
                    let mut factory_backoff = factory_retry_initial_backoff();
                    let mut last_factory_attempt = Instant::now();

                    loop {
                        // Check for shutdown
                        if shutdown_file.try_recv().is_ok() {
                            // Drain with 30s timeout
                            let deadline = Instant::now() + Duration::from_secs(30);
                            while let Ok(record) = rx_file.try_recv() {
                                let latency = Utc::now()
                                    .signed_duration_since(record.timestamp)
                                    .to_std()
                                    .unwrap_or(Duration::ZERO);
                                metrics_file.record_latency(latency);

                                // 降级模式（工厂持续失败）：无 sink 可写——保底到
                                // error sink 并计为 failed，绝不无声丢弃
                                let Some(sink) = sink.as_mut() else {
                                    record_sink_unavailable(
                                        &runtime_handle,
                                        &error_sink_file,
                                        &metrics_file,
                                        &record,
                                        "file",
                                    );
                                    if Instant::now() > deadline {
                                        break;
                                    }
                                    continue;
                                };

                                // Retry logic
                                let mut attempts = 0;
                                let mut write_succeeded = false;
                                while attempts < 3 {
                                    match runtime_handle
                                        .block_on(async { sink.write(&record).await })
                                    {
                                        Ok(_) => {
                                            metrics_file.inc_logs_written();
                                            metrics_file.update_sink_health("file", true, None);
                                            consecutive_failures = 0;
                                            last_failure_time = None;
                                            write_succeeded = true;
                                            break;
                                        }
                                        Err(e) => {
                                            attempts += 1;
                                            consecutive_failures += 1;
                                            last_failure_time = Some(Instant::now());

                                            // Log error to error.log
                                            // 锁内仅取句柄，异步写在锁外执行
                                            let error_sink_handle = error_sink_file
                                                .lock()
                                                .ok()
                                                .and_then(|guard| guard.clone());
                                            if let Some(error_sink) = error_sink_handle {
                                                let error_record = LogRecord {
                                                    timestamp: Utc::now(),
                                                    level: "ERROR".to_string(),
                                                    target: "inklog::file_sink".to_string(),
                                                    message: format!("File sink error: {}", e),
                                                    fields: Default::default(),
                                                    file: None,
                                                    line: None,
                                                    thread_id: thread::current()
                                                        .name()
                                                        .unwrap_or("unknown")
                                                        .to_string(),
                                                };
                                                let _ = runtime_handle.block_on(async {
                                                    error_sink.write(&error_record).await
                                                });
                                            }

                                            if attempts == 3 {
                                                metrics_file.inc_sink_error();
                                                metrics_file.update_sink_health(
                                                    "file",
                                                    false,
                                                    Some(e.to_string()),
                                                );
                                                // Fallback to console（与 console 热路径一致：
                                                // try_lock 争用时递增指标并跳过）
                                                let cs = console_sink_file
                                                    .try_lock()
                                                    .ok()
                                                    .map(|guard| Arc::clone(&*guard));
                                                if let Some(cs) = cs {
                                                    let _ = runtime_handle.block_on(async {
                                                        cs.write(&record).await
                                                    });
                                                } else {
                                                    metrics_file.inc_lock_contention();
                                                }
                                            } else {
                                                thread::sleep(Duration::from_millis(
                                                    10 * attempts as u64,
                                                ));
                                            }
                                        }
                                    }
                                }

                                // Auto-recovery trigger（与 DB worker 的 drain 循环保持一致）
                                if !write_succeeded
                                    && should_auto_recover(consecutive_failures, last_failure_time)
                                {
                                    tracing::warn!("{}", crate::i18n::tr("sink-file_auto_recovery"));
                                    if let Ok(new_sink) = file_sink_factory() {
                                        *sink = new_sink;
                                        consecutive_failures = 0;
                                        metrics_file.update_sink_health("file", true, None);
                                        tracing::info!(
                                            "{}",
                                            crate::i18n::tr("sink-file_auto_recovery_ok")
                                        );
                                    }
                                }

                                if Instant::now() > deadline {
                                    break;
                                }
                            }
                            if let Some(sink) = sink.as_ref() {
                                let _ = runtime_handle.block_on(async { sink.shutdown().await });
                            }
                            break;
                        }

                        // Check for control messages
                        if let Ok(control_msg) = control_rx_file.try_recv() {
                            match classify_control_message(&control_msg, "file") {
                                ControlAction::Recover => {
                                    tracing::info!(
                                        "{}",
                                        crate::i18n::tr("sink-file_recovery_received")
                                    );
                                    if let Ok(new_sink) = file_sink_factory() {
                                        sink = Some(new_sink);
                                        factory_backoff = factory_retry_initial_backoff();
                                        consecutive_failures = 0;
                                        last_failure_time = None;
                                        metrics_file.update_sink_health("file", true, None);
                                        tracing::info!(
                                            "{}",
                                            crate::i18n::tr("sink-file_recovered")
                                        );
                                    } else {
                                        tracing::error!(
                                            "{}",
                                            crate::i18n::tr("sink-file_recovery_failed")
                                        );
                                    }
                                }
                                ControlAction::Status => {
                                    // Status is already tracked in metrics
                                }
                                ControlAction::Ignore => {}
                            }
                        }

                        // 降级模式：工厂持续失败时的指数退避重试（1s→2s→…上限 30s），
                        // 持续重试而非放弃。用时间判断而非 sleep，循环保持即时响应
                        // shutdown 与控制消息；放在 recv 之前，降级路径 continue
                        // 跳过记录处理时也不会跳过重试。
                        if sink.is_none() && last_factory_attempt.elapsed() >= factory_backoff {
                            match file_sink_factory() {
                                Ok(new_sink) => {
                                    sink = Some(new_sink);
                                    factory_backoff = factory_retry_initial_backoff();
                                    metrics_file.update_sink_health("file", true, None);
                                    tracing::info!(
                                        "File sink factory succeeded after retry; worker resumed normal writes"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        next_retry_in_ms = factory_backoff.as_millis() as u64,
                                        "File sink factory retry failed; keeping the worker alive and retrying with exponential backoff"
                                    );
                                    factory_backoff =
                                        (factory_backoff * 2).min(FACTORY_RETRY_MAX_BACKOFF);
                                }
                            }
                            last_factory_attempt = Instant::now();
                        }

                        if let Ok(record) = rx_file.recv_timeout(Duration::from_millis(100)) {
                            let latency = Utc::now()
                                .signed_duration_since(record.timestamp)
                                .to_std()
                                .unwrap_or(Duration::ZERO);
                            metrics_file.record_latency(latency);

                            // 降级模式（工厂持续失败）：无 sink 可写——保底到
                            // error sink 并计为 failed，绝不无声丢弃
                            let Some(sink) = sink.as_mut() else {
                                record_sink_unavailable(
                                    &runtime_handle,
                                    &error_sink_file,
                                    &metrics_file,
                                    &record,
                                    "file",
                                );
                                continue;
                            };

                            // Retry logic with recovery detection
                            let mut attempts = 0;
                            let mut write_succeeded = false;
                            while attempts < 3 {
                                match runtime_handle.block_on(async { sink.write(&record).await }) {
                                    Ok(_) => {
                                        metrics_file.inc_logs_written();
                                        metrics_file.update_sink_health("file", true, None);
                                        consecutive_failures = 0;
                                        last_failure_time = None;
                                        write_succeeded = true;
                                        break;
                                    }
                                    Err(e) => {
                                        attempts += 1;
                                        consecutive_failures += 1;
                                        last_failure_time = Some(Instant::now());

                                        // Log error to error.log
                                        // 锁内仅取句柄，异步写在锁外执行
                                        let error_sink_handle = error_sink_file
                                            .lock()
                                            .ok()
                                            .and_then(|guard| guard.clone());
                                        if let Some(error_sink) = error_sink_handle {
                                            let error_record = LogRecord {
                                                timestamp: Utc::now(),
                                                level: "ERROR".to_string(),
                                                target: "inklog::file_sink".to_string(),
                                                message: format!("File sink error: {}", e),
                                                fields: Default::default(),
                                                file: None,
                                                line: None,
                                                thread_id: thread::current()
                                                    .name()
                                                    .unwrap_or("unknown")
                                                    .to_string(),
                                            };
                                            let _ = runtime_handle.block_on(async {
                                                error_sink.write(&error_record).await
                                            });
                                        }

                                        if attempts == 3 {
                                            metrics_file.inc_sink_error();
                                            metrics_file.update_sink_health(
                                                "file",
                                                false,
                                                Some(e.to_string()),
                                            );
                                            // Fallback to console（与 console 热路径一致：
                                            // try_lock 争用时递增指标并跳过）
                                            let cs = console_sink_file
                                                .try_lock()
                                                .ok()
                                                .map(|guard| Arc::clone(&*guard));
                                            if let Some(cs) = cs {
                                                let _ = runtime_handle.block_on(async {
                                                    cs.write(&record).await
                                                });
                                            } else {
                                                metrics_file.inc_lock_contention();
                                            }
                                        } else {
                                            thread::sleep(Duration::from_millis(
                                                10 * attempts as u64,
                                            ));
                                        }
                                    }
                                }
                            }

                            // Auto-recovery trigger
                            if !write_succeeded
                                && should_auto_recover(consecutive_failures, last_failure_time)
                            {
                                tracing::warn!("{}", crate::i18n::tr("sink-file_auto_recovery"));
                                if let Ok(new_sink) = file_sink_factory() {
                                    *sink = new_sink;
                                    consecutive_failures = 0;
                                    last_failure_time = None;
                                    metrics_file.update_sink_health("file", true, None);
                                    tracing::info!(
                                        "{}",
                                        crate::i18n::tr("sink-file_auto_recovery_ok")
                                    );
                                }
                            }
                        } else {
                            // Timeout, flush buffer（降级模式下无 sink 可 flush）
                            if let Some(sink) = sink.as_ref() {
                                let _ = runtime_handle.block_on(async { sink.flush().await });
                            }
                        }
                    }
                }
                metrics_file.active_workers.dec();
            })
        };

        // Thread 2: DB Sink
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let (shutdown_tx_db, shutdown_db) = bounded(1);
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let metrics_db = metrics.clone();
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let console_sink_db = console_sink.clone();
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let error_sink_db = error_sink.clone();
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let control_rx_db = control_rx.clone();
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let handle_db = {
            let runtime_handle = runtime_handle.clone();
            tokio::task::spawn_blocking(
                #[allow(unused_assignments)]
                move || {
                    metrics_db.active_workers.inc();
                    if let Some(cfg) = db_config
                        && cfg.enabled
                        && let Some(ref db) = database
                        && let Some(rx_db) = db_receiver
                    {
                        // Clone once before the loop for recovery use
                        let db_for_recovery = db.clone();
                        // 同 file worker：工厂启动失败进入降级模式（error 日志 +
                        // 指数退避持续重试，期间记录写入 error sink 并计为 failed），
                        // 不再静默跳过 worker 主体。下面的裸块保持既有作用域与缩进。
                        let mut sink: Option<Box<dyn LogSink>> =
                            match db_sink_factory(db.clone(), metrics_db.clone()) {
                                Ok(sink) => Some(sink),
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        "Database sink factory failed on startup; entering degraded retry mode (exponential backoff: 1s doubling up to 30s, retrying indefinitely); records arriving during retry are forwarded to the error sink and counted as failed"
                                    );
                                    metrics_db.update_sink_health(
                                        "database",
                                        false,
                                        Some(e.to_string()),
                                    );
                                    None
                                }
                            };
                        {
                            let mut consecutive_failures = 0;
                            #[allow(unused_assignments)]
                            let mut last_failure_time = None::<Instant>;
                            let mut factory_backoff = factory_retry_initial_backoff();
                            let mut last_factory_attempt = Instant::now();

                            loop {
                                if shutdown_db.try_recv().is_ok() {
                                    // Drain with 30s timeout
                                    let deadline = Instant::now() + Duration::from_secs(30);
                                    while let Ok(record) = rx_db.try_recv() {
                                        let latency = Utc::now()
                                            .signed_duration_since(record.timestamp)
                                            .to_std()
                                            .unwrap_or(Duration::ZERO);
                                        metrics_db.record_latency(latency);

                                        // 降级模式（工厂持续失败）：无 sink 可写——
                                        // 保底到 error sink 并计为 failed，绝不无声丢弃
                                        let Some(sink) = sink.as_mut() else {
                                            record_sink_unavailable(
                                                &runtime_handle,
                                                &error_sink_db,
                                                &metrics_db,
                                                &record,
                                                "database",
                                            );
                                            if Instant::now() > deadline {
                                                break;
                                            }
                                            continue;
                                        };

                                        // Retry logic
                                        let mut attempts = 0;
                                        let mut write_succeeded = false;
                                        let write_result: Result<(), InklogError> = runtime_handle
                                            .block_on(async { sink.write(&record).await });
                                        match write_result {
                                            Ok(_) => {
                                                metrics_db.inc_logs_written();
                                                metrics_db
                                                    .update_sink_health("database", true, None);
                                                consecutive_failures = 0;
                                                last_failure_time = None;
                                                write_succeeded = true;
                                            }
                                            Err(ref e) => {
                                                attempts += 1;
                                                consecutive_failures += 1;
                                                last_failure_time = Some(Instant::now());

                                                // Log error to error.log
                                                // 锁内仅取句柄，异步写在锁外执行
                                                let error_sink_handle = error_sink_db
                                                    .lock()
                                                    .ok()
                                                    .and_then(|guard| guard.clone());
                                                if let Some(error_sink) = error_sink_handle {
                                                    let error_record = LogRecord {
                                                        timestamp: Utc::now(),
                                                        level: "ERROR".to_string(),
                                                        target: "inklog::database_sink".to_string(),
                                                        message: format!(
                                                            "Database sink error: {}",
                                                            e
                                                        ),
                                                        fields: Default::default(),
                                                        file: None,
                                                        line: None,
                                                        thread_id: thread::current()
                                                            .name()
                                                            .unwrap_or("unknown")
                                                            .to_string(),
                                                    };
                                                    let _ = runtime_handle.block_on(async {
                                                        error_sink.write(&error_record).await
                                                    });
                                                }

                                                if attempts == 3 {
                                                    metrics_db.inc_sink_error();
                                                    let error_msg = format!("{e}");
                                                    metrics_db.update_sink_health(
                                                        "database",
                                                        false,
                                                        Some(error_msg),
                                                    );
                                                    // Fallback to console（与 console 热路径一致：
                                                    // try_lock 争用时递增指标并跳过）
                                                    let cs = console_sink_db
                                                        .try_lock()
                                                        .ok()
                                                        .map(|guard| Arc::clone(&*guard));
                                                    if let Some(cs) = cs {
                                                        let _ = runtime_handle.block_on(async {
                                                            cs.write(&record).await
                                                        });
                                                    } else {
                                                        metrics_db.inc_lock_contention();
                                                    }
                                                } else {
                                                    thread::sleep(Duration::from_millis(
                                                        10 * attempts as u64,
                                                    ));
                                                }
                                            }
                                        }

                                        // Auto-recovery trigger
                                        if !write_succeeded
                                            && should_auto_recover(
                                                consecutive_failures,
                                                last_failure_time,
                                            )
                                        {
                                            tracing::warn!(
                                                "{}",
                                                crate::i18n::tr("sink-db_auto_recovery")
                                            );
                                            if let Ok(new_sink) = db_sink_factory(
                                                db_for_recovery.clone(),
                                                metrics_db.clone(),
                                            ) {
                                                *sink = new_sink;
                                                consecutive_failures = 0;
                                                metrics_db
                                                    .update_sink_health("database", true, None);
                                                tracing::info!(
                                                    "{}",
                                                    crate::i18n::tr("sink-db_auto_recovery_ok")
                                                );
                                            }
                                        }

                                        if Instant::now() > deadline {
                                            break;
                                        }
                                    }
                                    if let Some(sink) = sink.as_ref() {
                                        let _ = runtime_handle
                                            .block_on(async { sink.shutdown().await });
                                    }
                                    break;
                                }

                                // Check for control messages
                                if let Ok(control_msg) = control_rx_db.try_recv() {
                                    match classify_control_message(&control_msg, "database") {
                                        ControlAction::Recover => {
                                            tracing::info!(
                                                "{}",
                                                crate::i18n::tr("sink-db_recovery_received")
                                            );
                                            if let Ok(new_sink) = db_sink_factory(
                                                db_for_recovery.clone(),
                                                metrics_db.clone(),
                                            ) {
                                                sink = Some(new_sink);
                                                factory_backoff =
                                                    factory_retry_initial_backoff();
                                                consecutive_failures = 0;
                                                last_failure_time = None;
                                                metrics_db
                                                    .update_sink_health("database", true, None);
                                                tracing::info!(
                                                    "{}",
                                                    crate::i18n::tr("sink-db_recovered")
                                                );
                                            } else {
                                                tracing::error!(
                                                    "{}",
                                                    crate::i18n::tr("sink-db_recovery_failed")
                                                );
                                            }
                                        }
                                        ControlAction::Status => {
                                            // Status is already tracked in metrics
                                        }
                                        ControlAction::Ignore => {}
                                    }
                                }

                                // 降级模式：工厂持续失败时的指数退避重试（1s→2s→…上限 30s），
                                // 持续重试而非放弃。用时间判断而非 sleep，循环保持即时响应
                                // shutdown 与控制消息；放在 recv 之前，降级路径 continue
                                // 跳过记录处理时也不会跳过重试。
                                if sink.is_none()
                                    && last_factory_attempt.elapsed() >= factory_backoff
                                {
                                    match db_sink_factory(
                                        db_for_recovery.clone(),
                                        metrics_db.clone(),
                                    ) {
                                        Ok(new_sink) => {
                                            sink = Some(new_sink);
                                            factory_backoff = factory_retry_initial_backoff();
                                            metrics_db.update_sink_health("database", true, None);
                                            tracing::info!(
                                                "Database sink factory succeeded after retry; worker resumed normal writes"
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                error = %e,
                                                next_retry_in_ms =
                                                    factory_backoff.as_millis() as u64,
                                                "Database sink factory retry failed; keeping the worker alive and retrying with exponential backoff"
                                            );
                                            factory_backoff = (factory_backoff * 2)
                                                .min(FACTORY_RETRY_MAX_BACKOFF);
                                        }
                                    }
                                    last_factory_attempt = Instant::now();
                                }

                                if let Ok(record) = rx_db.recv_timeout(Duration::from_millis(100)) {
                                    let latency = Utc::now()
                                        .signed_duration_since(record.timestamp)
                                        .to_std()
                                        .unwrap_or(Duration::ZERO);
                                    metrics_db.record_latency(latency);

                                    // 降级模式（工厂持续失败）：无 sink 可写——保底到
                                    // error sink 并计为 failed，绝不无声丢弃
                                    let Some(sink) = sink.as_mut() else {
                                        record_sink_unavailable(
                                            &runtime_handle,
                                            &error_sink_db,
                                            &metrics_db,
                                            &record,
                                            "database",
                                        );
                                        continue;
                                    };

                                    // Retry logic
                                    let mut attempts = 0;
                                    let mut write_succeeded = false;
                                    let write_result: Result<(), InklogError> = runtime_handle
                                        .block_on(async { sink.write(&record).await });
                                    match write_result {
                                        Ok(_) => {
                                            metrics_db.inc_logs_written();
                                            metrics_db.update_sink_health("database", true, None);
                                            consecutive_failures = 0;
                                            last_failure_time = None;
                                            write_succeeded = true;
                                        }
                                        Err(ref e) => {
                                            attempts += 1;
                                            consecutive_failures += 1;
                                            last_failure_time = Some(Instant::now());

                                            if attempts == 3 {
                                                metrics_db.inc_sink_error();
                                                let error_msg = format!("{e}");
                                                metrics_db.update_sink_health(
                                                    "database",
                                                    false,
                                                    Some(error_msg),
                                                );

                                                // Fallback chain: DB -> File -> Console
                                                // （与 console 热路径一致：try_lock 争用时
                                                // 递增指标并跳过）
                                                let cs = console_sink_db
                                                    .try_lock()
                                                    .ok()
                                                    .map(|guard| Arc::clone(&*guard));
                                                if let Some(cs) = cs {
                                                    let _ = runtime_handle.block_on(async {
                                                        cs.write(&record).await
                                                    });
                                                } else {
                                                    metrics_db.inc_lock_contention();
                                                }
                                            } else {
                                                thread::sleep(Duration::from_millis(
                                                    10 * attempts as u64,
                                                ));
                                            }
                                        }
                                    }

                                    // Auto-recovery trigger
                                    if !write_succeeded
                                        && should_auto_recover(
                                            consecutive_failures,
                                            last_failure_time,
                                        )
                                    {
                                        tracing::warn!(
                                            "{}",
                                            crate::i18n::tr("sink-db_auto_recovery")
                                        );
                                        if let Ok(new_sink) = db_sink_factory(
                                            db_for_recovery.clone(),
                                            metrics_db.clone(),
                                        ) {
                                            *sink = new_sink;
                                            consecutive_failures = 0;
                                            metrics_db.update_sink_health("database", true, None);
                                            tracing::info!(
                                                "{}",
                                                crate::i18n::tr("sink-db_auto_recovery_ok")
                                            );
                                        }
                                    }
                                } else {
                                    // Timeout, flush buffer（降级模式下无 sink 可 flush）
                                    if let Some(sink) = sink.as_ref() {
                                        let _ =
                                            runtime_handle.block_on(async { sink.flush().await });
                                    }
                                }
                            }
                        }
                    }
                    metrics_db.active_workers.dec();
                },
            )
        };

        #[cfg(not(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        )))]
        let _handle_db = tokio::task::spawn_blocking(|| {});

        // Health Check Thread
        let (shutdown_tx_health, shutdown_health) = bounded(1);
        let metrics_health = metrics.clone();
        let effective_capacity_health = effective_capacity.clone();
        let handle_health = tokio::task::spawn_blocking(move || {
            let mut last_recovery_attempt = std::collections::HashMap::<String, Instant>::new();
            let mut low_usage_since: Option<Instant> = None;
            let check_interval = Duration::from_secs(1);

            loop {
                if shutdown_health.recv_timeout(check_interval).is_ok() {
                    break;
                }

                // Active recovery logic with control channel
                let current_eff = effective_capacity_health.load(Ordering::Relaxed);
                let channel_len_now = receiver.len();
                let status = metrics_health.get_status(channel_len_now, current_eff);

                // Adaptive capacity strategy
                if config.performance.channel_strategy == crate::ChannelStrategy::Adaptive {
                    let new_cap = update_adaptive_capacity(
                        current_eff,
                        channel_len_now,
                        config.performance.min_capacity,
                        config.performance.max_capacity,
                        config.performance.expand_threshold_percent,
                        config.performance.shrink_threshold_percent,
                        Duration::from_secs(config.performance.shrink_wait_seconds),
                        &mut low_usage_since,
                    );
                    effective_capacity_health.store(new_cap, Ordering::Relaxed);
                }
                for (name, sink_status) in status.sinks {
                    if !sink_status.status.is_operational() {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("name", name.clone());
                        args.set("error", format!("{:?}", sink_status.last_error));
                        tracing::warn!("{}", crate::i18n::tr_args("sink-health_unhealthy", args));

                        // Check if we should attempt recovery
                        let should_recover = should_attempt_recovery(
                            last_recovery_attempt.get(&name),
                            Duration::from_secs(30),
                        );

                        if should_recover && sink_status.consecutive_failures > 3 {
                            let mut args = fluent_bundle::FluentArgs::new();
                            args.set("name", name.clone());
                            tracing::warn!(
                                "{}",
                                crate::i18n::tr_args("sink-health_attempting_recovery", args)
                            );

                            // Send recovery command
                            if let Err(e) =
                                control_tx.send(SinkControlMessage::RecoverSink(name.clone()))
                            {
                                let mut args = fluent_bundle::FluentArgs::new();
                                args.set("name", name.clone());
                                args.set("err", e.to_string());
                                tracing::error!(
                                    "{}",
                                    crate::i18n::tr_args("sink-health_send_failed", args)
                                );
                            } else {
                                last_recovery_attempt.insert(name.clone(), Instant::now());
                                tracing::info!(
                                    "Health Check: Recovery command sent for sink '{}'",
                                    name
                                );
                            }
                        }

                        // If error count is very high, trigger critical alert
                        if sink_status.consecutive_failures > 10 {
                            tracing::error!(
                                "CRITICAL: Sink '{}' has high error count ({})",
                                name,
                                sink_status.consecutive_failures
                            );
                        }
                    } else {
                        // Sink is healthy, clear recovery cooldown
                        last_recovery_attempt.remove(&name);
                    }
                }
            }
        });

        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let handles = vec![handle_console, handle_file, handle_db, handle_health];
        #[cfg(not(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        )))]
        let handles = vec![handle_console, handle_file, handle_health];

        // shutdown_txs 与 handles 一一对应，保持 cfg 一致性
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let shutdown_txs = vec![
            shutdown_tx_console,
            shutdown_tx_file,
            shutdown_tx_db,
            shutdown_tx_health,
        ];
        #[cfg(not(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        )))]
        let shutdown_txs = vec![shutdown_tx_console, shutdown_tx_file, shutdown_tx_health];

        Ok((handles, shutdown_txs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // should_auto_recover
    // ========================================================================

    #[test]
    fn test_should_auto_recover_low_failures() {
        assert!(!should_auto_recover(
            5,
            Some(Instant::now() - Duration::from_secs(120))
        ));
        assert!(!should_auto_recover(
            0,
            Some(Instant::now() - Duration::from_secs(120))
        ));
    }

    #[test]
    fn test_should_auto_recover_high_failures_no_time() {
        assert!(!should_auto_recover(10, None));
    }

    #[test]
    fn test_should_auto_recover_high_failures_recent() {
        assert!(!should_auto_recover(
            10,
            Some(Instant::now() - Duration::from_secs(30))
        ));
    }

    #[test]
    fn test_should_auto_recover_high_failures_old() {
        assert!(should_auto_recover(
            6,
            Some(Instant::now() - Duration::from_secs(61))
        ));
        assert!(should_auto_recover(
            100,
            Some(Instant::now() - Duration::from_secs(300))
        ));
    }

    // ========================================================================
    // should_attempt_recovery
    // ========================================================================

    #[test]
    fn test_should_attempt_recovery_never() {
        assert!(should_attempt_recovery(None, Duration::from_secs(30)));
    }

    #[test]
    fn test_should_attempt_recovery_within_cooldown() {
        let recent = Instant::now() - Duration::from_secs(10);
        assert!(!should_attempt_recovery(
            Some(&recent),
            Duration::from_secs(30)
        ));
    }

    #[test]
    fn test_should_attempt_recovery_after_cooldown() {
        let old = Instant::now() - Duration::from_secs(60);
        assert!(should_attempt_recovery(Some(&old), Duration::from_secs(30)));
    }

    // ========================================================================
    // classify_control_message
    // ========================================================================

    #[test]
    fn test_classify_control_recover_matching() {
        let msg = SinkControlMessage::RecoverSink("file".to_string());
        assert!(matches!(
            classify_control_message(&msg, "file"),
            ControlAction::Recover
        ));
    }

    #[test]
    fn test_classify_control_recover_non_matching() {
        let msg = SinkControlMessage::RecoverSink("database".to_string());
        assert!(matches!(
            classify_control_message(&msg, "file"),
            ControlAction::Ignore
        ));
    }

    #[test]
    fn test_classify_control_get_status() {
        let msg = SinkControlMessage::GetStatus;
        assert!(matches!(
            classify_control_message(&msg, "file"),
            ControlAction::Status
        ));
        assert!(matches!(
            classify_control_message(&msg, "database"),
            ControlAction::Status
        ));
    }

    // ========================================================================
    // update_adaptive_capacity
    // ========================================================================

    #[test]
    fn test_update_adaptive_capacity_expand() {
        let mut low_usage_since: Option<Instant> = None;
        // 80% usage → should expand
        let new_cap = update_adaptive_capacity(
            100,
            80,
            50,
            200,
            70,
            30,
            Duration::from_secs(60),
            &mut low_usage_since,
        );
        assert_eq!(new_cap, 150); // 100 + 100/2
        assert!(low_usage_since.is_none());
    }

    #[test]
    fn test_update_adaptive_capacity_shrink_after_wait() {
        let mut low_usage_since = Some(Instant::now() - Duration::from_secs(120));
        // 10% usage, low for 120s > 60s wait → should shrink
        let new_cap = update_adaptive_capacity(
            100,
            10,
            50,
            200,
            70,
            30,
            Duration::from_secs(60),
            &mut low_usage_since,
        );
        assert_eq!(new_cap, 70); // 100 * 70 / 100
        assert!(low_usage_since.is_none());
    }

    #[test]
    fn test_update_adaptive_capacity_shrink_starts_timer() {
        let mut low_usage_since: Option<Instant> = None;
        // 10% usage, first time → start timer, keep capacity
        let new_cap = update_adaptive_capacity(
            100,
            10,
            50,
            200,
            70,
            30,
            Duration::from_secs(60),
            &mut low_usage_since,
        );
        assert_eq!(new_cap, 100);
        assert!(low_usage_since.is_some());
    }

    #[test]
    fn test_update_adaptive_capacity_stable() {
        let mut low_usage_since: Option<Instant> = None;
        // 50% usage, between thresholds → no change
        let new_cap = update_adaptive_capacity(
            100,
            50,
            50,
            200,
            70,
            30,
            Duration::from_secs(60),
            &mut low_usage_since,
        );
        assert_eq!(new_cap, 100);
    }

    #[test]
    fn test_update_adaptive_capacity_respects_max() {
        let mut low_usage_since: Option<Instant> = None;
        // 90% usage but already at max → stay at max
        let new_cap = update_adaptive_capacity(
            200,
            180,
            50,
            200,
            70,
            30,
            Duration::from_secs(60),
            &mut low_usage_since,
        );
        assert_eq!(new_cap, 200);
    }

    #[test]
    fn test_update_adaptive_capacity_respects_min() {
        let mut low_usage_since = Some(Instant::now() - Duration::from_secs(120));
        // 0% usage, at min → stay at min
        let new_cap = update_adaptive_capacity(
            50,
            0,
            50,
            200,
            70,
            30,
            Duration::from_secs(60),
            &mut low_usage_since,
        );
        assert_eq!(new_cap, 50);
    }

    // ========================================================================
    // Console worker 并发回归测试：多任务并发写 console sink 不死锁/不 panic，
    // 所有 worker 在 shutdown 广播后均能终止（MutexGuard 不得横跨 block_on）
    // ========================================================================

    #[test]
    fn test_console_worker_concurrent_writes_terminate_without_deadlock() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build test runtime");

        let config = InklogConfig::default();
        let (file_tx, file_rx) = bounded::<Arc<LogRecord>>(100);
        let (console_tx, console_rx) = bounded::<Arc<LogRecord>>(2048);
        let (control_tx, control_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let effective_capacity = Arc::new(AtomicUsize::new(2048));
        let console_sink: Arc<Mutex<Arc<dyn LogSink>>> = Arc::new(Mutex::new(Arc::new(
            crate::support::io::ConsoleSink::new(
                config.console_sink.clone().unwrap_or_default(),
                crate::LogTemplate::new(&config.global.format),
            ),
        ) as Arc<dyn LogSink>));
        let error_sink: Arc<Mutex<Option<Arc<dyn LogSink>>>> = Arc::new(Mutex::new(None));

        let params = WorkerParams {
            config,
            receiver: file_rx,
            console_receiver: console_rx,
            control_rx,
            control_tx,
            metrics: metrics.clone(),
            console_sink,
            error_sink,
            effective_capacity,
            file_sink_factory: Box::new(|| {
                Err(InklogError::ConfigError("unused in test".to_string()))
            }),
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            db_sink_factory: Box::new(|_db, _metrics| {
                Err(InklogError::ConfigError("unused in test".to_string()))
            }),
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            database: None,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            db_receiver: None,
        };

        let (handles, shutdown_txs) = runtime
            .block_on(async { LoggerManager::start_workers(params).expect("start workers") });
        let _ = file_tx;

        // 多任务并发向 console channel 投递记录
        let producers: Vec<_> = (0..4)
            .map(|t| {
                let tx = console_tx.clone();
                thread::spawn(move || {
                    for i in 0..100 {
                        let record = Arc::new(LogRecord {
                            timestamp: Utc::now(),
                            level: "INFO".to_string(),
                            target: format!("concurrent::{t}"),
                            message: format!("concurrent write {t}-{i}"),
                            fields: Default::default(),
                            file: None,
                            line: None,
                            thread_id: "test".to_string(),
                        });
                        if tx.send(record).is_err() {
                            break;
                        }
                    }
                })
            })
            .collect();
        for producer in producers {
            producer.join().expect("producer thread panicked");
        }
        drop(console_tx);

        // 广播 shutdown 并等待所有 worker 终止；限时未结束即视为死锁回归
        for tx in &shutdown_txs {
            let _ = tx.send_timeout((), Duration::from_secs(2));
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut all_finished = true;
        for handle in handles {
            while !handle.is_finished() {
                if Instant::now() > deadline {
                    all_finished = false;
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            handle.abort();
        }
        assert!(all_finished, "workers must terminate without deadlock");
        assert_eq!(metrics.sink_errors(), 0, "console writes must not fail");
    }

    // ========================================================================
    // 缺陷回归：file sink 工厂启动失败时 worker 不得空转/静默丢弃——
    // 降级期间的记录必须计为 failed、worker 保持存活持续重试，
    // 且 shutdown 语义不变。
    // ========================================================================

    #[test]
    #[serial_test::serial]
    fn test_file_worker_failing_factory_counts_failed_and_stays_alive() {
        // 注入毫秒级退避，避免测试等待真实的秒级退避
        FACTORY_RETRY_INITIAL_BACKOFF_MS.store(20, Ordering::Relaxed);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build test runtime");

        let config = InklogConfig {
            file_sink: Some(crate::FileSinkConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (file_tx, file_rx) = bounded::<Arc<LogRecord>>(100);
        let (_console_tx, console_rx) = bounded::<Arc<LogRecord>>(100);
        let (control_tx, control_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let effective_capacity = Arc::new(AtomicUsize::new(100));
        let console_sink: Arc<Mutex<Arc<dyn LogSink>>> = Arc::new(Mutex::new(Arc::new(
            crate::support::io::ConsoleSink::new(
                config.console_sink.clone().unwrap_or_default(),
                crate::LogTemplate::new(&config.global.format),
            ),
        ) as Arc<dyn LogSink>));
        let error_sink: Arc<Mutex<Option<Arc<dyn LogSink>>>> = Arc::new(Mutex::new(None));

        let params = WorkerParams {
            config,
            receiver: file_rx,
            console_receiver: console_rx,
            control_rx,
            control_tx,
            metrics: metrics.clone(),
            console_sink,
            error_sink,
            effective_capacity,
            // 必然失败的工厂：模拟 FileSink::new 因路径/权限等原因持续失败
            file_sink_factory: Box::new(|| {
                Err(InklogError::ConfigError(
                    "factory always fails in this test".to_string(),
                ))
            }),
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            db_sink_factory: Box::new(|_db, _metrics| {
                Err(InklogError::ConfigError("unused in test".to_string()))
            }),
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            database: None,
            #[cfg(any(
                feature = "sqlite",
                feature = "postgres",
                feature = "mysql",
                feature = "duckdb"
            ))]
            db_receiver: None,
        };

        let (handles, shutdown_txs) = runtime
            .block_on(async { LoggerManager::start_workers(params).expect("start workers") });

        // 向 file channel 投递 N 条记录：降级模式下应被计为 failed 而非无声丢弃
        const N: u64 = 5;
        for i in 0..N {
            let record = Arc::new(LogRecord {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                target: "degraded::factory".to_string(),
                message: format!("degraded record {i}"),
                fields: Default::default(),
                file: None,
                line: None,
                thread_id: "test".to_string(),
            });
            file_tx.send(record).expect("send record");
        }
        drop(file_tx);

        // 等待 worker 消费并计数（退避 20ms，循环 tick 100ms）
        let deadline = Instant::now() + Duration::from_secs(5);
        while metrics.sink_errors() < N && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }

        // 1) N 条记录全部被计为 failed/dropped，而非静默丢失
        assert!(
            metrics.sink_errors() >= N,
            "records arriving while the factory fails must be counted as failed, got: {}",
            metrics.sink_errors()
        );
        assert!(
            metrics.logs_dropped() >= N,
            "records arriving while the factory fails must be counted as dropped, got: {}",
            metrics.logs_dropped()
        );
        // 2) worker 保持存活继续重试（未因工厂失败而退出）
        assert!(
            !handles[1].is_finished(),
            "file worker must stay alive while the factory keeps failing"
        );

        // 3) shutdown 语义不变：降级模式下广播后所有 worker 均终止
        for tx in &shutdown_txs {
            let _ = tx.send_timeout((), Duration::from_secs(2));
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut all_finished = true;
        for handle in handles {
            while !handle.is_finished() {
                if Instant::now() > deadline {
                    all_finished = false;
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            handle.abort();
        }
        assert!(
            all_finished,
            "workers must terminate after shutdown even in degraded retry mode"
        );

        // 恢复测试钩子，避免影响其他测试
        FACTORY_RETRY_INITIAL_BACKOFF_MS.store(0, Ordering::Relaxed);
    }
}
