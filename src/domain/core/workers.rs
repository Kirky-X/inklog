// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Worker thread management for log sinks.

use super::LoggerManager;
use super::recovery::SinkControlMessage;
use crate::InklogConfig;
use crate::Metrics;
use crate::support::io::{ConsoleSink, FileSink, LogSink};
use crate::{InklogError, LogRecord};
use chrono::Utc;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Parameters for worker threads
pub(crate) struct WorkerParams {
    pub(crate) config: InklogConfig,
    pub(crate) receiver: Receiver<Arc<LogRecord>>,
    pub(crate) console_receiver: Receiver<Arc<LogRecord>>,
    pub(crate) control_rx: Receiver<SinkControlMessage>,
    pub(crate) control_tx: Sender<SinkControlMessage>,
    pub(crate) metrics: Arc<Metrics>,
    pub(crate) console_sink: Arc<Mutex<ConsoleSink>>,
    pub(crate) error_sink: Arc<Mutex<Option<FileSink>>>,
    pub(crate) effective_capacity: Arc<AtomicUsize>,
    /// 注入的数据库依赖（DI 模式）
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub(crate) database: Option<Arc<dyn crate::integrations::Database>>,
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
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database,
        } = params;
        let file_config = config.file_sink.clone();
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let db_config = config.database_sink.clone();

        // 确保 database 始终有效：如果配置了数据库但没有提供 DI 依赖，则创建默认实现
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let database = {
            match database {
                Some(db) => Some(db),
                None => {
                    if let Some(ref cfg) = db_config {
                        if cfg.enabled {
                            // 获取当前 tokio runtime 并创建默认的 DbNexusAdapter
                            let handle = tokio::runtime::Handle::current();
                            let cfg_url = cfg.url.clone();
                            // Cap pool_size to min(configured, num_cpus, 4) to prevent resource exhaustion
                            let db_worker_limit =
                                crate::support::io::sink::database::effective_db_worker_limit();
                            let effective_pool_size = cfg.pool_size.min(db_worker_limit as u32);
                            if effective_pool_size < cfg.pool_size {
                                tracing::warn!(
                                    configured_pool_size = cfg.pool_size,
                                    effective_pool_size = effective_pool_size,
                                    limit = db_worker_limit,
                                    "Database pool_size capped to min(configured, num_cpus, 4)"
                                );
                            }
                            let adapter = handle.block_on(async {
                                crate::integrations::infra::DbNexusAdapter::with_full_config(
                                    &cfg_url,
                                    effective_pool_size,
                                    &cfg.table_name,
                                    cfg.permissions_path.clone(),
                                    &cfg.admin_role,
                                )
                                .await
                            })?;
                            Some(Arc::new(adapter) as Arc<dyn crate::integrations::infra::Database>)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }
        };

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

                            // Hot path: use try_lock to avoid blocking
                            match console_sink_console.try_lock() {
                                Ok(sink) => {
                                    if runtime_handle
                                        .block_on(async { sink.write(&record).await })
                                        .is_err()
                                    {
                                        metrics_console.inc_sink_error();
                                    }
                                }
                                Err(_) => {
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

                            // Hot path: use try_lock to avoid blocking
                            match console_sink_console.try_lock() {
                                Ok(sink) => {
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
                                Err(_) => {
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
                    let cfg_clone = cfg.clone(); // Clone for recovery attempts
                    if let Ok(mut sink) = FileSink::new(cfg) {
                        let mut consecutive_failures = 0;
                        #[allow(unused_assignments)]
                        let mut last_failure_time = None::<Instant>;

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

                                    // Retry logic
                                    let mut attempts = 0;
                                    while attempts < 3 {
                                        match runtime_handle
                                            .block_on(async { sink.write(&record).await })
                                        {
                                            Ok(_) => {
                                                metrics_file.inc_logs_written();
                                                metrics_file.update_sink_health("file", true, None);
                                                break;
                                            }
                                            Err(e) => {
                                                attempts += 1;
                                                // Log error to error.log
                                                if let Ok(mut error_sink_guard) =
                                                    error_sink_file.lock()
                                                    && let Some(sink) = error_sink_guard.as_mut()
                                                {
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
                                                        sink.write(&error_record).await
                                                    });
                                                }

                                                if attempts == 3 {
                                                    metrics_file.inc_sink_error();
                                                    metrics_file.update_sink_health(
                                                        "file",
                                                        false,
                                                        Some(e.to_string()),
                                                    );
                                                    // Fallback to console
                                                    if let Ok(cs) = console_sink_file.lock() {
                                                        let _ = runtime_handle.block_on(async {
                                                            cs.write(&record).await
                                                        });
                                                    }
                                                } else {
                                                    thread::sleep(Duration::from_millis(
                                                        10 * attempts as u64,
                                                    ));
                                                }
                                            }
                                        }
                                    }

                                    if Instant::now() > deadline {
                                        break;
                                    }
                                }
                                let _ = runtime_handle.block_on(async { sink.shutdown().await });
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
                                        if let Ok(new_sink) = FileSink::new(cfg_clone.clone()) {
                                            sink = new_sink;
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

                            if let Ok(record) = rx_file.recv_timeout(Duration::from_millis(100)) {
                                let latency = Utc::now()
                                    .signed_duration_since(record.timestamp)
                                    .to_std()
                                    .unwrap_or(Duration::ZERO);
                                metrics_file.record_latency(latency);

                                // Retry logic with recovery detection
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
                                            if let Ok(mut error_sink_guard) = error_sink_file.lock()
                                                && let Some(sink) = error_sink_guard.as_mut()
                                            {
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
                                                    sink.write(&error_record).await
                                                });
                                            }

                                            if attempts == 3 {
                                                metrics_file.inc_sink_error();
                                                metrics_file.update_sink_health(
                                                    "file",
                                                    false,
                                                    Some(e.to_string()),
                                                );
                                                // Fallback to console
                                                if let Ok(cs) = console_sink_file.lock() {
                                                    let _ = runtime_handle.block_on(async {
                                                        cs.write(&record).await
                                                    });
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
                                    tracing::warn!(
                                        "{}",
                                        crate::i18n::tr("sink-file_auto_recovery")
                                    );
                                    if let Ok(new_sink) = FileSink::new(cfg_clone.clone()) {
                                        sink = new_sink;
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
                                // Timeout, flush buffer
                                let _ = runtime_handle.block_on(async { sink.flush().await });
                            }
                        }
                    }
                }
                metrics_file.active_workers.dec();
            })
        };

        // Thread 2: DB Sink
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let rx_db = receiver.clone();
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let (shutdown_tx_db, shutdown_db) = bounded(1);
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let metrics_db = metrics.clone();
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let console_sink_db = console_sink.clone();
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let error_sink_db = error_sink.clone();
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let control_rx_db = control_rx.clone();
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let handle_db = {
            let runtime_handle = runtime_handle.clone();
            tokio::task::spawn_blocking(
                #[allow(unused_assignments)]
                move || {
                    metrics_db.active_workers.inc();
                    if let Some(cfg) = db_config
                        && cfg.enabled
                        && let Some(ref db) = database
                    {
                        // Clone once before the loop for recovery use
                        let db_for_recovery = db.clone();
                        if let Ok(sink_result) = crate::support::io::DatabaseSink::new(db.clone()) {
                            let mut sink: crate::support::io::DatabaseSink = sink_result;
                            runtime_handle
                                .block_on(async { sink.set_metrics(metrics_db.clone()).await });
                            let mut consecutive_failures = 0;
                            #[allow(unused_assignments)]
                            let mut last_failure_time = None::<Instant>;

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
                                                if let Ok(mut error_sink_guard) =
                                                    error_sink_db.lock()
                                                    && let Some(sink) = error_sink_guard.as_mut()
                                                {
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
                                                        sink.write(&error_record).await
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
                                                    // Fallback to console
                                                    if let Ok(cs) = console_sink_db.lock() {
                                                        let _ = runtime_handle.block_on(async {
                                                            cs.write(&record).await
                                                        });
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
                                            if let Ok(new_sink) =
                                                crate::support::io::DatabaseSink::new(
                                                    db_for_recovery.clone(),
                                                )
                                            {
                                                sink = new_sink;
                                                runtime_handle.block_on(async {
                                                    sink.set_metrics(metrics_db.clone()).await
                                                });
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
                                    let _ =
                                        runtime_handle.block_on(async { sink.shutdown().await });
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
                                            if let Ok(new_sink) =
                                                crate::support::io::DatabaseSink::new(
                                                    db_for_recovery.clone(),
                                                )
                                            {
                                                sink = new_sink;
                                                runtime_handle.block_on(async {
                                                    sink.set_metrics(metrics_db.clone()).await
                                                });
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

                                if let Ok(record) = rx_db.recv_timeout(Duration::from_millis(100)) {
                                    let latency = Utc::now()
                                        .signed_duration_since(record.timestamp)
                                        .to_std()
                                        .unwrap_or(Duration::ZERO);
                                    metrics_db.record_latency(latency);

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
                                                if let Ok(cs) = console_sink_db.lock() {
                                                    let _ = runtime_handle.block_on(async {
                                                        cs.write(&record).await
                                                    });
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
                                        if let Ok(new_sink) = crate::support::io::DatabaseSink::new(
                                            db_for_recovery.clone(),
                                        ) {
                                            sink = new_sink;
                                            runtime_handle.block_on(async {
                                                sink.set_metrics(metrics_db.clone()).await
                                            });
                                            consecutive_failures = 0;
                                            metrics_db.update_sink_health("database", true, None);
                                            tracing::info!(
                                                "{}",
                                                crate::i18n::tr("sink-db_auto_recovery_ok")
                                            );
                                        }
                                    }
                                } else {
                                    // Timeout, flush buffer
                                    let _ = runtime_handle.block_on(async { sink.flush().await });
                                }
                            }
                        }
                    }
                    metrics_db.active_workers.dec();
                },
            )
        };

        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
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

        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let handles = vec![handle_console, handle_file, handle_db, handle_health];
        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
        let handles = vec![handle_console, handle_file, handle_health];

        // shutdown_txs 与 handles 一一对应，保持 cfg 一致性
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let shutdown_txs = vec![
            shutdown_tx_console,
            shutdown_tx_file,
            shutdown_tx_db,
            shutdown_tx_health,
        ];
        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
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
}
