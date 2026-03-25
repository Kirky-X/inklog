// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::domain::core::subscriber::LoggerSubscriber;
#[cfg(feature = "dbnexus")]
use crate::integrations::infra::Database;
use crate::integrations::infra::{Cache, Config};
#[cfg(feature = "aws")]
use crate::integrations::storage::archive::{ArchiveService, ArchiveServiceBuilder};
use crate::support::io::sink::console::ConsoleSink;
#[cfg(feature = "dbnexus")]
use crate::support::io::sink::database::DatabaseSink;
use crate::support::io::sink::file::FileSink;
use crate::support::io::sink::LogSink;
use crate::support::io::{LogAdapter, LogLogger};
#[allow(unused_imports)]
use crate::ConsoleSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::LogTemplate;
use crate::{FileSinkConfig, InklogConfig};
use crate::{HealthStatus, Metrics};
use chrono::Utc;
use crossbeam_channel::{bounded, Receiver, Sender};
#[allow(unused_imports)]
use std::path::Path;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
#[cfg(feature = "aws")]
use tokio::sync::Mutex as AsyncMutex;
use tracing::error;
#[cfg(any(feature = "aws", feature = "http"))]
use tracing::info;
use tracing_subscriber::prelude::*;

// Control messages for sink recovery
/// Messages used to control sink recovery and status queries.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum SinkControlMessage {
    RecoverSink(String), // sink name
    GetStatus,
}

// Parameters for worker threads
struct WorkerParams {
    config: InklogConfig,
    receiver: Receiver<Arc<LogRecord>>,
    console_receiver: Receiver<Arc<LogRecord>>,
    shutdown_rx: Receiver<()>,
    control_rx: Receiver<SinkControlMessage>,
    control_tx: Sender<SinkControlMessage>,
    metrics: Arc<Metrics>,
    console_sink: Arc<Mutex<ConsoleSink>>,
    error_sink: Arc<Mutex<Option<FileSink>>>,
    effective_capacity: Arc<AtomicUsize>,
    /// 注入的数据库依赖（DI 模式）
    #[cfg(feature = "dbnexus")]
    database: Option<Arc<dyn Database>>,
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

/// LoggerManager 的依赖集合
///
/// 用于依赖注入模式，允许外部提供缓存、配置和数据库实现。
/// 所有字段都是可选的，未提供的依赖将使用默认实现。
///
/// # 示例
///
/// ```ignore
/// use std::sync::Arc;
/// use inklog::{LoggerManager, LoggerDependencies};
/// use inklog::infrastructure::{MockCache, MockConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let deps = LoggerDependencies {
///         cache: Some(Arc::new(MockCache::new())),
///         config: Some(Arc::new(MockConfig::new())),
///         #[cfg(feature = "dbnexus")]
///         database: None,
///     };
///     let logger = LoggerManager::with_dependencies(deps).await?;
///     Ok(())
/// }
/// ```
#[derive(Default)]
pub struct LoggerDependencies {
    /// 缓存依赖（可选）
    ///
    /// 用于缓存日志元数据、配置值等。
    /// 如果未提供，LoggerManager 将创建默认的内存缓存。
    pub cache: Option<Arc<dyn Cache>>,

    /// 配置依赖（可选）
    ///
    /// 用于动态获取配置值，支持运行时配置更新。
    /// 如果未提供，LoggerManager 将从文件系统加载配置。
    pub config: Option<Arc<dyn Config>>,

    /// 数据库依赖（可选，仅当启用 dbnexus feature 时）
    ///
    /// 用于日志记录的持久化存储。
    /// 如果未提供但配置了数据库 sink，LoggerManager 将创建默认连接池。
    #[cfg(feature = "dbnexus")]
    pub database: Option<Arc<dyn Database>>,
}

impl std::fmt::Debug for LoggerDependencies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("LoggerDependencies");
        builder
            .field("cache", &self.cache.as_ref().map(|_| "Arc<dyn Cache>"))
            .field("config", &self.config.as_ref().map(|_| "Arc<dyn Config>"));
        #[cfg(feature = "dbnexus")]
        builder.field(
            "database",
            &self.database.as_ref().map(|_| "Arc<dyn Database>"),
        );
        builder.finish()
    }
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
    sender: Sender<Arc<LogRecord>>,
    console_sender: Sender<Arc<LogRecord>>,
    shutdown_tx: Sender<()>,
    #[allow(dead_code)]
    console_sink: Arc<Mutex<ConsoleSink>>,
    metrics: Arc<Metrics>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
    control_tx: Sender<SinkControlMessage>,
    effective_capacity: Arc<AtomicUsize>,
    #[cfg(feature = "aws")]
    archive_service: Option<Arc<tokio::sync::Mutex<ArchiveService>>>,
    #[cfg(feature = "http")]
    http_server_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// 注入的缓存依赖
    cache: Option<Arc<dyn Cache>>,
    /// 注入的数据库依赖（需要 dbnexus feature）
    #[cfg(feature = "dbnexus")]
    database: Option<Arc<dyn Database>>,
}

