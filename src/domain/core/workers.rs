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
    /// Console sink（生产中从不被替换，直接共享句柄，无锁）
    pub(crate) console_sink: Arc<dyn LogSink>,
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
    /// 动态注册的第三方 sink：每项拥有独立 channel 接收端，
    /// 由通用 SinkWorker 消费。每个条目独立命名用于健康上报。
    pub(crate) custom_sinks: Vec<CustomSinkEntry>,
}

/// 动态注册 sink 的 worker 条目。
pub(crate) struct CustomSinkEntry {
    /// 健康上报与指标使用的 sink 名（"custom-N"）
    pub(crate) name: String,
    /// 第三方 sink 实例（零核心改动接入）
    pub(crate) sink: Arc<dyn LogSink>,
    /// 该 sink 专属 channel 的接收端
    pub(crate) receiver: Receiver<Arc<LogRecord>>,
}

/// `start_workers` 返回值类型别名，避免 clippy `type_complexity` 警告。
/// 第一项为 worker 线程句柄，第二项为每个 worker 对应的 shutdown 信号 sender。
pub(crate) type WorkerStartResult =
    Result<(Vec<tokio::task::JoinHandle<()>>, Vec<Sender<()>>), InklogError>;

/// 自定义 sink 写失败重试上限（与内置 worker 的 WRITE_MAX_ATTEMPTS 一致）。
const CUSTOM_SINK_WRITE_ATTEMPTS: u32 = 3;

/// 通用 SinkWorker 主循环，消费动态注册 sink 的专属 channel。
///
/// 语义与内置 file/db worker 对齐：
/// - 记录产生→处理的延迟计入 metrics；
/// - 写失败重试（最多 3 次，间隔 10ms×attempt），重试耗尽计 sink_error 并
///   降级写 console，绝不无声丢弃；
/// - shutdown 信号后限时排水，最后 flush + shutdown sink；
/// - channel 断开（所有发送端丢弃）且排空后 worker 退出。
pub(crate) fn run_custom_sink_worker(
    runtime_handle: &tokio::runtime::Handle,
    metrics: &Metrics,
    console_sink: &Arc<dyn LogSink>,
    entry: &CustomSinkEntry,
    receiver: &Receiver<Arc<LogRecord>>,
    shutdown: &Receiver<()>,
) {
    loop {
        if shutdown.try_recv().is_ok() {
            // Drain with 5s timeout
            let deadline = Instant::now() + Duration::from_secs(5);
            while let Ok(record) = receiver.try_recv() {
                metrics.record_latency(record_age(&record));
                let _ = runtime_handle.block_on(async { entry.sink.write(&record).await });
                if Instant::now() > deadline {
                    break;
                }
            }
            let _ = runtime_handle.block_on(async { entry.sink.flush().await });
            let _ = runtime_handle.block_on(async { entry.sink.shutdown().await });
            break;
        }

        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(record) => {
                metrics.record_latency(record_age(&record));
                let mut written = false;
                for attempt in 1..=CUSTOM_SINK_WRITE_ATTEMPTS {
                    match runtime_handle.block_on(async { entry.sink.write(&record).await }) {
                        Ok(_) => {
                            metrics.inc_logs_written();
                            metrics.update_sink_health(&entry.name, true, None);
                            written = true;
                            break;
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                attempt,
                                "custom sink '{}' write failed",
                                entry.name
                            );
                            if attempt == CUSTOM_SINK_WRITE_ATTEMPTS {
                                metrics.inc_sink_error();
                                metrics.update_sink_health(
                                    &entry.name,
                                    false,
                                    Some(e.to_string()),
                                );
                                let _ = runtime_handle
                                    .block_on(async { console_sink.write(&record).await });
                            } else {
                                thread::sleep(Duration::from_millis(10 * attempt as u64));
                            }
                        }
                    }
                }
                let _ = written;
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Idle tick: flush buffered records periodically
                let _ = runtime_handle.block_on(async { entry.sink.flush().await });
            }
            // 记录发送端全部丢弃且通道已空：worker 退出（与内置 worker 一致，
            // 避免 tokio Runtime drop 等待 blocking 任务时永久挂死）
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
}

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

