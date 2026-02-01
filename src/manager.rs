// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "aws")]
use crate::archive::{ArchiveService, ArchiveServiceBuilder};
#[allow(unused_imports)]
use crate::config::ConsoleSinkConfig;
use crate::config::{FileSinkConfig, InklogConfig};
use crate::error::InklogError;
use crate::log_adapter::{LogAdapter, LogLogger};
use crate::log_record::LogRecord;
use crate::metrics::{HealthStatus, Metrics};
use crate::sink::console::ConsoleSink;
use crate::sink::database::DatabaseSink;
use crate::sink::file::FileSink;
use crate::sink::LogSink;
use crate::subscriber::LoggerSubscriber;
use crate::template::LogTemplate;
use chrono::Utc;
use crossbeam_channel::{bounded, Receiver, Sender};
#[allow(unused_imports)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
#[cfg(feature = "aws")]
use tokio::sync::Mutex as AsyncMutex;
#[cfg(feature = "aws")]
use tracing::error;
#[cfg(any(feature = "aws", feature = "http"))]
use tracing::info;
use tracing_subscriber::prelude::*;

// Control messages for sink recovery
/// Messages used to control sink recovery and status queries.
#[derive(Debug, Clone)]
enum SinkControlMessage {
    RecoverSink(String), // sink name
    GetStatus,
}

// Parameters for worker threads
struct WorkerParams {
    config: InklogConfig,
    receiver: Receiver<LogRecord>,
    shutdown_rx: Receiver<()>,
    control_rx: Receiver<SinkControlMessage>,
    control_tx: Sender<SinkControlMessage>,
    metrics: Arc<Metrics>,
    console_sink: Arc<Mutex<ConsoleSink>>,
    error_sink: Arc<Mutex<Option<FileSink>>>,
}

/// 环境检测结果，用于智能配置
#[derive(Debug, Clone)]
struct EnvironmentProfile {
    /// 是否为交互式终端
    is_terminal: bool,
    /// 是否在容器环境中运行
    in_container: bool,
    /// 是否在 CI 环境中运行
    in_ci: bool,
    /// 是否在云环境中运行
    in_cloud: bool,
    /// CPU 核心数
    cpu_count: usize,
    /// 机器唯一标识
    machine_id: u64,
}

/// Core logging manager that coordinates log collection and routing to sinks.
///
/// LoggerManager is the main entry point for the inklog logging system.
/// It handles:
/// - Log message routing to configured sinks (console, file, database)
/// - Health monitoring and metrics collection
/// - Sink recovery on failure
/// - Archive service lifecycle (when S3 is enabled)
/// - HTTP server for health endpoints (when http feature is enabled)
///
/// # Examples
///
/// ```ignore
/// use inklog::{LoggerManager, InklogConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = InklogConfig::default();
///     let _logger = LoggerManager::with_config(config).await?;
///     Ok(())
/// }
/// ```
pub struct LoggerManager {
    config: InklogConfig,
    sender: Sender<LogRecord>,
    shutdown_tx: Sender<()>,
    console_sink: Arc<Mutex<ConsoleSink>>,
    metrics: Arc<Metrics>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
    control_tx: Sender<SinkControlMessage>,
    #[cfg(feature = "aws")]
    archive_service: Option<Arc<tokio::sync::Mutex<ArchiveService>>>,
}

impl LoggerManager {
    pub async fn new() -> Result<Self, InklogError> {
        Self::with_config(InklogConfig::default()).await
    }

    /// Creates a new LoggerManager with the given configuration.
    ///
    /// This is the primary entry point for initializing the logging system.
    /// The configuration determines which sinks are enabled and how logs are handled.
    ///
    /// # Arguments
    /// * `config` - Configuration for the logging system
    ///
    /// # Returns
    /// A Result containing the LoggerManager or an error if initialization fails
    ///
    /// # Example
    /// ```ignore
    /// use inklog::{LoggerManager, InklogConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = InklogConfig::default();
    ///     let _logger = LoggerManager::with_config(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_config(config: InklogConfig) -> Result<Self, InklogError> {
        // Security audit: Log logger initialization
        #[cfg(any(feature = "aws", feature = "http"))]
        tracing::info!(
            event = "security_logger_initialized",
            sinks = ?config.sinks_enabled(),
            masking_enabled = config.global.masking_enabled,
            "Logger manager initialized"
        );