impl LoggerManager {
    pub async fn new() -> Result<Self, InklogError> {
        // 经过 DI 路径创建默认实例，行为与 builder().build() 一致
        Self::with_dependencies(LoggerDependencies::default()).await
    }

    /// 完全依赖注入模式创建 LoggerManager
    ///
    /// 允许外部提供缓存、配置和数据库实现，用于测试和高级场景。
    /// 未提供的依赖将使用默认实现。
    ///
    /// # 参数
    ///
    /// * `deps` - 依赖集合，包含可选的缓存、配置和数据库实现
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(LoggerManager)`，失败返回 `Err(InklogError)`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::{LoggerManager, LoggerDependencies};
    /// use inklog::infrastructure::{MockCache, MockConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let deps = LoggerDependencies {
    ///         cache: Some(Arc::new(MockCache::new())),
    ///         config: Some(Arc::new(MockConfig::new())),
    ///     };
    ///     let logger = LoggerManager::with_dependencies(deps).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_dependencies(deps: LoggerDependencies) -> Result<Self, InklogError> {
        Self::build_with_deps(deps).await
    }

    /// 使用依赖注入构建 LoggerManager
    ///
    /// 内部方法，处理依赖解析和默认值填充。
    async fn build_with_deps(deps: LoggerDependencies) -> Result<Self, InklogError> {
        // 如果提供了 Config trait 实现，从中获取 InklogConfig
        // 否则使用默认配置加载流程
        let config = if let Some(ref config_provider) = deps.config {
            // 尝试从 Config trait 获取基本配置值
            // 由于 Config trait 只提供基本的 get_* 方法，
            // 我们需要构建一个 InklogConfig 实例
            let mut config = InklogConfig::default();

            // 应用配置值
            if let Some(level) = config_provider.get_string("global.level") {
                config.global.level = level;
            }
            if let Some(format) = config_provider.get_string("global.format") {
                config.global.format = format;
            }
            if let Some(masking) = config_provider.get_bool("global.masking_enabled") {
                config.global.masking_enabled = masking;
            }
            if let Some(fallback) = config_provider.get_bool("global.auto_fallback") {
                config.global.auto_fallback = fallback;
            }

            // File sink 配置
            if config_provider
                .get_bool("file_sink.enabled")
                .unwrap_or(false)
            {
                let path = config_provider
                    .get_string("file_sink.path")
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let max_size = config_provider
                    .get_string("file_sink.max_size")
                    .unwrap_or_else(|| "100MB".to_string());
                let compress = config_provider
                    .get_bool("file_sink.compress")
                    .unwrap_or(true);

                config.file_sink = Some(FileSinkConfig {
                    enabled: true,
                    path,
                    max_size,
                    compress,
                    ..Default::default()
                });
            }

            // HTTP server 配置
            if config_provider
                .get_bool("http_server.enabled")
                .unwrap_or(false)
            {
                let host = config_provider
                    .get_string("http_server.host")
                    .unwrap_or_else(|| "127.0.0.1".to_string());
                let port = config_provider
                    .get_int("http_server.port")
                    .map(|p| p as u16)
                    .unwrap_or(9090);

                config.http_server = Some(crate::HttpServerConfig {
                    enabled: true,
                    host,
                    port,
                    ..Default::default()
                });
            }

            // Performance 配置
            if let Some(threads) = config_provider.get_int("performance.worker_threads") {
                config.performance.worker_threads = threads as usize;
            }
            if let Some(capacity) = config_provider.get_int("performance.channel_capacity") {
                config.performance.channel_capacity = capacity as usize;
            }

            config
        } else {
            // 使用 confers 自动生成的方法加载配置
            InklogConfig::load_sync().unwrap_or_else(|_| InklogConfig::default())
        };

        // 注意：cache 和 database 依赖传递给 LoggerManager 内部使用
        // 它们可以通过 LoggerManager 传递给需要的服务（如 DatabaseSink）
        let cache = deps.cache;
        #[cfg(feature = "dbnexus")]
        let database = deps.database;

        // 使用解析后的配置调用现有的构建逻辑
        let (mut manager, _subscriber, _filter) = Self::build_detached(
            config,
            #[cfg(feature = "dbnexus")]
            database.clone(),
        )
        .await?;

        // 将 cache 依赖注入到 manager 中
        manager.cache = cache;

        // database 已经在 build_detached 中使用，同时也存储在 manager 中
        #[cfg(feature = "dbnexus")]
        {
            manager.database = database;
        }

        Ok(manager)
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

        let (manager, subscriber, filter) = Self::build_detached(
            config.clone(),
            #[cfg(feature = "dbnexus")]
            None,
        )
        .await?;

        // 1. 安装 tracing subscriber
        let registry = tracing_subscriber::registry().with(subscriber).with(filter);
        if let Err(ref e) = registry.try_init() {
            tracing::warn!("Failed to set global subscriber: {}", e);
        }

        // 2. 安装 log crate logger（原生支持，无需 tracing_log）
        let log_adapter = LogAdapter::new(
            manager.console_sender.clone(),
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

        // 3. 启动HTTP监控服务器（如果配置启用）
        #[cfg(feature = "http")]
        if let Some(ref http_cfg) = config.http_server {
            if http_cfg.enabled {
                if let Err(e) = manager.start_http_server(http_cfg).await {
                    match http_cfg.error_mode {
                        crate::HttpErrorMode::Warn => {
                            tracing::warn!("HTTP server startup failed (continuing): {}", e);
                        }
                        crate::HttpErrorMode::Strict => {
                            return Err(e);
                        }
                    }
                }
            }
        }

        Ok(manager)
    }

    /// 构建LoggerManager但不安装全局订阅者。
    /// 这主要用于测试和基准测试。
    pub async fn build_detached(
        config: InklogConfig,
        #[cfg(feature = "dbnexus")] database: Option<Arc<dyn Database>>,
    ) -> Result<
        (
            Self,
            LoggerSubscriber,
            tracing_subscriber::filter::LevelFilter,
        ),
        InklogError,
    > {
        let metrics = Arc::new(Metrics::new());
        let (sender, receiver) = bounded(config.performance.channel_capacity);
        let (console_sender, console_receiver) = bounded(config.performance.channel_capacity);
        let (shutdown_tx, shutdown_rx) = bounded(1);
        let (control_tx, control_rx) = bounded(10); // Control channel for recovery commands
        let effective_capacity = Arc::new(AtomicUsize::new(config.performance.channel_capacity));

        let console_sink = Arc::new(Mutex::new(ConsoleSink::new(
            config.console_sink.clone().unwrap_or_default(),
            LogTemplate::new(&config.global.format),
        )));

        // Initialize tracing subscriber with console_sender channel
        let subscriber =
            LoggerSubscriber::new(console_sender.clone(), sender.clone(), metrics.clone());

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
            console_receiver,
            shutdown_rx,
            control_rx,
            control_tx: control_tx.clone(),
            metrics: metrics.clone(),
            console_sink: console_sink.clone(),
            error_sink: error_sink.clone(),
            effective_capacity: effective_capacity.clone(),
            #[cfg(feature = "dbnexus")]
            database,
        })?;

        // Initialize archive service if configured
        #[cfg(feature = "aws")]
        let archive_service = if let Some(ref archive_config) = config.s3_archive {
            if archive_config.enabled {
                info!("Initializing S3 archive service");

                #[cfg(feature = "dbnexus")]
                let db_conn: Option<dbnexus::database::pool::DbPool> =
                    if let Some(ref db_cfg) = config.database_sink {
                        use dbnexus::database::pool::DbPoolBuilder;
                        let db_result = DbPoolBuilder::new()
                            .url(&db_cfg.url)
                            .max_connections(db_cfg.pool_size)
                            .build()
                            .await;

                        match db_result {
                            Ok(pool) => Some(pool),
                            Err(e) => {
                                tracing::warn!("Failed to create DbPool: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                #[cfg(feature = "dbnexus")]
                let mut archive_service_builder =
                    ArchiveServiceBuilder::new().config(archive_config.clone());
                #[cfg(not(feature = "dbnexus"))]
                let archive_service_builder =
                    ArchiveServiceBuilder::new().config(archive_config.clone());

                #[cfg(feature = "dbnexus")]
                #[allow(clippy::collapsible_match, clippy::match_result_ok)]
                if let Some(pool) = db_conn {
                    if let Some(s) = pool.get_session("admin").await.ok() {
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
            console_sender,
            shutdown_tx,
            console_sink,
            metrics,
            worker_handles: Mutex::new(handles),
            control_tx,
            effective_capacity: effective_capacity.clone(),
            archive_service,
            #[cfg(feature = "http")]
            http_server_handle: Mutex::new(None),
            cache: None,
            #[cfg(feature = "dbnexus")]
            database: None,
        };

        #[cfg(not(feature = "aws"))]
        let manager = Self {
            config,
            sender,
            console_sender,
            shutdown_tx,
            console_sink,
            metrics,
            worker_handles: Mutex::new(handles),
            control_tx,
            effective_capacity: effective_capacity.clone(),
            #[cfg(feature = "http")]
            http_server_handle: Mutex::new(None),
            cache: None,
            #[cfg(feature = "dbnexus")]
            database: None,
        };

        Ok((manager, subscriber, filter))
    }

    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    /// 从配置文件初始化LoggerManager
    ///
    /// # Arguments
    /// * `path` - 配置文件路径（TOML格式）
    ///
    /// # Returns
    /// 成功返回LoggerManager实例，失败返回错误
    ///
    /// # Example
    /// ```ignore
    /// use inklog::LoggerManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let _logger = LoggerManager::from_file("config.toml").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, InklogError> {
        let config = InklogConfig::load_file(path.as_ref()).map_err(|e| {
            InklogError::ConfigError(format!("Failed to load config from file: {}", e))
        })?;
        Self::with_config(config).await
    }

    /// 自动搜索并加载配置文件初始化LoggerManager
    ///
    /// 搜索路径优先级：
    /// 1. 环境变量 `INKLOG_CONFIG_PATH` 指定的路径
    /// 2. 当前目录下的 `inklog_config.toml`
    /// 3. 用户配置目录 `~/.config/inklog/config.toml`
    /// 4. 系统配置目录 `/etc/inklog/config.toml`
    ///
    /// # Returns
    /// 成功返回LoggerManager实例，失败返回错误
    ///
    /// # Example
    /// ```ignore
    /// use inklog::LoggerManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let _logger = LoggerManager::load().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn load() -> Result<Self, InklogError> {
        let config = InklogConfig::load_sync()
            .map_err(|e| InklogError::ConfigError(format!("Failed to load config: {}", e)))?;
        Self::with_config(config).await
    }

    /// 启动HTTP监控服务器
    ///
    /// 提供健康检查和Prometheus指标端点
    /// 支持 Bearer Token 认证和 IP 白名单
    #[cfg(feature = "http")]
    async fn start_http_server(&self, config: &crate::HttpServerConfig) -> Result<(), InklogError> {
        use axum::{
            extract::{ConnectInfo, State},
            http::{header, Request, StatusCode},
            middleware::{self, Next},
            response::{IntoResponse, Response},
            routing::get,
            Router,
        };
        use std::net::SocketAddr;

        let metrics = self.metrics.clone();
        let health_path = config.health_path.clone();
        let metrics_path = config.metrics_path.clone();

        let health_status_getter = {
            let sender = self.sender.clone();
            let effective_capacity = self.effective_capacity.clone();
            let metrics_clone = metrics.clone();
            move || {
                let channel_len = sender.len();
                let channel_cap = effective_capacity.load(std::sync::atomic::Ordering::Relaxed);
                metrics_clone.get_status(channel_len, channel_cap)
            }
        };

        #[derive(Clone)]
        struct HttpAuthState {
            auth_enabled: bool,
            token_env: Option<String>,
            ip_whitelist: Option<Vec<String>>,
        }

        let auth_state = HttpAuthState {
            auth_enabled: config.auth.as_ref().map(|a| a.enabled).unwrap_or(false),
            token_env: config.auth.as_ref().map(|a| a.token_env.clone()),
            ip_whitelist: config.ip_whitelist.clone(),
        };

        async fn auth_middleware(
            State(state): State<HttpAuthState>,
            ConnectInfo(addr): ConnectInfo<SocketAddr>,
            request: Request<axum::body::Body>,
            next: Next,
        ) -> Response {
            if state.auth_enabled {
                if let Some(ref token_env) = state.token_env {
                    let expected_token = match std::env::var(token_env) {
                        Ok(t) => t,
                        Err(_) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Auth token not configured",
                            )
                                .into_response();
                        }
                    };

                    let auth_header = request
                        .headers()
                        .get(header::AUTHORIZATION)
                        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

                    match auth_header {
                        Some(h) if h.starts_with("Bearer ") => {
                            let token = &h[7..];
                            if !subtle_constant_time_compare(
                                token.as_bytes(),
                                expected_token.as_bytes(),
                            ) {
                                return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
                            }
                        }
                        _ => {
                            return (
                                StatusCode::UNAUTHORIZED,
                                "Missing or invalid Authorization header",
                            )
                                .into_response();
                        }
                    }
                }
            }

            if let Some(ref whitelist) = state.ip_whitelist {
                let client_ip = addr.ip().to_string();
                if !whitelist.iter().any(|allowed| {
                    if allowed.ends_with(".*") {
                        let prefix = &allowed[..allowed.len() - 2];
                        client_ip.starts_with(prefix)
                    } else if allowed.contains('/') {
                        matches!(parse_cidr(allowed), Some(network) if network.contains(&addr.ip()))
                    } else {
                        client_ip == *allowed
                    }
                }) {
                    return (StatusCode::FORBIDDEN, "IP not in whitelist").into_response();
                }
            }

            next.run(request).await
        }

        fn subtle_constant_time_compare(a: &[u8], b: &[u8]) -> bool {
            a.ct_eq(b).unwrap_u8() == 1
        }

        fn parse_cidr(cidr: &str) -> Option<ipnet::IpNet> {
            cidr.parse().ok()
        }

        let app = Router::new()
            .route(
                &health_path,
                get(|| async move {
                    let status = health_status_getter();
                    axum::Json(serde_json::to_value(&status).unwrap_or_default())
                }),
            )
            .route(
                &metrics_path,
                get(move || async move { metrics.export_prometheus() }),
            )
            .layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            ))
            .with_state(auth_state);

        let addr: std::net::SocketAddr = format!("{}:{}", config.host, config.port)
            .parse()
            .map_err(|e| InklogError::ConfigError(format!("Invalid HTTP server address: {}", e)))?;

        let auth_enabled = config.auth.as_ref().map(|a| a.enabled).unwrap_or(false);
        let ip_whitelist = config.ip_whitelist.clone();

        let handle = tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("Failed to bind HTTP server to {}: {}", addr, e);
                    return;
                }
            };
            info!(
                "HTTP server started on {} (auth: {}, ip_whitelist: {:?})",
                addr, auth_enabled, ip_whitelist
            );
            match axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            {
                Ok(_) => info!("HTTP server stopped"),
                Err(e) => tracing::error!("HTTP server error: {}", e),
            }
        });

        match self.http_server_handle.lock() {
            Ok(mut guard) => *guard = Some(handle),
            Err(e) => {
                tracing::error!("HTTP server handle lock poisoned: {}", e);
            }
        }

        info!("HTTP monitoring server configured on {}", addr);
        Ok(())
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
            console_receiver,
            shutdown_rx,
            control_rx,
            control_tx,
            metrics,
            console_sink,
            error_sink,
            effective_capacity,
            #[cfg(feature = "dbnexus")]
            database,
        } = params;
        let file_config = config.file_sink.clone();
        #[allow(unused_variables)]
        let db_config = config.database_sink.clone();

        // 确保 database 始终有效：如果配置了数据库但没有提供 DI 依赖，则创建默认实现
        #[cfg(feature = "dbnexus")]
        let database = {
            match database {
                Some(db) => Some(db),
                None => {
                    if let Some(ref cfg) = db_config {
                        if cfg.enabled {
                            // 获取当前 tokio runtime 并创建默认的 DbNexusAdapter
                            let handle = tokio::runtime::Handle::current();
                            let cfg_url = cfg.url.clone();
                            let cfg_pool_size = cfg.pool_size;
                            let adapter = handle.block_on(async {
                                crate::integrations::infra::DbNexusAdapter::new(
                                    &cfg_url,
                                    cfg_pool_size,
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
        let shutdown_console = shutdown_rx.clone();
        let metrics_console = metrics.clone();
        let console_sink_console = console_sink.clone();
        let handle_console = thread::spawn(move || {
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
                                if sink.write(&record).is_err() {
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
                                if sink.write(&record).is_err() {
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
        });

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
                                                    if let Ok(cs) = console_sink_file.lock() {
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
                                                if let Ok(cs) = console_sink_file.lock() {
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
        #[cfg(feature = "dbnexus")]
        let rx_db = receiver.clone();
        #[cfg(feature = "dbnexus")]
        let shutdown_db = shutdown_rx.clone();
        #[cfg(feature = "dbnexus")]
        let metrics_db = metrics.clone();
        #[cfg(feature = "dbnexus")]
        let console_sink_db = console_sink.clone();
        #[cfg(feature = "dbnexus")]
        let control_rx_db = control_rx.clone();
        #[cfg(feature = "dbnexus")]
        let handle_db = thread::spawn(
            #[allow(unused_assignments)]
            move || {
                metrics_db.active_workers.inc();
                if let Some(cfg) = db_config {
                    if cfg.enabled {
                        if let Some(ref db) = database {
                            // Clone once before the loop for recovery use
                            let db_for_recovery = db.clone();
                            if let Ok(sink_result) = DatabaseSink::new(db.clone()) {
                                let mut sink: DatabaseSink = sink_result;
                                sink.set_metrics(metrics_db.clone());
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
                                            let write_result: Result<(), InklogError> =
                                                sink.write(&record);
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
                                                            let _ = cs.write(&record);
                                                        }
                                                    } else {
                                                        thread::sleep(Duration::from_millis(
                                                            10 * attempts as u64,
                                                        ));
                                                    }
                                                }
                                            }

                                            // Auto-recovery trigger
                                            if !write_succeeded && consecutive_failures > 5 {
                                                if let Some(last_failure) = last_failure_time {
                                                    if last_failure.elapsed()
                                                        > Duration::from_secs(60)
                                                    {
                                                        eprintln!("Database sink: Triggering auto-recovery due to consecutive failures");
                                                        if let Ok(new_sink) = DatabaseSink::new(
                                                            db_for_recovery.clone(),
                                                        ) {
                                                            sink = new_sink;
                                                            sink.set_metrics(metrics_db.clone());
                                                            consecutive_failures = 0;
                                                            metrics_db.update_sink_health(
                                                                "database", true, None,
                                                            );
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
                                                eprintln!(
                                                    "Database sink: Received recovery command"
                                                );
                                                // Attempt to recreate the sink
                                                if let Ok(new_sink) =
                                                    DatabaseSink::new(db_for_recovery.clone())
                                                {
                                                    sink = new_sink;
                                                    sink.set_metrics(metrics_db.clone());
                                                    consecutive_failures = 0;
                                                    last_failure_time = None;
                                                    metrics_db
                                                        .update_sink_health("database", true, None);
                                                    eprintln!(
                                                        "Database sink: Successfully recovered"
                                                    );
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

                                    if let Ok(record) =
                                        rx_db.recv_timeout(Duration::from_millis(100))
                                    {
                                        let latency = Utc::now()
                                            .signed_duration_since(record.timestamp)
                                            .to_std()
                                            .unwrap_or(Duration::ZERO);
                                        metrics_db.record_latency(latency);

                                        // Retry logic
                                        let mut attempts = 0;
                                        let mut write_succeeded = false;
                                        let write_result: Result<(), InklogError> =
                                            sink.write(&record);
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
                                                    let error_msg = format!("{e}");
                                                    metrics_db.update_sink_health(
                                                        "database",
                                                        false,
                                                        Some(error_msg),
                                                    );

                                                    // Fallback chain: DB -> File -> Console
                                                    if let Ok(cs) = console_sink_db.lock() {
                                                        let _ = cs.write(&record);
                                                    }
                                                } else {
                                                    thread::sleep(Duration::from_millis(
                                                        10 * attempts as u64,
                                                    ));
                                                }
                                            }
                                        }

                                        // Auto-recovery trigger
                                        if !write_succeeded && consecutive_failures > 5 {
                                            if let Some(last_failure) = last_failure_time {
                                                if last_failure.elapsed() > Duration::from_secs(60)
                                                {
                                                    eprintln!("Database sink: Triggering auto-recovery due to consecutive failures");
                                                    if let Ok(new_sink) =
                                                        DatabaseSink::new(db_for_recovery.clone())
                                                    {
                                                        sink = new_sink;
                                                        sink.set_metrics(metrics_db.clone());
                                                        consecutive_failures = 0;
                                                        metrics_db.update_sink_health(
                                                            "database", true, None,
                                                        );
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
                }
                metrics_db.active_workers.dec();
            },
        );

        #[cfg(not(feature = "dbnexus"))]
        let _handle_db = thread::spawn(|| {});

        // Health Check Thread
        let shutdown_health = shutdown_rx.clone();
        let metrics_health = metrics.clone();
        let effective_capacity_health = effective_capacity.clone();
        let handle_health = thread::spawn(move || {
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

        #[cfg(feature = "dbnexus")]
        let handles = vec![handle_console, handle_file, handle_db, handle_health];
        #[cfg(not(feature = "dbnexus"))]
        let handles = vec![handle_console, handle_file, handle_health];

        Ok(handles)
    }

    pub fn get_health_status(&self) -> HealthStatus {
        let channel_len = self.sender.len();
        let channel_cap = self.effective_capacity.load(Ordering::Relaxed);
        self.metrics.get_status(channel_len, channel_cap)
    }

    pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError> {
        self.control_tx
            .send(SinkControlMessage::RecoverSink(sink_name.to_string()))
            .map_err(|e| {
                InklogError::ChannelError(format!("Failed to send recovery command: {}", e))
            })
    }

    pub fn effective_channel_capacity(&self) -> usize {
        self.effective_capacity.load(Ordering::Relaxed)
    }

    pub fn channel_len(&self) -> usize {
        self.sender.len()
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

        // 关闭HTTP服务器
        #[cfg(feature = "http")]
        {
            if let Ok(mut handle_guard) = self.http_server_handle.lock() {
                if let Some(handle) = handle_guard.take() {
                    handle.abort();
                    info!("HTTP server shutdown signal sent");
                }
            }
        }

        // Take all handles from the struct
        let handles = match self.worker_handles.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(e) => {
                error!("Worker handles lock poisoned: {}", e);
                Vec::new()
            }
        };

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

/// Logger 构建器，支持链式配置和依赖注入
///
/// 支持两种配置模式：
/// 1. **纯配置模式**：通过 `.level()`, `.file()` 等方法配置
/// 2. **依赖注入模式**：通过 `.cache()`, `.config()`, `.database()` 注入实现
/// 3. **混合模式**：同时使用配置和依赖注入
///
/// # 示例
///
/// ## 纯配置模式
/// ```ignore
/// let logger = LoggerManager::builder()
///     .level("debug")
///     .file("logs/app.log")
///     .build().await?;
/// ```
///
/// ## 依赖注入模式
/// ```ignore
/// let logger = LoggerManager::builder()
///     .cache(Arc::new(MockCache::new()))
///     .config(Arc::new(MockConfig::new()))
///     .build().await?;
/// ```
///
/// ## 混合模式
/// ```ignore
/// let logger = LoggerManager::builder()
///     .level("debug")
///     .cache(Arc::new(MockCache::new()))  // 使用自定义缓存，其他用配置
///     .build().await?;
/// ```
#[derive(Default)]
pub struct LoggerBuilder {
    config: InklogConfig,
    deps: LoggerDependencies,
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

    #[cfg(feature = "dbnexus")]
    pub fn database(mut self, url: impl Into<String>) -> Self {
        let url_str = url.into();
        let config = crate::DatabaseSinkConfig {
            name: "default".to_string(),
            enabled: true,
            driver: crate::DatabaseDriver::default(),
            url: url_str,
            pool_size: 10,
            batch_size: 100,
            flush_interval_ms: 500,
            partition: crate::PartitionStrategy::default(),
            archive_to_s3: false,
            archive_after_days: 30,
            s3_bucket: None,
            s3_region: None,
            table_name: "logs".to_string(),
            archive_format: "json".to_string(),
            parquet_config: crate::ParquetConfig::default(),
        };
        self.config.database_sink = Some(config);
        self
    }

    #[cfg(feature = "aws")]
    pub fn s3_archive(mut self, bucket: impl Into<String>, region: impl Into<String>) -> Self {
        let bucket_str = bucket.into();
        let region_str = region.into();
        self.config.s3_archive = Some(crate::integrations::storage::archive::S3ArchiveConfig {
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

    // === HTTP Server 配置快捷方法 ===

    /// 启用或禁用HTTP监控服务器
    ///
    /// # Arguments
    /// * `enabled` - 是否启用HTTP服务器
    ///
    /// # Example
    /// ```ignore
    /// let _logger = LoggerManager::builder()
    ///     .enable_http_server(true)
    ///     .build()
    ///     .await?;
    /// ```
    #[cfg(feature = "http")]
    pub fn enable_http_server(mut self, enabled: bool) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.enabled = enabled;
        } else if enabled {
            self.config.http_server = Some(crate::HttpServerConfig::default());
        }
        self
    }

    /// 设置HTTP服务器监听主机
    ///
    /// # Arguments
    /// * `host` - 监听主机地址（如 "127.0.0.1" 或 "0.0.0.0"）
    #[cfg(feature = "http")]
    pub fn http_host(mut self, host: impl Into<String>) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.host = host.into();
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                host: host.into(),
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器监听端口
    ///
    /// # Arguments
    /// * `port` - 监听端口号
    #[cfg(feature = "http")]
    pub fn http_port(mut self, port: u16) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.port = port;
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                port,
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器指标路径
    ///
    /// # Arguments
    /// * `path` - Prometheus指标端点路径（默认 "/metrics"）
    #[cfg(feature = "http")]
    pub fn http_metrics_path(mut self, path: impl Into<String>) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.metrics_path = path.into();
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                metrics_path: path.into(),
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器健康检查路径
    ///
    /// # Arguments
    /// * `path` - 健康检查端点路径（默认 "/health"）
    #[cfg(feature = "http")]
    pub fn http_health_path(mut self, path: impl Into<String>) -> Self {
        if let Some(ref mut http) = self.config.http_server {
            http.health_path = path.into();
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                health_path: path.into(),
                ..Default::default()
            });
        }
        self
    }

    /// 设置HTTP服务器错误处理模式
    ///
    /// # Arguments
    /// * `mode` - 错误处理模式（"warn" 或 "strict"）
    #[cfg(feature = "http")]
    pub fn http_error_mode(mut self, mode: impl Into<String>) -> Self {
        let error_mode = match mode.into().to_lowercase().as_str() {
            "warn" => crate::HttpErrorMode::Warn,
            "strict" => crate::HttpErrorMode::Strict,
            _ => crate::HttpErrorMode::default(),
        };
        if let Some(ref mut http) = self.config.http_server {
            http.error_mode = error_mode;
        } else {
            self.config.http_server = Some(crate::HttpServerConfig {
                error_mode,
                ..Default::default()
            });
        }
        self
    }

    // === 依赖注入方法 ===

    /// 注入自定义 Cache 实现
    ///
    /// 用于测试场景或需要自定义缓存行为的场景。
    /// 如果未调用此方法，LoggerManager 将创建默认的内存缓存。
    ///
    /// # Arguments
    /// * `cache` - 实现 `Cache` trait 的缓存实例
    ///
    /// # Example
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::infrastructure::MockCache;
    ///
    /// let logger = LoggerManager::builder()
    ///     .cache(Arc::new(MockCache::new()))
    ///     .build().await?;
    /// ```
    pub fn cache(mut self, cache: Arc<dyn Cache>) -> Self {
        self.deps.cache = Some(cache);
        self
    }

    /// 注入自定义 Config 实现
    ///
    /// 用于动态配置场景，允许运行时更新配置值。
    /// 如果未调用此方法，LoggerManager 将从文件系统加载配置。
    ///
    /// # Arguments
    /// * `config` - 实现 `Config` trait 的配置实例
    ///
    /// # Example
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::infrastructure::MockConfig;
    ///
    /// let logger = LoggerManager::builder()
    ///     .config(Arc::new(MockConfig::new()))
    ///     .build().await?;
    /// ```
    pub fn config(mut self, config: Arc<dyn Config>) -> Self {
        self.deps.config = Some(config);
        self
    }

    /// 注入自定义 Database 实现
    ///
    /// 用于数据库 sink 的自定义连接管理。
    /// 如果未调用此方法但配置了数据库 sink，LoggerManager 将创建默认连接池。
    ///
    /// # Arguments
    /// * `database` - 实现 `Database` trait 的数据库实例
    ///
    /// # Example
    /// ```ignore
    /// use std::sync::Arc;
    /// use inklog::infrastructure::MockDatabaseAdapter;
    ///
    /// let logger = LoggerManager::builder()
    ///     .with_database(Arc::new(MockDatabaseAdapter::new()))
    ///     .build().await?;
    /// ```
    #[cfg(feature = "dbnexus")]
    pub fn with_database(mut self, database: Arc<dyn Database>) -> Self {
        self.deps.database = Some(database);
        self
    }

    /// 构建 LoggerManager 实例
    ///
    /// 根据配置和注入的依赖创建 LoggerManager。
    /// 优先使用注入的依赖，未注入的依赖将使用配置创建默认实现。
    ///
    /// # Returns
    /// 成功返回 `Ok(LoggerManager)`，失败返回 `Err(InklogError)`
    pub async fn build(self) -> Result<LoggerManager, InklogError> {
        // 如果有任何注入的依赖，使用 with_dependencies
        let has_deps = self.deps.cache.is_some() || self.deps.config.is_some() || {
            #[cfg(feature = "dbnexus")]
            {
                self.deps.database.is_some()
            }
            #[cfg(not(feature = "dbnexus"))]
            {
                false
            }
        };

        if has_deps {
            // 有依赖注入，使用 with_dependencies
            // 但需要先把 config 中的配置应用到 deps.config
            let mut deps = self.deps;

            // 如果注入了 Config trait，将 InklogConfig 的值应用到它
            // 注意：这里我们不覆盖已注入的 config，因为用户明确注入了
            // 但我们可以保留 self.config 用于其他配置项

            // 如果没有注入 config，但有其他注入，我们需要创建一个包含 self.config 的 deps
            if deps.config.is_none() {
                // 将 self.config 通过 ConfersAdapter 注入
                // 这允许 mixed mode 正常工作
                deps.config = Some(Arc::new(
                    crate::integrations::infra::ConfersAdapter::from_config(self.config.clone()),
                ));
            }

            LoggerManager::with_dependencies(deps).await
        } else {
            // 纯配置模式
            LoggerManager::with_config(self.config).await
        }
    }
}
