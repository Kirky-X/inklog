#[cfg(feature = "aws")]
use crate::archive::{ArchiveService, ArchiveServiceBuilder};
#[allow(unused_imports)]
use crate::config::{ConsoleSinkConfig, DatabaseSinkConfig};
use crate::config::{FileSinkConfig, InklogConfig};
use crate::error::InklogError;
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
#[derive(Debug, Clone)]
pub enum SinkControlMessage {
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
        let registry = tracing_subscriber::registry().with(subscriber).with(filter);
        if let Err(_e) = registry.try_init() {
            // eprintln!("Failed to set global subscriber: {}", e);
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

                let mut archive_service_builder =
                    ArchiveServiceBuilder::new().config(archive_config.clone());

                if let Some(db_conn) = db_conn {
                    archive_service_builder = archive_service_builder.database_connection(db_conn);
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

        // Wait for workers
        if let Ok(mut handles) = self.worker_handles.lock() {
            while let Some(handle) = handles.pop() {
                let _ = handle.join();
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
