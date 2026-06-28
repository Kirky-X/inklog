// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! S3归档服务
//!
//! 提供定时归档和后台任务管理功能

#[cfg(feature = "aws")]
use super::ArchiveMetadata;
use super::S3ArchiveConfig;
#[cfg(feature = "aws")]
use super::S3ArchiveManager;
use crate::InklogError;
use chrono::{DateTime, Duration, Utc};
#[cfg(feature = "dbnexus")]
use dbnexus::database::pool::Session;
use parking_lot::Mutex;
#[cfg(feature = "dbnexus")]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::mpsc;
use tokio_cron_scheduler::{Job, JobScheduler};
#[cfg(feature = "dbnexus")]
use tracing::debug;
use tracing::{error, info, warn};

/// 归档服务
pub struct ArchiveService {
    config: S3ArchiveConfig,
    #[cfg(feature = "aws")]
    archive_manager: Arc<S3ArchiveManager>,
    #[cfg(not(feature = "aws"))]
    archive_manager: Arc<()>, // 占位符
    #[cfg(feature = "dbnexus")]
    database_session: Option<Arc<Session>>,
    scheduler: JobScheduler,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    /// 调度状态跟踪（用于并发控制和持久化）
    schedule_state: Mutex<super::ScheduleState>,
}

impl ArchiveService {
    /// 创建新的归档服务
    #[cfg(feature = "dbnexus")]
    pub async fn new(
        config: S3ArchiveConfig,
        database_session: Option<Session>,
    ) -> Result<Self, InklogError> {
        #[cfg(feature = "aws")]
        let archive_manager = Arc::new(S3ArchiveManager::new(config.clone()).await?);
        #[cfg(not(feature = "aws"))]
        let archive_manager = Arc::new(());

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        fs::create_dir_all(&config.local_retention_path)
            .await
            .map_err(|e| {
                InklogError::IoError(std::io::Error::other(format!(
                    "Failed to create local retention directory: {}",
                    e
                )))
            })?;

        // 创建调度器
        let scheduler = JobScheduler::new().await.map_err(|e| {
            InklogError::ConfigError(format!("Failed to create job scheduler: {}", e))
        })?;

        Ok(Self {
            config: config.clone(),
            archive_manager,
            database_session: database_session.map(Arc::new),
            scheduler,
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
            schedule_state: Mutex::new(super::ScheduleState::default()),
        })
    }

    /// 创建新的归档服务（非 dbnexus 版本）
    #[cfg(not(feature = "dbnexus"))]
    pub async fn new(config: S3ArchiveConfig) -> Result<Self, InklogError> {
        #[cfg(feature = "aws")]
        let archive_manager = Arc::new(S3ArchiveManager::new(config.clone()).await?);
        #[cfg(not(feature = "aws"))]
        let archive_manager = Arc::new(());

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        fs::create_dir_all(&config.local_retention_path)
            .await
            .map_err(|e| {
                InklogError::IoError(std::io::Error::other(format!(
                    "Failed to create local retention directory: {}",
                    e
                )))
            })?;

        let scheduler = JobScheduler::new().await.map_err(|e| {
            InklogError::ConfigError(format!("Failed to create job scheduler: {}", e))
        })?;

        Ok(Self {
            config: config.clone(),
            archive_manager,
            #[cfg(feature = "dbnexus")]
            database_session: None,
            scheduler,
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
            schedule_state: Mutex::new(super::ScheduleState::default()),
        })
    }

    /// 启动归档服务
    pub async fn start(&mut self) -> Result<(), InklogError> {
        info!("Starting S3 archive service");

        // 将 schedule_state 转换为 Arc 以便在闭包中共享
        let schedule_state: Arc<Mutex<super::ScheduleState>> =
            Arc::new(std::mem::take(&mut self.schedule_state));
        let mut shutdown_rx = self.shutdown_rx.take().ok_or_else(|| {
            InklogError::ConfigError("Shutdown receiver already taken".to_string())
        })?;

        // 克隆 Arc 引用供闭包使用
        let config = self.config.clone();
        let archive_manager = Arc::clone(&self.archive_manager);
        #[cfg(feature = "dbnexus")]
        let db_conn = self.database_session.clone();

        // 预先克隆配置供闭包使用
        let config_for_archive = config.clone();
        let config_for_cleanup = config.clone();

        // 添加归档任务（根据配置选择调度方式）
        #[cfg(feature = "dbnexus")]
        if let Some(cron_expr) = &config.schedule_expression {
            // 使用 cron 表达式调度
            let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
                let archive_manager = Arc::clone(&archive_manager);
                let db_conn = db_conn.clone();
                let config = config_for_archive.clone();
                let schedule_state = schedule_state.clone();
                Box::pin(async move {
                    if let Err(e) = Self::perform_archive_with_deps(
                        &config,
                        &archive_manager,
                        db_conn,
                        &schedule_state,
                    )
                    .await
                    {
                        error!("Archive task failed: {}", e);
                    }
                })
            })
            .map_err(|e| {
                InklogError::ConfigError(format!("Failed to create archive job: {}", e))
            })?;

