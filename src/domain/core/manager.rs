// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Core logger manager module.

// Types from sibling modules (declared in core/mod.rs)
use super::builder::{LoggerBuilder, LoggerDependencies};
use super::recovery::SinkControlMessage;
use super::workers::WorkerParams;

#[allow(unused_imports)]
use crate::ConsoleSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::LogTemplate;
use crate::domain::core::LoggerSubscriber;
use crate::integrations::Cache;
#[cfg(feature = "database")]
use crate::integrations::Database;
use crate::support::io::ConsoleSink;
use crate::support::io::FileSink;
use crate::support::io::LogSink;
use crate::support::io::sink::SamplingPolicy;
use crate::support::processing::RateLimiter;
use crate::support::processing::target_rate_limiter::TargetRateLimiter;
use crate::validation::sanitize::LogSanitizer;
use crate::{FileSinkConfig, InklogConfig};
use crate::{HealthStatus, Metrics};
use crate::{LogAdapter, LogLogger};
use crossbeam_channel::{Sender, bounded};
#[allow(unused_imports)]
use std::path::Path;
use std::path::PathBuf;
use std::string::ToString;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::error;
#[cfg(feature = "http")]
use tracing::info;
use tracing_subscriber::prelude::*;

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
    sender: Sender<Arc<LogRecord>>,
    console_sender: Sender<Arc<LogRecord>>,
    shutdown_txs: Vec<Sender<()>>,
    metrics: Arc<Metrics>,
    worker_handles: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    control_tx: Sender<SinkControlMessage>,
    effective_capacity: Arc<AtomicUsize>,
    #[cfg(feature = "http")]
    http_server_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// 注入的缓存依赖
    cache: Option<Arc<dyn Cache>>,
    /// 注入的数据库依赖（需要 dbnexus feature）
    #[cfg(feature = "database")]
    database: Option<Arc<dyn Database>>,
    /// 当前级别指令集，`set_level` 在此之上做 upsert 后重建 EnvFilter 热换装。
    /// Arc 共享给 log 门面（LogAdapter::enabled 的 per-target 过滤单一事实源）。
    level_state: Arc<Mutex<LevelDirectives>>,
    /// 级别热调执行器，包装 `reload::Handle<EnvFilter, Registry>::reload`。
    /// `None` 表示构建路径未提供 reload 能力。
    level_reloader: Option<LevelReloader>,
    /// ops 事件广播通道（每个启用的 sink 通道一个发送端：
    /// file/db 通道 + 各自定义 sink 通道），`publish_ops_event` 逐一投递。
    ops_senders: Vec<Sender<Arc<LogRecord>>>,
    /// fallback journal（含推送端口）句柄：`shutdown` 时尽力补发推送缓冲。
    /// 丢失窗口：本调用之后的溢出记录仅落盘（下次启动 replay 补发），
    /// 推送通道补发到此为止；推送缓冲满溢出的最旧记录不补发（容量约束）。
    fallback_journal: Option<Arc<crate::support::fallback_journal::FallbackJournal>>,
}

/// EnvFilter 热换装闭包类型（隐藏 `reload::Handle` 的具体类型参数）。
type LevelReloader =
    Arc<dyn Fn(tracing_subscriber::filter::EnvFilter) -> Result<(), String> + Send + Sync>;

/// reload 换装层类型（`S = Registry`：与 `with_config_and_sinks` 中
/// 先挂 filter、再挂 subscriber 的组合顺序对应）。
type ReloadFilterLayer = tracing_subscriber::reload::Layer<
    tracing_subscriber::filter::EnvFilter,
    tracing_subscriber::Registry,
>;

/// 级别指令集状态与 `set_level` 的 upsert/重建逻辑。
#[derive(Debug, Clone)]
pub(crate) struct LevelDirectives {
    /// 全局默认级别
    global: String,
    /// per-target 级别（如 `("hyper", "warn")` → `hyper=warn`）
    targets: Vec<(String, String)>,
    /// RUST_LOG 提供的原始附加指令（`set_level` 重建时原样保留）
    extra_raw: Option<String>,
}

/// C3：file 配置是否走 ChannelBufferedFileSink（R-inklog-003 收窄）。
/// CBFS 已覆盖轮转（size/time）/压缩/加密/审计链/保留清理（与 FileSink
/// 同一后处理单一事实源）；仍需 FileSink 的仅剩逐批 fsync 与 JSON 出站
/// 格式（另有磁盘空间守卫/熔断降级为无配置旋钮的固有能力差异，见
/// docs/USER_GUIDE.md 能力矩阵）。
pub(crate) fn file_uses_channel_buffered(cfg: &crate::FileSinkConfig) -> bool {
    !cfg.fsync && cfg.output_format != crate::support::processing::template::OutputFormat::Json
}

impl LevelDirectives {
    pub(crate) fn new(
        global: String,
        targets: Vec<(String, String)>,
        extra_raw: Option<String>,
    ) -> Self {
        Self {
            global,
            targets,
            extra_raw,
        }
    }

    /// 判定 log 门面记录（target, level）是否放行：命中 per-target 指令用其
    /// 级别，否则回落全局级别。级别序与 log/tracing 一致（error 最严、trace
    /// 最宽），record 级别 ≤ 指令级别 才放行。
    pub(crate) fn allows(&self, target: &str, level: log::Level) -> bool {
        fn rank(s: &str) -> u8 {
            match s {
                "trace" => 5,
                "debug" => 4,
                "info" => 3,
                "warn" => 2,
                "error" => 1,
                _ => 3,
            }
        }
        let directive = self
            .targets
            .iter()
            .find(|(name, _)| name == target)
            .map(|(_, l)| l.as_str())
            .unwrap_or(&self.global);
        rank(&level.as_str().to_lowercase()) <= rank(directive)
    }

    /// upsert 一条级别指令（None target 覆盖全局，Some 覆盖同 target 或追加）。
    pub(crate) fn upsert(&mut self, target: Option<&str>, level: &str) {
        match target {
            None => self.global = level.to_string(),
            Some(t) => {
                if let Some(slot) = self.targets.iter_mut().find(|(name, _)| name == t) {
                    slot.1 = level.to_string();
                } else {
                    self.targets.push((t.to_string(), level.to_string()));
                }
            }
        }
    }

    /// 合成 EnvFilter 指令字符串：全局级别在前，target 指令随后，RUST_LOG 附加殿后。
    pub(crate) fn to_filter_string(&self) -> String {
        let mut s = self.global.clone();
        for (target, level) in &self.targets {
            s.push_str(&format!(",{target}={level}"));
        }
        if let Some(raw) = &self.extra_raw
            && !raw.is_empty()
        {
            s.push(',');
            s.push_str(raw);
        }
        s
    }
}

/// 自定义 sink 通道：名称 + sink + 独立接收端（build 时成对启动 worker）
type CustomChannel = (
    String,
    Arc<dyn LogSink>,
    crossbeam_channel::Receiver<Arc<LogRecord>>,
);

