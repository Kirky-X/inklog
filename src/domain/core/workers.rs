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
        #[allow(unused_variables)]
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
                                crate::integrations::infra::DbNexusAdapter::new(
                                    &cfg_url,
                                    effective_pool_size,
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
                                                if let Ok(mut error_sink_guard) = error_sink.lock()
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
                                match control_msg {
                                    SinkControlMessage::RecoverSink(sink_name)
                                        if sink_name == "file" =>
                                    {
                                        eprintln!("File sink: Received recovery command");
                                        // Attempt to recreate the sink
                                        if let Ok(new_sink) = FileSink::new(cfg_clone.clone()) {
                                            sink = new_sink;
                                            consecutive_failures = 0;
                                            last_failure_time = None;
                                            metrics_file.update_sink_health("file", true, None);
                                            eprintln!("File sink: Successfully recovered");
                                        } else {
                                            eprintln!("File sink: Recovery failed");
                                        }
                                    }
                                    SinkControlMessage::GetStatus => {
                                        // Status is already tracked in metrics
                                    }
                                    _ => {} // Ignore messages for other sinks
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
                                            if let Ok(mut error_sink_guard) = error_sink.lock()
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

                                // Auto-recovery trigger: if we have too many consecutive failures
                                if !write_succeeded
                                    && consecutive_failures > 5
                                    && let Some(last_failure) = last_failure_time
                                    && last_failure.elapsed() > Duration::from_secs(60)
                                {
                                    eprintln!(
                                        "File sink: Triggering auto-recovery due to consecutive failures"
                                    );
                                    // Attempt to recreate the sink
                                    if let Ok(new_sink) = FileSink::new(cfg_clone.clone()) {
                                        sink = new_sink;
                                        consecutive_failures = 0;
                                        last_failure_time = None;
                                        metrics_file.update_sink_health("file", true, None);
                                        eprintln!("File sink: Auto-recovery successful");
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

                                                if attempts == 3 {
                                                    metrics_db.inc_sink_error();
                                                    let error_msg =
                                                        crate::InklogError::to_string(e);
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
                                            && consecutive_failures > 5
                                            && let Some(last_failure) = last_failure_time
                                            && last_failure.elapsed() > Duration::from_secs(60)
                                        {
                                            eprintln!(
                                                "Database sink: Triggering auto-recovery due to consecutive failures"
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
                                                eprintln!(
                                                    "Database sink: Auto-recovery successful"
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
                                    match control_msg {
                                        SinkControlMessage::RecoverSink(sink_name)
                                            if sink_name == "database" =>
                                        {
                                            eprintln!("Database sink: Received recovery command");
                                            // Attempt to recreate the sink
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
                                                eprintln!("Database sink: Successfully recovered");
                                            } else {
                                                eprintln!("Database sink: Recovery failed");
                                            }
                                        }
                                        SinkControlMessage::GetStatus => {
                                            // Status is already tracked in metrics
                                        }
                                        _ => {} // Ignore messages for other sinks
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
                                        && consecutive_failures > 5
                                        && let Some(last_failure) = last_failure_time
                                        && last_failure.elapsed() > Duration::from_secs(60)
                                    {
                                        eprintln!(
                                            "Database sink: Triggering auto-recovery due to consecutive failures"
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
                                            eprintln!("Database sink: Auto-recovery successful");
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
                    let usage = if current_eff > 0 {
                        channel_len_now as f64 / current_eff as f64
                    } else {
                        0.0
                    };
                    let usage_percent = (usage * 100.0).round() as u8;

                    // Expand when usage is high
                    if usage_percent >= config.performance.expand_threshold_percent
                        && current_eff < config.performance.max_capacity
                    {
                        let grow_to =
                            (current_eff + current_eff / 2).min(config.performance.max_capacity);
                        effective_capacity_health.store(grow_to, Ordering::Relaxed);
                        low_usage_since = None;
                    } else if usage_percent <= config.performance.shrink_threshold_percent
                        && current_eff > config.performance.min_capacity
                    {
                        // Track low usage duration for shrink
                        match low_usage_since {
                            None => low_usage_since = Some(Instant::now()),
                            Some(inst) => {
                                if inst.elapsed()
                                    >= Duration::from_secs(config.performance.shrink_wait_seconds)
                                {
                                    let shrink_to = (current_eff.saturating_mul(70) / 100)
                                        .max(config.performance.min_capacity);
                                    effective_capacity_health.store(shrink_to, Ordering::Relaxed);
                                    low_usage_since = None;
                                }
                            }
                        }
                    } else {
                        low_usage_since = None;
                    }
                }
                for (name, sink_status) in status.sinks {
                    if !sink_status.status.is_operational() {
                        eprintln!(
                            "Health Check: Sink '{}' is unhealthy. Last error: {:?}",
                            name, sink_status.last_error
                        );

                        // Check if we should attempt recovery
                        let should_recover = {
                            let last_attempt = last_recovery_attempt.get(&name);
                            match last_attempt {
                                None => true,                                           // Never attempted
                                Some(inst) => inst.elapsed() > Duration::from_secs(30), // 30s cooldown
                            }
                        };

                        if should_recover && sink_status.consecutive_failures > 3 {
                            eprintln!("Health Check: Attempting recovery for sink '{}'", name);

                            // Send recovery command
                            if let Err(e) =
                                control_tx.send(SinkControlMessage::RecoverSink(name.clone()))
                            {
                                eprintln!(
                                    "Health Check: Failed to send recovery command for '{}': {}",
                                    name, e
                                );
                            } else {
                                last_recovery_attempt.insert(name.clone(), Instant::now());
                                eprintln!(
                                    "Health Check: Recovery command sent for sink '{}'",
                                    name
                                );
                            }
                        }

                        // If error count is very high, trigger critical alert
                        if sink_status.consecutive_failures > 10 {
                            eprintln!(
                                "CRITICAL: Sink '{}' has high error count ({})",
                                name, sink_status.consecutive_failures
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
