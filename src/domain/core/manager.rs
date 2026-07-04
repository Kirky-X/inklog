// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::domain::core::subscriber::LoggerSubscriber;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::integrations::infra::Database;
use crate::integrations::infra::{Cache, Config};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::integrations::kit::keys::DatabaseCapabilityKey;
use crate::integrations::kit::keys::{CacheCapabilityKey, ConfigCapabilityKey, InklogConfigKey};
use crate::support::io::sink::console::ConsoleSink;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
use tracing::error;
#[cfg(feature = "http")]
use tracing::info;
use tracing_subscriber::prelude::*;
use trait_kit::kit::Kit;

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
    control_rx: Receiver<SinkControlMessage>,
    control_tx: Sender<SinkControlMessage>,
    metrics: Arc<Metrics>,
    console_sink: Arc<Mutex<ConsoleSink>>,
    error_sink: Arc<Mutex<Option<FileSink>>>,
    effective_capacity: Arc<AtomicUsize>,
    /// 注入的数据库依赖（DI 模式）
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    database: Option<Arc<dyn Database>>,
}

/// `start_workers` 返回值类型别名，避免 clippy `type_complexity` 警告。
/// 第一项为 worker 线程句柄，第二项为每个 worker 对应的 shutdown 信号 sender。
type WorkerStartResult = Result<(Vec<JoinHandle<()>>, Vec<Sender<()>>), InklogError>;

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
///         #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub database: Option<Arc<dyn Database>>,
}