        let (manager, subscriber, filter) = Self::build_detached(config.clone()).await?;

        // 1. 安装 tracing subscriber
        let registry = tracing_subscriber::registry().with(subscriber).with(filter);
        if let Err(ref e) = registry.try_init() {
            tracing::warn!("Failed to set global subscriber: {}", e);
        }

        // 2. 安装 log crate logger（原生支持，无需 tracing_log）
        let log_adapter = LogAdapter::new(
            manager.console_sink.clone(),
            manager.sender.clone(),
            manager.metrics.clone(),
        );
        let max_level = config
            .global
            .level
            .parse::<tracing::Level>()
            .unwrap_or(tracing::Level::INFO);
        let log_level = match max_level {
            tracing::Level::TRACE => log::LevelFilter::Trace,
            tracing::Level::DEBUG => log::LevelFilter::Debug,
            tracing::Level::INFO => log::LevelFilter::Info,
            tracing::Level::WARN => log::LevelFilter::Warn,
            tracing::Level::ERROR => log::LevelFilter::Error,
        };
        let log_logger = LogLogger::new(log_adapter, log_level);
        if let Err(e) = log_logger.install() {
            tracing::warn!("Failed to set log crate logger: {}", e);
        }

        Ok(manager)
    }



    /// 构建LoggerManager但不安装全局订阅者。
    /// 这主要用于测试和基准测试。
    pub async fn build_detached(
        config: InklogConfig,
    ) -> Result<
        (
            Self,
            LoggerSubscriber,
            tracing_subscriber::filter::LevelFilter,
        ),
        InklogError,
    > {
        config.validate()?;

        let metrics = Arc::new(Metrics::new());
        let (sender, receiver) = bounded(config.performance.channel_capacity);
        let (shutdown_tx, shutdown_rx) = bounded(1);
        let (control_tx, control_rx) = bounded(10); // Control channel for recovery commands

        let console_sink = Arc::new(Mutex::new(ConsoleSink::new(
            config.console_sink.clone().unwrap_or_default(),
            LogTemplate::new(&config.global.format),
        )));

        // Initialize tracing subscriber
        let subscriber =
            LoggerSubscriber::new(console_sink.clone(), sender.clone(), metrics.clone());

        // Filter
        let level = config
            .global
            .level
            .parse::<tracing::Level>()
            .unwrap_or(tracing::Level::INFO);
        let filter = tracing_subscriber::filter::LevelFilter::from_level(level);

        // Create error sink for logging system errors
        let error_sink_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("logs/error.log"),
            ..Default::default()
        };
        let error_sink = Arc::new(Mutex::new(FileSink::new(error_sink_config).ok()));

        let handles = Self::start_workers(WorkerParams {
            config: config.clone(),
            receiver,
            shutdown_rx,
            control_rx,
            control_tx: control_tx.clone(),
            metrics: metrics.clone(),
            console_sink: console_sink.clone(),
            error_sink: error_sink.clone(),
        })?;

        // Initialize archive service if configured
        #[cfg(feature = "aws")]
        let archive_service = if let Some(ref archive_config) = config.s3_archive {
            if archive_config.enabled {
                info!("Initializing S3 archive service");

                let db_conn = if let Some(ref db_cfg) = config.database_sink {
                    use dbnexus::DbConfigBuilder;
                    let config = DbConfigBuilder::new()
                        .url(&db_cfg.url)
                        .max_connections(db_cfg.pool_size as u32)
                        .build()
                        .map_err(|e| {
                            tracing::warn!("Failed to build DbConfig: {}", e);
                        })
                        .ok();
                    match config {
                        Some(cfg) => match dbnexus::pool::DbPool::with_config(cfg).await {
                            Ok(pool) => Some(pool),
                            Err(e) => {
                                tracing::warn!("Failed to create DbPool: {}", e);
                                None
                            }
                        },
                        None => None,
                    }
                } else {
                    None
                };

                let mut archive_service_builder =
                    ArchiveServiceBuilder::new().config(archive_config.clone());

                #[allow(clippy::collapsible_match, clippy::match_result_ok)]
                if let Some(pool) = db_conn {
                    if let Some(s) = pool.get_session("").await.ok() {
                        archive_service_builder = archive_service_builder.database_session(s);
                    }
                }

                match archive_service_builder.build().await {
                    Ok(service) => Some(Arc::new(AsyncMutex::new(service))),
                    Err(e) => {
                        error!("Failed to initialize archive service: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        #[cfg(feature = "aws")]
        let manager = Self {
            config,
            sender,
            shutdown_tx,
            console_sink,
            metrics,
            worker_handles: Mutex::new(handles),
            control_tx,
            archive_service,
        };

        #[cfg(not(feature = "aws"))]
        let manager = Self {
            config,
            sender,
            shutdown_tx,
            console_sink,
            metrics,
            worker_handles: Mutex::new(handles),
            control_tx,
        };

        Ok((manager, subscriber, filter))
    }



    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    /// 启动S3归档服务
    #[cfg(feature = "aws")]
    pub async fn start_archive_service(&self) -> Result<(), InklogError> {
        if let Some(ref archive_service) = self.archive_service {
            let service = archive_service.clone();
            tokio::spawn(async move {
                let mut service_guard = service.lock().await;
                if let Err(e) = service_guard.start().await {
                    error!("Archive service failed: {}", e);
                }
            });
            Ok(())
        } else {
            info!("Archive service not configured, skipping startup");
            Ok(())
        }
    }

    /// 停止S3归档服务
    #[cfg(feature = "aws")]
    pub async fn stop_archive_service(&self) -> Result<(), InklogError> {
        if let Some(ref archive_service) = self.archive_service {
            let service = archive_service.clone();
            tokio::spawn(async move {
                let service_guard = service.lock().await;
                if let Err(e) = service_guard.stop().await {
                    error!("Failed to stop archive service: {}", e);
                }
            });
            info!("Archive service shutdown signal sent");
        }
        Ok(())
    }

    /// 执行手动归档
    #[cfg(feature = "aws")]
    pub async fn trigger_archive(&self) -> Result<String, InklogError> {
        if let Some(ref archive_service) = self.archive_service {
            let service = archive_service.clone();
            let archive_key = tokio::spawn(async move {
                let service_guard = service.lock().await;
                service_guard.archive_now().await
            })
            .await
            .map_err(|e| InklogError::RuntimeError(format!("Archive task failed: {}", e)))?
            .map_err(|e| InklogError::S3Error(format!("Archive operation failed: {}", e)))?;

            info!("Manual archive completed: {}", archive_key);
            Ok(archive_key)
        } else {
            Err(InklogError::ConfigError(
                "Archive service not configured".to_string(),
            ))
        }
    }

    fn start_workers(params: WorkerParams) -> Result<Vec<JoinHandle<()>>, InklogError> {
        let WorkerParams {
            config,
            receiver,
            shutdown_rx,
            control_rx,
            control_tx,
            metrics,
            console_sink,
            error_sink,
        } = params;
        let file_config = config.file_sink.clone();
        let db_config = config.database_sink.clone();

        // Thread 1: File Sink
        let rx_file = receiver.clone();
        let shutdown_file = shutdown_rx.clone();
        let metrics_file = metrics.clone();
        let console_sink_file = console_sink.clone();
        let control_rx_file = control_rx.clone();
        let handle_file = thread::spawn(move || {
            metrics_file.active_workers.inc();
            if let Some(cfg) = file_config {
                if cfg.enabled {
                    let cfg_clone = cfg.clone(); // Clone for recovery attempts
                    if let Ok(mut sink) = FileSink::new(cfg) {
                        let mut consecutive_failures = 0;
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
                                        match sink.write(&record) {
                                            Ok(_) => {
                                                metrics_file.inc_logs_written();
                                                metrics_file.update_sink_health("file", true, None);
                                                break;
                                            }
                                            Err(e) => {
                                                attempts += 1;
                                                // Log error to error.log
                                                if let Ok(mut error_sink_guard) = error_sink.lock()
                                                {
                                                    if let Some(sink) = error_sink_guard.as_mut() {
                                                        let error_record = LogRecord {
                                                            timestamp: Utc::now(),
                                                            level: "ERROR".to_string(),
                                                            target: "inklog::file_sink".to_string(),
                                                            message: format!(
                                                                "File sink error: {}",
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
                                                        let _ = sink.write(&error_record);
                                                    }
                                                }

                                                if attempts == 3 {
                                                    metrics_file.inc_sink_error();
                                                    metrics_file.update_sink_health(
                                                        "file",
                                                        false,
                                                        Some(e.to_string()),
                                                    );
                                                    // Fallback to console
                                                    if let Ok(mut cs) = console_sink_file.lock() {
                                                        let _ = cs.write(&record);
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
                                let _ = sink.shutdown();
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
                                    match sink.write(&record) {
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
                                            if let Ok(mut error_sink_guard) = error_sink.lock() {
                                                if let Some(sink) = error_sink_guard.as_mut() {
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
                                                    let _ = sink.write(&error_record);
                                                }
                                            }

                                            if attempts == 3 {
                                                metrics_file.inc_sink_error();
                                                metrics_file.update_sink_health(
                                                    "file",
                                                    false,
                                                    Some(e.to_string()),
                                                );
                                                // Fallback to console
                                                if let Ok(mut cs) = console_sink_file.lock() {
                                                    let _ = cs.write(&record);
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
                                if !write_succeeded && consecutive_failures > 5 {
                                    if let Some(last_failure) = last_failure_time {
                                        if last_failure.elapsed() > Duration::from_secs(60) {
                                            eprintln!("File sink: Triggering auto-recovery due to consecutive failures");
                                            // Attempt to recreate the sink
                                            if let Ok(new_sink) = FileSink::new(cfg_clone.clone()) {
                                                sink = new_sink;
                                                consecutive_failures = 0;
                                                last_failure_time = None;
                                                metrics_file.update_sink_health("file", true, None);
                                                eprintln!("File sink: Auto-recovery successful");
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Timeout, flush buffer
                                let _ = sink.flush();
                            }
                        }
                    }
                }
            }
            metrics_file.active_workers.dec();
        });

        // Thread 2: DB Sink
        let rx_db = receiver.clone();
        let shutdown_db = shutdown_rx.clone();
        let metrics_db = metrics.clone();
        let console_sink_db = console_sink.clone();
        let control_rx_db = control_rx.clone();
        let handle_db = thread::spawn(move || {
            metrics_db.active_workers.inc();
            if let Some(cfg) = db_config {
                if cfg.enabled {
                    let cfg_clone = cfg.clone(); // Clone for recovery attempts
                    if let Ok(mut sink) = DatabaseSink::new(&cfg) {
                        let mut consecutive_failures = 0;
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
                                    while attempts < 3 {
                                        match sink.write(&record) {
                                            Ok(_) => {
                                                metrics_db.inc_logs_written();
                                                metrics_db
                                                    .update_sink_health("database", true, None);
                                                consecutive_failures = 0;
                                                last_failure_time = None;
                                                write_succeeded = true;
                                                break;
                                            }
                                            Err(e) => {
                                                attempts += 1;
                                                consecutive_failures += 1;
                                                last_failure_time = Some(Instant::now());

                                                if attempts == 3 {
                                                    metrics_db.inc_sink_error();
                                                    metrics_db.update_sink_health(
                                                        "database",
                                                        false,
                                                        Some(e.to_string()),
                                                    );
                                                    // Fallback to console
                                                    if let Ok(mut cs) = console_sink_db.lock() {
                                                        let _ = cs.write(&record);
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
                                    if !write_succeeded && consecutive_failures > 5 {
                                        if let Some(last_failure) = last_failure_time {
                                            if last_failure.elapsed() > Duration::from_secs(60) {
                                                eprintln!("Database sink: Triggering auto-recovery due to consecutive failures");
                                                if let Ok(new_sink) =
                                                    DatabaseSink::new(&cfg_clone.clone())
                                                {
                                                    sink = new_sink;
                                                    consecutive_failures = 0;
                                                    last_failure_time = None;
                                                    metrics_db
                                                        .update_sink_health("database", true, None);
                                                    eprintln!(
                                                        "Database sink: Auto-recovery successful"
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    if Instant::now() > deadline {
                                        break;
                                    }
                                }
                                let _ = sink.shutdown();
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
                                        if let Ok(new_sink) = DatabaseSink::new(&cfg_clone.clone())
                                        {
                                            sink = new_sink;
                                            consecutive_failures = 0;
                                            last_failure_time = None;
                                            metrics_db.update_sink_health("database", true, None);
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
                                while attempts < 3 {
                                    match sink.write(&record) {
                                        Ok(_) => {
                                            metrics_db.inc_logs_written();
                                            metrics_db.update_sink_health("database", true, None);
                                            consecutive_failures = 0;
                                            last_failure_time = None;
                                            write_succeeded = true;
                                            break;
                                        }
                                        Err(e) => {
                                            attempts += 1;
                                            consecutive_failures += 1;
                                            last_failure_time = Some(Instant::now());

                                            if attempts == 3 {
                                                metrics_db.inc_sink_error();
                                                metrics_db.update_sink_health(
                                                    "database",
                                                    false,
                                                    Some(e.to_string()),
                                                );

                                                // Fallback chain: DB -> File -> Console
                                                if let Ok(mut cs) = console_sink_db.lock() {
                                                    let _ = cs.write(&record);
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
                                if !write_succeeded && consecutive_failures > 5 {
                                    if let Some(last_failure) = last_failure_time {
                                        if last_failure.elapsed() > Duration::from_secs(60) {
                                            eprintln!("Database sink: Triggering auto-recovery due to consecutive failures");
                                            if let Ok(new_sink) =
                                                DatabaseSink::new(&cfg_clone.clone())
                                            {
                                                sink = new_sink;
                                                consecutive_failures = 0;
                                                last_failure_time = None;
                                                metrics_db
                                                    .update_sink_health("database", true, None);
                                                eprintln!(
                                                    "Database sink: Auto-recovery successful"
                                                );
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Timeout, flush buffer
                                let _ = sink.flush();
                            }
                        }
                    }
                }
            }
            metrics_db.active_workers.dec();
        });

        // Health Check Thread
        let shutdown_health = shutdown_rx.clone();
        let metrics_health = metrics.clone();
        let handle_health = thread::spawn(move || {
            let mut last_recovery_attempt = std::collections::HashMap::<String, Instant>::new();

            loop {
                if shutdown_health
                    .recv_timeout(Duration::from_secs(10))
                    .is_ok()
                {
                    break;
                }

                // Active recovery logic with control channel
                let status =
                    metrics_health.get_status(receiver.len(), config.performance.channel_capacity);
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

        Ok(vec![handle_file, handle_db, handle_health])
    }

    pub fn get_health_status(&self) -> HealthStatus {
        let channel_len = self.sender.len();
        let channel_cap = self.sender.capacity().unwrap_or(0);
        self.metrics.get_status(channel_len, channel_cap)
    }

    pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError> {
        self.control_tx
            .send(SinkControlMessage::RecoverSink(sink_name.to_string()))
            .map_err(|e| {
                InklogError::ChannelError(format!("Failed to send recovery command: {}", e))
            })
    }

    pub fn trigger_recovery_for_unhealthy_sinks(&self) -> Result<Vec<String>, InklogError> {
        let health_status = self.get_health_status();
        let mut recovered_sinks = Vec::new();

        for (sink_name, sink_status) in &health_status.sinks {
            if !sink_status.status.is_operational() && self.recover_sink(sink_name).is_ok() {
                recovered_sinks.push(sink_name.clone());
            }
        }

        Ok(recovered_sinks)
    }

    pub fn shutdown(&self) -> Result<(), InklogError> {
        let _ = self.shutdown_tx.send(());

        // Take all handles from the struct
        let handles = std::mem::take(&mut *self.worker_handles.lock().unwrap());

        // Use a timeout-based join to avoid deadlocks
        // Each handle gets up to 5 seconds to complete
        for handle in handles {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct LoggerBuilder {
    config: InklogConfig,
}

impl LoggerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.config.global.level = level.into();
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.config.global.format = format.into();
        self
    }

    pub fn console(mut self, enabled: bool) -> Self {
        if let Some(ref mut console) = self.config.console_sink {
            console.enabled = enabled;
        } else if enabled {
            self.config.console_sink = Some(ConsoleSinkConfig::default());
        }
        self
    }

    pub fn file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.enabled = true;
            file.path = path.into();
        } else {
            let path_buf = path.into();
            self.config.file_sink = Some(FileSinkConfig {
                enabled: true,
                path: path_buf,
                ..Default::default()
            });
        }
        self
    }

    pub fn database(mut self, url: impl Into<String>) -> Self {
        let url_str = url.into();
        let config = crate::config::DatabaseSinkConfig {
            name: "default".to_string(),
            enabled: true,
            driver: crate::config::DatabaseDriver::default(),
            url: url_str,
            pool_size: 10,
            batch_size: 100,
            flush_interval_ms: 500,
            partition: crate::config::PartitionStrategy::default(),
            archive_to_s3: false,
            archive_after_days: 30,
            s3_bucket: None,
            s3_region: None,
            table_name: "logs".to_string(),
            archive_format: "json".to_string(),
            parquet_config: crate::config::ParquetConfig::default(),
        };
        self.config.database_sink = Some(config);
        self
    }

    #[cfg(feature = "aws")]
    pub fn s3_archive(mut self, bucket: impl Into<String>, region: impl Into<String>) -> Self {
        let bucket_str = bucket.into();
        let region_str = region.into();
        self.config.s3_archive = Some(crate::archive::S3ArchiveConfig {
            enabled: true,
            bucket: bucket_str,
            region: region_str,
            ..Default::default()
        });
        self
    }

    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.config.performance.channel_capacity = capacity;
        self
    }

    pub fn worker_threads(mut self, threads: usize) -> Self {
        self.config.performance.worker_threads = threads;
        self
    }

    // === Console 配置快捷方法 ===

    pub fn console_colored(mut self, colored: bool) -> Self {
        if let Some(ref mut console) = self.config.console_sink {
            console.colored = colored;
        } else if colored {
            self.config.console_sink = Some(ConsoleSinkConfig {
                colored,
                ..Default::default()
            });
        }
        self
    }

    pub fn console_stderr_levels(mut self, levels: &[&str]) -> Self {
        if let Some(ref mut console) = self.config.console_sink {
            console.stderr_levels = levels.iter().map(|s| (*s).to_string()).collect();
        } else {
            self.config.console_sink = Some(ConsoleSinkConfig {
                stderr_levels: levels.iter().map(|s| (*s).to_string()).collect(),
                ..Default::default()
            });
        }
        self
    }

    // === File 配置快捷方法 ===

    pub fn file_max_size(mut self, max_size: impl Into<String>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.max_size = max_size.into();
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                max_size: max_size.into(),
                ..Default::default()
            });
        }
        self
    }

    pub fn file_compress(mut self, compress: bool) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.compress = compress;
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                compress,
                ..Default::default()
            });
        }
        self
    }

    pub fn file_rotation_time(mut self, rotation: impl Into<String>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.rotation_time = rotation.into();
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                rotation_time: rotation.into(),
                ..Default::default()
            });
        }
        self
    }

    pub fn file_keep_files(mut self, keep: u32) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.keep_files = keep;
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                keep_files: keep,
                ..Default::default()
            });
        }
        self
    }

    pub async fn build(self) -> Result<LoggerManager, InklogError> {
        LoggerManager::with_config(self.config).await
    }
}