/// sink 写失败重试上限（含首次写入）：达到该次数后计 sink_error、
/// 健康置 false 并降级写 console。
const WRITE_MAX_ATTEMPTS: u32 = 3;

/// 同类 sink worker 间的静态差异：健康/降级记录使用的名字与文案 key。
struct SinkWorkerDescriptor {
    /// 健康状态与降级记录使用的 sink 名
    name: &'static str,
    /// 日志文案中的 sink 标签
    label: &'static str,
    /// error.log 记录的 target
    error_target: &'static str,
    /// 手动恢复（控制消息）文案 key
    recovery_received_key: &'static str,
    recovered_key: &'static str,
    recovery_failed_key: &'static str,
    /// 自动恢复文案 key
    auto_recovery_key: &'static str,
    auto_recovery_ok_key: &'static str,
}

const FILE_SINK_WORKER: SinkWorkerDescriptor = SinkWorkerDescriptor {
    name: "file",
    label: "File",
    error_target: "inklog::file_sink",
    recovery_received_key: "sink-file_recovery_received",
    recovered_key: "sink-file_recovered",
    recovery_failed_key: "sink-file_recovery_failed",
    auto_recovery_key: "sink-file_auto_recovery",
    auto_recovery_ok_key: "sink-file_auto_recovery_ok",
};

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "duckdb"
))]
const DB_SINK_WORKER: SinkWorkerDescriptor = SinkWorkerDescriptor {
    name: "database",
    label: "Database",
    error_target: "inklog::database_sink",
    recovery_received_key: "sink-db_recovery_received",
    recovered_key: "sink-db_recovered",
    recovery_failed_key: "sink-db_recovery_failed",
    auto_recovery_key: "sink-db_auto_recovery",
    auto_recovery_ok_key: "sink-db_auto_recovery_ok",
};

/// sink 工厂的动态引用（初始创建/工厂重试/恢复共用）。
type SinkFactory<'a> = &'a mut dyn FnMut() -> Result<Box<dyn LogSink>, InklogError>;

/// 跨记录维护的写失败状态（驱动重试计数与自动恢复判定）。
#[derive(Default)]
struct FailureState {
    consecutive_failures: u32,
    last_failure_time: Option<Instant>,
}

/// 单个 sink worker 的跨记录可变状态。
struct SinkWorkerState {
    /// 当前 sink；None 表示降级模式（工厂持续失败）
    sink: Option<Box<dyn LogSink>>,
    failures: FailureState,
    /// 工厂重试退避（1s 起指数翻倍，上限 30s）
    factory_backoff: Duration,
    last_factory_attempt: Instant,
}

/// sink worker 共享上下文：drain/主循环共用的重试、降级与恢复逻辑。
struct SinkWorker<'a> {
    desc: &'static SinkWorkerDescriptor,
    runtime_handle: &'a tokio::runtime::Handle,
    metrics: &'a Metrics,
    console_sink: &'a Arc<dyn LogSink>,
    error_sink: &'a Arc<Mutex<Option<Arc<dyn LogSink>>>>,
}

/// 记录从产生到被处理的延迟。
fn record_age(record: &LogRecord) -> Duration {
    Utc::now()
        .signed_duration_since(record.timestamp)
        .to_std()
        .unwrap_or(Duration::ZERO)
}