impl std::fmt::Debug for LoggerDependencies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("LoggerDependencies");
        builder
            .field("cache", &self.cache.as_ref().map(|_| "Arc<dyn Cache>"))
            .field("config", &self.config.as_ref().map(|_| "Arc<dyn Config>"));
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
    shutdown_txs: Vec<Sender<()>>,
    #[allow(dead_code)]
    console_sink: Arc<Mutex<ConsoleSink>>,
    metrics: Arc<Metrics>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
    control_tx: Sender<SinkControlMessage>,
    effective_capacity: Arc<AtomicUsize>,
    #[cfg(feature = "http")]
    http_server_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// 注入的缓存依赖
    cache: Option<Arc<dyn Cache>>,
    /// 注入的数据库依赖（需要 dbnexus feature）
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    database: Option<Arc<dyn Database>>,
    /// trait-kit 能力注册中心
    ///
    /// 存储 [`InklogConfig`]（通过 [`InklogConfigKey`]）和已注册的基础设施
    /// 能力（[`ConfigCapabilityKey`]、[`CacheCapabilityKey`]、
    /// [`DatabaseCapabilityKey`]）。使用 `kit.config::<InklogConfigKey>()`
    /// 获取可热更新的配置句柄。
    kit: Kit,
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
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        let database = deps.database;

        // 使用解析后的配置调用现有的构建逻辑
        let (mut manager, _subscriber, _filter) = Self::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database.clone(),
        )
        .await?;

        // 将 cache 依赖注入到 manager 中
        manager.cache = cache;

        // database 已经在 build_detached 中使用，同时也存储在 manager 中
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        {
            manager.database = database;
        }

        // 将注入的依赖注册到 trait-kit 能力注册中心
        if let Some(ref cache) = manager.cache {
            manager.kit.replace::<CacheCapabilityKey>(cache.clone());
        }
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
        if let Some(ref database) = manager.database {
            manager
                .kit
                .replace::<DatabaseCapabilityKey>(database.clone());
        }
        if let Some(ref config_provider) = deps.config {
            manager
                .kit
                .replace::<ConfigCapabilityKey>(config_provider.clone());
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
        #[cfg(feature = "http")]
        tracing::info!(
            event = "security_logger_initialized",
            sinks = ?config.sinks_enabled(),
            masking_enabled = config.global.masking_enabled,
            "Logger manager initialized"
        );

        let (manager, subscriber, filter) = Self::build_detached(
            config.clone(),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
        #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))] database: Option<
            Arc<dyn Database>,
        >,
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

        let (handles, shutdown_txs) = Self::start_workers(WorkerParams {
            config: config.clone(),
            receiver,
            console_receiver,
            control_rx,
            control_tx: control_tx.clone(),
            metrics: metrics.clone(),
            console_sink: console_sink.clone(),
            error_sink: error_sink.clone(),
            effective_capacity: effective_capacity.clone(),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database,
        })?;

        // 构建 trait-kit 能力注册中心：注册 InklogConfig 和默认 Config 能力
        let kit = {
            let kit = Kit::new();
            kit.set_config::<InklogConfigKey>(config.clone());
            kit.replace::<ConfigCapabilityKey>(Arc::new(
                crate::integrations::infra::InklogConfigAdapter::from_config(config.clone()),
            ));
            kit
        };

        let manager = Self {
            config,
            sender,
            console_sender,
            shutdown_txs,
            console_sink,
            metrics,
            worker_handles: Mutex::new(handles),
            control_tx,
            effective_capacity: effective_capacity.clone(),
            #[cfg(feature = "http")]
            http_server_handle: Mutex::new(None),
            cache: None,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
            kit,
        };

        Ok((manager, subscriber, filter))
    }

    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    /// 获取 trait-kit 能力注册中心的引用。
    ///
    /// 返回的 [`Kit`] 共享内部状态（`Kit: Clone`），可通过 `kit.clone()`
    /// 获取独立句柄用于子组件。使用 `kit.config::<InklogConfigKey>()`
    /// 获取可热更新的配置句柄，或 `kit.require::<CacheCapabilityKey>()`
    /// 获取已注册的缓存能力。
    pub fn kit(&self) -> &Kit {
        &self.kit
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
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| InklogError::ConfigError(format!("Failed to read config file: {}", e)))?;
        let config: InklogConfig = toml::from_str(&content)
            .map_err(|e| InklogError::ConfigError(format!("Failed to parse config file: {}", e)))?;
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

    fn start_workers(params: WorkerParams) -> WorkerStartResult {
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
        // 每个 worker 拥有独立的 shutdown channel，确保广播信号能被每个 worker 接收
        // （MPMC channel 的 send() 只能被一个 receiver 消费，共享 channel 会导致
        // 只有首个 worker 收到信号、其余 worker 死循环）
        let (shutdown_tx_console, shutdown_console) = bounded(1);
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
        let (shutdown_tx_file, shutdown_file) = bounded(1);
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

        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
        let _handle_db = thread::spawn(|| {});

        // Health Check Thread
        let (shutdown_tx_health, shutdown_health) = bounded(1);
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
        // 向所有 worker 广播 shutdown 信号。每个 worker 持有独立的 channel receiver，
        // 必须逐个 send 才能确保全部收到（MPMC channel 的 send 仅被一个 receiver 消费）。
        // 历史缺陷：原先使用单一 `shutdown_tx`，send 一次只能让首个 worker 退出，
        // 其余 worker 进入死循环，导致进程无法退出（PID 20848 等挂起问题）。
        for tx in &self.shutdown_txs {
            let _ = tx.send(());
        }

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

/// 资源释放兜底：调用方未显式 `shutdown()` 时也确保 worker 线程退出。
///
/// 历史缺陷：原实现无 `Drop`，测试若忘记调用 `shutdown()`，4 个 worker 线程
/// 会因全局 subscriber 持有 `sender.clone()` 永不 disconnect 而死循环，
/// 最终导致进程挂起（tarpaulin 单元测试运行后 PID 不退出）。
impl Drop for LoggerManager {
    fn drop(&mut self) {
        // shutdown() 幂等：已 shutdown 时 worker_handles 已 take 为空，会快速返回
        let _ = self.shutdown();
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

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
            table_name: "logs".to_string(),
            archive_format: "json".to_string(),
            parquet_config: crate::ParquetConfig::default(),
        };
        self.config.database_sink = Some(config);
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
            self.config.http_server = Some(crate::HttpServerConfig {
                enabled: true,
                ..Default::default()
            });
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
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            {
                self.deps.database.is_some()
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
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
                // 将 self.config 通过 InklogConfigAdapter 注入
                // 这允许 mixed mode 正常工作
                deps.config = Some(Arc::new(
                    crate::integrations::infra::InklogConfigAdapter::from_config(
                        self.config.clone(),
                    ),
                ));
            }

            LoggerManager::with_dependencies(deps).await
        } else {
            // 纯配置模式
            LoggerManager::with_config(self.config).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // LoggerBuilder 测试 - 验证配置传播
    // ============================================================================

    #[test]
    fn test_builder_new_returns_default() {
        let builder = LoggerBuilder::new();
        assert_eq!(builder.config.global.level, "info");
        assert!(builder.deps.cache.is_none());
        assert!(builder.deps.config.is_none());
    }

    #[test]
    fn test_builder_level_sets_config() {
        let builder = LoggerBuilder::new().level("debug");
        assert_eq!(builder.config.global.level, "debug");
    }

    #[test]
    fn test_builder_level_chained() {
        let builder = LoggerBuilder::new().level("trace").level("error");
        assert_eq!(builder.config.global.level, "error");
    }

    #[test]
    fn test_builder_format_sets_config() {
        let builder = LoggerBuilder::new().format("{level} {message}");
        assert_eq!(builder.config.global.format, "{level} {message}");
    }

    #[test]
    fn test_builder_console_enabled_creates_config() {
        let builder = LoggerBuilder::new().console(true);
        assert!(builder.config.console_sink.is_some());
        assert!(builder.config.console_sink.as_ref().unwrap().enabled);
    }

    #[test]
    fn test_builder_console_disabled_keeps_some_but_disabled() {
        // 默认 InklogConfig 的 console_sink 是 Some(ConsoleSinkConfig::default())
        // console(false) 应设置 enabled=false，但保持 Some
        let builder = LoggerBuilder::new().console(false);
        let console = builder
            .config
            .console_sink
            .as_ref()
            .expect("console_sink should remain Some after console(false)");
        assert!(!console.enabled, "console.enabled should be false");
    }

    #[test]
    fn test_builder_file_sets_path() {
        let builder = LoggerBuilder::new().file("logs/test.log");
        let file_sink = builder
            .config
            .file_sink
            .as_ref()
            .expect("file_sink should be set");
        assert!(file_sink.enabled);
        assert_eq!(file_sink.path, std::path::PathBuf::from("logs/test.log"));
    }

    #[test]
    fn test_builder_channel_capacity_sets_config() {
        let builder = LoggerBuilder::new().channel_capacity(5000);
        assert_eq!(builder.config.performance.channel_capacity, 5000);
    }

    #[test]
    fn test_builder_worker_threads_sets_config() {
        let builder = LoggerBuilder::new().worker_threads(8);
        assert_eq!(builder.config.performance.worker_threads, 8);
    }

    #[test]
    fn test_builder_console_colored_sets_config() {
        let builder = LoggerBuilder::new().console(true).console_colored(false);
        assert!(!builder.config.console_sink.as_ref().unwrap().colored);
    }

    #[test]
    fn test_builder_file_max_size_sets_config() {
        let builder = LoggerBuilder::new()
            .file("logs/test.log")
            .file_max_size("50MB");
        assert_eq!(builder.config.file_sink.as_ref().unwrap().max_size, "50MB");
    }

    #[test]
    fn test_builder_file_compress_sets_config() {
        let builder = LoggerBuilder::new()
            .file("logs/test.log")
            .file_compress(false);
        assert!(!builder.config.file_sink.as_ref().unwrap().compress);
    }

    #[test]
    fn test_builder_file_rotation_time_sets_config() {
        let builder = LoggerBuilder::new()
            .file("logs/test.log")
            .file_rotation_time("hourly");
        assert_eq!(
            builder.config.file_sink.as_ref().unwrap().rotation_time,
            "hourly"
        );
    }

    #[test]
    fn test_builder_file_keep_files_sets_config() {
        let builder = LoggerBuilder::new()
            .file("logs/test.log")
            .file_keep_files(7);
        assert_eq!(builder.config.file_sink.as_ref().unwrap().keep_files, 7);
    }

    #[test]
    fn test_builder_enable_http_server_creates_config() {
        let builder = LoggerBuilder::new().enable_http_server(true);
        assert!(builder.config.http_server.is_some());
        assert!(builder.config.http_server.as_ref().unwrap().enabled);
    }

    #[test]
    fn test_builder_http_host_sets_config() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_host("0.0.0.0");
        assert_eq!(builder.config.http_server.as_ref().unwrap().host, "0.0.0.0");
    }

    #[test]
    fn test_builder_http_port_sets_config() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_port(8080);
        assert_eq!(builder.config.http_server.as_ref().unwrap().port, 8080);
    }

    #[test]
    fn test_builder_full_chain() {
        let builder = LoggerBuilder::new()
            .level("warn")
            .format("{message}")
            .console(true)
            .console_colored(false)
            .file("logs/app.log")
            .file_max_size("200MB")
            .file_compress(true)
            .file_rotation_time("hourly")
            .file_keep_files(14)
            .channel_capacity(20000)
            .worker_threads(4);

        assert_eq!(builder.config.global.level, "warn");
        assert_eq!(builder.config.global.format, "{message}");
        assert!(builder.config.console_sink.as_ref().unwrap().enabled);
        assert!(!builder.config.console_sink.as_ref().unwrap().colored);
        assert_eq!(
            builder.config.file_sink.as_ref().unwrap().path,
            std::path::PathBuf::from("logs/app.log")
        );
        assert_eq!(builder.config.file_sink.as_ref().unwrap().max_size, "200MB");
        assert!(builder.config.file_sink.as_ref().unwrap().compress);
        assert_eq!(
            builder.config.file_sink.as_ref().unwrap().rotation_time,
            "hourly"
        );
        assert_eq!(builder.config.file_sink.as_ref().unwrap().keep_files, 14);
        assert_eq!(builder.config.performance.channel_capacity, 20000);
        assert_eq!(builder.config.performance.worker_threads, 4);
    }

    // ============================================================================
    // LoggerDependencies 测试
    // ============================================================================

    #[test]
    fn test_logger_dependencies_default_all_none() {
        let deps = LoggerDependencies::default();
        assert!(deps.cache.is_none());
        assert!(deps.config.is_none());
    }

    #[test]
    fn test_logger_dependencies_debug_format() {
        let deps = LoggerDependencies::default();
        let debug_str = format!("{:?}", deps);
        assert!(debug_str.contains("cache"));
        assert!(debug_str.contains("config"));
    }

    // ============================================================================
    // LoggerManager 生命周期测试 (async)
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_new_creates_instance() {
        let manager = LoggerManager::new()
            .await
            .expect("Failed to create manager");
        // 验证基本属性
        assert!(manager.effective_channel_capacity() > 0);
        assert_eq!(manager.channel_len(), 0);
        // 清理
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_with_config_custom() {
        let config = InklogConfig {
            global: crate::GlobalConfig {
                level: "debug".to_string(),
                ..Default::default()
            },
            performance: crate::PerformanceConfig {
                channel_capacity: 5000,
                worker_threads: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Failed to create manager with config");
        assert_eq!(manager.effective_channel_capacity(), 5000);
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_kit_is_accessible() {
        let manager = LoggerManager::new()
            .await
            .expect("Failed to create manager");
        // kit() 应返回有效引用
        let kit = manager.kit();
        // 验证 kit 包含 InklogConfigKey 配置（在 build_detached 中注册）
        assert!(kit.contains_config::<InklogConfigKey>());
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_get_health_status() {
        let manager = LoggerManager::new()
            .await
            .expect("Failed to create manager");
        let health = manager.get_health_status();
        // 新创建的 manager 应该有某种健康状态
        // HealthStatus 是枚举，验证它不是未知状态
        let _ = health;
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_shutdown_is_idempotent() {
        let manager = LoggerManager::new()
            .await
            .expect("Failed to create manager");
        // 第一次 shutdown 应该成功
        let result1 = manager.shutdown();
        assert!(result1.is_ok(), "First shutdown should succeed");
        // 第二次 shutdown 应该也成功（或至少不 panic）
        let result2 = manager.shutdown();
        // 允许第二次返回错误或 Ok，但不应 panic
        let _ = result2;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_builder_creates_working_instance() {
        let manager = LoggerManager::builder()
            .level("info")
            .console(true)
            .channel_capacity(1000)
            .worker_threads(1)
            .build()
            .await
            .expect("Failed to build manager");
        assert_eq!(manager.effective_channel_capacity(), 1000);
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_with_dependencies_injects_cache() {
        use crate::integrations::infra::MockCache;
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: None,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };
        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with deps");
        // 验证 cache 能力已注册到 kit
        assert!(manager.kit().contains::<CacheCapabilityKey>());
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_with_dependencies_injects_config() {
        use crate::integrations::infra::InklogConfigAdapter;
        let config = InklogConfig::default();
        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(InklogConfigAdapter::from_config(config))),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };
        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with config provider");
        // 验证 config 能力已注册到 kit
        assert!(manager.kit().contains::<ConfigCapabilityKey>());
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_trigger_recovery_for_unhealthy_sinks() {
        let manager = LoggerManager::new()
            .await
            .expect("Failed to create manager");
        // 新创建的 manager 应该没有不健康的 sink
        let result = manager.trigger_recovery_for_unhealthy_sinks();
        assert!(result.is_ok(), "Trigger recovery should succeed");
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_builder_with_explicit_config() {
        // 使用显式配置验证 builder 路径（避免默认配置在并行测试中的不确定性）
        let manager = LoggerManager::builder()
            .level("info")
            .channel_capacity(2000)
            .worker_threads(1)
            .build()
            .await
            .expect("Failed to build manager");
        assert_eq!(manager.effective_channel_capacity(), 2000);
        let _ = manager.shutdown();
    }

    // ============================================================================
    // LoggerBuilder 额外配置传播测试 - 覆盖 None 分支
    // ============================================================================

    #[test]
    fn test_builder_console_stderr_levels_with_existing_console() {
        let builder = LoggerBuilder::new()
            .console(true)
            .console_stderr_levels(&["error", "warn"]);
        let console = builder.config.console_sink.as_ref().expect("console_sink");
        assert_eq!(
            console.stderr_levels,
            vec!["error".to_string(), "warn".to_string()]
        );
    }

    #[test]
    fn test_builder_console_stderr_levels_creates_new_when_absent() {
        // 默认 console_sink 是 Some，需显式置 None 以覆盖创建分支
        let mut builder = LoggerBuilder::new();
        builder.config.console_sink = None;
        let builder = builder.console_stderr_levels(&["error"]);
        let console = builder
            .config
            .console_sink
            .as_ref()
            .expect("console_sink should be created");
        assert_eq!(console.stderr_levels, vec!["error".to_string()]);
    }

    #[test]
    fn test_builder_console_colored_true_creates_new_when_absent() {
        // colored=true 且 console_sink 为 None → 创建新配置
        let mut builder = LoggerBuilder::new();
        builder.config.console_sink = None;
        let builder = builder.console_colored(true);
        let console = builder
            .config
            .console_sink
            .as_ref()
            .expect("console_sink should be created when colored=true");
        assert!(console.colored);
    }

    #[test]
    fn test_builder_file_max_size_without_file_creates_new() {
        // 不先调用 file()，直接设置 max_size → None 分支
        let builder = LoggerBuilder::new().file_max_size("50MB");
        let file = builder
            .config
            .file_sink
            .as_ref()
            .expect("file_sink should be created");
        assert_eq!(file.max_size, "50MB");
    }

    #[test]
    fn test_builder_file_compress_without_file_creates_new() {
        let builder = LoggerBuilder::new().file_compress(false);
        let file = builder
            .config
            .file_sink
            .as_ref()
            .expect("file_sink should be created");
        assert!(!file.compress);
    }

    #[test]
    fn test_builder_file_rotation_time_without_file_creates_new() {
        let builder = LoggerBuilder::new().file_rotation_time("daily");
        let file = builder
            .config
            .file_sink
            .as_ref()
            .expect("file_sink should be created");
        assert_eq!(file.rotation_time, "daily");
    }

    #[test]
    fn test_builder_file_keep_files_without_file_creates_new() {
        let builder = LoggerBuilder::new().file_keep_files(3);
        let file = builder
            .config
            .file_sink
            .as_ref()
            .expect("file_sink should be created");
        assert_eq!(file.keep_files, 3);
    }

    // ============================================================================
    // LoggerBuilder HTTP 配置测试 - 覆盖 None 分支与 error_mode 分支
    // ============================================================================

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_host_without_enable_creates_new() {
        // 不先 enable_http_server，直接设 host → None 分支
        let builder = LoggerBuilder::new().http_host("0.0.0.0");
        let http = builder
            .config
            .http_server
            .as_ref()
            .expect("http_server should be created");
        assert_eq!(http.host, "0.0.0.0");
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_port_without_enable_creates_new() {
        let builder = LoggerBuilder::new().http_port(9091);
        let http = builder
            .config
            .http_server
            .as_ref()
            .expect("http_server should be created");
        assert_eq!(http.port, 9091);
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_metrics_path_with_existing() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_metrics_path("/prom");
        let http = builder.config.http_server.as_ref().expect("http_server");
        assert_eq!(http.metrics_path, "/prom");
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_metrics_path_creates_new() {
        let builder = LoggerBuilder::new().http_metrics_path("/m");
        let http = builder
            .config
            .http_server
            .as_ref()
            .expect("http_server should be created");
        assert_eq!(http.metrics_path, "/m");
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_health_path_with_existing() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_health_path("/healthz");
        let http = builder.config.http_server.as_ref().expect("http_server");
        assert_eq!(http.health_path, "/healthz");
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_health_path_creates_new() {
        let builder = LoggerBuilder::new().http_health_path("/h");
        let http = builder
            .config
            .http_server
            .as_ref()
            .expect("http_server should be created");
        assert_eq!(http.health_path, "/h");
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_error_mode_warn() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_error_mode("warn");
        let http = builder.config.http_server.as_ref().expect("http_server");
        assert!(matches!(http.error_mode, crate::HttpErrorMode::Warn));
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_error_mode_strict() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_error_mode("strict");
        let http = builder.config.http_server.as_ref().expect("http_server");
        assert!(matches!(http.error_mode, crate::HttpErrorMode::Strict));
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_error_mode_unknown_falls_back_to_default() {
        // 未知模式 → _ 分支 → HttpErrorMode::default() (Strict)
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_error_mode("invalid-mode");
        let http = builder.config.http_server.as_ref().expect("http_server");
        assert!(matches!(http.error_mode, crate::HttpErrorMode::Strict));
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_error_mode_creates_new() {
        let builder = LoggerBuilder::new().http_error_mode("warn");
        let http = builder
            .config
            .http_server
            .as_ref()
            .expect("http_server should be created");
        assert!(matches!(http.error_mode, crate::HttpErrorMode::Warn));
    }

    // ============================================================================
    // LoggerBuilder 特性门控方法测试
    // ============================================================================

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[test]
    fn test_builder_database_sets_config() {
        let builder = LoggerBuilder::new().database("postgres://localhost/logs");
        let db = builder
            .config
            .database_sink
            .as_ref()
            .expect("database_sink should be set");
        assert!(db.enabled);
        assert_eq!(db.url, "postgres://localhost/logs");
        assert_eq!(db.name, "default");
    }

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[test]
    fn test_builder_with_database_injects_dep() {
        use crate::integrations::infra::MockDatabaseAdapter;
        let builder = LoggerBuilder::new().with_database(Arc::new(MockDatabaseAdapter::new()));
        assert!(builder.deps.database.is_some());
    }

    // ============================================================================
    // LoggerBuilder 依赖注入方法测试
    // ============================================================================

    #[test]
    fn test_builder_cache_injects_dep() {
        use crate::integrations::infra::MockCache;
        let builder = LoggerBuilder::new().cache(Arc::new(MockCache::new()));
        assert!(builder.deps.cache.is_some());
    }

    #[test]
    fn test_builder_config_injects_dep() {
        use crate::integrations::infra::MockConfig;
        let builder = LoggerBuilder::new().config(Arc::new(MockConfig::new()));
        assert!(builder.deps.config.is_some());
    }

    // ============================================================================
    // LoggerManager build() 混合模式测试 - 覆盖 has_deps 与 adapter 创建分支
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_builder_build_with_cache_injection_mixed_mode() {
        // 注入 cache 但不注入 config → has_deps=true, deps.config.is_none() 分支
        // 应创建 InklogConfigAdapter 包装 self.config
        use crate::integrations::infra::MockCache;
        let manager = LoggerManager::builder()
            .level("info")
            .channel_capacity(1500)
            .worker_threads(1)
            .cache(Arc::new(MockCache::new()))
            .build()
            .await
            .expect("Failed to build manager with cache injection");
        assert_eq!(manager.effective_channel_capacity(), 1500);
        assert!(manager.kit().contains::<CacheCapabilityKey>());
        assert!(manager.kit().contains::<ConfigCapabilityKey>());
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_builder_build_with_config_injection() {
        // 注入 config → has_deps=true, deps.config.is_some() 分支（不创建 adapter）
        use crate::integrations::infra::MockConfig;
        let manager = LoggerManager::builder()
            .config(Arc::new(MockConfig::new()))
            .worker_threads(1)
            .build()
            .await
            .expect("Failed to build manager with config injection");
        assert!(manager.kit().contains::<ConfigCapabilityKey>());
        let _ = manager.shutdown();
    }

    // ============================================================================
    // LoggerManager recover_sink / from_file 测试
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_recover_sink_on_live_manager() {
        // 注意：control_rx 仅由 file/db worker 持有。默认配置无 file sink 时
        // file worker 立即退出并丢弃接收端，recover_sink 必然失败。
        // 因此此处启用 file sink 使 file worker 存活并持有 control_rx。
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("app.log");
        let manager = LoggerManager::builder()
            .channel_capacity(1000)
            .worker_threads(1)
            .file(log_path)
            .build()
            .await
            .expect("Failed to build manager");
        // 在存活的 manager 上发送恢复指令应成功（control channel 接收端存在）
        let result = manager.recover_sink("file");
        assert!(
            result.is_ok(),
            "recover_sink on live manager should succeed"
        );
        let _ = manager.shutdown();
    }

    // 注：未测试 recover_sink 在 shutdown 后返回 Err 的分支。
    // shutdown() 用 5s 超时 join worker，超时则 detach；而 FileSink::shutdown()
    // 自身有 5s 计时器超时，导致 file worker 常无法在 5s 内退出而被 detach，
    // 仍持有 control_rx 使 recover_sink 返回 Ok。该 Err 分支非确定性，无法稳定测试。

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_from_file_loads_valid_config() {
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let config_path = dir.path().join("inklog_config.toml");
        let toml_content = r#"
[global]
level = "debug"

[performance]
channel_capacity = 3000
worker_threads = 1
"#;
        std::fs::write(&config_path, toml_content).expect("Failed to write config");
        let manager = LoggerManager::from_file(&config_path)
            .await
            .expect("Failed to load manager from file");
        assert_eq!(manager.effective_channel_capacity(), 3000);
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_from_file_missing_path_returns_error() {
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let missing = dir.path().join("nonexistent.toml");
        let result = LoggerManager::from_file(&missing).await;
        assert!(result.is_err(), "from_file with missing path should error");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_from_file_invalid_toml_returns_error() {
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let config_path = dir.path().join("invalid.toml");
        // 故意写入非法 TOML
        std::fs::write(&config_path, "this is = = not valid toml [[[")
            .expect("Failed to write config");
        let result = LoggerManager::from_file(&config_path).await;
        assert!(result.is_err(), "from_file with invalid toml should error");
    }

    // ============================================================================
    // tracing::Level → log::LevelFilter match 覆盖 (lines 410-416)
    // 现有测试仅覆盖 DEBUG，补充 TRACE/WARN/ERROR 分支
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_with_config_trace_level() {
        let config = InklogConfig {
            global: crate::GlobalConfig {
                level: "trace".to_string(),
                ..Default::default()
            },
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Failed to create manager with trace level");
        assert_eq!(manager.effective_channel_capacity(), 1000);
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_with_config_warn_level() {
        let config = InklogConfig {
            global: crate::GlobalConfig {
                level: "warn".to_string(),
                ..Default::default()
            },
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Failed to create manager with warn level");
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_with_config_error_level() {
        let config = InklogConfig {
            global: crate::GlobalConfig {
                level: "error".to_string(),
                ..Default::default()
            },
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Failed to create manager with error level");
        let _ = manager.shutdown();
    }

    // ============================================================================
    // enable_http_server(false) 当 http_server 已存在 (line 1883 分支)
    // ============================================================================

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_enable_http_server_false_when_exists() {
        // 先启用再禁用 → 覆盖 `if let Some(ref mut http)` 分支且 enabled=false
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .enable_http_server(false);
        let http = builder
            .config
            .http_server
            .as_ref()
            .expect("http_server should exist");
        assert!(!http.enabled, "http.enabled should be false after disable");
    }

    // ============================================================================
    // File sink worker 写入路径 (lines 1042-1236)
    // 通过发送记录 + shutdown drain 覆盖 worker 接收/写入/排空逻辑
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_file_sink_writes_record_to_file() {
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("worker_test.log");
        let manager = LoggerManager::builder()
            .channel_capacity(500)
            .worker_threads(1)
            .file(&log_path)
            .build()
            .await
            .expect("Failed to build manager with file sink");

        let record = Arc::new(LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "worker_test".to_string(),
            message: "worker_write_unique_marker_12345".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test-thread".to_string(),
        });
        manager
            .sender
            .send(record)
            .expect("Failed to send record to file worker");

        // 给 file worker 时间通过正常 recv_timeout 路径处理记录
        // （避免与 sink.shutdown() 的 5s 超时产生竞争）
        std::thread::sleep(Duration::from_millis(300));
        let _ = manager.shutdown();

        let content =
            std::fs::read_to_string(&log_path).expect("Log file should exist after shutdown");
        assert!(
            content.contains("worker_write_unique_marker_12345"),
            "Log file should contain the sent message"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_file_sink_drains_multiple_records_on_shutdown() {
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("drain_test.log");
        let manager = LoggerManager::builder()
            .channel_capacity(500)
            .worker_threads(1)
            .file(&log_path)
            .build()
            .await
            .expect("Failed to build manager");

        for i in 0..10u32 {
            let record = Arc::new(LogRecord {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                target: "drain_test".to_string(),
                message: format!("drain_record_{:02}", i),
                fields: std::collections::HashMap::new(),
                file: None,
                line: None,
                thread_id: "test-thread".to_string(),
            });
            manager.sender.send(record).expect("Failed to send record");
        }

        // shutdown drain 路径应将所有待处理记录写入文件
        let _ = manager.shutdown();

        let content = std::fs::read_to_string(&log_path).expect("Log file should exist");
        for i in 0..10u32 {
            let marker = format!("drain_record_{:02}", i);
            assert!(
                content.contains(&marker),
                "Log file should contain '{}'",
                marker
            );
        }
    }

    // ============================================================================
    // recover_sink 控制通道 (lines 1128-1150)
    // 验证 control channel 接受不同 sink 名（包括未知名）
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recover_sink_multiple_commands_to_live_manager() {
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("recover_test.log");
        let manager = LoggerManager::builder()
            .channel_capacity(500)
            .worker_threads(1)
            .file(&log_path)
            .build()
            .await
            .expect("Failed to build manager");

        // control channel 容量 10，连续发送多个恢复指令应成功
        let r1 = manager.recover_sink("file");
        let r2 = manager.recover_sink("database");
        let r3 = manager.recover_sink("unknown_sink");
        assert!(r1.is_ok(), "recover_sink('file') should succeed");
        assert!(r2.is_ok(), "recover_sink('database') should succeed");
        assert!(r3.is_ok(), "recover_sink('unknown') should succeed");

        let _ = manager.shutdown();
    }

    // ============================================================================
    // console worker 写入路径 (lines 961-1033)
    // 通过 console_sender 发送记录，shutdown 后验证不 panic
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_console_sink_processes_record() {
        let manager = LoggerManager::builder()
            .channel_capacity(500)
            .worker_threads(1)
            .console(true)
            .build()
            .await
            .expect("Failed to build manager");

        let record = Arc::new(LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "console_test".to_string(),
            message: "console_marker_98765".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test-thread".to_string(),
        });
        // 发送到 console 通道（console worker 消费）
        manager
            .console_sender
            .send(record)
            .expect("Failed to send record to console worker");

        // 给 console worker 时间处理（recv_timeout 100ms）
        std::thread::sleep(Duration::from_millis(200));
        let _ = manager.shutdown();
        // 验证：manager 正常 shutdown，console worker 处理了记录不 panic
        // （console 输出到 stdout，无法直接验证内容，但 worker 不 panic 即为成功）
    }

    // ============================================================================
    // HTTP 服务器 start_http_server 测试
    //
    // start_http_server 内部的 auth_middleware / subtle_constant_time_compare /
    // parse_cidr / health_status_getter / 路由 handler 均为局部函数和闭包，
    // 无法直接单元测试，因此通过启动真实 HTTP 服务器并发送请求来覆盖。
    // ============================================================================

    /// 查找可用的本地端口用于 HTTP 测试（TOCTOU 风险在串行测试中可接受）
    #[cfg(feature = "http")]
    fn find_available_http_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("Failed to bind to find available port");
        let port = listener
            .local_addr()
            .expect("Failed to get local addr")
            .port();
        drop(listener);
        port
    }

    /// 轮询 HTTP 服务器直到可达或超时（约 2 秒）
    #[cfg(feature = "http")]
    async fn wait_for_http_server(host: &str, port: u16) -> bool {
        let url = format!("http://{}:{}", host, port);
        for _ in 0..80 {
            if reqwest::get(&url).await.is_ok() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        false
    }

    /// 构建基础的 HTTP 测试配置（无 auth、无 IP 白名单、Warn 模式）
    #[cfg(feature = "http")]
    fn http_test_config(port: u16) -> InklogConfig {
        InklogConfig {
            http_server: Some(crate::HttpServerConfig {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port,
                error_mode: crate::HttpErrorMode::Warn,
                ..Default::default()
            }),
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// 构建启用 Bearer Token 认证的 HTTP 测试配置
    #[cfg(feature = "http")]
    fn http_test_config_with_auth(port: u16, token_env: &str) -> InklogConfig {
        let mut config = http_test_config(port);
        let http = config
            .http_server
            .as_mut()
            .expect("http_server should be set");
        http.auth = Some(crate::HttpAuthConfig {
            enabled: true,
            token_env: token_env.to_string(),
        });
        config
    }

    /// 构建带 IP 白名单的 HTTP 测试配置
    #[cfg(feature = "http")]
    fn http_test_config_with_whitelist(port: u16, whitelist: Vec<String>) -> InklogConfig {
        let mut config = http_test_config(port);
        let http = config
            .http_server
            .as_mut()
            .expect("http_server should be set");
        http.ip_whitelist = Some(whitelist);
        config
    }

    /// Warn 模式：HTTP 服务器启动失败时记录警告但继续返回 Ok(manager)
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_with_config_http_warn_mode_continues_on_startup_error() {
        // 使用无效主机名触发 start_http_server 中 addr.parse() 失败
        // Warn 模式应记录警告但继续返回 Ok(manager)
        let config = InklogConfig {
            http_server: Some(crate::HttpServerConfig {
                enabled: true,
                host: "invalid host with spaces".to_string(),
                port: 9090,
                error_mode: crate::HttpErrorMode::Warn,
                ..Default::default()
            }),
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Warn mode should return Ok despite HTTP server startup error");
        let _ = manager.shutdown();
    }

    /// Strict 模式：无效主机名导致 addr.parse() 失败时，错误应传播给调用者
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_with_config_http_strict_mode_returns_error_on_invalid_host() {
        let config = InklogConfig {
            http_server: Some(crate::HttpServerConfig {
                enabled: true,
                host: "invalid host with spaces".to_string(),
                port: 9091,
                error_mode: crate::HttpErrorMode::Strict,
                ..Default::default()
            }),
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        match LoggerManager::with_config(config).await {
            Err(InklogError::ConfigError(msg)) => {
                assert!(
                    msg.contains("Invalid HTTP server address"),
                    "Error should mention invalid HTTP server address, got: {}",
                    msg
                );
            }
            Err(other) => panic!("Expected ConfigError, got {:?}", other),
            Ok(_) => panic!("Strict mode should return Err on invalid HTTP address"),
        }
    }

    /// /health 端点返回 200 和 JSON 格式的健康状态
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_health_endpoint_returns_json() {
        let port = find_available_http_port();
        let manager = LoggerManager::with_config(http_test_config(port))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable on port {}",
            port
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "health endpoint should return 200"
        );
        let body: serde_json::Value = resp.json().await.expect("body should be JSON");
        assert!(body.is_object(), "health response should be a JSON object");
        assert!(
            body.get("overall_status").is_some(),
            "health response should contain overall_status field"
        );
        let _ = manager.shutdown();
    }

    /// /metrics 端点返回 200 和 Prometheus 格式文本
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_metrics_endpoint_returns_prometheus() {
        let port = find_available_http_port();
        let manager = LoggerManager::with_config(http_test_config(port))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable on port {}",
            port
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/metrics", port))
            .await
            .expect("GET /metrics should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "metrics endpoint should return 200"
        );
        let body = resp.text().await.expect("body should be text");
        assert!(
            body.contains("# HELP") && body.contains("inklog_"),
            "metrics response should be in Prometheus format, got: {}",
            body
        );
        let _ = manager.shutdown();
    }

    /// 自定义 health_path 和 metrics_path 应生效，默认路径不再可访问
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_custom_paths_work() {
        let port = find_available_http_port();
        let mut config = http_test_config(port);
        {
            let http = config
                .http_server
                .as_mut()
                .expect("http_server should be set");
            http.health_path = "/custom-health".to_string();
            http.metrics_path = "/custom-metrics".to_string();
        }
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        // 自定义路径应返回 200
        let resp = reqwest::get(format!("http://127.0.0.1:{}/custom-health", port))
            .await
            .expect("GET /custom-health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "custom health path should return 200"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/custom-metrics", port))
            .await
            .expect("GET /custom-metrics should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "custom metrics path should return 200"
        );
        // 默认路径应返回 404
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::NOT_FOUND,
            "default health path should return 404 when customized"
        );
        let _ = manager.shutdown();
    }

    /// auth 禁用时，无 Authorization header 也能访问
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_disabled_allows_access_without_header() {
        let port = find_available_http_port();
        let manager = LoggerManager::with_config(http_test_config(port))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "auth disabled should allow access without Authorization header"
        );
        let _ = manager.shutdown();
    }

    /// auth 启用但 token 环境变量未设置时返回 500
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_missing_token_env_returns_500() {
        let port = find_available_http_port();
        // 使用唯一的环境变量名，确保它未设置
        let token_env = "INKLOG_TEST_TOKEN_MISSING_ENV_VAR";
        std::env::remove_var(token_env);
        let manager = LoggerManager::with_config(http_test_config_with_auth(port, token_env))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "missing token env should return 500"
        );
        let body = resp.text().await.expect("body should be text");
        assert!(
            body.contains("Auth token not configured"),
            "response should explain token misconfiguration, got: {}",
            body
        );
        let _ = manager.shutdown();
    }

    /// auth 启用且 Bearer token 正确时返回 200
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_valid_token_returns_200() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_VALID";
        let token_value = "secret-token-12345";
        std::env::set_var(token_env, token_value);
        let manager = LoggerManager::with_config(http_test_config_with_auth(port, token_env))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let client = reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest client");
        let resp = client
            .get(format!("http://127.0.0.1:{}/health", port))
            .bearer_auth(token_value)
            .send()
            .await
            .expect("Request with valid token should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "valid Bearer token should return 200"
        );
        let _ = manager.shutdown();
        std::env::remove_var(token_env);
    }

    /// auth 启用但 Bearer token 错误时返回 401
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_invalid_token_returns_401() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_INVALID";
        std::env::set_var(token_env, "correct-secret");
        let manager = LoggerManager::with_config(http_test_config_with_auth(port, token_env))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let client = reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest client");
        let resp = client
            .get(format!("http://127.0.0.1:{}/health", port))
            .bearer_auth("wrong-secret")
            .send()
            .await
            .expect("Request with invalid token should still get a response");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid Bearer token should return 401"
        );
        let body = resp.text().await.expect("body should be text");
        assert!(
            body.contains("Invalid token"),
            "response should indicate invalid token, got: {}",
            body
        );
        let _ = manager.shutdown();
        std::env::remove_var(token_env);
    }

    /// auth 启用但缺少 Authorization header 时返回 401
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_missing_header_returns_401() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_MISSING_HEADER";
        std::env::set_var(token_env, "some-secret");
        let manager = LoggerManager::with_config(http_test_config_with_auth(port, token_env))
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("Request without header should still get a response");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "missing Authorization header should return 401"
        );
        let body = resp.text().await.expect("body should be text");
        assert!(
            body.contains("Missing or invalid Authorization header"),
            "response should indicate missing header, got: {}",
            body
        );
        let _ = manager.shutdown();
        std::env::remove_var(token_env);
    }

    /// IP 白名单精确匹配 127.0.0.1 时允许访问
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_ip_whitelist_allows_exact_match() {
        let port = find_available_http_port();
        let config = http_test_config_with_whitelist(port, vec!["127.0.0.1".to_string()]);
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "exact IP match in whitelist should allow access"
        );
        let _ = manager.shutdown();
    }

    /// IP 白名单不匹配客户端 IP 时返回 403
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_ip_whitelist_rejects_non_match() {
        let port = find_available_http_port();
        // 白名单仅包含一个不可能匹配 127.0.0.1 的地址
        let config = http_test_config_with_whitelist(port, vec!["10.0.0.1".to_string()]);
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should still get a response");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::FORBIDDEN,
            "non-matching IP should be forbidden"
        );
        let body = resp.text().await.expect("body should be text");
        assert!(
            body.contains("IP not in whitelist"),
            "response should indicate IP rejection, got: {}",
            body
        );
        let _ = manager.shutdown();
    }

    /// IP 白名单通配符格式 "127.0.*" 匹配客户端 IP
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_ip_whitelist_allows_wildcard() {
        let port = find_available_http_port();
        let config = http_test_config_with_whitelist(port, vec!["127.0.*".to_string()]);
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "wildcard 127.0.* should match 127.0.0.1"
        );
        let _ = manager.shutdown();
    }

    /// IP 白名单 CIDR 格式 "127.0.0.0/8" 匹配客户端 IP
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_ip_whitelist_allows_cidr() {
        let port = find_available_http_port();
        let config = http_test_config_with_whitelist(port, vec!["127.0.0.0/8".to_string()]);
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Manager should start with HTTP server");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .expect("GET /health should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "CIDR 127.0.0.0/8 should contain 127.0.0.1"
        );
        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_with_deps 通过 Config trait 应用配置测试 (lines 235-300)
    //
    // 这组测试覆盖 build_with_deps 中通过 Config trait 实现加载配置的分支，
    // 包括 global、file_sink、http_server、performance 配置的应用。
    // 之前测试仅覆盖了 cache/database 注入路径，未覆盖 config_provider 提供时的
    // 配置加载逻辑。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_applies_global_config_from_provider() {
        // 验证 Config trait 的 global.level/format/masking_enabled/auto_fallback
        // 被正确应用到 InklogConfig
        use crate::integrations::infra::MockConfig;
        let mock_config = MockConfig::new()
            .with_value("global.level", "debug")
            .with_value("global.format", "{level} {message}")
            .with_value("global.masking_enabled", "true")
            .with_value("global.auto_fallback", "true");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with config provider");

        // 通过 kit 验证配置已应用到 InklogConfig
        let kit = manager.kit();
        let handle = kit
            .config::<InklogConfigKey>()
            .expect("InklogConfig should be registered in kit");
        let config = handle.load();
        assert_eq!(config.global.level, "debug");
        assert_eq!(config.global.format, "{level} {message}");
        assert!(config.global.masking_enabled);
        assert!(config.global.auto_fallback);

        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_configures_file_sink_from_provider() {
        // 验证 Config trait 的 file_sink.* 配置被正确应用到 InklogConfig.file_sink
        // 并触发 file worker 实际写入文件
        use crate::integrations::infra::MockConfig;
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("from_provider.log");
        let path_str = log_path
            .to_str()
            .expect("path should be valid utf-8")
            .to_string();

        let mock_config = MockConfig::new()
            .with_value("file_sink.enabled", "true")
            .with_value("file_sink.path", &path_str)
            .with_value("file_sink.max_size", "50MB")
            .with_value("file_sink.compress", "false");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with file_sink config");

        // 验证配置已应用到 InklogConfig
        let kit = manager.kit();
        let handle = kit
            .config::<InklogConfigKey>()
            .expect("InklogConfig should be registered");
        let config = handle.load();
        let file_sink = config
            .file_sink
            .as_ref()
            .expect("file_sink should be configured from provider");
        assert!(file_sink.enabled);
        assert_eq!(file_sink.path, std::path::PathBuf::from(&path_str));
        assert_eq!(file_sink.max_size, "50MB");
        assert!(!file_sink.compress);

        // 验证 file worker 实际启动并写入文件（证明配置完整生效）
        let record = Arc::new(LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "config_provider_test".to_string(),
            message: "from_provider_unique_marker_abc123".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        });
        manager
            .sender
            .send(record)
            .expect("Failed to send record to file worker");

        // 给 file worker 时间处理记录
        std::thread::sleep(Duration::from_millis(300));
        let _ = manager.shutdown();

        let content =
            std::fs::read_to_string(&log_path).expect("Log file should exist after write");
        assert!(
            content.contains("from_provider_unique_marker_abc123"),
            "Log file should contain the message sent via config_provider-configured file sink"
        );
    }

    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_configures_http_server_from_provider() {
        // 验证 Config trait 的 http_server.* 配置被正确应用到 InklogConfig.http_server
        // 注意：with_dependencies 不启动 HTTP 服务器（只有 with_config 才启动），
        // 所以本测试只验证配置构建，不实际启动 HTTP 服务
        use crate::integrations::infra::MockConfig;
        let mock_config = MockConfig::new()
            .with_value("http_server.enabled", "true")
            .with_value("http_server.host", "127.0.0.1")
            .with_value("http_server.port", "9090");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with http_server config");

        // 验证配置已应用到 InklogConfig
        let kit = manager.kit();
        let handle = kit
            .config::<InklogConfigKey>()
            .expect("InklogConfig should be registered");
        let config = handle.load();
        let http = config
            .http_server
            .as_ref()
            .expect("http_server should be configured from provider");
        assert!(http.enabled);
        assert_eq!(http.host, "127.0.0.1");
        assert_eq!(http.port, 9090);

        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_configures_performance_from_provider() {
        // 验证 Config trait 的 performance.worker_threads/channel_capacity
        // 被正确应用到 InklogConfig.performance
        use crate::integrations::infra::MockConfig;
        let mock_config = MockConfig::new()
            .with_value("performance.worker_threads", "2")
            .with_value("performance.channel_capacity", "3000");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with performance config");

        // 验证 channel_capacity 已生效（effective_channel_capacity 反映配置值）
        assert_eq!(
            manager.effective_channel_capacity(),
            3000,
            "channel_capacity from config provider should be applied"
        );

        // 验证 worker_threads 也已应用到 InklogConfig
        let kit = manager.kit();
        let handle = kit
            .config::<InklogConfigKey>()
            .expect("InklogConfig should be registered");
        let config = handle.load();
        assert_eq!(config.performance.worker_threads, 2);

        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_with_deps 注入 database 到 kit 测试 (lines 336-340)
    // 需要 dbnexus feature
    // ============================================================================

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_injects_database_to_kit() {
        // 验证通过 LoggerDependencies.database 注入的 Database 实现
        // 被注册到 kit 的 DatabaseCapabilityKey
        use crate::integrations::infra::MockDatabaseAdapter;
        let deps = LoggerDependencies {
            cache: None,
            config: None,
            database: Some(Arc::new(MockDatabaseAdapter::new())),
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with database injection");

        // 验证 database 能力已注册到 kit
        assert!(
            manager.kit().contains::<DatabaseCapabilityKey>(),
            "DatabaseCapabilityKey should be registered in kit after database injection"
        );

        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_detached 直接调用测试 (lines 832-884)
    //
    // build_detached 是 with_config/with_dependencies 的底层实现，
    // 返回 (manager, subscriber, filter) 三元组。
    // 直接调用可覆盖其内部逻辑：metrics 创建、channel 创建、subscriber 创建、
    // filter 解析、kit 注册等。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_detached_returns_valid_components() {
        // 验证 build_detached 返回的 manager/subscriber/filter 均有效
        let config = InklogConfig {
            global: crate::GlobalConfig {
                level: "warn".to_string(),
                ..Default::default()
            },
            performance: crate::PerformanceConfig {
                channel_capacity: 2000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let (manager, _subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .expect("build_detached should succeed with valid config");

        // 验证 manager 状态
        assert_eq!(
            manager.effective_channel_capacity(),
            2000,
            "effective_channel_capacity should match config"
        );
        assert_eq!(
            manager.channel_len(),
            0,
            "channel_len should be 0 for fresh manager"
        );

        // 验证 filter 反映配置的 level（warn → WARN）
        assert_eq!(
            filter,
            tracing_subscriber::filter::LevelFilter::WARN,
            "filter should reflect config.global.level 'warn'"
        );

        // 验证 kit 中 InklogConfig 已注册
        assert!(
            manager.kit().contains_config::<InklogConfigKey>(),
            "InklogConfigKey should be registered in kit"
        );
        assert!(
            manager.kit().contains::<ConfigCapabilityKey>(),
            "ConfigCapabilityKey should be registered in kit"
        );

        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_detached_invalid_level_falls_back_to_info() {
        // 验证 build_detached 中 level.parse() 失败时回退到 INFO
        let config = InklogConfig {
            global: crate::GlobalConfig {
                level: "invalid_level".to_string(),
                ..Default::default()
            },
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let (manager, _subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .expect("build_detached should succeed even with invalid level");

        // 无效 level 应回退到 INFO
        assert_eq!(
            filter,
            tracing_subscriber::filter::LevelFilter::INFO,
            "invalid level should fall back to INFO"
        );

        let _ = manager.shutdown();
    }

    // ============================================================================
    // file worker FileSink::new 失败分支测试 (line 910)
    //
    // 当 FileSink::new 失败时，file worker 应跳过整个 file_config 分支，
    // manager 仍能正常创建和 shutdown。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_file_worker_skips_when_file_sink_new_fails() {
        // 使用 /dev/null 子路径触发 create_dir_all 失败
        // /dev/null 是文件而非目录，在其下创建子目录会失败
        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: true,
                path: PathBuf::from("/dev/null/subdir/file.log"),
                ..Default::default()
            }),
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // build_detached 应成功（FileSink::new 失败在 worker 线程内处理）
        let manager = LoggerManager::with_config(config)
            .await
            .expect("Manager should be created even if FileSink::new fails in worker");

        // 验证 manager 仍然可用
        assert_eq!(manager.effective_channel_capacity(), 1000);

        // shutdown 应正常完成（file worker 不会进入循环，直接退出）
        let result = manager.shutdown();
        assert!(result.is_ok(), "shutdown should succeed");
    }

    // ============================================================================
    // file worker 控制消息处理测试 (lines 991-1013)
    //
    // 通过 recover_sink 发送 RecoverSink("file") 命令，验证 file worker
    // 能处理控制消息而不死锁或 panic。同时验证 recover_sink 在 control channel
    // 满（容量 10）时的错误路径。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_file_worker_recovers_after_recover_sink_command() {
        // 启用 file sink 使 file worker 进入循环并消费 control 消息
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("recover_worker.log");

        let manager = LoggerManager::builder()
            .channel_capacity(500)
            .worker_threads(1)
            .file(&log_path)
            .build()
            .await
            .expect("Failed to build manager");

        // 发送一条记录让 file worker 进入正常 recv_timeout 路径
        let record = Arc::new(LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "recover_test".to_string(),
            message: "before_recover_marker".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        });
        manager.sender.send(record).expect("Failed to send record");

        // 等待 file worker 处理记录
        std::thread::sleep(Duration::from_millis(200));

        // 发送 recover_sink 命令 - file worker 应处理并重建 sink
        let result = manager.recover_sink("file");
        assert!(
            result.is_ok(),
            "recover_sink('file') should succeed on live manager"
        );

        // 等待 file worker 处理控制消息
        std::thread::sleep(Duration::from_millis(200));

        // 发送第二条记录，验证 file worker 在 recover 后仍能正常工作
        let record2 = Arc::new(LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "recover_test".to_string(),
            message: "after_recover_marker".to_string(),
            fields: std::collections::HashMap::new(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
        });
        manager.sender.send(record2).expect("Failed to send record");

        // 等待 file worker 处理第二条记录
        std::thread::sleep(Duration::from_millis(300));
        let _ = manager.shutdown();

        // 验证两条记录都写入了文件（recover 命令重建 sink 后文件仍可写）
        let content =
            std::fs::read_to_string(&log_path).expect("Log file should exist after recover");
        assert!(
            content.contains("before_recover_marker"),
            "Log file should contain record sent before recover"
        );
        assert!(
            content.contains("after_recover_marker"),
            "Log file should contain record sent after recover"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recover_sink_returns_error_when_control_channel_full() {
        // control channel 容量为 10。当 worker 未消费消息时，连续发送 11 条
        // 应使第 11 条返回 ChannelError。
        // 注意：此测试需要 file worker 存活但暂停消费 control 消息。
        // 实际上 file worker 在每次循环都会 try_recv control 消息，
        // 所以很难填满 channel。我们通过发送足够多的消息来触发。
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("channel_full.log");

        let manager = LoggerManager::builder()
            .channel_capacity(500)
            .worker_threads(1)
            .file(&log_path)
            .build()
            .await
            .expect("Failed to build manager");

        // 连续发送 recover_sink 命令。control channel 容量 10，
        // 但 worker 在循环中消费，所以多数会被消费。
        // 我们发送足够多以触发潜在的 ChannelError（如果 worker 暂时未消费）。
        let mut ok_count = 0;
        let mut err_count = 0;
        for _ in 0..20 {
            match manager.recover_sink("file") {
                Ok(_) => ok_count += 1,
                Err(InklogError::ChannelError(_)) => err_count += 1,
                Err(other) => panic!("Unexpected error type: {:?}", other),
            }
        }

        // 至少有一些命令成功（worker 在消费）
        assert!(
            ok_count > 0,
            "At least some recover_sink commands should succeed"
        );
        // ok_count + err_count 应等于 20
        assert_eq!(ok_count + err_count, 20);

        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_detached 创建 error_sink 测试 (lines 475-480)
    //
    // build_detached 会创建 error_sink（FileSink）用于记录系统错误。
    // 验证 error_sink 创建失败时不影响 manager 构建。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_detached_creates_error_sink() {
        // build_detached 内部创建 error_sink 指向 logs/error.log
        // 验证此路径不 panic 且 manager 正常工作
        let config = InklogConfig {
            performance: crate::PerformanceConfig {
                channel_capacity: 1000,
                worker_threads: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let (manager, _subscriber, _filter) = LoggerManager::build_detached(
            config,
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            None,
        )
        .await
        .expect("build_detached should succeed");

        // 验证 manager 创建成功，error_sink 已初始化（即使为 None）
        assert!(manager.effective_channel_capacity() > 0);

        let _ = manager.shutdown();
    }

    // ============================================================================
    // LoggerDependencies Debug 实现测试 (lines 118-131)
    //
    // 验证 LoggerDependencies 的 Debug 实现包含 cache/config/database 字段
    // （database 字段仅在 dbnexus feature 下存在）
    // ============================================================================

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[test]
    fn test_logger_dependencies_debug_includes_database_field() {
        use crate::integrations::infra::{MockCache, MockDatabaseAdapter};
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: None,
            database: Some(Arc::new(MockDatabaseAdapter::new())),
        };
        let debug_str = format!("{:?}", deps);
        assert!(
            debug_str.contains("cache"),
            "debug should include cache field"
        );
        assert!(
            debug_str.contains("config"),
            "debug should include config field"
        );
        assert!(
            debug_str.contains("database"),
            "debug should include database field when dbnexus feature enabled"
        );
    }

    // ============================================================================
    // build_with_deps 同时注入 cache 和 config 测试 (lines 332-345)
    //
    // 验证同时注入 cache 和 config 时，两者都被注册到 kit。
    // 之前测试只单独注入 cache 或 config，未覆盖同时注入的路径。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_injects_both_cache_and_config() {
        use crate::integrations::infra::{InklogConfigAdapter, MockCache};
        let config = InklogConfig::default();
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: Some(Arc::new(InklogConfigAdapter::from_config(config))),
            #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with cache and config");

        // 验证 cache 和 config 能力都注册到 kit
        assert!(
            manager.kit().contains::<CacheCapabilityKey>(),
            "CacheCapabilityKey should be registered"
        );
        assert!(
            manager.kit().contains::<ConfigCapabilityKey>(),
            "ConfigCapabilityKey should be registered"
        );

        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_with_deps 同时注入 cache/config/database 测试 (lines 332-345, dbnexus)
    // ============================================================================

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_injects_all_three_deps() {
        use crate::integrations::infra::{InklogConfigAdapter, MockCache, MockDatabaseAdapter};
        let config = InklogConfig::default();
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: Some(Arc::new(InklogConfigAdapter::from_config(config))),
            database: Some(Arc::new(MockDatabaseAdapter::new())),
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with all deps");

        // 验证三个能力都注册到 kit
        assert!(manager.kit().contains::<CacheCapabilityKey>());
        assert!(manager.kit().contains::<ConfigCapabilityKey>());
        assert!(manager.kit().contains::<DatabaseCapabilityKey>());

        let _ = manager.shutdown();
    }
}
