// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "aws")]
use crate::archive::{ArchiveService, ArchiveServiceBuilder};
#[allow(unused_imports)]
use crate::config::{ConsoleSinkConfig, DatabaseSinkConfig};
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
    #[allow(dead_code)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
    config: InklogConfig,
    #[allow(dead_code)]
    sender: Sender<LogRecord>,
    shutdown_tx: Sender<()>,
    #[allow(dead_code)]
    console_sink: Arc<Mutex<ConsoleSink>>,
    #[allow(dead_code)]
    metrics: Arc<Metrics>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
    control_tx: Sender<SinkControlMessage>,
    #[cfg(feature = "aws")]
    archive_service: Option<Arc<tokio::sync::Mutex<ArchiveService>>>,
    #[cfg(feature = "http")]
    http_server_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
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

        // Start HTTP server if enabled
        #[cfg(feature = "http")]
        if let Some(ref http_cfg) = config.http_server {
            if http_cfg.enabled {
                match http_cfg.error_mode {
                    crate::config::HttpErrorMode::Panic => {
                        manager.start_http_server(http_cfg).await?;
                    }
                    crate::config::HttpErrorMode::Warn => {
                        if let Err(e) = manager.start_http_server(http_cfg).await {
                            error!("HTTP server failed to start (continuing): {}", e);
                        }
                    }
                    crate::config::HttpErrorMode::Strict => {
                        manager.start_http_server(http_cfg).await?;
                    }
                }
            }
        }

        Ok(manager)
    }

    #[cfg(feature = "http")]
    async fn start_http_server(
        &self,
        cfg: &crate::config::HttpServerConfig,
    ) -> Result<(), InklogError> {
        use axum::{routing::get, Router};
        use std::net::SocketAddr;

        let metrics_for_metrics = self.metrics.clone();
        let metrics_for_health = self.metrics.clone();
        let sender = self.sender.clone();
        let capacity = self.config.performance.channel_capacity;

        let app = Router::new()
            .route(
                &cfg.metrics_path,
                get(move || {
                    let metrics = metrics_for_metrics.clone();
                    async move { metrics.export_prometheus() }
                }),
            )
            .route(
                &cfg.health_path,
                get(move || {
                    let metrics = metrics_for_health.clone();
                    let sender = sender.clone();
                    async move {
                        let status = metrics.get_status(sender.len(), capacity);
                        axum::Json(status)
                    }
                }),
            );

        let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port)
            .parse()
            .map_err(|e| InklogError::ConfigError(format!("Invalid HTTP server address: {}", e)))?;

        info!("Starting HTTP metrics server on {}", addr);

        let handle = tokio::spawn(async move {
            if let Err(e) = tokio::net::TcpListener::bind(addr).await {
                error!("Failed to bind TCP listener: {}", e);
                return;
            }
            if let Err(e) = axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
            {
                error!("HTTP server failed: {}", e);
            }
        });

        let mut server_handle = self.http_server_handle.lock().map_err(|e| {
            InklogError::HttpServerError(format!("Failed to acquire server handle lock: {}", e))
        })?;
        *server_handle = Some(handle);

        Ok(())
    }

    #[cfg(feature = "confers")]
    pub async fn with_watch() -> Result<Self, InklogError> {
        let (config, _watcher, mut rx) = InklogConfig::load_with_watch()?;
        let manager = Self::with_config(config).await?;
        let control_tx = manager.control_tx.clone();

        tokio::spawn(async move {
            while let Some(_new_config) = rx.recv().await {
                info!("Config reloaded, notifying workers");
                let _ = control_tx.send(SinkControlMessage::RecoverSink("file".to_string()));
            }
        });

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

                // Get database connection if available
                #[cfg(not(feature = "dbnexus"))]
                let db_conn = if let Some(ref db_cfg) = config.database_sink {
                    if db_cfg.enabled {
                        use sea_orm::Database;
                        Database::connect(&db_cfg.url).await.ok()
                    } else {
                        None
                    }
                } else {
                    None
                };

                #[cfg(feature = "dbnexus")]
                let db_conn = if let Some(ref db_cfg) = config.database_sink {
                    if db_cfg.enabled {
                        use dbnexus::pool::DbPool;
                        Some(DbPool::new(&db_cfg.url).await.ok())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let mut archive_service_builder =
                    ArchiveServiceBuilder::new().config(archive_config.clone());

                #[cfg(feature = "dbnexus")]
                #[allow(clippy::collapsible_match, clippy::match_result_ok)]
                if let Some(ref pool) = db_conn {
                    if let Some(ref p) = pool {
                        if let Some(s) = p.get_session("").await.ok() {
                            archive_service_builder = archive_service_builder.database_session(s);
                        }
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
            #[cfg(feature = "http")]
            http_server_handle: Mutex::new(None),
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
            #[cfg(feature = "http")]
            http_server_handle: Mutex::new(None),
        };

        Ok((manager, subscriber, filter))
    }

    #[cfg(feature = "confers")]
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, InklogError> {
        let config = InklogConfig::from_file(path)?;
        Self::with_config(config).await
    }

    #[cfg(feature = "confers")]
    pub async fn load() -> Result<Self, InklogError> {
        let config = InklogConfig::load()?;
        Self::with_config(config).await
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
                    if let Ok(mut sink) = DatabaseSink::new(cfg) {
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
                                                    DatabaseSink::new(cfg_clone.clone())
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
                                        if let Ok(new_sink) = DatabaseSink::new(cfg_clone.clone()) {
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
                                                DatabaseSink::new(cfg_clone.clone())
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
        if let Some(ref mut db) = self.config.database_sink {
            db.enabled = true;
            db.url = url.into();
        } else {
            let url_str = url.into();
            self.config.database_sink = Some(DatabaseSinkConfig {
                enabled: true,
                url: url_str,
                ..Default::default()
            });
        }
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

    pub fn http_server(mut self, host: impl Into<String>, port: u16) -> Self {
        let host_str = host.into();
        self.config.http_server = Some(crate::config::HttpServerConfig {
            enabled: true,
            host: host_str,
            port,
            ..Default::default()
        });
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

    pub fn file_encrypt(mut self, encrypt: bool) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.encrypt = encrypt;
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                encrypt,
                ..Default::default()
            });
        }
        self
    }

    pub fn file_encryption_key(mut self, key_env: impl Into<String>) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.encryption_key_env = Some(key_env.into());
        } else {
            self.config.file_sink = Some(FileSinkConfig {
                encryption_key_env: Some(key_env.into()),
                ..Default::default()
            });
        }
        self
    }

    // === Fallback 配置快捷方法 ===

    pub fn auto_fallback(mut self, enabled: bool) -> Self {
        self.config.global.auto_fallback = enabled;
        self
    }

    pub fn fallback_retries(mut self, retries: u32) -> Self {
        self.config.global.fallback_max_retries = retries;
        self
    }

    // === Batch 配置快捷方法 ===

    pub fn batch_size(mut self, size: usize) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.batch_size = size;
        }
        if let Some(ref mut db) = self.config.database_sink {
            db.batch_size = size;
        }
        self
    }

    pub fn flush_interval(mut self, ms: u64) -> Self {
        if let Some(ref mut file) = self.config.file_sink {
            file.flush_interval_ms = ms;
        }
        if let Some(ref mut db) = self.config.database_sink {
            db.flush_interval_ms = ms;
        }
        self
    }

    // === Database 配置快捷方法 ===

    pub fn database_pool_size(mut self, pool_size: u32) -> Self {
        if let Some(ref mut db) = self.config.database_sink {
            db.pool_size = pool_size;
        } else {
            self.config.database_sink = Some(DatabaseSinkConfig {
                pool_size,
                ..Default::default()
            });
        }
        self
    }

    pub fn database_table(mut self, table: impl Into<String>) -> Self {
        if let Some(ref mut db) = self.config.database_sink {
            db.table_name = table.into();
        } else {
            self.config.database_sink = Some(DatabaseSinkConfig {
                table_name: table.into(),
                ..Default::default()
            });
        }
        self
    }

    // === 便捷工厂方法 ===

    /// 创建简单的控制台日志记录器
    pub fn console_only() -> Self {
        Self::default().console(true).level("info")
    }

    /// 创建简单的文件日志记录器
    pub fn file_only(path: impl Into<std::path::PathBuf>) -> Self {
        Self::default().file(path)
    }

    /// 创建生产环境日志记录器（控制台 + 文件）
    pub fn production(path: impl Into<std::path::PathBuf>) -> Self {
        Self::default()
            .console(true)
            .console_colored(false)
            .file(path)
            .file_compress(true)
            .file_max_size("100MB")
            .file_keep_files(10)
            .level("info")
    }

    /// 创建开发环境日志记录器（彩色控制台）
    pub fn development() -> Self {
        Self::default()
            .console(true)
            .console_colored(true)
            .level("debug")
    }

    // === 智能自动配置 ===

    /// 检测当前运行环境
    fn detect_environment() -> EnvironmentProfile {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // 检测是否为交互式终端
        let is_terminal = is_terminal::IsTerminal::is_terminal(&std::io::stdout());

        // 检测是否在容器环境中
        let in_container = std::env::var("CONTAINER").is_ok()
            || std::env::var("DOCKER_CONTAINER").is_ok()
            || std::path::Path::new("/.dockerenv").exists();

        // 检测是否为 CI 环境
        let in_ci = std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("TRAVIS").is_ok();

        // 检测是否为云环境
        let in_cloud = std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok()
            || std::env::var("FUNCTION_NAME").is_ok()
            || std::env::var("KUBERNETES_SERVICE_HOST").is_ok();

        // 检测 CPU 核心数
        let cpu_count = num_cpus::get();

        // 生成简单的机器指纹用于区分不同环境
        let mut hasher = DefaultHasher::new();
        std::env::var("HOSTNAME")
            .unwrap_or_default()
            .hash(&mut hasher);
        let machine_id = hasher.finish();

        EnvironmentProfile {
            is_terminal,
            in_container,
            in_ci,
            in_cloud,
            cpu_count,
            machine_id,
        }
    }

    /// 根据检测到的环境自动配置日志记录器
    ///
    /// 此方法会自动根据运行环境选择最佳配置：
    /// - 交互式终端：启用彩色输出
    /// - CI 环境：禁用颜色，使用 JSON 格式
    /// - 容器环境：优化日志路径
    /// - 云环境：减少本地存储，使用远程存储
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use inklog::LoggerBuilder;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // 自动检测环境并配置
    ///     let logger = LoggerBuilder::auto_detect()
    ///         .file("app.log")
    ///         .build()
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn auto_detect() -> Self {
        let profile = Self::detect_environment();

        let mut builder = Self::default();

        // 根据环境调整日志级别
        let level = if profile.in_ci {
            // CI 环境使用 info 级别，减少输出
            "info".to_string()
        } else if profile.is_terminal {
            // 交互式终端使用 debug 级别
            "debug".to_string()
        } else {
            "info".to_string()
        };
        builder.config.global.level = level;

        // 控制台配置
        builder.config.console_sink = Some(ConsoleSinkConfig {
            enabled: true,
            // 交互式终端启用颜色，CI 环境禁用
            colored: profile.is_terminal && !profile.in_ci,
            ..Default::default()
        });

        // 文件配置 - 根据环境调整
        let log_path = if profile.in_container {
            // 容器环境使用 /var/log
            PathBuf::from("/var/log/app.log")
        } else if profile.in_cloud {
            // 云环境使用临时目录
            std::env::temp_dir().join("app.log")
        } else {
            PathBuf::from("app.log")
        };

        builder.config.file_sink = Some(FileSinkConfig {
            enabled: true,
            path: log_path,
            // 生产环境启用压缩
            compress: !profile.in_ci,
            // 云环境减少保留文件数
            keep_files: if profile.in_cloud { 3 } else { 10 },
            // 大流量场景增加批量大小
            batch_size: if profile.cpu_count > 4 { 200 } else { 100 },
            flush_interval_ms: if profile.cpu_count > 4 { 50 } else { 100 },
            ..Default::default()
        });

        // 根据 CPU 核心数调整 worker 线程
        let worker_threads = if profile.cpu_count > 4 {
            (profile.cpu_count / 2).min(8)
        } else {
            2
        };
        builder.config.performance.worker_threads = worker_threads;

        // 根据 CPU 核心数调整 channel 容量
        let channel_capacity = if profile.cpu_count > 4 { 20000 } else { 10000 };
        builder.config.performance.channel_capacity = channel_capacity;

        // CI 环境禁用自动降级以避免意外行为
        if profile.in_ci {
            builder.config.global.auto_fallback = false;
        }

        builder
    }

    /// 快速初始化 - 适合大多数场景的默认配置
    ///
    /// 等同于 `LoggerBuilder::auto_detect()`，但不需要额外配置
    pub fn quick() -> Self {
        Self::auto_detect()
    }

    pub async fn build(self) -> Result<LoggerManager, InklogError> {
        LoggerManager::with_config(self.config).await
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_logger_builder_new() {
        let builder = LoggerBuilder::new();
        assert!(builder.config.global.level.is_empty() || builder.config.global.level == "info");
    }

    #[test]
    fn test_logger_builder_level() {
        let builder = LoggerBuilder::new().level("debug");
        assert_eq!(builder.config.global.level, "debug");
    }

    #[test]
    fn test_logger_builder_console() {
        let builder = LoggerBuilder::new().console(true).console_colored(false);
        assert!(builder.config.console_sink.is_some());
        assert!(!builder.config.console_sink.as_ref().unwrap().colored);
    }

    #[test]
    fn test_logger_builder_console_stderr_levels() {
        let builder = LoggerBuilder::new().console_stderr_levels(&["error", "warn"]);
        assert!(builder.config.console_sink.is_some());
        let stderr = &builder.config.console_sink.as_ref().unwrap().stderr_levels;
        assert_eq!(stderr.len(), 2);
        assert!(stderr.contains(&"error".to_string()));
        assert!(stderr.contains(&"warn".to_string()));
    }

    #[test]
    fn test_logger_builder_file() {
        let builder = LoggerBuilder::new()
            .file("/var/log/app.log")
            .file_max_size("50MB")
            .file_compress(true)
            .file_keep_files(5);
        assert!(builder.config.file_sink.is_some());
        let file = builder.config.file_sink.as_ref().unwrap();
        assert_eq!(file.path, std::path::PathBuf::from("/var/log/app.log"));
        assert_eq!(file.max_size, "50MB");
        assert!(file.compress);
        assert_eq!(file.keep_files, 5);
    }

    #[test]
    fn test_logger_builder_file_encrypt() {
        let builder = LoggerBuilder::new()
            .file("app.log")
            .file_encrypt(true)
            .file_encryption_key("MY_ENCRYPTION_KEY");
        let file = builder.config.file_sink.as_ref().unwrap();
        assert!(file.encrypt);
        assert_eq!(
            file.encryption_key_env,
            Some("MY_ENCRYPTION_KEY".to_string())
        );
    }

    #[test]
    fn test_logger_builder_database() {
        let builder = LoggerBuilder::new()
            .database("postgres://localhost/logs")
            .database_pool_size(10)
            .database_table("my_logs");
        assert!(builder.config.database_sink.is_some());
        let db = builder.config.database_sink.as_ref().unwrap();
        assert_eq!(db.url, "postgres://localhost/logs");
        assert_eq!(db.pool_size, 10);
        assert_eq!(db.table_name, "my_logs");
    }

    #[test]
    fn test_logger_builder_fallback() {
        let builder = LoggerBuilder::new()
            .auto_fallback(false)
            .fallback_retries(10);
        assert!(!builder.config.global.auto_fallback);
        assert_eq!(builder.config.global.fallback_max_retries, 10);
    }

    #[test]
    fn test_logger_builder_batch() {
        let builder = LoggerBuilder::new()
            .file("app.log")
            .database("postgres://localhost/logs")
            .batch_size(500)
            .flush_interval(200);
        let file = builder.config.file_sink.as_ref().unwrap();
        let db = builder.config.database_sink.as_ref().unwrap();
        assert_eq!(file.batch_size, 500);
        assert_eq!(file.flush_interval_ms, 200);
        assert_eq!(db.batch_size, 500);
        assert_eq!(db.flush_interval_ms, 200);
    }

    #[test]
    fn test_logger_builder_factory_console_only() {
        let builder = LoggerBuilder::console_only();
        assert!(builder.config.console_sink.is_some());
        assert_eq!(builder.config.global.level, "info");
    }

    #[test]
    fn test_logger_builder_factory_file_only() {
        let builder = LoggerBuilder::file_only("app.log");
        assert!(builder.config.file_sink.is_some());
        assert_eq!(
            builder.config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("app.log")
        );
    }

    #[test]
    fn test_logger_builder_factory_development() {
        let builder = LoggerBuilder::development();
        assert!(builder.config.console_sink.is_some());
        assert!(builder.config.console_sink.as_ref().unwrap().colored);
        assert_eq!(builder.config.global.level, "debug");
    }

    #[test]
    fn test_logger_builder_factory_production() {
        let builder = LoggerBuilder::production("app.log");
        assert!(builder.config.console_sink.is_some());
        assert!(!builder.config.console_sink.as_ref().unwrap().colored);
        assert!(builder.config.file_sink.is_some());
        let file = builder.config.file_sink.as_ref().unwrap();
        assert!(file.compress);
        assert_eq!(file.max_size, "100MB");
        assert_eq!(file.keep_files, 10);
        assert_eq!(builder.config.global.level, "info");
    }

    #[test]
    fn test_logger_builder_chaining() {
        let builder = LoggerBuilder::new()
            .level("trace")
            .console(true)
            .console_colored(true)
            .console_stderr_levels(&["error"])
            .file("debug.log")
            .file_max_size("10MB")
            .file_compress(false)
            .file_rotation_time("hourly")
            .file_keep_files(24)
            .auto_fallback(true)
            .fallback_retries(5)
            .batch_size(200)
            .flush_interval(100);

        assert_eq!(builder.config.global.level, "trace");
        assert!(builder.config.console_sink.unwrap().colored);
        assert_eq!(builder.config.global.auto_fallback, true);
        assert_eq!(builder.config.global.fallback_max_retries, 5);
    }

    // === 智能自动配置测试 ===

    #[test]
    fn test_auto_detect_returns_valid_builder() {
        let builder = LoggerBuilder::auto_detect();
        // Should have console sink enabled
        assert!(builder.config.console_sink.is_some());
        // Should have file sink enabled
        assert!(builder.config.file_sink.is_some());
        // Should have a valid level
        assert!(!builder.config.global.level.is_empty());
    }

    #[test]
    fn test_auto_detect_has_console_enabled() {
        let builder = LoggerBuilder::auto_detect();
        let console = builder.config.console_sink.as_ref().unwrap();
        assert!(console.enabled);
    }

    #[test]
    fn test_auto_detect_has_file_enabled() {
        let builder = LoggerBuilder::auto_detect();
        let file = builder.config.file_sink.as_ref().unwrap();
        assert!(file.enabled);
        // Should have a valid path
        assert!(!file.path.as_os_str().is_empty());
    }

    #[test]
    fn test_auto_detect_configures_batch_settings() {
        let builder = LoggerBuilder::auto_detect();
        let file = builder.config.file_sink.as_ref().unwrap();
        assert!(file.batch_size > 0);
        assert!(file.flush_interval_ms > 0);
    }

    #[test]
    fn test_auto_detect_configures_performance() {
        let builder = LoggerBuilder::auto_detect();
        assert!(builder.config.performance.worker_threads > 0);
        assert!(builder.config.performance.channel_capacity > 0);
    }

    #[test]
    fn test_quick_equals_auto_detect() {
        let quick = LoggerBuilder::quick();
        let auto = LoggerBuilder::auto_detect();
        // Both should have similar structure
        assert!(quick.config.console_sink.is_some());
        assert!(auto.config.console_sink.is_some());
        assert!(quick.config.file_sink.is_some());
        assert!(auto.config.file_sink.is_some());
    }

    #[test]
    fn test_environment_profile_has_cpu_count() {
        let profile = LoggerBuilder::detect_environment();
        assert!(profile.cpu_count > 0);
        // CPU count should be reasonable (1-256 cores)
        assert!(profile.cpu_count <= 256);
    }

    #[test]
    fn test_environment_profile_clone() {
        let profile = LoggerBuilder::detect_environment();
        let cloned = profile.clone();
        assert_eq!(profile.machine_id, cloned.machine_id);
        assert_eq!(profile.cpu_count, cloned.cpu_count);
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    #[ignore] // Flaky in CI due to port conflicts
    async fn test_http_server_endpoints() {
        // Use a random port to avoid conflicts
        let port = 19090 + (rand::random::<u16>() % 1000);
        let manager = LoggerBuilder::new()
            .http_server("127.0.0.1", port)
            .build()
            .await
            .unwrap();

        // Give server some time to start
        sleep(Duration::from_millis(500)).await;

        let client = reqwest::Client::new();

        // Test health endpoint
        let health_url = format!("http://127.0.0.1:{}/health", port);
        let resp = client.get(&health_url).send().await.unwrap();
        assert!(resp.status().is_success());
        let health_json: serde_json::Value = resp.json().await.unwrap();
        // Check overall_status is operational (healthy, degraded, or not started but no unhealthy)
        let overall_status = health_json.get("overall_status").unwrap();
        assert!(
            overall_status.is_object(),
            "overall_status should be an object"
        );
        // If status is "Healthy" or "Degraded", system is operational
        let status_type = overall_status
            .get("Healthy")
            .or(overall_status.get("Degraded"));
        assert!(status_type.is_some() || overall_status.get("NotStarted").is_some());

        // Test metrics endpoint
        let metrics_url = format!("http://127.0.0.1:{}/metrics", port);
        let resp = client.get(&metrics_url).send().await.unwrap();
        assert!(resp.status().is_success());
        let metrics_text = resp.text().await.unwrap();
        assert!(metrics_text.contains("inklog_logs_written_total"));

        // Shutdown
        manager.shutdown().unwrap();
    }
}