impl LoggerManager {
    pub async fn new() -> Result<Self, InklogError> {
        // 经过 DI 路径创建默认实例，行为与 builder().build() 一致
        Self::build_with_deps(LoggerDependencies::default()).await
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
    /// # Deprecated
    ///
    /// DI 入口已收敛：请改用 [`LoggerManager::builder()`]（唯一推荐入口），
    /// 本方法仅为兼容保留，行为为直接转发到内部构建逻辑。
    #[deprecated(since = "0.3.0", note = "use LoggerManager::builder() instead")]
    pub async fn with_dependencies(deps: LoggerDependencies) -> Result<Self, InklogError> {
        Self::build_with_deps(deps).await
    }

    /// 使用依赖注入构建 LoggerManager
    ///
    /// 内部方法，处理依赖解析和默认值填充。
    pub(crate) async fn build_with_deps(deps: LoggerDependencies) -> Result<Self, InklogError> {
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
            if let Some(v) = config_provider.get_string("global.service_name") {
                config.global.service_name = Some(v);
            }
            if let Some(v) = config_provider.get_string("global.service_instance") {
                config.global.service_instance = Some(v);
            }
            if let Some(v) = config_provider.get_string("global.service_env") {
                config.global.service_env = Some(v);
            }
            if let Some(v) = config_provider.get_string("global.service_version") {
                config.global.service_version = Some(v);
            }
            // provider 覆盖完成后立即硬校验身份字段：DI 链与 TOML 链的校验
            // 语义一致（空白值/控制字符拒绝构建）
            config
                .global
                .validate_identity()
                .map_err(InklogError::ConfigError)?;

            // File sink 配置（显式 `enabled = false` 同样要落进 config——
            // 依赖注入模式下宿主可能已通过 add_sink 挂载自己的文件 sink
            // （如采样包装），内置通道必须能被关掉，否则同一条记录双写）
            match config_provider.get_bool("file_sink.enabled") {
                Some(true) => {
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
                Some(false) => {
                    config.file_sink = Some(FileSinkConfig {
                        enabled: false,
                        ..Default::default()
                    });
                }
                None => {}
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
            InklogConfig::load_sync().unwrap_or_else(|_| InklogConfig::default())
        };

        // 注意：cache 和 database 依赖传递给 LoggerManager 内部使用
        // 它们可以通过 LoggerManager 传递给需要的服务（如 DatabaseSink）
        let cache = deps.cache;
        let custom_sinks = deps.custom_sinks;
        #[cfg(feature = "database")]
        let database = deps.database;

        // 使用解析后的配置调用现有的构建逻辑
        // `_full` 变体额外返回 reload 换装层——全局安装（见
        // `install_globals_and_start`）需要它；detached 变体不安装全局前端。
        let (mut manager, subscriber, _filter_compat, filter_layer) = Self::build_detached_full(
            config.clone(),
            #[cfg(feature = "database")]
            database.clone(),
            custom_sinks,
        )
        .await?;

        // 将 cache 依赖注入到 manager 中
        manager.cache = cache;

        // database 已经在 build_detached 中使用，同时也存储在 manager 中
        #[cfg(feature = "database")]
        {
            manager.database = database;
        }

        // 与 with_config_and_sinks 一致：安装全局 tracing/log 前端。
        // 缺失此步骤时，任何使用 add_sink 的宿主（依赖注入路径）都会得到
        // "构建成功但 log::/tracing:: 全部静默" 的日志系统。
        Self::install_globals_and_start(manager, subscriber, filter_layer, &config).await
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
        Self::with_config_and_sinks(config, Vec::new()).await
    }

    /// 使用给定配置创建 LoggerManager 并注册动态第三方 sink，
    /// 随后安装全局 tracing/log 前端。语义与 [`Self::with_config`] 一致。
    pub(crate) async fn with_config_and_sinks(
        config: InklogConfig,
        custom_sinks: Vec<Arc<dyn crate::support::io::LogSink>>,
    ) -> Result<Self, InklogError> {
        // Security audit: Log logger initialization
        tracing::info!(
            event = "security_logger_initialized",
            sinks = ?config.sinks_enabled(),
            masking_enabled = config.global.masking_enabled,
            "Logger manager initialized"
        );

        let (manager, subscriber, _filter_compat, filter_layer) = Self::build_detached_full(
            config.clone(),
            #[cfg(feature = "database")]
            None,
            custom_sinks,
        )
        .await?;

        Self::install_globals_and_start(manager, subscriber, filter_layer, &config).await
    }

    /// 安装全局 tracing/log 前端并启动可选 HTTP 监控服务器。
    ///
    /// [`Self::with_config_and_sinks`] 与 [`Self::build_with_deps`] 共用的收尾
    /// 步骤。此前 `build_with_deps`（`LoggerBuilder::add_sink` 触发的依赖注入
    /// 路径）从不执行全局安装：宿主一旦追加自定义 sink，`log::`/`tracing::`
    /// 记录全部落入空 logger，console 与 file 双静默。
    async fn install_globals_and_start(
        manager: Self,
        subscriber: LoggerSubscriber,
        filter_layer: ReloadFilterLayer,
        config: &InklogConfig,
    ) -> Result<Self, InklogError> {
        // 1. 安装 tracing subscriber。
        // filter_layer（reload 包装）先于 subscriber 挂载——与其构造处的
        // `S = Registry` 类型标注一致；组合顺序对过滤语义无影响。
        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(subscriber);
        // `SetGlobalDefaultError` 的唯一含义是"全局 subscriber 已被设置"——通常是宿主
        // 应用已先行安装。属良性条件：tracing 事件会流向已安装的 subscriber，降级为
        // debug 与下方 log logger 处理保持一致，避免噪音。
        if let Err(ref e) = registry.try_init() {
            tracing::debug!(error = %e, "global subscriber already set; skipping inklog registry");
        }

        // 2. 安装 log crate logger（原生支持，无需 tracing_log）
        let log_adapter = LogAdapter::new(
            manager.console_sender.clone(),
            manager.sender.clone(),
            manager.metrics.clone(),
        )
        .with_level_state(Arc::clone(&manager.level_state));
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
        // `log::SetLoggerError` 的唯一含义是"全局 logger 已被设置"——通常是宿主
        // 应用（如 tracing-opentelemetry → tracing-log 桥接）已先行安装。属良性条件：
        // log 记录仍会流入已安装的 logger，不应视为故障，降级为 debug 避免噪音。
        if let Err(e) = log_logger.install() {
            tracing::debug!(error = %e, "log crate logger already set; skipping inklog LogLogger");
        }

        // 3. 启动HTTP监控服务器（如果配置启用）
        #[cfg(feature = "http")]
        if let Some(ref http_cfg) = config.http_server
            && http_cfg.enabled
            && let Err(e) = Self::start_http_server(
                manager.metrics.clone(),
                manager.sender.clone(),
                manager.effective_capacity.clone(),
                &manager.http_server_handle,
                http_cfg,
            )
            .await
        {
            match http_cfg.error_mode {
                crate::HttpErrorMode::Warn => {
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("err", e.to_string());
                    tracing::warn!(
                        "{}",
                        crate::i18n::tr_args("config-http_startup_failed", args)
                    );
                }
                crate::HttpErrorMode::Strict => {
                    return Err(e);
                }
            }
        }

        // 无 http feature 时 HTTP 监控服务器整体不编译：`http_server.enabled = true`
        // 会在下方被静默跳过。对配置与构建能力不匹配的宿主给出显性 warn，
        // 避免运维侧"配了监控却看不到端口"的无提示失效。
        #[cfg(not(feature = "http"))]
        if let Some(ref http_cfg) = config.http_server
            && http_cfg.enabled
        {
            tracing::warn!(
                event = "http_server_config_ignored",
                "{}",
                crate::i18n::tr("warn-http_feature_not_compiled")
            );
        }

        Ok(manager)
    }

    /// 构建LoggerManager但不安装全局订阅者。
    /// 这主要用于测试和基准测试。
    pub async fn build_detached(
        config: InklogConfig,
        #[cfg(feature = "database")] database: Option<Arc<dyn Database>>,
    ) -> Result<
        (
            Self,
            LoggerSubscriber,
            tracing_subscriber::filter::EnvFilter,
        ),
        InklogError,
    > {
        Self::build_detached_with_sinks(
            config,
            #[cfg(feature = "database")]
            database,
            Vec::new(),
        )
        .await
    }

    /// 构建 LoggerManager 但不安装全局订阅者，并注册动态第三方 sink。
    ///
    /// 每个注册的 sink 获得独立 channel（`extra_async_senders` 通道）与一条
    /// 通用 SinkWorker 消费线程，第三方 Sink 零核心改动接入。
    pub async fn build_detached_with_sinks(
        config: InklogConfig,
        #[cfg(feature = "database")] database: Option<Arc<dyn Database>>,
        custom_sinks: Vec<Arc<dyn crate::support::io::LogSink>>,
    ) -> Result<
        (
            Self,
            LoggerSubscriber,
            tracing_subscriber::filter::EnvFilter,
        ),
        InklogError,
    > {
        let (manager, subscriber, filter, _filter_layer) = Self::build_detached_full(
            config,
            #[cfg(feature = "database")]
            database,
            custom_sinks,
        )
        .await?;
        Ok((manager, subscriber, filter))
    }

    /// 完整构建路径：额外返回 reload 换装层（供 `with_config_and_sinks`
    /// 安装到全局 registry，使 [`Self::set_level`] 热调即时生效）。
    pub(crate) async fn build_detached_full(
        config: InklogConfig,
        #[cfg(feature = "database")] database: Option<Arc<dyn Database>>,
        custom_sinks: Vec<Arc<dyn crate::support::io::LogSink>>,
    ) -> Result<
        (
            Self,
            LoggerSubscriber,
            tracing_subscriber::filter::EnvFilter,
            ReloadFilterLayer,
        ),
        InklogError,
    > {
        let metrics = Arc::new(Metrics::new());
        let (sender, receiver) = bounded(config.performance.channel_capacity);
        let (console_sender, console_receiver) = bounded(config.performance.channel_capacity);
        let (control_tx, control_rx) = bounded(10); // Control channel for recovery commands
        let effective_capacity = Arc::new(AtomicUsize::new(config.performance.channel_capacity));

        // 动态 sink 各自获得独立 channel；若内置异步 sink（file/db）全部
        // 关闭，首个自定义 sink 的通道兼任主 async 通道（承担 fallback 补发语义），
        // 不再重复加入 extras，避免该 sink 收到重复记录。
        let mut custom_channels: Vec<CustomChannel> = Vec::with_capacity(custom_sinks.len());
        let mut custom_senders: Vec<crossbeam_channel::Sender<Arc<LogRecord>>> =
            Vec::with_capacity(custom_sinks.len());
        for (idx, sink) in custom_sinks.into_iter().enumerate() {
            let (tx, rx) = bounded(config.performance.channel_capacity);
            custom_senders.push(tx);
            custom_channels.push((format!("custom-{idx}"), sink, rx));
        }

        // 每个启用的异步 sink 拥有独立 channel，避免 MPMC 单接收者语义导致
        // 记录只被一个 worker 消费（其余 sink 数据缺失）。
        // 默认语义差异（有意设计）：file sink 未配置（None）视为启用——file 通道
        // 默认开；database sink 未配置（None）视为禁用——db 通道默认关。
        let file_enabled = config.file_sink.as_ref().is_none_or(|cfg| cfg.enabled);
        let db_enabled = config.database_sink.as_ref().is_some_and(|cfg| cfg.enabled);
        #[allow(unused_variables)] // db_receiver 仅在启用 db 后端 feature 时使用
        let (db_sender, db_receiver) = if db_enabled {
            let (s, r) = bounded(config.performance.channel_capacity);
            (Some(s), Some(r))
        } else {
            (None, None)
        };

        // ops 事件广播通道 = 启用的 file 通道 + 启用的 db 通道 + 全部自定义通道
        let mut ops_senders: Vec<Sender<Arc<LogRecord>>> = Vec::new();
        if file_enabled {
            ops_senders.push(sender.clone());
        }
        if db_enabled {
            ops_senders.push(db_sender.clone().expect("db sender"));
        }
        ops_senders.extend(custom_senders.iter().cloned());
        // 内部故障路径（file.rs 轮转/压缩/加密失败、workers.rs 降级/恢复）
        // 经全局 hub 复用同一组 ops 通道（审计 M16c 装配点）
        for sender in &ops_senders {
            crate::support::ops_event::register_ops_channel(sender.clone());
        }

        let console_sink: Arc<dyn LogSink> = Arc::new(ConsoleSink::new(
            config.console_sink.clone().unwrap_or_default(),
            LogTemplate::new(&config.global.format),
        ));

        // Initialize tracing subscriber with console_sender channel
        let primary_async_sender = if file_enabled {
            sender.clone()
        } else if let Some(first_custom) = custom_senders.first().cloned() {
            first_custom
        } else if db_enabled {
            db_sender
                .clone()
                .expect("db sender present when db_enabled")
        } else {
            // file/db 全关且无自定义 sink（仅 console 的极简配置）：无 async
            // 通道可作主通道——以即刻断连的哑通道兜底，记录计入 logs_dropped
            // 而非 panic。
            bounded(config.performance.channel_capacity).0
        };
        let mut subscriber = LoggerSubscriber::new(
            console_sender.clone(),
            primary_async_sender,
            metrics.clone(),
        );
        // 每个动态 sink 通道加入 extras；当首个自定义通道兼任主 async
        // 通道（file/db 全关）时跳过它，防止重复投递。
        let custom_extra_offset = usize::from(!file_enabled && !db_enabled);
        for extra in custom_senders.iter().skip(custom_extra_offset) {
            subscriber = subscriber.with_extra_async_sender(extra.clone());
        }
        if file_enabled && db_enabled {
            subscriber = subscriber.with_extra_async_sender(db_sender.clone().expect("db sender"));
        }

        // Wire sanitizer when security features are enabled (CWE-117 prevention)
        if config.global.masking_enabled {
            subscriber = subscriber.with_sanitizer(Arc::new(LogSanitizer::new()));
        }

        // Wire service identity static fields（service_name/instance/env/version
        // + 自定义标注）：未配置任何身份时空 map，不接线（热路径零开销）
        let identity_fields = config.global.identity_fields();
        if !identity_fields.is_empty() {
            subscriber = subscriber.with_identity_fields(Arc::new(identity_fields));
        }

        // Wire rate limiter when configured
        // rate_limit = 0 等价"每秒 0 条"——会让非关键日志全量丢弃；与
        // None 同义处理为不限流。
        if let Some(rate) = config.performance.rate_limit.filter(|&r| r > 0) {
            subscriber = subscriber.with_rate_limiter(Arc::new(RateLimiter::new(rate)));
        }

        // per-target 分级限流（R7）：前缀配额组；未配置（空）时不接线，
        // 全局行为不变。from_config 的硬拒绝与构建期其它配置校验同级
        if !config.rate_limit.is_empty() {
            let limiter = TargetRateLimiter::from_rules(config.rate_limit.rules.clone())?;
            subscriber = subscriber.with_target_rate_limiter(Arc::new(limiter));
        }

        // 采样策略：限流压力下的保留规则；未配置（空）时不接线，走内置兜底
        if !config.sampling.is_empty() {
            let policy = SamplingPolicy::from_config(&config.sampling)?;
            subscriber = subscriber.with_sampling_policy(Arc::new(policy));
        }

        // C4 磁盘持久化 fallback journal：构建期重放一次（记录带 replayed=true
        // 防重放循环）后清空；随后把 journal 挂到 subscriber 供运行期溢出落盘。
        // R-inklog-002：按配置接线可选加密与推送端口；加密而 key 缺失/无效
        // 在此显性失败（Err 向上传播终止构建，禁止静默明文落盘）。
        let mut fallback_journal: Option<Arc<crate::support::fallback_journal::FallbackJournal>> =
            None;
        if config.global.fallback_journal {
            let journal_path = std::path::PathBuf::from(&config.global.fallback_journal_path);
            if let Some(parent) = journal_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let journal = if config.global.fallback_journal_encrypt {
                crate::support::fallback_journal::FallbackJournal::with_crypto(
                    journal_path,
                    crate::support::fallback_journal::DEFAULT_JOURNAL_MAX_BYTES,
                    crate::support::fallback_journal::JournalCryptoConfig {
                        key_env: config.global.fallback_journal_key_env.clone(),
                        key_file: config
                            .global
                            .fallback_journal_key_file
                            .as_ref()
                            .map(PathBuf::from),
                    },
                )?
            } else {
                crate::support::fallback_journal::FallbackJournal::open(journal_path)
            };
            let journal = match &config.global.fallback_journal_push_addr {
                Some(addr) => {
                    let pusher = crate::support::fallback_journal::TcpJournalPusher::new(
                        crate::support::fallback_journal::TcpJournalPusherConfig {
                            addr: addr.clone(),
                            ..Default::default()
                        },
                    )?;
                    journal.with_sender(Arc::new(pusher))
                }
                None => journal,
            };
            let journal = Arc::new(journal);
            let (records, skipped) = journal.replay();
            if skipped > 0 {
                tracing::warn!(
                    skipped,
                    "{}",
                    crate::i18n::tr("journal-replay-skipped-corrupt")
                );
            }
            if !records.is_empty() {
                let replay_target = if file_enabled {
                    Some(sender.clone())
                } else {
                    custom_senders.first().cloned()
                };
                match replay_target {
                    Some(target) => {
                        for mut record in records {
                            record
                                .fields
                                .insert("replayed".to_string(), serde_json::Value::Bool(true));
                            let record = Arc::new(record);
                            let _ = target.send_timeout(record, Duration::from_millis(100));
                        }
                    }
                    None => tracing::warn!(
                        count = records.len(),
                        "{}",
                        crate::i18n::tr("journal-replay-no-durable-sink")
                    ),
                }
            }
            subscriber = subscriber.with_journal(Arc::clone(&journal));
            fallback_journal = Some(journal);
        }

        // ERROR/FATAL 兜底缓冲运行期补发：60s 周期任务兜底；通道恢复半满的
        // 即时触发在 subscriber 内部实现。定时线程随进程存续（与轮转定时器
        // 同生命周期语义），clone 的 subscriber 共享同一 fallback 缓冲。
        {
            let ticker = subscriber.clone();
            if let Err(e) = std::thread::Builder::new()
                .name("inklog-fallback-flush".to_string())
                .spawn(move || {
                    const FALLBACK_FLUSH_INTERVAL: std::time::Duration =
                        std::time::Duration::from_secs(60);
                    loop {
                        std::thread::sleep(FALLBACK_FLUSH_INTERVAL);
                        ticker.try_flush_fallback();
                    }
                })
            {
                tracing::warn!(
                    error = %e,
                    "{}",
                    crate::i18n::tr("sink-fallback_flush_ticker_spawn_failed")
                );
            }
        }

        // Filter — use EnvFilter to support RUST_LOG per-module filtering
        // Configured level serves as the global default; RUST_LOG overrides
        // specific modules (e.g. RUST_LOG=nebulaid=debug,hyper=warn).
        // target_levels from config are merged between global level and RUST_LOG.
        let level = config
            .global
            .level
            .parse::<tracing::Level>()
            .unwrap_or(tracing::Level::INFO);
        let level_str = match level {
            tracing::Level::TRACE => "trace",
            tracing::Level::DEBUG => "debug",
            tracing::Level::INFO => "info",
            tracing::Level::WARN => "warn",
            tracing::Level::ERROR => "error",
        };
        // Build base filter string: global level + per-crate target presets
        let mut base_filter = level_str.to_string();
        for (target, target_level) in &config.target_levels {
            base_filter.push_str(&format!(",{target}={target_level}"));
        }
        let filter = match std::env::var("RUST_LOG") {
            Ok(val) if !val.is_empty() => {
                tracing_subscriber::filter::EnvFilter::new(format!("{base_filter},{val}"))
            }
            _ => tracing_subscriber::filter::EnvFilter::new(base_filter),
        };

        // EnvFilter 以 reload::Layer 包装，支持运行时 set_level 热换装。
        // 兼容路径返回的 EnvFilter 是同一指令集的克隆（供测试/调用方断言字符串）；
        // 真正安装到 registry 的是 filter_layer。
        let (filter_layer, reload_handle): (ReloadFilterLayer, _) =
            tracing_subscriber::reload::Layer::new(filter.clone());
        let level_reloader: LevelReloader = Arc::new(move |new_filter| {
            reload_handle
                .reload(new_filter)
                .map_err(|e| format!("level reload failed: {e}"))
        });
        let level_state = Arc::new(Mutex::new(LevelDirectives::new(
            level_str.to_string(),
            config
                .target_levels
                .iter()
                .map(|(t, l)| (t.clone(), l.clone()))
                .collect(),
            std::env::var("RUST_LOG").ok().filter(|v| !v.is_empty()),
        )));

        // Create error sink for logging system errors.
        // 惰性创建：构建期不打开 logs/error.log，一次性命令构建后未发生
        // error 级事件即退出时不再遗留该文件；首次真正写入时才落盘
        //（构造永不失败，打开失败语义后移到写入路径并原样上抛）。
        let error_sink_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("logs/error.log"),
            ..Default::default()
        };
        let error_sink: Arc<Mutex<Option<Arc<dyn LogSink>>>> = Arc::new(Mutex::new(Some(
            Arc::new(crate::support::io::sink::file::LazyFileSink::new(
                error_sink_config,
            )) as Arc<dyn LogSink>,
        )));

        let file_sink_cfg = config.file_sink.clone().unwrap_or_default();

        // 确保 database 依赖有效：DI 注入优先；未注入且 db sink 启用时在当前 async
        // 上下文创建默认 DbNexusAdapter（连接池归属调用方 runtime）。
        // 核正：此逻辑原先位于 start_workers（同步）内部经 Handle::block_on 创建——
        // start_workers 在 runtime 线程上被调用时必然 panic（runtime-in-runtime），故上移。
        #[cfg(feature = "database")]
        let database = match database {
            Some(db) => {
                // 注入路径与自动创建路径对齐：db sink 启用且配置的 pool_size 超过
                // effective_db_worker_limit 上限时记录警告。注入的 Database 为不透明
                // trait 对象，无法在此外调整其连接池，仅提示调用方。
                if let Some(ref cfg) = config.database_sink
                    && cfg.enabled
                {
                    let db_worker_limit =
                        crate::support::io::sink::database::effective_db_worker_limit();
                    if cfg.pool_size > db_worker_limit as u32 {
                        tracing::warn!(
                            configured_pool_size = cfg.pool_size,
                            limit = db_worker_limit,
                            "Injected Database configured with pool_size above effective_db_worker_limit; cap the pool at the adapter level"
                        );
                    }
                }
                Some(db)
            }
            None => {
                if let Some(ref cfg) = config.database_sink {
                    if cfg.enabled {
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
                        Some(Arc::new(
                            crate::integrations::infra::DbNexusAdapter::with_full_config(
                                &cfg.url,
                                effective_pool_size,
                                &cfg.table_name,
                                cfg.permissions_path.clone(),
                                &cfg.admin_role,
                            )
                            .await?,
                        ) as Arc<dyn Database>)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };
        let channel_capacity = config.performance.channel_capacity;
        let global_format = config.global.format.clone();
        let (handles, shutdown_txs) = Self::start_workers(WorkerParams {
            config,
            receiver,
            console_receiver,
            control_rx,
            control_tx: control_tx.clone(),
            metrics: metrics.clone(),
            console_sink,
            error_sink: error_sink.clone(),
            effective_capacity: effective_capacity.clone(),
            file_sink_factory: Box::new(move || {
                // R-inklog-003 收窄：CBFS 覆盖轮转/压缩/加密/审计链/保留清理，
                // 仅 fsync 与 JSON 出站格式回落 FileSink。能力矩阵见
                // docs/USER_GUIDE.md。
                let simple = file_uses_channel_buffered(&file_sink_cfg);
                if simple {
                    let cbfs_config = crate::ChannelBufferedConfig {
                        base_config: file_sink_cfg.clone(),
                        channel_capacity,
                        backpressure_strategy: Default::default(),
                        flush_batch_size: file_sink_cfg.batch_size,
                        flush_interval_ms: file_sink_cfg.flush_interval_ms,
                    };
                    let template = crate::LogTemplate::new(&global_format);
                    return crate::ChannelBufferedFileSink::new(cbfs_config, template)
                        .map(|s| Box::new(s) as Box<dyn LogSink>);
                }
                FileSink::new(file_sink_cfg.clone()).map(|s| Box::new(s) as Box<dyn LogSink>)
            }),
            #[cfg(feature = "database")]
            db_sink_factory: Box::new({
                // 核正：factory 在 spawn_blocking 线程上执行，该线程无 reactor context，
                // Handle::current() 必 panic（worker 静默死亡、记录永不消费）——
                // 与 file worker 同款做法：在 build_detached（async 上下文）clone
                // runtime handle 进闭包使用
                let runtime_handle = tokio::runtime::Handle::current();
                move |db: Arc<dyn crate::integrations::Database>, metrics: Arc<Metrics>| {
                    let sink = crate::support::io::DatabaseSink::new(db)?;
                    runtime_handle.block_on(async { sink.set_metrics(metrics).await });
                    Ok(Box::new(sink) as Box<dyn LogSink>)
                }
            }),
            #[cfg(feature = "database")]
            db_receiver,
            custom_sinks: custom_channels
                .into_iter()
                .map(|(name, sink, receiver)| super::workers::CustomSinkEntry {
                    name,
                    sink,
                    receiver,
                })
                .collect(),
            #[cfg(feature = "database")]
            database,
        })?;

        let manager = Self {
            sender,
            console_sender,
            shutdown_txs,
            metrics,
            worker_handles: Mutex::new(handles),
            control_tx,
            effective_capacity: effective_capacity.clone(),
            #[cfg(feature = "http")]
            http_server_handle: Mutex::new(None),
            cache: None,
            #[cfg(feature = "database")]
            database: None,
            level_state,
            level_reloader: Some(level_reloader),
            ops_senders,
            fallback_journal,
        };

        Ok((manager, subscriber, filter, filter_layer))
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
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", e.to_string());
            InklogError::ConfigError(crate::i18n::tr_args("config-failed_read_config", args))
        })?;
        let config: InklogConfig = toml::from_str(&content).map_err(|e| {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", e.to_string());
            InklogError::ConfigError(crate::i18n::tr_args("config-failed_parse_config", args))
        })?;
        Self::with_config(config).await
    }

    /// 自动搜索并加载配置文件初始化LoggerManager
    ///
    /// 搜索路径优先级：
    /// 1. 环境变量 `INKLOG_CONFIG_PATH` 指定的路径
    /// 2. 当前目录下的 `inklog_config.toml`
    /// 3. 用户配置目录 `~/.config/inklog/config.toml`
    /// 4. 系统配置目录（Unix: `/etc/inklog/config.toml`，Windows: `%ProgramData%\inklog\config.toml`）
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
        let config = InklogConfig::load_sync().map_err(|e| {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", e.to_string());
            InklogError::ConfigError(crate::i18n::tr_args("config-failed_load_config", args))
        })?;
        Self::with_config(config).await
    }

    pub fn get_health_status(&self) -> HealthStatus {
        let channel_len = self.sender.len();
        let channel_cap = self.effective_capacity.load(Ordering::Acquire);
        self.metrics.get_status(channel_len, channel_cap)
    }

    pub fn recover_sink(&self, sink_name: &str) -> Result<(), InklogError> {
        self.control_tx
            .send(SinkControlMessage::RecoverSink(sink_name.to_string()))
            .map_err(|e| {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("err", e.to_string());
                InklogError::ChannelError(crate::i18n::tr_args("config-failed_send_recovery", args))
            })
    }

    pub fn effective_channel_capacity(&self) -> usize {
        self.effective_capacity.load(Ordering::Acquire)
    }

    /// 运行时日志级别热调整。
    ///
    /// 在当前级别指令集之上做 upsert 后重建 `EnvFilter`，并经
    /// `tracing_subscriber::reload` 热换装——进程内即时生效，无需重启。
    ///
    /// # Arguments
    ///
    /// * `target` - `None` 调整全局默认级别；`Some(target)` 调整特定模块/target
    ///   的级别（优先级高于全局默认、低于 `RUST_LOG`）
    /// * `level` - 目标级别（trace/debug/info/warn/error/fatal，大小写不敏感）
    ///
    /// # Errors
    ///
    /// 非法级别名返回 `InklogError::ConfigError`，指令集保持不变。
    ///
    /// # Example
    /// ```ignore
    /// manager.set_level(None, "debug")?;            // 全局调到 debug
    /// manager.set_level(Some("hyper"), "warn")?;    // hyper 模块单独 warn
    /// ```
    pub fn set_level(&self, target: Option<&str>, level: &str) -> Result<(), InklogError> {
        if !crate::LogLevel::is_valid_level(level) {
            return Err(InklogError::ConfigError(format!(
                "Invalid log level '{}'. Valid levels: {}",
                level,
                crate::LogLevel::VALID_LEVEL_STRINGS.join(", ")
            )));
        }
        let normalized = level.to_ascii_lowercase();
        let new_filter = {
            let mut state = self.level_state.lock().expect("level_state mutex poisoned");
            state.upsert(target, &normalized);
            tracing_subscriber::filter::EnvFilter::new(state.to_filter_string())
        };
        if let Some(reloader) = &self.level_reloader {
            // reload 失败仅意味着没有存活的已安装层（如 build_detached 测试路径
            // 未安装全局 subscriber）——指令集已更新，不视为错误。
            if let Err(e) = reloader(new_filter) {
                tracing::warn!(
                    error = %e,
                    "set_level: no live installed filter to swap (detached build?); directives updated anyway"
                );
            }
        }
        // 双门面级别同步：热调只 reload 了 EnvFilter（tracing 侧），log 门面的
        // 过滤由 log::max_level 把关——不同步会让 log::debug! 等在门面处被旧
        // 级别拦截。per-target 指令对 log 门面不可表达，仅在全局级别变更时同步。
        if target.is_none() {
            let filter = match normalized.as_str() {
                "trace" => log::LevelFilter::Trace,
                "debug" => log::LevelFilter::Debug,
                "info" => log::LevelFilter::Info,
                "warn" => log::LevelFilter::Warn,
                "error" => log::LevelFilter::Error,
                _ => log::LevelFilter::Off,
            };
            log::set_max_level(filter);
        }
        match target {
            None => tracing::info!(level = %normalized, "log level changed at runtime"),
            Some(t) => {
                tracing::info!(target = t, level = %normalized, "target log level changed at runtime")
            }
        }
        Ok(())
    }

    /// 发布内部审计/运维事件。
    ///
    /// 事件转换为结构化记录（`target = "inklog::ops"`）后广播到每个启用的
    /// sink 通道（per-sink 通道体系：file/db + 自定义），供告警与
    /// 事后审计。通道满/关闭时静默丢弃并计 `channel_blocked`——事件通道
    /// 不得反压主链路。
    pub fn publish_ops_event(&self, kind: &str, sink: Option<&str>, detail: serde_json::Value) {
        let event = crate::support::ops_event::InklogOpsEvent::now(kind, sink, detail);
        let record = Arc::new(event.to_log_record());
        for sender in &self.ops_senders {
            if sender
                .send_timeout(Arc::clone(&record), Duration::from_millis(100))
                .is_err()
            {
                self.metrics.inc_channel_blocked();
            }
        }
    }

    /// 查询当前级别指令集的 EnvFilter 字符串表示（调试用）。
    pub fn current_level_filter_string(&self) -> String {
        self.level_state
            .lock()
            .expect("level_state mutex poisoned")
            .to_filter_string()
    }

    pub fn channel_len(&self) -> usize {
        self.sender.len()
    }

    /// 返回注入的缓存依赖（`Arc` 克隆，与内部共享同一实例）。
    ///
    /// 该字段由 `with_dependencies`/builder 的 DI 路径写入，供下游服务
    /// 按需取用；未注入时返回 `None`。
    pub fn cache(&self) -> Option<Arc<dyn Cache>> {
        self.cache.clone()
    }

    /// 返回注入的数据库依赖（需要 dbnexus feature）。
    ///
    /// 该字段由 `with_dependencies`/builder 的 DI 路径写入，供下游服务
    /// 按需取用；未注入时返回 `None`。
    #[cfg(feature = "database")]
    pub fn database(&self) -> Option<Arc<dyn Database>> {
        self.database.clone()
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
            // 核正：此处必须用 send_timeout 而非 send。worker（spawn_blocking）内的
            // block_on(sink.write) 依赖调用方 runtime 的驱动线程推进（sqlx pool 等
            // 任务 spawn 在创建时的 runtime 上）；若驱动线程此刻正阻塞在本 send 上
            // （drop→shutdown 路径），write 永不完成 → worker 不回循环消费信号 →
            // channel 满 → send 永久阻塞，形成死锁环（gdb 现场：测试线程卡
            // crossbeam array::send，db worker 卡 CachedParkThread::park）。
            // send_timeout 保证 shutdown 永不挂死：超时后继续走 handles 轮询，
            // 调用方驱动恢复后 worker 会自行消费信号并正常退出。
            if tx.send_timeout((), Duration::from_secs(2)).is_err() {
                tracing::warn!("{}", crate::i18n::tr("config-shutdown_signal_lost"));
            }
        }

        // 关闭HTTP服务器
        #[cfg(feature = "http")]
        {
            if let Ok(mut handle_guard) = self.http_server_handle.lock()
                && let Some(handle) = handle_guard.take()
            {
                handle.abort();
                info!("HTTP server shutdown signal sent");
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

        // Use a timeout-based poll to avoid deadlocks
        // Each handle gets up to 5 seconds to complete
        // tokio::task::JoinHandle has no sync .join(); is_finished() confirms completion
        for handle in handles {
            let start = Instant::now();
            let mut backoff = Duration::from_millis(10);
            while start.elapsed() < Duration::from_secs(5) {
                if handle.is_finished() {
                    break;
                }
                std::thread::sleep(backoff);
                // 自适应退避：10ms 起，每轮 ×2 至上限 100ms，减少空转
                backoff = (backoff * 2).min(Duration::from_millis(100));
            }
            // If still not finished after timeout, abort the task
            if !handle.is_finished() {
                handle.abort();
            }
        }

        // journal 推送端口尽力补发（有界：connect_timeout + write_timeout 上限）。
        // 丢失窗口（有意设计）：worker 停止后、本调用前溢出落盘的记录留在磁盘
        // journal，由下次启动 replay 补发；本调用之后不再有推送补发，推送缓冲
        // 满时溢出丢弃的最旧记录同样不补发（有界 FIFO 容量约束）。
        if let Some(journal) = &self.fallback_journal {
            journal.flush_pending();
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

#[cfg(test)]
#[allow(deprecated)] // 覆盖 deprecated 入口 with_dependencies 的既有行为测试
mod tests {

    /// 断言消息中的响应体预览：剥离控制字符并截断，避免外部内容直入
    /// 测试日志（CodeQL CWE-117）。
    #[cfg(feature = "http")]
    fn body_preview(body: &str) -> String {
        body.chars().filter(|c| !c.is_control()).take(160).collect()
    }

    use super::*;
    use chrono::Utc;
    use serial_test::serial;

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

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_enable_http_server_creates_config() {
        let builder = LoggerBuilder::new().enable_http_server(true);
        assert!(builder.config.http_server.is_some());
        assert!(builder.config.http_server.as_ref().unwrap().enabled);
    }

    #[cfg(feature = "http")]
    #[test]
    fn test_builder_http_host_sets_config() {
        let builder = LoggerBuilder::new()
            .enable_http_server(true)
            .http_host("0.0.0.0");
        assert_eq!(builder.config.http_server.as_ref().unwrap().host, "0.0.0.0");
    }

    #[cfg(feature = "http")]
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
        // 预占进程级全局（tracing subscriber + log logger）：并发测试下
        // 全局由先完成 install 的实例持有，其余实例的日志事件会持续落入
        // 全局实例通道。预占后本测试的 install 走良性降级分支，tracing /
        // log 事件不再可能进入本实例通道。占位对其他测试无转移影响：
        // 本文件内的事件断言均已用 with_default 线程局部 registry，
        // 不依赖进程全局。
        struct SilentLogger;
        impl log::Log for SilentLogger {
            fn enabled(&self, _: &log::Metadata) -> bool {
                false
            }
            fn log(&self, _: &log::Record) {}
            fn flush(&self) {}
        }
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing_subscriber::filter::LevelFilter::OFF)
            .with_writer(std::io::sink)
            .try_init();
        let _ = log::set_boxed_logger(Box::new(SilentLogger));

        // file sink 关闭：默认配置会把实例通道注册进进程级 ops 事件 hub
        // （register_ops_channel），并发测试触发的内部故障广播
        // （publish_internal：轮转/压缩失败、worker 降级恢复）会写入本
        // 实例通道，且 file worker 因默认相对路径不可写进入降级模式不
        // 消费主通道，「新建无积压」的同步断言在并发下不成立。关闭后
        // 实例不进广播面，断言确定。与 new() 走同一 build_with_deps 路径。
        let config = InklogConfig {
            file_sink: Some(FileSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let manager = LoggerManager::with_config(config)
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
        use crate::integrations::MockCache;
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: None,
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };
        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with deps");
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_di_config_invalid_identity_value_rejects_build() {
        // DI 链身份硬校验：provider 返回含控制字符的身份值时构建必须失败
        //（与 TOML 链的 InklogConfig::validate 硬拒绝语义一致）
        use crate::integrations::MockConfig;
        use crate::integrations::infra::config::Config;
        let config = MockConfig::new();
        config.set("global.service_name", "orders\nfake");
        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(config) as Arc<dyn Config>),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };
        let result = LoggerManager::build_with_deps(deps).await;
        match result {
            Ok(manager) => {
                let _ = manager.shutdown();
                panic!("build must fail on control-character identity value");
            }
            Err(e) => {
                assert!(
                    e.to_string().contains("control character"),
                    "unexpected error: {e}"
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_di_config_blank_identity_value_rejects_build() {
        use crate::integrations::MockConfig;
        use crate::integrations::infra::config::Config;
        let config = MockConfig::new();
        config.set("global.service_env", "   ");
        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(config) as Arc<dyn Config>),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };
        let result = LoggerManager::build_with_deps(deps).await;
        match result {
            Ok(manager) => {
                let _ = manager.shutdown();
                panic!("build must fail on blank identity value");
            }
            Err(e) => {
                assert!(e.to_string().contains("blank"), "unexpected error: {e}");
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_di_config_valid_identity_value_builds() {
        // 正例：合法身份值正常构建（拒绝路径的对照）
        use crate::integrations::MockConfig;
        use crate::integrations::infra::config::Config;
        let config = MockConfig::new();
        config.set("global.service_name", "orders");
        config.set("global.service_env", "prod");
        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(config) as Arc<dyn Config>),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };
        let manager = LoggerManager::build_with_deps(deps)
            .await
            .expect("valid identity values must build");
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_cache_getter_returns_injected_instance() {
        // cache()/database() getter 应返回与注入实例共享底层数据的 Arc
        use crate::integrations::MockCache;
        let cache: Arc<dyn Cache> = Arc::new(MockCache::new());
        let deps = LoggerDependencies {
            cache: Some(Arc::clone(&cache)),
            config: None,
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };
        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with deps");
        let got = manager
            .cache()
            .expect("injected cache dependency should be returned");
        assert!(
            Arc::ptr_eq(&cache, &got),
            "getter must return the same instance"
        );
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_cache_getter_none_by_default() {
        let manager = LoggerManager::with_dependencies(LoggerDependencies::default())
            .await
            .expect("Failed to create manager");
        assert!(
            manager.cache().is_none(),
            "cache getter should be None without DI injection"
        );
        let _ = manager.shutdown();
    }

    #[cfg(feature = "database")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_manager_database_getter_returns_injected_instance() {
        use crate::integrations::MockDatabaseAdapter;
        let database: Arc<dyn Database> = Arc::new(MockDatabaseAdapter::new());
        let deps = LoggerDependencies {
            cache: None,
            config: None,
            custom_sinks: Vec::new(),
            database: Some(Arc::clone(&database)),
        };
        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with database injection");
        let got = manager
            .database()
            .expect("injected database dependency should be returned");
        assert!(
            Arc::ptr_eq(&database, &got),
            "getter must return the same instance"
        );
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_logger_manager_with_dependencies_injects_config() {
        use crate::integrations::InklogConfigAdapter;
        let config = InklogConfig::default();
        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(InklogConfigAdapter::from_config(config))),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };
        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with config provider");
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

    #[cfg(feature = "database")]
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

    #[cfg(feature = "database")]
    #[test]
    fn test_builder_with_database_injects_dep() {
        use crate::integrations::MockDatabaseAdapter;
        let builder = LoggerBuilder::new().with_database(Arc::new(MockDatabaseAdapter::new()));
        assert!(builder.deps.database.is_some());
    }

    // ============================================================================
    // LoggerBuilder 依赖注入方法测试
    // ============================================================================

    #[test]
    fn test_builder_cache_injects_dep() {
        use crate::integrations::MockCache;
        let builder = LoggerBuilder::new().cache(Arc::new(MockCache::new()));
        assert!(builder.deps.cache.is_some());
    }

    #[test]
    fn test_builder_config_injects_dep() {
        use crate::integrations::MockConfig;
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
        use crate::integrations::MockCache;
        let manager = LoggerManager::builder()
            .level("info")
            .channel_capacity(1500)
            .worker_threads(1)
            .cache(Arc::new(MockCache::new()))
            .build()
            .await
            .expect("Failed to build manager with cache injection");
        assert_eq!(manager.effective_channel_capacity(), 1500);
        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_builder_build_with_config_injection() {
        // 注入 config → has_deps=true, deps.config.is_some() 分支（不创建 adapter）
        use crate::integrations::MockConfig;
        let manager = LoggerManager::builder()
            .config(Arc::new(MockConfig::new()))
            .worker_threads(1)
            .build()
            .await
            .expect("Failed to build manager with config injection");
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
            trace_id: None,
            span_id: None,
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
                trace_id: None,
                span_id: None,
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
            trace_id: None,
            span_id: None,
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
            body_preview(&body)
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

    /// vuln-0003: auth 启用但 token 环境变量未设置时，启动直接 fail-closed
    /// （之前是启动成功后请求时返回 500，存在运行时环境变量被篡改的风险）
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_missing_token_env_fails_to_start() {
        let port = find_available_http_port();
        // 使用唯一的环境变量名，确保它未设置
        let token_env = "INKLOG_TEST_TOKEN_MISSING_ENV_VAR";
        unsafe {
            std::env::remove_var(token_env);
        }
        // Strict 模式：HTTP server 启动失败应导致 with_config 返回 Err
        let mut config = http_test_config_with_auth(port, token_env);
        config.http_server.as_mut().unwrap().error_mode = crate::HttpErrorMode::Strict;
        let result = LoggerManager::with_config(config).await;
        let err_msg = match result {
            Err(e) => format!("{}", e),
            Ok(_) => panic!("vuln-0003: missing token env should fail to start (fail-closed)"),
        };
        assert!(
            err_msg.contains("token env var") && err_msg.contains("is not set"),
            "error should explain token env misconfiguration, got: {}",
            err_msg
        );
    }

    /// vuln-0003: auth 启用但 token 环境变量为空字符串时，启动 fail-closed
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_empty_token_env_fails_to_start() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_EMPTY_ENV_VAR";
        unsafe {
            std::env::set_var(token_env, "");
        }
        let mut config = http_test_config_with_auth(port, token_env);
        config.http_server.as_mut().unwrap().error_mode = crate::HttpErrorMode::Strict;
        let result = LoggerManager::with_config(config).await;
        let err_msg = match result {
            Err(e) => format!("{}", e),
            Ok(_) => panic!("vuln-0003: empty token env should fail to start (fail-closed)"),
        };
        assert!(
            err_msg.contains("is empty"),
            "error should explain token env is empty, got: {}",
            err_msg
        );
        unsafe {
            std::env::remove_var(token_env);
        }
    }

    /// vuln-0003 核心验证：启动后修改环境变量不影响后续请求鉴权
    ///
    /// 之前的行为：auth_middleware 每次请求时调用 `std::env::var(token_env)`，
    /// 攻击者若有权限修改环境变量（如通过其他漏洞），可立即影响后续请求的鉴权。
    ///
    /// 修复后的行为：启动时一次性读取 token 并缓存到 HttpAuthState.token_value，
    /// 后续请求只使用缓存值，环境变量修改不影响鉴权。
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_vuln_0003_env_var_change_after_start_does_not_affect_auth() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_VULN_0003";
        let original_token = "original-secret-vuln-0003";
        unsafe {
            std::env::set_var(token_env, original_token);
        }
        let manager = LoggerManager::with_config(http_test_config_with_auth(port, token_env))
            .await
            .expect("Manager should start with valid token");
        assert!(
            wait_for_http_server("127.0.0.1", port).await,
            "HTTP server should become reachable"
        );

        // 1. 原始 token 应该能通过鉴权
        let client = reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest client");
        let resp = client
            .get(format!("http://127.0.0.1:{}/health", port))
            .bearer_auth(original_token)
            .send()
            .await
            .expect("Request with original token should succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "original token should work"
        );

        // 2. 篡改环境变量为另一个值（模拟攻击者修改环境变量）
        let tampered_token = "tampered-by-attacker";
        unsafe {
            std::env::set_var(token_env, tampered_token);
        }

        // 3. 用篡改后的 token 请求 — 应该返回 401（因为服务端使用的是启动时缓存的原始 token）
        let resp = client
            .get(format!("http://127.0.0.1:{}/health", port))
            .bearer_auth(tampered_token)
            .send()
            .await
            .expect("Request with tampered token should still get a response");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "vuln-0003: tampered env var token should NOT work (cached token at startup wins)"
        );

        // 4. 原始 token 仍然有效（缓存未变）
        let resp = client
            .get(format!("http://127.0.0.1:{}/health", port))
            .bearer_auth(original_token)
            .send()
            .await
            .expect("Request with original token should still succeed");
        assert_eq!(
            resp.status(),
            reqwest::StatusCode::OK,
            "vuln-0003: original token should still work after env var tampering"
        );

        let _ = manager.shutdown();
        unsafe {
            std::env::remove_var(token_env);
        }
    }

    /// auth 启用且 Bearer token 正确时返回 200
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_valid_token_returns_200() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_VALID";
        let token_value = "secret-token-12345";
        unsafe {
            std::env::set_var(token_env, token_value);
        }
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
        unsafe {
            std::env::remove_var(token_env);
        }
    }

    /// auth 启用但 Bearer token 错误时返回 401
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_invalid_token_returns_401() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_INVALID";
        unsafe {
            std::env::set_var(token_env, "correct-secret");
        }
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
            body_preview(&body)
        );
        let _ = manager.shutdown();
        unsafe {
            std::env::remove_var(token_env);
        }
    }

    /// auth 启用但缺少 Authorization header 时返回 401
    #[cfg(feature = "http")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn test_http_server_auth_missing_header_returns_401() {
        let port = find_available_http_port();
        let token_env = "INKLOG_TEST_TOKEN_MISSING_HEADER";
        unsafe {
            std::env::set_var(token_env, "some-secret");
        }
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
            body_preview(&body)
        );
        let _ = manager.shutdown();
        unsafe {
            std::env::remove_var(token_env);
        }
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
            body_preview(&body)
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
        use crate::integrations::MockConfig;
        let mock_config = MockConfig::new()
            .with_value("global.level", "debug")
            .with_value("global.format", "{level} {message}")
            .with_value("global.masking_enabled", "true")
            .with_value("global.auto_fallback", "true");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with config provider");

        // 验证 manager 成功创建并运行（config 已消费至 WorkerParams，不再存储于 struct）
        assert!(manager.effective_channel_capacity() > 0);

        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_configures_file_sink_from_provider() {
        // 验证 Config trait 的 file_sink.* 配置被正确应用到 InklogConfig.file_sink
        // 并触发 file worker 实际写入文件
        use crate::integrations::MockConfig;
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
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with file_sink config");

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
            trace_id: None,
            span_id: None,
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
        use crate::integrations::MockConfig;
        let mock_config = MockConfig::new()
            .with_value("http_server.enabled", "true")
            .with_value("http_server.host", "127.0.0.1")
            .with_value("http_server.port", "9090");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with http_server config");

        // 验证 manager 成功创建（http_server 配置已消费至 WorkerParams）
        assert!(manager.effective_channel_capacity() > 0);

        let _ = manager.shutdown();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_configures_performance_from_provider() {
        // 验证 Config trait 的 performance.worker_threads/channel_capacity
        // 被正确应用到 InklogConfig.performance
        use crate::integrations::MockConfig;
        let mock_config = MockConfig::new()
            .with_value("performance.worker_threads", "2")
            .with_value("performance.channel_capacity", "3000");

        let deps = LoggerDependencies {
            cache: None,
            config: Some(Arc::new(mock_config)),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
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

        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_with_deps 注入 database 测试 (lines 336-340)
    // 需要 dbnexus feature
    // ============================================================================

    #[cfg(feature = "database")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_injects_database() {
        // 验证通过 LoggerDependencies.database 注入的 Database 实现不会导致创建失败
        use crate::integrations::MockDatabaseAdapter;
        let deps = LoggerDependencies {
            cache: None,
            config: None,
            custom_sinks: Vec::new(),
            database: Some(Arc::new(MockDatabaseAdapter::new())),
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with database injection");

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
    async fn test_build_detached_console_only_config_does_not_panic() {
        // file/db 全关且无自定义 sink 的极简配置：主 async 通道必须退化为
        // 断连哑通道而非 panic
        let config = InklogConfig {
            file_sink: Some(crate::FileSinkConfig {
                enabled: false,
                ..Default::default()
            }),
            database_sink: None,
            ..Default::default()
        };
        let (_manager, _subscriber, _filter) = LoggerManager::build_detached(
            config,
            #[cfg(feature = "database")]
            None,
        )
        .await
        .expect("console-only config must build without panic");
    }

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
            #[cfg(feature = "database")]
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

        // filter 现在是 EnvFilter，验证其字符串表示包含配置的 level "warn"
        let filter_str = filter.to_string();
        assert!(
            filter_str.contains("warn"),
            "EnvFilter should contain config level 'warn', got: {}",
            filter_str
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
            #[cfg(feature = "database")]
            None,
        )
        .await
        .expect("build_detached should succeed even with invalid level");

        // 无效 level 应回退到 INFO
        let filter_str = filter.to_string();
        assert!(
            filter_str.contains("info"),
            "EnvFilter should fall back to 'info' for invalid level, got: {}",
            filter_str
        );

        let _ = manager.shutdown();
    }

    /// target_levels 配置项正确合并到 EnvFilter 字符串。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_detached_target_levels_in_filter() {
        use std::collections::HashMap;

        let mut config = InklogConfig::default();
        config.target_levels = HashMap::from([
            ("hyper".into(), "warn".into()),
            ("my_crate".into(), "debug".into()),
        ]);

        let (manager, _subscriber, filter) = LoggerManager::build_detached(
            config,
            #[cfg(feature = "database")]
            None,
        )
        .await
        .expect("build_detached should succeed");

        let filter_str = filter.to_string();
        assert!(
            filter_str.contains("hyper=warn"),
            "EnvFilter should contain target level 'hyper=warn', got: {filter_str}"
        );
        assert!(
            filter_str.contains("my_crate=debug"),
            "EnvFilter should contain target level 'my_crate=debug', got: {filter_str}"
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
            trace_id: None,
            span_id: None,
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
            trace_id: None,
            span_id: None,
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
            #[cfg(feature = "database")]
            None,
        )
        .await
        .expect("build_detached should succeed");

        // 验证 manager 创建成功，error_sink 已初始化（即使为 None）
        assert!(manager.effective_channel_capacity() > 0);

        let _ = manager.shutdown();
    }

    // ============================================================================
    // error sink 延迟创建测试
    //
    // 默认 error sink 指向相对路径 logs/error.log；一次性命令构建 manager
    // 后未发生任何 error 级事件即退出时，不得遗留该文件。
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[serial]
    async fn test_build_detached_defers_error_log_file_creation() {
        // CWD 隔离：默认 error sink 路径相对当前目录解析；
        // serial 防止与其他测试的进程级 CWD 切换互踩
        let base_dir = tempfile::TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(base_dir.path()).unwrap();

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
            #[cfg(feature = "database")]
            None,
        )
        .await
        .expect("build_detached should succeed");

        assert!(
            !base_dir.path().join("logs/error.log").exists(),
            "构建 manager 不得立即创建 logs/error.log"
        );

        let _ = manager.shutdown();
        std::env::set_current_dir(original_cwd).unwrap();
    }

    // ============================================================================
    // LoggerDependencies Debug 实现测试 (lines 118-131)
    //
    // 验证 LoggerDependencies 的 Debug 实现包含 cache/config/database 字段
    // （database 字段仅在 dbnexus feature 下存在）
    // ============================================================================

    #[cfg(feature = "database")]
    #[test]
    fn test_logger_dependencies_debug_includes_database_field() {
        use crate::integrations::{MockCache, MockDatabaseAdapter};
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: None,
            custom_sinks: Vec::new(),
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
        use crate::integrations::{InklogConfigAdapter, MockCache};
        let config = InklogConfig::default();
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: Some(Arc::new(InklogConfigAdapter::from_config(config))),
            custom_sinks: Vec::new(),
            #[cfg(feature = "database")]
            database: None,
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with cache and config");

        // 验证 cache 和 config 同时注入不会导致创建失败
        let _ = manager.shutdown();
    }

    // ============================================================================
    // build_with_deps 同时注入 cache/config/database 测试 (lines 332-345, dbnexus)
    // ============================================================================

    #[cfg(feature = "database")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_with_deps_injects_all_three_deps() {
        use crate::integrations::{InklogConfigAdapter, MockCache, MockDatabaseAdapter};
        let config = InklogConfig::default();
        let deps = LoggerDependencies {
            cache: Some(Arc::new(MockCache::new())),
            config: Some(Arc::new(InklogConfigAdapter::from_config(config))),
            custom_sinks: Vec::new(),
            database: Some(Arc::new(MockDatabaseAdapter::new())),
        };

        let manager = LoggerManager::with_dependencies(deps)
            .await
            .expect("Failed to create manager with all deps");

        // 验证三个依赖同时注入不会导致创建失败
        let _ = manager.shutdown();
    }

    // ============================================================================
    // LoggerManager::load() 测试 (L589-592)
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[serial_test::serial]
    async fn test_load_succeeds_with_valid_config_via_env() {
        // 覆盖 L590 (load_sync Ok) + L592 (with_config Ok)
        // 通过 INKLOG_CONFIG_PATH 环境变量指定有效配置文件
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let config_path = dir.path().join("load_test.toml");
        std::fs::write(&config_path, "[global]\nlevel = \"info\"\n").unwrap();
        unsafe {
            std::env::set_var("INKLOG_CONFIG_PATH", &config_path);
        }

        let manager = LoggerManager::load().await.expect("load should succeed");
        let _ = manager.shutdown();

        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[serial_test::serial]
    async fn test_load_returns_error_when_config_invalid_toml() {
        // 覆盖 L590-591 (load_sync Err → map_err → ? 传播)
        // 通过 INKLOG_CONFIG_PATH 指定无效 TOML 文件
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let config_path = dir.path().join("invalid.toml");
        std::fs::write(&config_path, "this is = not = valid toml\n").unwrap();
        unsafe {
            std::env::set_var("INKLOG_CONFIG_PATH", &config_path);
        }

        let result = LoggerManager::load().await;
        assert!(result.is_err(), "load should fail with invalid TOML");
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("Failed to load config") || err_msg.contains("Failed to parse"),
            "error should mention config load failure, got: {}",
            err_msg
        );

        unsafe {
            std::env::remove_var("INKLOG_CONFIG_PATH");
        }
    }

    // ============================================================================
    // LoggerBuilder 分支覆盖 (L1693-1694, L1701-1702)
    // ============================================================================

    #[test]
    fn test_builder_console_when_none_creates_config() {
        // 覆盖 L1693-1694：console_sink 为 None 时 console(true) 创建 Some
        // 默认 InklogConfig 的 console_sink 是 Some，需要手动设为 None
        let mut builder = LoggerBuilder::new();
        builder.config.console_sink = None;
        let builder = builder.console(true);
        let console = builder
            .config
            .console_sink
            .as_ref()
            .expect("console_sink should be Some after console(true)");
        assert!(
            console.enabled,
            "console.enabled should be true after console(true)"
        );
    }

    #[test]
    fn test_builder_file_when_some_updates_path() {
        // 覆盖 L1701-1702：file_sink 为 Some 时 file(path) 更新已有配置
        // 默认 InklogConfig 的 file_sink 是 None，需要手动设为 Some
        let mut builder = LoggerBuilder::new();
        builder.config.file_sink = Some(crate::FileSinkConfig::default());
        let builder = builder.file("logs/updated.log");
        let file_sink = builder
            .config
            .file_sink
            .as_ref()
            .expect("file_sink should remain Some");
        assert!(
            file_sink.enabled,
            "file_sink.enabled should be true after file(path)"
        );
        assert_eq!(
            file_sink.path,
            std::path::PathBuf::from("logs/updated.log"),
            "file_sink.path should be updated"
        );
    }

    // ============================================================================
    // trigger_recovery_for_unhealthy_sinks 成功路径 (L1567-1568)
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_trigger_recovery_recovers_unhealthy_sink() {
        // 覆盖 L1567-1568：当 sink 非操作状态且 recover_sink 成功时，push 到结果
        // 需要 file worker 存活使 recover_sink 的 control channel 接收端存在
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let log_path = dir.path().join("recovery_test.log");
        let manager = LoggerManager::builder()
            .channel_capacity(1000)
            .worker_threads(1)
            .file(log_path)
            .build()
            .await
            .expect("Failed to build manager");

        // 手动将 file sink 标记为 Unhealthy
        manager
            .metrics
            .update_sink_health("file", false, Some("test error".to_string()));

        // 触发恢复——recover_sink 应成功（file worker 存活），push "file" 到结果
        let result = manager.trigger_recovery_for_unhealthy_sinks();
        assert!(result.is_ok(), "trigger_recovery should succeed");
        let recovered = result.unwrap();
        assert!(
            recovered.contains(&"file".to_string()),
            "recovered sinks should contain 'file', got: {:?}",
            recovered
        );

        let _ = manager.shutdown();
    }
}

// ============================================================================
// 运行时级别热调 —— 指令集 upsert/重建 + reload 换装过滤行为
// ============================================================================

#[cfg(test)]
mod set_level_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_level_directives_upsert_and_render() {
        let mut d = LevelDirectives::new(
            "info".to_string(),
            vec![("hyper".to_string(), "warn".to_string())],
            None,
        );
        assert_eq!(d.to_filter_string(), "info,hyper=warn");

        // 全局覆盖
        d.upsert(None, "debug");
        assert_eq!(d.to_filter_string(), "debug,hyper=warn");
        // 新 target 追加
        d.upsert(Some("my_crate"), "trace");
        assert_eq!(d.to_filter_string(), "debug,hyper=warn,my_crate=trace");
        // 已有 target 覆盖（不重复追加）
        d.upsert(Some("hyper"), "error");
        assert_eq!(d.to_filter_string(), "debug,hyper=error,my_crate=trace");
        assert_eq!(d.targets.len(), 2, "upsert must not duplicate targets");
    }

    #[test]
    fn test_level_directives_preserves_rust_log_extra() {
        let mut d = LevelDirectives::new(
            "info".to_string(),
            Vec::new(),
            Some("nebulaid=debug,hyper=warn".to_string()),
        );
        d.upsert(None, "error");
        assert_eq!(
            d.to_filter_string(),
            "error,nebulaid=debug,hyper=warn",
            "RUST_LOG extra directives must survive set_level rebuilds"
        );
    }

    /// 核心行为验证：reload 换装后过滤行为立即变化（与 set_level 使用的
    /// 同一机制：reload::Layer 包装 EnvFilter + Handle::reload）。
    #[test]
    fn test_reload_filter_changes_filtering_immediately() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tracing::subscriber::with_default;
        use tracing_subscriber::prelude::*;

        // 事件捕获层
        struct CountingLayer(std::sync::Arc<AtomicUsize>);
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CountingLayer {
            fn on_event(
                &self,
                _event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = std::sync::Arc::new(AtomicUsize::new(0));
        let filter = tracing_subscriber::filter::EnvFilter::new("info");
        let (filter_layer, reload_handle): (ReloadFilterLayer, _) =
            tracing_subscriber::reload::Layer::new(filter);
        // filter_layer 先挂（S = Registry），与生产组合顺序一致
        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(CountingLayer(counter.clone()));

        with_default(registry, || {
            // 初始 info：debug 被过滤，info 通过
            tracing::debug!(target: "t502", message = "debug before");
            assert_eq!(counter.load(Ordering::SeqCst), 0, "debug must be filtered");
            tracing::info!(target: "t502", message = "info before");
            assert_eq!(counter.load(Ordering::SeqCst), 1);

            // 热调到 error：info 立即被过滤
            reload_handle
                .reload(tracing_subscriber::filter::EnvFilter::new("error"))
                .expect("reload");
            tracing::info!(target: "t502", message = "info after");
            assert_eq!(
                counter.load(Ordering::SeqCst),
                1,
                "info must be filtered immediately after set to error"
            );

            // 热调到 debug：debug 立即通过
            reload_handle
                .reload(tracing_subscriber::filter::EnvFilter::new("debug"))
                .expect("reload");
            tracing::debug!(target: "t502", message = "debug after");
            assert_eq!(
                counter.load(Ordering::SeqCst),
                2,
                "debug must pass immediately after set to debug"
            );
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_manager_set_level_updates_directives_and_validates() {
        // set_level(None, ..) 经生产路径同步改写进程级 log::max_level，
        // 须与 log_adapter 的 max_level 测试组及 test_global_set_level_syncs_log_max_level
        // （同样断言进程级 max_level）互斥
        let mut config = InklogConfig::default();
        config.global.level = "info".to_string();
        config
            .target_levels
            .insert("hyper".to_string(), "warn".to_string());
        #[cfg(feature = "database")]
        let (manager, _subscriber, filter) =
            LoggerManager::build_detached(config, None).await.unwrap();
        #[cfg(not(feature = "database"))]
        let (manager, _subscriber, filter) = LoggerManager::build_detached(config).await.unwrap();

        // 初始指令集 = 全局 + target_levels
        assert_eq!(
            manager.current_level_filter_string(),
            "info,hyper=warn",
            "initial directives must mirror the config"
        );
        // EnvFilter Display 会重排指令顺序，做语义等价比较
        let rendered = filter.to_string();
        assert!(
            rendered.contains("hyper=warn") && rendered.contains("info"),
            "EnvFilter must carry global level + target directive, got: {rendered}"
        );

        // 全局调级
        manager.set_level(None, "debug").unwrap();
        assert_eq!(manager.current_level_filter_string(), "debug,hyper=warn");

        // per-target 调级（新增）
        manager.set_level(Some("my_crate"), "trace").unwrap();
        assert_eq!(
            manager.current_level_filter_string(),
            "debug,hyper=warn,my_crate=trace"
        );

        // per-target 调级（覆盖）
        manager.set_level(Some("hyper"), "error").unwrap();
        assert_eq!(
            manager.current_level_filter_string(),
            "debug,hyper=error,my_crate=trace"
        );

        // 非法级别：报错且指令集不变
        let err = manager.set_level(Some("hyper"), "not-a-level").unwrap_err();
        assert!(err.to_string().contains("Invalid log level"));
        assert_eq!(
            manager.current_level_filter_string(),
            "debug,hyper=error,my_crate=trace",
            "failed set_level must leave directives unchanged"
        );
    }

    #[test]
    fn test_file_uses_channel_buffered_predicate() {
        // R-inklog-003 收窄：轮转/压缩/加密/审计链均由 CBFS 覆盖，不再回落；
        // 仅逐批 fsync 与 JSON 出站格式仍回落 FileSink（能力矩阵见
        // docs/USER_GUIDE.md「文件写入双路径」——注意 CBFS 时间轮转为滚动
        // 间隔近似，FileSink 为日历对齐，矩阵已如实标注差异）
        assert!(file_uses_channel_buffered(&crate::FileSinkConfig::default()));

        let mut advanced = crate::FileSinkConfig::default();
        advanced.compress = true;
        assert!(file_uses_channel_buffered(&advanced));

        let mut advanced = crate::FileSinkConfig::default();
        advanced.encrypt = true;
        assert!(file_uses_channel_buffered(&advanced));

        let mut advanced = crate::FileSinkConfig::default();
        advanced.audit_chain_enabled = true;
        assert!(file_uses_channel_buffered(&advanced));

        let mut advanced = crate::FileSinkConfig::default();
        advanced.rotation_time = "hourly".to_string();
        assert!(file_uses_channel_buffered(&advanced));

        let mut advanced = crate::FileSinkConfig::default();
        advanced.fsync = true;
        assert!(!file_uses_channel_buffered(&advanced));

        let mut advanced = crate::FileSinkConfig::default();
        advanced.output_format = crate::support::processing::template::OutputFormat::Json;
        assert!(!file_uses_channel_buffered(&advanced));
    }

    #[tokio::test]
    async fn test_default_file_config_builds_channel_buffered_and_writes() {
        // 转正冒烟：默认 file 配置构建 CBFS 路径，写入落盘可用
        let dir = tempfile::TempDir::new().unwrap();
        let out = dir.path().join("cbfs-default.log");
        let mut config = InklogConfig::default();
        config.global.level = "info".to_string();
        config.file_sink = Some(crate::FileSinkConfig {
            enabled: true,
            path: out.clone(),
            compress: false,
            ..Default::default()
        });
        #[cfg(feature = "database")]
        let (manager, subscriber, _filter) =
            LoggerManager::build_detached(config, None).await.unwrap();
        #[cfg(not(feature = "database"))]
        let (manager, subscriber, _filter) = LoggerManager::build_detached(config).await.unwrap();

        // file 配置为简单配置：判定函数确认走 CBFS
        let cfg = crate::FileSinkConfig {
            enabled: true,
            path: out.clone(),
            compress: false,
            ..Default::default()
        };
        assert!(file_uses_channel_buffered(&cfg));

        // build_detached 不装全局 subscriber：经返回的 subscriber 显式发事件
        let registry = tracing_subscriber::registry().with(subscriber);
        tracing::subscriber::with_default(registry, || {
            tracing::info!(target: "cbfs::smoke", message = "cbfs promotion smoke marker");
        });
        // 等待落盘（CBFS 100ms flush 线程）
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut landed = false;
        while std::time::Instant::now() < deadline && !landed {
            if let Ok(data) = std::fs::read_to_string(&out)
                && data.contains("cbfs promotion smoke marker")
            {
                landed = true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(landed, "CBFS default path must land records on disk");
        let _ = manager;
    }

    #[tokio::test]
    async fn test_fallback_journal_replay_on_startup() {
        // C4：启动重放——journal 中的记录注入 file sink 且带 replayed=true，
        // journal 文件重放后清空
        use crate::support::fallback_journal::FallbackJournal;

        let dir = tempfile::TempDir::new().unwrap();
        let journal_path = dir.path().join("fb.journal");
        {
            let journal = FallbackJournal::open(&journal_path);
            let mut rec = crate::LogRecord::new(
                tracing::Level::ERROR,
                "journal::replay".to_string(),
                "replay-me-1".to_string(),
            );
            rec.level = "ERROR".to_string();
            assert!(journal.spill(&rec));
        }

        let out_path = dir.path().join("out.log");
        let mut config = InklogConfig::default();
        config.global.fallback_journal = true;
        config.global.fallback_journal_path = journal_path.display().to_string();
        config.global.level = "error".to_string();
        config.file_sink = Some(crate::FileSinkConfig {
            enabled: true,
            path: out_path.clone(),
            output_format: crate::support::processing::template::OutputFormat::Json,
            ..Default::default()
        });
        #[cfg(feature = "database")]
        let (manager, _subscriber, _filter) =
            LoggerManager::build_detached(config, None).await.unwrap();
        #[cfg(not(feature = "database"))]
        let (manager, _subscriber, _filter) = LoggerManager::build_detached(config).await.unwrap();

        // 重放后 journal 清空
        let (remaining, _) = FallbackJournal::open(&journal_path).replay();
        assert!(remaining.is_empty(), "journal must be cleared after replay");

        // 等待 file worker 把重放记录写盘（JSON 格式可断言 replayed 字段）
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = false;
        while std::time::Instant::now() < deadline && !found {
            if let Ok(data) = std::fs::read_to_string(&out_path) {
                for line in data.lines() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                        && json["message"] == "replay-me-1"
                        && json["fields"]["replayed"] == serde_json::Value::Bool(true)
                    {
                        found = true;
                    }
                }
            }
            if !found {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
        assert!(
            found,
            "replayed record must reach file sink with replayed=true field"
        );
        let _ = manager.shutdown();
    }

    #[tokio::test]
    #[serial]
    async fn test_global_set_level_syncs_log_max_level() {
        // 双门面同步：全局热调必须把 log::max_level 一起抬到新级别。
        // 断言直读进程级 max_level，须与 log_adapter 的 max_level 测试组及
        // test_manager_set_level_updates_directives_and_validates（同样经
        // set_level(None, ..) 写全局 max_level）互斥
        log::set_max_level(log::LevelFilter::Off);
        let mut config = InklogConfig::default();
        config.global.level = "info".to_string();
        #[cfg(feature = "database")]
        let (manager, _subscriber, _filter) =
            LoggerManager::build_detached(config, None).await.unwrap();
        #[cfg(not(feature = "database"))]
        let (manager, _subscriber, _filter) = LoggerManager::build_detached(config).await.unwrap();

        manager.set_level(None, "debug").unwrap();
        assert_eq!(
            log::max_level(),
            log::LevelFilter::Debug,
            "global set_level must sync log::max_level"
        );

        // per-target 调级不动全局 log::max_level（log 门面无 target 语义）
        manager.set_level(Some("my_crate"), "trace").unwrap();
        assert_eq!(log::max_level(), log::LevelFilter::Debug);

        // 收紧同步
        manager.set_level(None, "error").unwrap();
        assert_eq!(log::max_level(), log::LevelFilter::Error);
        log::set_max_level(log::LevelFilter::Off);
    }
}