            self.scheduler.add(job).await.map_err(|e| {
                InklogError::ConfigError(format!("Failed to add archive job: {}", e))
            })?;

            info!("Using cron schedule: {}", cron_expr);
        } else {
            // 使用间隔调度: 每天凌晨 2 点执行 + 程序内日期检查
            let cron_expr = "0 0 2 * * *".to_string(); // 每天 02:00:00
            let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
                let archive_manager = Arc::clone(&archive_manager);
                let db_conn = db_conn.clone();
                let config = config_for_archive.clone();
                let schedule_state = Arc::clone(&schedule_state);
                Box::pin(async move {
                    if let Err(e) = Self::perform_archive_with_deps(
                        &config,
                        &archive_manager,
                        db_conn,
                        &schedule_state,
                    )
                    .await
                    {
                        error!("Archive task failed: {}", e);
                    }
                })
            })
            .map_err(|e| {
                InklogError::ConfigError(format!("Failed to create interval job: {}", e))
            })?;

            self.scheduler.add(job).await.map_err(|e| {
                InklogError::ConfigError(format!("Failed to add interval job: {}", e))
            })?;

            info!(
                "Archive service started with interval: {} days",
                config.archive_interval_days
            );
        }

        #[cfg(not(feature = "dbnexus"))]
        if let Some(cron_expr) = &config.schedule_expression {
            let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
                let archive_manager = Arc::clone(&archive_manager);
                let config = config_for_archive.clone();
                let schedule_state = schedule_state.clone();
                Box::pin(async move {
                    if let Err(e) =
                        Self::perform_archive_simple(&config, &archive_manager, &schedule_state)
                            .await
                    {
                        error!("Archive task failed: {}", e);
                    }
                })
            })
            .map_err(|e| {
                InklogError::ConfigError(format!("Failed to create archive job: {}", e))
            })?;

            self.scheduler.add(job).await.map_err(|e| {
                InklogError::ConfigError(format!("Failed to add archive job: {}", e))
            })?;

            info!("Using cron schedule: {}", cron_expr);
        } else {
            let cron_expr = "0 0 2 * * *".to_string();
            let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
                let archive_manager = Arc::clone(&archive_manager);
                let config = config_for_archive.clone();
                let schedule_state = Arc::clone(&schedule_state);
                Box::pin(async move {
                    if let Err(e) =
                        Self::perform_archive_simple(&config, &archive_manager, &schedule_state)
                            .await
                    {
                        error!("Archive task failed: {}", e);
                    }
                })
            })
            .map_err(|e| {
                InklogError::ConfigError(format!("Failed to create interval job: {}", e))
            })?;

            self.scheduler.add(job).await.map_err(|e| {
                InklogError::ConfigError(format!("Failed to add interval job: {}", e))
            })?;

            info!(
                "Archive service started with interval: {} days",
                config.archive_interval_days
            );
        }

        // 添加每日清理任务
        let cleanup_job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let config = config_for_cleanup.clone();
            Box::pin(async move {
                if let Err(e) = Self::perform_cleanup_with_deps(&config).await {
                    error!("Cleanup task failed: {}", e);
                }
            })
        })
        .map_err(|e| InklogError::ConfigError(format!("Failed to create cleanup job: {}", e)))?;

        self.scheduler
            .add(cleanup_job)
            .await
            .map_err(|e| InklogError::ConfigError(format!("Failed to add cleanup job: {}", e)))?;

        // 启动调度器
        self.scheduler
            .start()
            .await
            .map_err(|e| InklogError::ConfigError(format!("Failed to start scheduler: {}", e)))?;

        // 等待关闭信号
        shutdown_rx.recv().await.ok_or_else(|| {
            InklogError::ChannelError("Failed to receive shutdown signal".to_string())
        })?;

        // 停止调度器
        let _ = self.scheduler.shutdown().await;

        info!("Archive service stopped");
        Ok(())
    }

    /// 执行归档任务（供调度器调用）- 包含并发控制和重试
    #[cfg(feature = "dbnexus")]
    async fn perform_archive_with_deps(
        config: &S3ArchiveConfig,
        archive_manager: &Arc<S3ArchiveManager>,
        db_session: Option<Arc<Session>>,
        schedule_state: &Arc<Mutex<super::ScheduleState>>,
    ) -> Result<(), InklogError> {
        // 并发控制：检查是否可以执行（在锁内）
        let _can_run = {
            let mut state = schedule_state.lock();
            if !state.can_run_today() {
                info!("Archive already running or completed today, skipping");
                return Ok(());
            }
            state.start_execution();
            // 返回需要的信息后释放锁
            state.locked_date
        };

        #[cfg(feature = "aws")]
        {
            use crate::support::io::sink::database::convert_logs_to_parquet;

            let start_date = Utc::now() - Duration::days(config.archive_interval_days as i64);
            let end_date = Utc::now();

            let log_records: Vec<crate::LogRecord> = if let Some(session) = &db_session {
                Self::query_database_records(session, start_date, end_date).await?
            } else {
                Vec::new()
            };

            if log_records.is_empty() {
                debug!("No logs to archive");
                let mut state = schedule_state.lock();
                state.mark_success();
                return Ok(());
            }

            // 根据配置选择归档格式
            let log_data = if config.archive_format.to_lowercase() == "parquet" {
                // 带重试的 Parquet 转换
                Self::retry_with_backoff(|| async {
                    convert_logs_to_parquet(&log_records, &config.parquet_config).map_err(
                        |e: String| {
                            InklogError::SerializationError(serde_json::Error::io(
                                std::io::Error::other(e),
                            ))
                        },
                    )
                })
                .await?
            } else {
                serde_json::to_vec(&log_records).map_err(|e: serde_json::Error| {
                    InklogError::SerializationError(serde_json::Error::io(std::io::Error::other(
                        e.to_string(),
                    )))
                })?
            };

            let metadata = ArchiveMetadata::new(
                log_data.len() as i64,
                log_data.len() as i64,
                "database_logs",
            )
            .with_tag("automated")
            .with_tag("daily");

            // 带重试的 S3 上传
            let result = Self::retry_with_backoff(|| async {
                archive_manager
                    .archive_logs(log_data.clone(), start_date, end_date, metadata.clone())
                    .await
            })
            .await;

            match result {
                Ok(archive_key) => {
                    // 更新状态（在await之后）
                    {
                        let mut state = schedule_state.lock();
                        state.mark_success();
                    }
                    info!("Archived {} logs to S3: {}", log_records.len(), archive_key);
                }
                Err(e) => {
                    // S3上传失败时保存到本地保留目录
                    let local_path = config.local_retention_path.join(format!(
                        "archive_{}_{}_failed.json",
                        start_date.format("%Y%m%d_%H%M%S"),
                        end_date.format("%Y%m%d_%H%M%S")
                    ));

                    warn!(
                        "S3 archive failed, saving to local retention: {} (error: {})",
                        local_path.display(),
                        e
                    );

                    // 尝试保存到本地（先释放锁）
                    let save_result = fs::write(&local_path, &log_data).await;

                    // 更新状态
                    {
                        let mut state = schedule_state.lock();
                        state.mark_failed();
                    }

                    if let Err(local_err) = save_result {
                        error!(
                            "Failed to save archive to local retention {}: {}",
                            local_path.display(),
                            local_err
                        );
                        return Err(InklogError::ArchiveError(format!(
                            "S3 upload failed: {}; Local save also failed: {}",
                            e, local_err
                        )));
                    }

                    info!(
                        "Archive saved to local retention: {} ({} bytes)",
                        local_path.display(),
                        log_data.len()
                    );
                }
            }
        }
        #[cfg(not(feature = "aws"))]
        {
            warn!("AWS feature not enabled, skipping S3 archive");
        }

        Ok(())
    }

    /// 执行归档任务（非 dbnexus 版本）- 简单实现，不包含数据库查询
    #[cfg(not(feature = "dbnexus"))]
    #[cfg(feature = "aws")]
    async fn perform_archive_simple(
        _config: &S3ArchiveConfig,
        _archive_manager: &Arc<S3ArchiveManager>,
        schedule_state: &Arc<Mutex<super::ScheduleState>>,
    ) -> Result<(), InklogError> {
        // 并发控制：检查是否可以执行
        {
            let mut state = schedule_state.lock();
            if !state.can_run_today() {
                info!("Archive already running or completed today, skipping");
                return Ok(());
            }
            state.start_execution();
        }

        // 非 dbnexus 版本只执行清理任务，不进行数据库归档
        warn!("Archive service running without database support - only cleanup will be performed");

        let mut state = schedule_state.lock();
        state.mark_success();

        Ok(())
    }

    #[cfg(not(feature = "dbnexus"))]
    #[cfg(not(feature = "aws"))]
    async fn perform_archive_simple(
        _config: &S3ArchiveConfig,
        _archive_manager: &Arc<()>,
        schedule_state: &Arc<Mutex<super::ScheduleState>>,
    ) -> Result<(), InklogError> {
        warn!("AWS feature not enabled, skipping S3 archive");
        let mut state = schedule_state.lock();
        state.mark_success();
        Ok(())
    }

    /// 指数退避重试辅助函数
    #[cfg(feature = "dbnexus")]
    async fn retry_with_backoff<T, F, Fut>(mut attempt: F) -> Result<T, InklogError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, InklogError>>,
    {
        let mut retries = 0;
        let max_retries = 3;
        let base_delay = std::time::Duration::from_secs(1);

        loop {
            match attempt().await {
                Ok(result) => return Ok(result),
                Err(e) if retries < max_retries => {
                    retries += 1;
                    let delay = base_delay * 2_u32.pow(retries - 1);
                    warn!(
                        "Archive attempt {} failed: {}, retrying in {:?}",
                        retries, e, delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// 执行清理任务（供调度器调用）
    async fn perform_cleanup_with_deps(config: &S3ArchiveConfig) -> Result<(), InklogError> {
        let retention_path = &config.local_retention_path;
        if !retention_path.exists() {
            return Ok(());
        }

        let cutoff = Utc::now() - Duration::days(config.local_retention_days as i64);

        let entries = fs::read_dir(retention_path).await.map_err(|e| {
            InklogError::IoError(std::io::Error::other(format!(
                "Failed to read retention directory: {}",
                e
            )))
        })?;

        let mut entries = Box::pin(entries);
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            InklogError::IoError(std::io::Error::other(format!(
                "Failed to read directory entry: {}",
                e
            )))
        })? {
            let path = entry.path();

            // Defensive check: verify path still exists before deletion
            // This mitigates TOCTOU race conditions
            if !path.exists() {
                continue;
            }

            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Some(modified_date) = DateTime::from_timestamp(
                        modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        0,
                    ) {
                        if modified_date < cutoff {
                            if let Err(e) = fs::remove_file(&path).await {
                                error!("Failed to remove old log file: {}", e);
                            } else {
                                info!("Removed old log file: {:?}", path);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 停止归档服务
    pub async fn stop(&self) -> Result<(), InklogError> {
        self.shutdown_tx
            .send(())
            .await
            .map_err(|_| InklogError::ChannelError("Failed to send shutdown signal".to_string()))?;
        Ok(())
    }

    /// 获取日志数据
    #[cfg(feature = "aws")]
    async fn fetch_log_data(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<u8>, InklogError> {
        #[cfg(feature = "dbnexus")]
        if let Some(ref session) = self.database_session {
            // 从数据库获取日志数据
            return self
                .fetch_database_logs(session, start_date, end_date)
                .await;
        }
        // 从文件系统获取日志数据
        self.fetch_file_logs(start_date, end_date).await
    }

    /// 从数据库获取日志数据
    #[cfg(feature = "dbnexus")]
    async fn fetch_database_logs(
        &self,
        session: &Session,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<u8>, InklogError> {
        let log_records = Self::query_database_records(session, start_date, end_date).await?;
        serde_json::to_vec(&log_records).map_err(|e: serde_json::Error| {
            InklogError::SerializationError(serde_json::Error::io(std::io::Error::other(
                e.to_string(),
            )))
        })
    }

    #[cfg(feature = "dbnexus")]
    async fn query_database_records(
        session: &Session,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<crate::LogRecord>, InklogError> {
        let conn = session
            .connection()
            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
        let models = crate::support::io::sink::entity::Entity::find()
            .filter(crate::support::io::sink::entity::Column::Timestamp.gte(start_date.naive_utc()))
            .filter(crate::support::io::sink::entity::Column::Timestamp.lt(end_date.naive_utc()))
            .order_by_asc(crate::support::io::sink::entity::Column::Timestamp)
            .all(conn)
            .await
            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

        let mut records = Vec::with_capacity(models.len());
        for model in models {
            let fields = match model.fields {
                Some(value) => serde_json::from_str(&value).unwrap_or_default(),
                None => std::collections::HashMap::new(),
            };
            let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(model.timestamp, Utc);
            let line = model
                .line
                .and_then(|value| if value >= 0 { Some(value as u32) } else { None });
            records.push(crate::LogRecord {
                timestamp,
                level: model.level,
                target: model.target,
                message: model.message,
                fields,
                file: model.file,
                line,
                thread_id: model.thread_id,
            });
        }
        Ok(records)
    }

    /// 从文件系统获取日志数据 (异步版本)
    #[cfg(feature = "aws")]
    async fn fetch_file_logs(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<u8>, InklogError> {
        // 假设日志文件存储在 "logs/" 目录下
        let log_dir = PathBuf::from("logs");
        let mut all_data = Vec::new();

        let entries = match fs::read_dir(&log_dir).await {
            Ok(dir) => dir,
            Err(_) => return Ok(Vec::new()),
        };

        let mut entries = Box::pin(entries);
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "log") {
                let metadata = match path.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let modified = match metadata.modified() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let modified_utc: DateTime<Utc> = modified.into();
                if modified_utc >= start_date && modified_utc < end_date {
                    match fs::read(&path).await {
                        Ok(data) => all_data.extend_from_slice(&data),
                        Err(_) => continue,
                    }
                }
            }
        }

        if all_data.is_empty() {
            return Ok(Vec::new());
        }

        // 这里可以将原始日志行转换为 Parquet，或者直接返回
        // 由于 FileSink 记录的是文本，转换会比较复杂，这里先返回原始数据
        // 在生产环境中，应该解析日志行并转换为结构化格式（如 Parquet）
        Ok(all_data)
    }

    /// 估算记录数量
    #[cfg(feature = "aws")]
    fn estimate_record_count(&self, data: &[u8]) -> i64 {
        // 简单的估算：假设每条记录平均100字节
        (data.len() / 100) as i64
    }

    /// 手动触发归档
    pub async fn archive_now(&self) -> Result<String, InklogError> {
        #[cfg(not(feature = "aws"))]
        {
            return Err(InklogError::S3Error(
                "S3 archive is disabled (feature 'aws' not enabled)".to_string(),
            ));
        }

        #[cfg(feature = "aws")]
        {
            let end_date = Utc::now();
            let start_date = end_date - Duration::days(self.config.archive_interval_days as i64);

            let log_data = self.fetch_log_data(start_date, end_date).await?;

            if log_data.is_empty() {
                return Err(InklogError::S3Error("No log data to archive".to_string()));
            }

            let metadata = ArchiveMetadata::new(
                self.estimate_record_count(&log_data),
                log_data.len() as i64,
                "manual_archive",
            )
            .with_tag("manual");

            self.archive_manager
                .archive_logs(log_data, start_date, end_date, metadata)
                .await
        }
    }

    /// 列出归档文件
    pub async fn list_archives(
        &self,
        _start_date: Option<DateTime<Utc>>,
        _end_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<super::ArchiveInfo>, InklogError> {
        #[cfg(not(feature = "aws"))]
        {
            return Err(InklogError::S3Error("S3 archive is disabled".to_string()));
        }

        #[cfg(feature = "aws")]
        self.archive_manager
            .list_archives(_start_date, _end_date, None)
            .await
    }

    /// 恢复归档文件
    pub async fn restore_archive(&self, _key: &str) -> Result<Vec<u8>, InklogError> {
        #[cfg(not(feature = "aws"))]
        {
            return Err(InklogError::S3Error("S3 archive is disabled".to_string()));
        }

        #[cfg(feature = "aws")]
        self.archive_manager.restore_archive(_key).await
    }

    /// 删除归档文件
    pub async fn delete_archive(&self, _key: &str) -> Result<(), InklogError> {
        #[cfg(not(feature = "aws"))]
        {
            return Err(InklogError::S3Error("S3 archive is disabled".to_string()));
        }

        #[cfg(feature = "aws")]
        self.archive_manager.delete_archive(_key).await
    }

    /// 获取S3存储桶名称
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// 获取AWS区域
    pub fn region(&self) -> &str {
        &self.config.region
    }

    /// 获取归档间隔天数
    pub fn archive_interval_days(&self) -> u32 {
        self.config.archive_interval_days
    }

    /// 获取本地保留天数
    pub fn local_retention_days(&self) -> u32 {
        self.config.local_retention_days
    }

    /// 获取压缩类型
    pub fn compression(&self) -> &crate::integrations::storage::archive::CompressionType {
        &self.config.compression
    }

    /// 获取存储类型
    pub fn storage_class(&self) -> &crate::integrations::storage::archive::StorageClass {
        &self.config.storage_class
    }
}

/// 归档服务构建器
pub struct ArchiveServiceBuilder {
    config: Option<S3ArchiveConfig>,
    #[cfg(feature = "dbnexus")]
    database_session: Option<Session>,
}

impl ArchiveServiceBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            config: None,
            #[cfg(feature = "dbnexus")]
            database_session: None,
        }
    }

    /// 设置配置
    pub fn config(mut self, config: S3ArchiveConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// 设置数据库 Session
    #[cfg(feature = "dbnexus")]
    pub fn database_session(mut self, session: Session) -> Self {
        self.database_session = Some(session);
        self
    }

    /// 构建归档服务
    pub async fn build(self) -> Result<ArchiveService, InklogError> {
        let config = self
            .config
            .ok_or_else(|| InklogError::ConfigError("S3 archive config is required".to_string()))?;

        #[cfg(feature = "dbnexus")]
        {
            let session = self.database_session;
            ArchiveService::new(config, session).await
        }
        #[cfg(not(feature = "dbnexus"))]
        {
            ArchiveService::new(config).await
        }
    }

    /// 构建用于测试的归档服务（不初始化 S3 管理器）
    #[cfg(all(test, feature = "aws"))]
    pub async fn build_test(self) -> Result<ArchiveService, InklogError> {
        let config = self
            .config
            .ok_or_else(|| InklogError::ConfigError("S3 archive config is required".to_string()))?;
        let (shutdown_tx, _) = tokio::sync::mpsc::channel(1);

        #[cfg(feature = "dbnexus")]
        {
            Ok(ArchiveService {
                config: config.clone(),
                archive_manager: Arc::new(S3ArchiveManager::new(config.clone()).await?),
                database_session: self.database_session.map(std::sync::Arc::new),
                scheduler: JobScheduler::new().await?,
                shutdown_tx,
                shutdown_rx: None,
                schedule_state: Mutex::new(super::ScheduleState::default()),
            })
        }
        #[cfg(not(feature = "dbnexus"))]
        {
            Ok(ArchiveService {
                config: config.clone(),
                archive_manager: Arc::new(S3ArchiveManager::new(config.clone()).await?),
                scheduler: JobScheduler::new().await?,
                shutdown_tx,
                shutdown_rx: None,
                schedule_state: Mutex::new(super::ScheduleState::default()),
            })
        }
    }
}

impl Default for ArchiveServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::storage::archive::SecretString;
    #[cfg(all(feature = "aws", not(feature = "dbnexus")))]
    use crate::integrations::storage::archive::{ArchiveStatus, ScheduleState};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archive_service_builder() {
        // 测试构建器的基本功能
        let builder = ArchiveServiceBuilder::new();
        assert!(builder.config.is_none());
        #[cfg(feature = "dbnexus")]
        assert!(builder.database_session.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[cfg(feature = "dbnexus")]
    async fn test_fetch_database_logs() {
        use chrono::{Duration, Utc};
        use dbnexus::database::pool::DbPool;
        use sea_orm::{ActiveModelTrait, ConnectionTrait, Schema, Set};

        let pool = DbPool::new("sqlite::memory:").await.unwrap();
        let session = pool.get_session("admin").await.unwrap();
        let conn = session.connection().unwrap();

        let schema = Schema::new(conn.get_database_backend());
        conn.execute(
            schema
                .create_table_from_entity(crate::support::io::sink::entity::Entity)
                .if_not_exists(),
        )
        .await
        .unwrap();

        let now = Utc::now();
        for i in 0..3 {
            let record = crate::support::io::sink::entity::ActiveModel {
                timestamp: Set((now - Duration::hours(i as i64)).naive_utc()),
                level: Set("INFO".to_string()),
                target: Set("test_target".to_string()),
                message: Set(format!("Test message {}", i)),
                fields: Set(Some(r#"{}"#.to_string())),
                file: Set(Some("test.rs".to_string())),
                line: Set(Some(100 + i)),
                thread_id: Set("test-thread".to_string()),
                module_path: Set(Some("test_module".to_string())),
                metadata: Set(Some(r#"{}"#.to_string())),
                ..Default::default()
            };
            record.insert(conn).await.unwrap();
        }

        let config = S3ArchiveConfig {
            enabled: true,
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            archive_interval_days: 1,
            local_retention_days: 7,
            skip_bucket_validation: true,
            ..Default::default()
        };

        let service_session = pool.get_session("admin").await.unwrap();
        let service = ArchiveService::new(config.clone(), Some(service_session))
            .await
            .unwrap();

        let start_date = now - Duration::hours(3);
        let end_date = now;
        let data = service
            .fetch_database_logs(&session, start_date, end_date)
            .await
            .unwrap();

        assert!(!data.is_empty());
    }

    #[test]
    fn test_archive_service_builder_default() {
        let builder = ArchiveServiceBuilder::default();
        assert!(builder.config.is_none());
    }

    #[test]
    fn test_archive_service_builder_config_sets_field() {
        let config = S3ArchiveConfig {
            bucket: "builder-bucket".to_string(),
            ..Default::default()
        };
        let builder = ArchiveServiceBuilder::new().config(config);
        assert!(builder.config.is_some());
        assert_eq!(builder.config.as_ref().unwrap().bucket, "builder-bucket");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archive_service_builder_build_without_config_errors() {
        // build() must short-circuit with a ConfigError before constructing any
        // S3 client, so this test performs no network access.
        let builder = ArchiveServiceBuilder::new();
        let result = builder.build().await;
        // Use `err()` (not `unwrap_err()`) because ArchiveService does not impl Debug.
        let err = result.err().expect("expected build to fail without config");
        assert!(matches!(err, InklogError::ConfigError(_)));
    }

    #[cfg(feature = "aws")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archive_service_builder_build_test_without_config_errors() {
        // build_test() also short-circuits when no config is provided.
        let builder = ArchiveServiceBuilder::new();
        let result = builder.build_test().await;
        // Use `err()` (not `unwrap_err()`) because ArchiveService does not impl Debug.
        let err = result
            .err()
            .expect("expected build_test to fail without config");
        assert!(matches!(err, InklogError::ConfigError(_)));
    }

    // ============================================================================
    // perform_cleanup_with_deps 测试 — 纯文件系统逻辑，无 S3 依赖
    // 覆盖 lines 502-555
    // ============================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_perform_cleanup_nonexistent_dir_returns_ok() {
        // retention_path 不存在时应提前返回 Ok（line 505-507）
        let config = S3ArchiveConfig {
            local_retention_path: PathBuf::from("/nonexistent/cleanup/path/that/does/not/exist"),
            ..Default::default()
        };
        let result = ArchiveService::perform_cleanup_with_deps(&config).await;
        assert!(
            result.is_ok(),
            "Cleanup of non-existent dir should return Ok"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_perform_cleanup_deletes_old_files() {
        // 超过 retention_days 的文件应被删除（lines 509-552）
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let old_file = dir.path().join("old.log");
        std::fs::write(&old_file, "old data").expect("Failed to write file");

        // 将修改时间设为 60 天前（默认 retention 30 天）
        let old_time =
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 24 * 3600);
        let f = std::fs::File::open(&old_file).expect("Failed to open file");
        f.set_modified(old_time)
            .expect("Failed to set modification time");

        let config = S3ArchiveConfig {
            local_retention_days: 30,
            local_retention_path: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = ArchiveService::perform_cleanup_with_deps(&config).await;
        assert!(result.is_ok(), "Cleanup should succeed");
        assert!(!old_file.exists(), "Old file should be deleted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_perform_cleanup_keeps_recent_files() {
        // 未超过 retention_days 的文件应保留（lines 533-550 保留路径）
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let recent_file = dir.path().join("recent.log");
        std::fs::write(&recent_file, "recent data").expect("Failed to write file");
        // 修改时间为 now（刚创建）

        let config = S3ArchiveConfig {
            local_retention_days: 30,
            local_retention_path: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = ArchiveService::perform_cleanup_with_deps(&config).await;
        assert!(result.is_ok(), "Cleanup should succeed");
        assert!(recent_file.exists(), "Recent file should be kept");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_perform_cleanup_handles_unreadable_metadata_gracefully() {
        // 子目录（非文件）应被跳过，不导致错误
        let dir = tempfile::tempdir().expect("Failed to create tempdir");
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).expect("Failed to create subdir");

        let config = S3ArchiveConfig {
            local_retention_days: 30,
            local_retention_path: dir.path().to_path_buf(),
            ..Default::default()
        };
        let result = ArchiveService::perform_cleanup_with_deps(&config).await;
        assert!(result.is_ok(), "Cleanup with subdir should succeed");
        assert!(subdir.exists(), "Subdir should not be deleted");
    }

    // ============================================================================
    // ArchiveService getters 测试 — 需构造实例
    // 使用 skip_bucket_validation + 显式凭证避免 S3 网络调用
    // 覆盖 lines 769-796
    // ============================================================================

    #[cfg(feature = "aws")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_archive_service_getters_return_config_values() {
        let retention_dir =
            tempfile::tempdir().expect("Failed to create tempdir for retention path");
        let config = S3ArchiveConfig {
            enabled: true,
            bucket: "getter-test-bucket".to_string(),
            region: "ap-southeast-1".to_string(),
            archive_interval_days: 14,
            local_retention_days: 60,
            local_retention_path: retention_dir.path().to_path_buf(),
            skip_bucket_validation: true,
            access_key_id: SecretString::from("test-access".to_string()),
            secret_access_key: SecretString::from("test-secret".to_string()),
            ..Default::default()
        };
        #[cfg(feature = "dbnexus")]
        let service = ArchiveService::new(config, None)
            .await
            .expect("Failed to create ArchiveService");
        #[cfg(not(feature = "dbnexus"))]
        let service = ArchiveService::new(config)
            .await
            .expect("Failed to create ArchiveService");

        assert_eq!(service.bucket(), "getter-test-bucket");
        assert_eq!(service.region(), "ap-southeast-1");
        assert_eq!(service.archive_interval_days(), 14);
        assert_eq!(service.local_retention_days(), 60);
    }

    // ============================================================================
    // perform_archive_simple 测试 (aws, not dbnexus)
    // 覆盖 lines 432-457：schedule_state 状态转换逻辑
    // ============================================================================

    #[cfg(all(feature = "aws", not(feature = "dbnexus")))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_perform_archive_simple_success_path() {
        let retention_dir = tempfile::tempdir().expect("Failed to create tempdir");
        let config = S3ArchiveConfig {
            enabled: true,
            bucket: "simple-test-bucket".to_string(),
            region: "us-east-1".to_string(),
            local_retention_path: retention_dir.path().to_path_buf(),
            skip_bucket_validation: true,
            access_key_id: SecretString::from("test-access".to_string()),
            secret_access_key: SecretString::from("test-secret".to_string()),
            ..Default::default()
        };
        let archive_manager = Arc::new(
            S3ArchiveManager::new(config.clone())
                .await
                .expect("Failed to create S3ArchiveManager"),
        );
        let schedule_state = Arc::new(Mutex::new(ScheduleState::default()));

        let result =
            ArchiveService::perform_archive_simple(&config, &archive_manager, &schedule_state)
                .await;
        assert!(result.is_ok(), "perform_archive_simple should succeed");

        // 验证状态已正确更新：成功后 is_running=false
        let state = schedule_state.lock();
        assert!(
            !state.is_running,
            "is_running should be false after success"
        );
        assert_eq!(
            state.last_run_status,
            Some(ArchiveStatus::Success),
            "last_run_status should be Success"
        );
    }

    #[cfg(all(feature = "aws", not(feature = "dbnexus")))]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_perform_archive_simple_skips_when_already_running() {
        // can_run_today() 返回 false 时应跳过执行（lines 442-447）
        let retention_dir = tempfile::tempdir().expect("Failed to create tempdir");
        let config = S3ArchiveConfig {
            enabled: true,
            bucket: "skip-test-bucket".to_string(),
            region: "us-east-1".to_string(),
            local_retention_path: retention_dir.path().to_path_buf(),
            skip_bucket_validation: true,
            access_key_id: SecretString::from("test-access".to_string()),
            secret_access_key: SecretString::from("test-secret".to_string()),
            ..Default::default()
        };
        let archive_manager = Arc::new(
            S3ArchiveManager::new(config.clone())
                .await
                .expect("Failed to create S3ArchiveManager"),
        );
        // 设置 is_running=true 且 locked_date=today → can_run_today()=false
        let schedule_state = Arc::new(Mutex::new(ScheduleState {
            is_running: true,
            locked_date: Some(Utc::now().date_naive()),
            ..Default::default()
        }));

        let result =
            ArchiveService::perform_archive_simple(&config, &archive_manager, &schedule_state)
                .await;
        assert!(result.is_ok(), "should return Ok when skipping");
        // 跳过时不应修改状态
        let state = schedule_state.lock();
        assert!(
            state.is_running,
            "is_running should remain true when skipped"
        );
    }

    // ============================================================================
    // ArchiveServiceBuilder database_session 方法 (cfg dbnexus)
    // 覆盖 line 824
    // ============================================================================

    #[cfg(feature = "dbnexus")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_builder_database_session_sets_field() {
        use dbnexus::database::pool::DbPool;
        let pool = DbPool::new("sqlite::memory:").await.unwrap();
        let session = pool.get_session("admin").await.unwrap();

        let builder = ArchiveServiceBuilder::new().database_session(session);
        assert!(
            builder.database_session.is_some(),
            "database_session should be set"
        );
    }

    // ============================================================================
    // ArchiveServiceBuilder config 链式调用返回 Self 验证
    // ============================================================================

    #[test]
    fn test_builder_config_chaining_preserves_config() {
        let config = S3ArchiveConfig {
            bucket: "chain-bucket".to_string(),
            region: "eu-west-1".to_string(),
            archive_interval_days: 3,
            ..Default::default()
        };
        // config() 应返回 Self 并存储配置
        let builder = ArchiveServiceBuilder::new().config(config);
        let stored = builder.config.as_ref().expect("config should be set");
        assert_eq!(stored.bucket, "chain-bucket");
        assert_eq!(stored.region, "eu-west-1");
        assert_eq!(stored.archive_interval_days, 3);
    }

    #[test]
    fn test_builder_config_overwrite_replaces_previous() {
        // 多次调用 config() 应覆盖前一次
        let builder = ArchiveServiceBuilder::new()
            .config(S3ArchiveConfig {
                bucket: "first".to_string(),
                ..Default::default()
            })
            .config(S3ArchiveConfig {
                bucket: "second".to_string(),
                ..Default::default()
            });
        assert_eq!(
            builder.config.as_ref().unwrap().bucket,
            "second",
            "Second config() should overwrite the first"
        );
    }
}