impl SinkWorker<'_> {
    /// 初始创建 sink；失败进入降级模式（指数退避持续重试工厂，
    /// 期间到达的记录写入 error sink 并计为 failed），worker 保持存活。
    fn create_initial_state(&self, create_sink: SinkFactory<'_>) -> SinkWorkerState {
        let sink = match create_sink() {
            Ok(sink) => Some(sink),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "{} sink factory failed on startup; entering degraded retry mode (exponential backoff: 1s doubling up to 30s, retrying indefinitely); records arriving during retry are forwarded to the error sink and counted as failed",
                    self.desc.label
                );
                self.metrics
                    .update_sink_health(self.desc.name, false, Some(e.to_string()));
                None
            }
        };
        SinkWorkerState {
            sink,
            failures: FailureState::default(),
            factory_backoff: factory_retry_initial_backoff(),
            last_factory_attempt: Instant::now(),
        }
    }

    /// 处理单条记录：记录延迟；降级模式保底写 error sink；
    /// 否则带重试写入，最终失败后触发自动恢复。
    fn handle_record(
        &self,
        state: &mut SinkWorkerState,
        record: &Arc<LogRecord>,
        create_sink: SinkFactory<'_>,
    ) {
        self.metrics.record_latency(record_age(record));

        // 降级模式（工厂持续失败）：无 sink 可写——保底到
        // error sink 并计为 failed，绝不无声丢弃
        let Some(sink) = state.sink.as_mut() else {
            self.handle_sink_unavailable(record);
            return;
        };

        let write_succeeded = self.write_with_retry(sink, record, &mut state.failures);
        if !write_succeeded {
            self.maybe_auto_recover(&mut state.failures, sink, create_sink);
        }
    }

    /// 降级模式（工厂持续失败）下到达记录的降级处理：
    /// 尽力写入 error sink 保留内容，并递增 failed/dropped 指标与 sink 健康状态，
    /// 绝不无声丢弃。锁内仅取句柄，异步写在锁外执行（与热路径约定一致）。
    fn handle_sink_unavailable(&self, record: &Arc<LogRecord>) {
        self.metrics.inc_sink_error();
        self.metrics.inc_logs_dropped();
        self.metrics.update_sink_health(
            self.desc.name,
            false,
            Some("sink unavailable: factory keeps failing".to_string()),
        );
        let error_sink_handle = self.error_sink.lock().ok().and_then(|guard| guard.clone());
        if let Some(error_sink) = error_sink_handle {
            let _ = self
                .runtime_handle
                .block_on(async { error_sink.write(record).await });
        }
    }

    /// 单条记录的带重试写入；返回是否最终成功。
    /// 契约：最多 3 次尝试，失败间隔 sleep(10ms×attempt)；第 3 次失败后
    /// 计 sink_error、健康置 false 并降级写 console。
    fn write_with_retry(
        &self,
        sink: &mut Box<dyn LogSink>,
        record: &Arc<LogRecord>,
        failures: &mut FailureState,
    ) -> bool {
        let mut attempts = 0;
        while attempts < WRITE_MAX_ATTEMPTS {
            match self.runtime_handle.block_on(async { sink.write(record).await }) {
                Ok(_) => {
                    self.metrics.inc_logs_written();
                    self.metrics.update_sink_health(self.desc.name, true, None);
                    failures.consecutive_failures = 0;
                    failures.last_failure_time = None;
                    return true;
                }
                Err(e) => {
                    attempts += 1;
                    failures.consecutive_failures += 1;
                    failures.last_failure_time = Some(Instant::now());

                    self.write_error_log(&e);

                    if attempts == WRITE_MAX_ATTEMPTS {
                        self.metrics.inc_sink_error();
                        self.metrics
                            .update_sink_health(self.desc.name, false, Some(e.to_string()));
                        self.fallback_to_console(record);
                    } else {
                        thread::sleep(Duration::from_millis(10 * attempts as u64));
                    }
                }
            }
        }
        false
    }

    /// 写 error.log。锁内仅取句柄，异步写在锁外执行。
    fn write_error_log(&self, error: &InklogError) {
        let error_sink_handle = self.error_sink.lock().ok().and_then(|guard| guard.clone());
        let Some(error_sink) = error_sink_handle else {
            return;
        };
        let error_record = LogRecord {
            timestamp: Utc::now(),
            level: "ERROR".to_string(),
            target: self.desc.error_target.to_string(),
            message: format!("{} sink error: {}", self.desc.label, error),
            fields: Default::default(),
            file: None,
            line: None,
            thread_id: thread::current().name().unwrap_or("unknown").to_string(),
            trace_id: None,
            span_id: None,
        };
        let _ = self
            .runtime_handle
            .block_on(async { error_sink.write(&error_record).await });
    }

    /// 重试耗尽后的 console 降级。
    fn fallback_to_console(&self, record: &Arc<LogRecord>) {
        let _ = self
            .runtime_handle
            .block_on(async { self.console_sink.write(record).await });
    }

    /// 写失败后的自动恢复触发（连续失败 > 5 且距上次失败 > 60s）。
    fn maybe_auto_recover(
        &self,
        failures: &mut FailureState,
        sink: &mut Box<dyn LogSink>,
        create_sink: SinkFactory<'_>,
    ) {
        if !should_auto_recover(failures.consecutive_failures, failures.last_failure_time) {
            return;
        }
        tracing::warn!("{}", crate::i18n::tr(self.desc.auto_recovery_key));
        if let Ok(new_sink) = create_sink() {
            *sink = new_sink;
            failures.consecutive_failures = 0;
            failures.last_failure_time = None;
            self.metrics.update_sink_health(self.desc.name, true, None);
            tracing::info!("{}", crate::i18n::tr(self.desc.auto_recovery_ok_key));
        }
    }

    /// 处理一条控制消息（每次循环迭代最多一条，与主循环节奏一致）。
    fn handle_control_message(
        &self,
        state: &mut SinkWorkerState,
        msg: &SinkControlMessage,
        create_sink: SinkFactory<'_>,
    ) {
        match classify_control_message(msg, self.desc.name) {
            ControlAction::Recover => {
                tracing::info!("{}", crate::i18n::tr(self.desc.recovery_received_key));
                if let Ok(new_sink) = create_sink() {
                    state.sink = Some(new_sink);
                    state.factory_backoff = factory_retry_initial_backoff();
                    state.failures.consecutive_failures = 0;
                    state.failures.last_failure_time = None;
                    self.metrics.update_sink_health(self.desc.name, true, None);
                    tracing::info!("{}", crate::i18n::tr(self.desc.recovered_key));
                } else {
                    tracing::error!("{}", crate::i18n::tr(self.desc.recovery_failed_key));
                }
            }
            ControlAction::Ignore => {}
        }
    }

    /// 降级模式下按指数退避重试工厂（1s→2s→…上限 30s），持续重试而非放弃。
    /// 用时间判断而非 sleep，循环保持即时响应 shutdown 与控制消息；
    /// 放在 recv 之前，降级路径跳过记录处理时也不会跳过重试。
    fn retry_factory_if_due(&self, state: &mut SinkWorkerState, create_sink: SinkFactory<'_>) {
        if state.sink.is_some() || state.last_factory_attempt.elapsed() < state.factory_backoff {
            return;
        }
        match create_sink() {
            Ok(new_sink) => {
                state.sink = Some(new_sink);
                state.factory_backoff = factory_retry_initial_backoff();
                self.metrics.update_sink_health(self.desc.name, true, None);
                tracing::info!(
                    "{} sink factory succeeded after retry; worker resumed normal writes",
                    self.desc.label
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    next_retry_in_ms = state.factory_backoff.as_millis() as u64,
                    "{} sink factory retry failed; keeping the worker alive and retrying with exponential backoff",
                    self.desc.label
                );
                state.factory_backoff = (state.factory_backoff * 2).min(FACTORY_RETRY_MAX_BACKOFF);
            }
        }
        state.last_factory_attempt = Instant::now();
    }

    /// shutdown 后的排水：限时尽力写完 channel 剩余记录，最后 shutdown sink。
    fn drain(
        &self,
        state: &mut SinkWorkerState,
        receiver: &Receiver<Arc<LogRecord>>,
        timeout: Duration,
        create_sink: SinkFactory<'_>,
    ) {
        let deadline = Instant::now() + timeout;
        while let Ok(record) = receiver.try_recv() {
            self.handle_record(state, &record, create_sink);
            if Instant::now() > deadline {
                break;
            }
        }
        if let Some(sink) = state.sink.as_ref() {
            let _ = self.runtime_handle.block_on(async { sink.shutdown().await });
        }
    }

    /// 空闲超时：flush sink 缓冲（降级模式下无 sink 可 flush）。
    fn flush_idle(&self, state: &SinkWorkerState) {
        if let Some(sink) = state.sink.as_ref() {
            let _ = self.runtime_handle.block_on(async { sink.flush().await });
        }
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
            custom_sinks,
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
                            metrics_console.record_latency(record_age(&record));

                            if runtime_handle
                                .block_on(async { console_sink_console.write(&record).await })
                                .is_err()
                            {
                                metrics_console.inc_sink_error();
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
                            metrics_console.record_latency(record_age(&record));

                            match runtime_handle
                                .block_on(async { console_sink_console.write(&record).await })
                            {
                                Ok(_) => {
                                    metrics_console.inc_logs_written();
                                    metrics_console.update_sink_health("console", true, None);
                                }
                                Err(_) => {
                                    metrics_console.inc_sink_error();
                                    metrics_console.update_sink_health(
                                        "console",
                                        false,
                                        Some("Write error".to_string()),
                                    );
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
                    // 工厂启动失败不退出 worker：进入降级模式（见
                    // create_initial_state / retry_factory_if_due）——error 日志
                    // + 指数退避持续重试工厂，期间到达的记录写入 error sink 并
                    // 计为 failed（handle_sink_unavailable），绝不无声丢弃。
                    let worker = SinkWorker {
                        desc: &FILE_SINK_WORKER,
                        runtime_handle: &runtime_handle,
                        metrics: &metrics_file,
                        console_sink: &console_sink_file,
                        error_sink: &error_sink_file,
                    };
                    let mut create_sink = || file_sink_factory();
                    let mut state = worker.create_initial_state(&mut create_sink);

                    loop {
                        // Check for shutdown
                        if shutdown_file.try_recv().is_ok() {
                            // Drain with 30s timeout
                            worker.drain(&mut state, &rx_file, Duration::from_secs(30), &mut create_sink);
                            break;
                        }

                        // Check for control messages
                        if let Ok(control_msg) = control_rx_file.try_recv() {
                            worker.handle_control_message(&mut state, &control_msg, &mut create_sink);
                        }

                        // 降级模式：工厂重试放在 recv 之前，跳过记录处理时也不会跳过重试
                        worker.retry_factory_if_due(&mut state, &mut create_sink);

                        match rx_file.recv_timeout(Duration::from_millis(100)) {
                            Ok(record) => {
                                worker.handle_record(&mut state, &record, &mut create_sink);
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                                // Timeout, flush buffer
                                worker.flush_idle(&state);
                            }
                            // 记录发送端全部丢弃且通道已空：worker 退出而非空转
                            // （否则 tokio Runtime drop 等待 blocking 任务时永久挂死）
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
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
            tokio::task::spawn_blocking(move || {
                metrics_db.active_workers.inc();
                // database 依赖由调用方（build_detached，async 上下文）保证有效
                if let Some(cfg) = db_config
                    && cfg.enabled
                    && let Some(ref db) = database
                    && let Some(rx_db) = db_receiver
                {
                    let worker = SinkWorker {
                        desc: &DB_SINK_WORKER,
                        runtime_handle: &runtime_handle,
                        metrics: &metrics_db,
                        console_sink: &console_sink_db,
                        error_sink: &error_sink_db,
                    };
                    let mut create_sink = || db_sink_factory(db.clone(), metrics_db.clone());
                    let mut state = worker.create_initial_state(&mut create_sink);

                    loop {
                        // Check for shutdown
                        if shutdown_db.try_recv().is_ok() {
                            // Drain with 30s timeout
                            worker.drain(&mut state, &rx_db, Duration::from_secs(30), &mut create_sink);
                            break;
                        }

                        // Check for control messages
                        if let Ok(control_msg) = control_rx_db.try_recv() {
                            worker.handle_control_message(&mut state, &control_msg, &mut create_sink);
                        }

                        // 降级模式：工厂重试放在 recv 之前，跳过记录处理时也不会跳过重试
                        worker.retry_factory_if_due(&mut state, &mut create_sink);

                        match rx_db.recv_timeout(Duration::from_millis(100)) {
                            Ok(record) => {
                                worker.handle_record(&mut state, &record, &mut create_sink);
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                                // Timeout, flush buffer
                                worker.flush_idle(&state);
                            }
                            // 同 file worker：发送端全部丢弃且通道已空时退出
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                }
                metrics_db.active_workers.dec();
            })
        };


        // dynamic third-party sinks — one generic SinkWorker per entry,
        // each consuming its own dedicated channel (no MPMC contention with
        // the built-in file/db workers).
        let mut custom_handles = Vec::with_capacity(custom_sinks.len());
        let mut custom_shutdown_txs = Vec::with_capacity(custom_sinks.len());
        for entry in custom_sinks {
            let (shutdown_tx, shutdown_rx) = bounded(1);
            let metrics_custom = metrics.clone();
            let console_sink_custom = console_sink.clone();
            let runtime_handle_custom = runtime_handle.clone();
            custom_handles.push(tokio::task::spawn_blocking(move || {
                metrics_custom.active_workers.inc();
                run_custom_sink_worker(
                    &runtime_handle_custom,
                    &metrics_custom,
                    &console_sink_custom,
                    &entry,
                    &entry.receiver,
                    &shutdown_rx,
                );
                metrics_custom.active_workers.dec();
            }));
            custom_shutdown_txs.push(shutdown_tx);
        }

        // Health Check Thread
        let (shutdown_tx_health, shutdown_health) = bounded(1);
        let metrics_health = metrics.clone();
        let effective_capacity_health = effective_capacity.clone();
        let handle_health = tokio::task::spawn_blocking(move || {
            let mut last_recovery_attempt = std::collections::HashMap::<String, Instant>::new();
            let mut low_usage_since: Option<Instant> = None;
            let check_interval = Duration::from_secs(1);

            loop {
                match shutdown_health.recv_timeout(check_interval) {
                    // 正常 shutdown 信号
                    Ok(_) => break,
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                    // 发送端已全部丢弃（如 manager 未调 shutdown 即被丢弃）：
                    // 必须退出，否则 recv_timeout 立即返回 Disconnected，
                    // 本线程全速空转且 tokio Runtime drop 永久等待
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
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
        let mut handles = vec![handle_console, handle_file, handle_db, handle_health];
        #[cfg(not(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        )))]
        let mut handles = vec![handle_console, handle_file, handle_health];
        handles.extend(custom_handles);

        // shutdown_txs 与 handles 一一对应，保持 cfg 一致性
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        let mut shutdown_txs = vec![
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
        let mut shutdown_txs = vec![shutdown_tx_console, shutdown_tx_file, shutdown_tx_health];
        shutdown_txs.extend(custom_shutdown_txs);

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
    // 所有 worker 在 shutdown 广播后均能终止
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
        let console_sink = Arc::new(crate::support::io::ConsoleSink::new(
            config.console_sink.clone().unwrap_or_default(),
            crate::LogTemplate::new(&config.global.format),
        )) as Arc<dyn LogSink>;
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
            custom_sinks: Vec::new(),
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
                            trace_id: None,
                            span_id: None,
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
    fn test_file_worker_failing_factory_counts_failed_and_exits_on_sender_drop() {
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
        let console_sink = Arc::new(crate::support::io::ConsoleSink::new(
            config.console_sink.clone().unwrap_or_default(),
            crate::LogTemplate::new(&config.global.format),
        )) as Arc<dyn LogSink>;
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
            custom_sinks: Vec::new(),
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
                trace_id: None,
                span_id: None,
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
        // 2) 记录发送端全部丢弃且通道排空后，worker 应在有限时间内退出
        //    （即使工厂仍在失败）——而非空转驻留；tokio Runtime drop 会等待
        //    blocking 任务，空转驻留会让整个测试进程永久挂死
        let deadline = Instant::now() + Duration::from_secs(5);
        while !handles[1].is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            handles[1].is_finished(),
            "file worker must exit after all record senders are dropped, even with a failing factory"
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

    // ========================================================================
    // 漂移修复回归：db worker 主循环写失败分支必须与 drain 路径一致地写
    // error.log（"Database sink error: ..."），重试耗尽后计 sink_error。
    // ========================================================================

    /// 捕获写入内容的 error sink（验证 error.log 记录）。
    /// 仅 db worker 测试使用，随其 cfg 门控。
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    struct CapturingSink {
        messages: Mutex<Vec<String>>,
    }

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    #[async_trait::async_trait]
    impl LogSink for CapturingSink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            self.messages.lock().unwrap().push(record.message.clone());
            Ok(())
        }

        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    /// 写入必然失败的 db sink（模拟运行期写库失败）。
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    struct FailingDbSink;

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql", feature = "duckdb"))]
    #[async_trait::async_trait]
    impl LogSink for FailingDbSink {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            Err(InklogError::DatabaseError {
                message: "mock db write failure".to_string(),
                source: None,
            })
        }

        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[cfg(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "duckdb"
    ))]
    #[test]
    fn test_db_worker_write_failure_writes_error_log_in_main_loop() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build test runtime");

        let mut config = InklogConfig::default();
        // db worker 主体受 db_config.enabled 守卫，必须显式启用才会进入主循环
        config.database_sink = Some(crate::domain::config::DatabaseSinkConfig {
            enabled: true,
            ..Default::default()
        });
        let (_file_tx, file_rx) = bounded::<Arc<LogRecord>>(100);
        let (_console_tx, console_rx) = bounded::<Arc<LogRecord>>(100);
        let (db_tx, db_rx) = bounded::<Arc<LogRecord>>(100);
        let (control_tx, control_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let effective_capacity = Arc::new(AtomicUsize::new(100));
        let console_sink = Arc::new(crate::support::io::ConsoleSink::new(
            config.console_sink.clone().unwrap_or_default(),
            crate::LogTemplate::new(&config.global.format),
        )) as Arc<dyn LogSink>;
        let captured = Arc::new(CapturingSink {
            messages: Mutex::new(Vec::new()),
        });
        let error_sink: Arc<Mutex<Option<Arc<dyn LogSink>>>> =
            Arc::new(Mutex::new(Some(captured.clone() as Arc<dyn LogSink>)));

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
            db_sink_factory: Box::new(|_db, _metrics| Ok(Box::new(FailingDbSink) as Box<dyn LogSink>)),
            database: Some(Arc::new(crate::integrations::MockDatabaseAdapter::new())
                as Arc<dyn crate::integrations::Database>),
            db_receiver: Some(db_rx),
            custom_sinks: Vec::new(),
        };

        let (handles, shutdown_txs) = runtime
            .block_on(async { LoggerManager::start_workers(params).expect("start workers") });

        // 向 db channel 投递 N 条记录：主循环写失败应计 sink_error 并写 error.log
        const N: u64 = 3;
        for i in 0..N {
            let record = Arc::new(LogRecord {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                target: "db::write_failure".to_string(),
                message: format!("db write failure record {i}"),
                fields: Default::default(),
                file: None,
                line: None,
                thread_id: "test".to_string(),
                trace_id: None,
                span_id: None,
            });
            db_tx.send(record).expect("send db record");
        }
        drop(db_tx);

        // 等待 worker 完成 3 次重试并计数（退避 10ms+20ms/条）
        let deadline = Instant::now() + Duration::from_secs(5);
        while metrics.sink_errors() < N && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }

        // 1) 重试耗尽后计 sink_error（含 db 主循环，与 file worker 一致）
        assert!(
            metrics.sink_errors() >= N,
            "db main-loop write failures must be counted as sink errors, got: {}",
            metrics.sink_errors()
        );
        // 2) error.log 与 drain 路径一致：主循环失败分支写入 "Database sink error: ..."
        //    （每次失败尝试各写一条，N 条记录 × 3 次尝试）
        let error_messages = captured.messages.lock().unwrap();
        assert!(
            error_messages.len() >= N as usize,
            "db main-loop failures must write error.log, got: {}",
            error_messages.len()
        );
        assert!(
            error_messages
                .iter()
                .all(|m| m.starts_with("Database sink error: ")),
            "error.log records must come from the database sink failure path, got: {:?}",
            *error_messages
        );
        drop(error_messages);

        // 3) shutdown 语义不变
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
        assert!(all_finished, "workers must terminate after shutdown");
    }
}

// =========================================================================
// 通用 SinkWorker（动态注册 sink）单元测试
// =========================================================================

#[cfg(test)]
mod custom_sink_worker_tests {
    use super::*;
    use parking_lot::Mutex;

    /// 捕获写入内容的内存 sink（第三方 sink 的测试替身）。
    struct MemorySink {
        records: Mutex<Vec<String>>,
        fail_times: AtomicUsize,
    }

    impl MemorySink {
        fn new(fail_times: usize) -> Self {
            Self {
                records: Mutex::new(Vec::new()),
                fail_times: AtomicUsize::new(fail_times),
            }
        }
    }

    #[async_trait::async_trait]
    impl LogSink for MemorySink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            if self.fail_times.load(Ordering::SeqCst) > 0 {
                self.fail_times.fetch_sub(1, Ordering::SeqCst);
                return Err(InklogError::ConfigError("transient failure".to_string()));
            }
            self.records.lock().push(record.message.clone());
            Ok(())
        }

        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[test]
    fn test_custom_sink_worker_consumes_records_and_drains_on_shutdown() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();

        let sink = Arc::new(MemorySink::new(0));
        let (tx, rx) = bounded::<Arc<LogRecord>>(100);
        let entry = CustomSinkEntry {
            name: "custom-0".to_string(),
            sink: sink.clone() as Arc<dyn LogSink>,
            receiver: rx,
        };
        let (_shutdown_tx, shutdown_rx) = bounded(1);
        let metrics = Arc::new(Metrics::new());
        let console_sink = Arc::new(crate::support::io::ConsoleSink::new(
            Default::default(),
            crate::LogTemplate::new("{timestamp} [{level}] {target} - {message}"),
        )) as Arc<dyn LogSink>;

        let handle = {
            let metrics = metrics.clone();
            let console = console_sink.clone();
            let runtime_handle = runtime.handle().clone();
            thread::spawn(move || {
                run_custom_sink_worker(
                    &runtime_handle,
                    &metrics,
                    &console,
                    &entry,
                    &entry.receiver,
                    &shutdown_rx,
                );
            })
        };

        // 投递 3 条记录
        for i in 0..3 {
            tx.send(Arc::new(LogRecord {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                target: "custom::sink".to_string(),
                message: format!("custom record {i}"),
                fields: Default::default(),
                file: None,
                line: None,
                thread_id: "test".to_string(),
                trace_id: None,
                span_id: None,
                ..Default::default()
            }))
            .unwrap();
        }

        // 等待消费
        let deadline = Instant::now() + Duration::from_secs(5);
        while sink.records.lock().len() < 3 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(sink.records.lock().len(), 3, "all records must be written");
        assert!(metrics.logs_written() >= 3, "writes must be counted");

        // 记录发送端全部丢弃且通道排空 → worker 退出（Disconnected 路径）
        drop(tx);
        drop(_shutdown_tx);
        handle.join().expect("worker must exit cleanly");
    }

    #[test]
    fn test_custom_sink_worker_retries_and_falls_back_to_console() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();

        // 前 3 次写失败（单条记录的 3 次重试全部耗尽）→ 计 sink_error 并降级 console
        let sink = Arc::new(MemorySink::new(3));
        let (tx, rx) = bounded::<Arc<LogRecord>>(10);
        let entry = CustomSinkEntry {
            name: "custom-fail".to_string(),
            sink: sink.clone() as Arc<dyn LogSink>,
            receiver: rx,
        };
        let (_shutdown_tx, shutdown_rx) = bounded::<()>(1);
        let metrics = Arc::new(Metrics::new());
        let console_sink = Arc::new(crate::support::io::ConsoleSink::new(
            Default::default(),
            crate::LogTemplate::new("t"),
        )) as Arc<dyn LogSink>;

        {
            let metrics = metrics.clone();
            let console = console_sink.clone();
            let runtime_handle = runtime.handle().clone();
            thread::spawn(move || {
                run_custom_sink_worker(
                    &runtime_handle,
                    &metrics,
                    &console,
                    &entry,
                    &entry.receiver,
                    &shutdown_rx,
                );
            });
        }

        tx.send(Arc::new(LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "custom::fail".to_string(),
            message: "doomed record".to_string(),
            fields: Default::default(),
            file: None,
            line: None,
            thread_id: "test".to_string(),
            trace_id: None,
            span_id: None,
            ..Default::default()
        }))
        .unwrap();

        // 等待重试耗尽（10ms + 20ms 间隔）
        let deadline = Instant::now() + Duration::from_secs(5);
        while metrics.sink_errors() < 1 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            metrics.sink_errors() >= 1,
            "exhausted retries must count a sink error"
        );
        assert_eq!(
            sink.records.lock().len(),
            0,
            "record must not be written after all attempts failed"
        );

        drop(tx);
    }
}
