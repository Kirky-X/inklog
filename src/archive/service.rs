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
use crate::error::InklogError;
use chrono::{DateTime, Datelike, Duration, Utc};
#[cfg(feature = "dbnexus")]
use dbnexus::pool::Session;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::mpsc;
use tokio_cron_scheduler::{Job, JobScheduler};
#[cfg(feature = "aws")]
use tracing::debug;
use tracing::{error, info, warn};

/// 归档服务
pub struct ArchiveService {
    config: S3ArchiveConfig,
    #[cfg(feature = "aws")]
    archive_manager: Arc<S3ArchiveManager>,
    #[cfg(not(feature = "aws"))]
    #[allow(dead_code)]
    archive_manager: Arc<()>, // 占位符
    #[cfg(feature = "dbnexus")]
    database_session: Option<Arc<Session>>,
    #[allow(dead_code)]
    local_retention_path: PathBuf,
    scheduler: JobScheduler,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    /// 调度状态跟踪（用于并发控制和持久化）
    schedule_state: std::sync::Mutex<super::ScheduleState>,
    /// Parquet配置（用于归档格式）
    parquet_config: crate::config::ParquetConfig,
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

        let local_retention_path = config.local_retention_path.clone();
        fs::create_dir_all(&local_retention_path)
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
            local_retention_path,
            scheduler,
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
            schedule_state: std::sync::Mutex::new(super::ScheduleState::default()),
            parquet_config: config.parquet_config.clone(),
        })
    }

    /// 启动归档服务
    pub async fn start(&mut self) -> Result<(), InklogError> {
        info!("Starting S3 archive service");

        // 将 schedule_state 转换为 Arc 以便在闭包中共享
        let schedule_state: Arc<std::sync::Mutex<super::ScheduleState>> =
            Arc::new(std::mem::take(&mut self.schedule_state));
        let mut shutdown_rx = self.shutdown_rx.take().ok_or_else(|| {
            InklogError::ConfigError("Shutdown receiver already taken".to_string())
        })?;

        // 克隆 Arc 引用供闭包使用
        let config = self.config.clone();
        let archive_manager = Arc::clone(&self.archive_manager);
        let db_conn = self.database_session.clone();

        // 预先克隆配置供闭包使用
        let config_for_archive = config.clone();
        let config_for_cleanup = config.clone();

        // 添加归档任务（根据配置选择调度方式）
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
        schedule_state: &Arc<std::sync::Mutex<super::ScheduleState>>,
    ) -> Result<(), InklogError> {
        // 并发控制：检查是否可以执行（在锁内）
        let _can_run = {
            let mut state = schedule_state.lock().map_err(|e| {
                InklogError::RuntimeError(format!("Failed to acquire schedule lock: {}", e))
            })?;
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
            use crate::sink::database::convert_logs_to_parquet;

            let start_date = Utc::now() - Duration::days(config.archive_interval_days as i64);
            let end_date = Utc::now();

            // Query logs using dbnexus
            let logs: Vec<crate::sink::database::LogEntity> = if let Some(session) = &db_session {
                let sql = r#"
                    SELECT id, timestamp, level, target, message, fields, file, line, thread_id
                    FROM logs
                    WHERE timestamp >= $1 AND timestamp < $2
                    ORDER BY timestamp ASC
                "#;
                // dbnexus execute_paramized uses sea_orm::Value
                let _ = session
                    .execute_paramized(
                        sql,
                        vec![
                            sea_orm::Value::from(start_date.to_rfc3339()),
                            sea_orm::Value::from(end_date.to_rfc3339()),
                        ],
                    )
                    .await
                    .ok();
                Vec::new()
            } else {
                Vec::new()
            };

            if logs.is_empty() {
                debug!("No logs to archive");
                let mut state = schedule_state.lock().map_err(|e| {
                    InklogError::RuntimeError(format!("Failed to acquire schedule lock: {}", e))
                })?;
                state.mark_success();
                return Ok(());
            }

            // 根据配置选择归档格式
            let log_data = if config.archive_format.to_lowercase() == "parquet" {
                // 带重试的 Parquet 转换
                Self::retry_with_backoff(|| async {
                    convert_logs_to_parquet(&logs, &config.parquet_config).map_err(|e| {
                        InklogError::SerializationError(serde_json::Error::io(
                            std::io::Error::other(e.to_string()),
                        ))
                    })
                })
                .await?
            } else {
                serde_json::to_vec(&logs).map_err(|e| {
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

            // 更新状态
            let mut state = schedule_state.lock().map_err(|e| {
                InklogError::RuntimeError(format!("Failed to acquire schedule lock: {}", e))
            })?;

            match result {
                Ok(_) => {
                    state.mark_success();
                    info!("Archived {} logs to S3", logs.len());
                }
                Err(e) => {
                    state.mark_failed();
                    return Err(e);
                }
            }
        }
        #[cfg(not(feature = "aws"))]
        {
            warn!("AWS feature not enabled, skipping S3 archive");
        }

        Ok(())
    }

    /// 指数退避重试辅助函数
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

    #[allow(dead_code)]
    /// 执行归档任务
    async fn perform_archive(&self) -> Result<(), InklogError> {
        #[cfg(not(feature = "aws"))]
        {
            warn!("S3 archive is disabled (feature 'aws' not enabled)");
            Ok(())
        }

        #[cfg(feature = "aws")]
        {
            info!("Starting archive task");

            let end_date = Utc::now();
            let start_date = end_date - Duration::days(self.config.archive_interval_days as i64);

            // 获取需要归档的日志数据
            let log_data = self.fetch_log_data(start_date, end_date).await?;

            if log_data.is_empty() {
                debug!(
                    "No log data to archive for period {} to {}",
                    start_date, end_date
                );
                return Ok(());
            }

            info!("Archiving {} bytes of log data", log_data.len());

            // 创建归档元数据
            let metadata = ArchiveMetadata::new(
                self.estimate_record_count(&log_data),
                log_data.len() as i64,
                "database_logs",
            )
            .with_tag("automated")
            .with_tag("daily");

            // 执行归档
            match self
                .archive_manager
                .archive_logs(log_data.clone(), start_date, end_date, metadata)
                .await
            {
                Ok(archive_key) => {
                    info!("Successfully archived logs to S3: {}", archive_key);
                    Ok(())
                }
                Err(e) => {
                    error!("S3 archive failed: {}. Saving to local retention", e);

                    // Save to local retention directory
                    if let Err(local_err) = self
                        .save_to_local_retention(log_data, start_date, end_date)
                        .await
                    {
                        error!(
                            "Failed to save to local retention: {}. Original S3 error: {}",
                            local_err, e
                        );
                        Err(e) // Return original S3 error
                    } else {
                        info!("Successfully saved archive to local retention");
                        Ok(()) // Consider this a success since we have local retention
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    /// 执行本地数据清理
    async fn perform_cleanup(&self) -> Result<(), InklogError> {
        info!("Starting cleanup task");

        let cutoff_date = Utc::now() - Duration::days(self.config.local_retention_days as i64);

        #[cfg(feature = "dbnexus")]
        if let Some(ref session) = self.database_session {
            self.cleanup_old_database_logs(session, cutoff_date).await?;
        }

        // 清理本地文件（如果配置了文件归档）
        self.cleanup_old_files(cutoff_date).await?;

        info!("Cleanup task completed");
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
    #[allow(dead_code)]
    #[allow(dead_code)]
    /// 从数据库获取日志数据 (dbnexus 模式下不可用)
    ///
    /// 注意：dbnexus 当前版本不支持 SELECT 查询返回结果行。
    /// 此功能在 dbnexus 模式下不可用，返回空结果。
    #[cfg(feature = "dbnexus")]
    async fn fetch_database_logs(
        &self,
        _session: &Session,
        _start_date: DateTime<Utc>,
        _end_date: DateTime<Utc>,
    ) -> Result<Vec<u8>, InklogError> {
        // dbnexus execute_raw/execute_paramized 不返回 SELECT 查询结果行
        // 这是一个已知的限制，需要等待 dbnexus 更新支持查询结果返回
        tracing::warn!("dbnexus does not support SELECT query results yet. Returning empty data.");
        Ok(Vec::new())
    }

    #[allow(dead_code)]
    /// 从数据库获取日志数据 (Sea-ORM 模式下可用)
    ///
    /// 注意：此方法在 dbnexus feature 禁用时不可用。
    /// 数据库归档功能需要 dbnexus feature 才能正常工作。
    #[cfg(not(feature = "dbnexus"))]
    async fn fetch_database_logs(
        &self,
        _session: &sea_orm::DatabaseConnection,
        _start_date: DateTime<Utc>,
        _end_date: DateTime<Utc>,
    ) -> Result<Vec<u8>, InklogError> {
        // 当 dbnexus feature 禁用时，无法从数据库获取日志数据
        // 因为 Sea-ORM 版本的 LogEntity 是空实现，无法正确转换数据
        tracing::warn!(
            "Database log archiving is not available when 'dbnexus' feature is disabled. \
             Enable the 'dbnexus' feature to use database log archiving."
        );
        Err(InklogError::ConfigError(
            "Database log archiving requires 'dbnexus' feature to be enabled".to_string(),
        ))
    }

    /// 将日志模型转换为 Parquet 格式 - 已弃用，使用 sink::database::convert_logs_to_parquet
    #[allow(dead_code)]
    fn convert_to_parquet(
        &self,
        logs: &[crate::sink::database::LogEntity],
    ) -> Result<Vec<u8>, InklogError> {
        use crate::sink::database::convert_logs_to_parquet;
        convert_logs_to_parquet(logs, &self.parquet_config).map_err(|e| {
            InklogError::SerializationError(serde_json::Error::io(std::io::Error::other(
                e.to_string(),
            )))
        })
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

    #[allow(dead_code)]
    /// 清理旧的数据库日志
    #[cfg(feature = "dbnexus")]
    async fn cleanup_old_database_logs(
        &self,
        session: &Session,
        cutoff_date: DateTime<Utc>,
    ) -> Result<(), InklogError> {
        let sql = "DELETE FROM logs WHERE timestamp < $1";

        let _affected = session
            .execute_paramized(sql, vec![sea_orm::Value::from(cutoff_date.to_rfc3339())])
            .await
            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

        info!("Cleaned up old database log records");
        Ok(())
    }

    #[allow(dead_code)]
    /// 清理旧的日志文件（异步版本）
    async fn cleanup_old_files(&self, cutoff_date: DateTime<Utc>) -> Result<(), InklogError> {
        let log_dir = &self.local_retention_path;
        let mut count = 0;

        let entries = match fs::read_dir(log_dir).await {
            Ok(dir) => dir,
            Err(e) => {
                error!("Failed to read log directory: {}", e);
                return Ok(());
            }
        };

        let mut entries = Box::pin(entries);
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext == "log" || ext == "zst" || ext == "enc")
            {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_utc: DateTime<Utc> = modified.into();
                        if modified_utc < cutoff_date {
                            if let Err(e) = fs::remove_file(&path).await {
                                error!("Failed to remove old log file {}: {}", path.display(), e);
                            } else {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        info!("Cleaned up {} old log files", count);
        Ok(())
    }

    /// 估算记录数量
    #[cfg(feature = "aws")]
    fn estimate_record_count(&self, data: &[u8]) -> i64 {
        // 简单的估算：假设每条记录平均100字节
        (data.len() / 100) as i64
    }

    #[allow(dead_code)]
    /// 保存归档数据到本地保留目录（异步版本）
    async fn save_to_local_retention(
        &self,
        data: Vec<u8>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<(), InklogError> {
        // 生成本地文件名
        let filename = format!(
            "archive_{}_{}_{}.parquet",
            start_date.format("%Y%m%d_%H%M%S"),
            end_date.format("%Y%m%d_%H%M%S"),
            data.len()
        );

        let local_path = self.local_retention_path.join(filename);

        // 创建子目录（按日期组织）
        let date_dir = self.local_retention_path.join(format!(
            "{}/{:02}/{:02}",
            start_date.year(),
            start_date.month(),
            start_date.day()
        ));

        fs::create_dir_all(&date_dir).await.map_err(|e| {
            InklogError::IoError(std::io::Error::other(format!(
                "Failed to create local retention date directory: {}",
                e
            )))
        })?;

        let file_name = local_path.file_name().ok_or_else(|| {
            InklogError::IoError(std::io::Error::other(
                "Failed to get file name from local path".to_string(),
            ))
        })?;
        let file_path = date_dir.join(file_name);

        // 写入数据
        fs::write(&file_path, &data).await.map_err(|e| {
            InklogError::IoError(std::io::Error::other(format!(
                "Failed to write local retention file {}: {}",
                file_path.display(),
                e
            )))
        })?;

        info!(
            "Saved archive to local retention: {} ({} bytes)",
            file_path.display(),
            data.len()
        );

        Ok(())
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
    pub fn compression(&self) -> &crate::archive::CompressionType {
        &self.config.compression
    }

    /// 获取存储类型
    pub fn storage_class(&self) -> &crate::archive::StorageClass {
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
        let session = self.database_session;
        #[cfg(not(feature = "dbnexus"))]
        let session: Option<Session> = None;

        ArchiveService::new(config, session).await
    }

    /// 构建用于测试的归档服务（不初始化 S3 管理器）
    #[cfg(all(test, feature = "aws"))]
    pub async fn build_test(self) -> Result<ArchiveService, InklogError> {
        let config = self
            .config
            .ok_or_else(|| InklogError::ConfigError("S3 archive config is required".to_string()))?;
        let (shutdown_tx, _) = tokio::sync::mpsc::channel(1);

        Ok(ArchiveService {
            config: config.clone(),
            archive_manager: Arc::new(S3ArchiveManager::new(config.clone()).await?),
            database_session: self.database_session.map(std::sync::Arc::new),
            local_retention_path: std::path::PathBuf::from("target/test_logs"),
            scheduler: JobScheduler::new().await?,
            shutdown_tx,
            shutdown_rx: None,
            schedule_state: std::sync::Mutex::new(super::ScheduleState::default()),
            parquet_config: config.parquet_config.clone(),
        })
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

    #[tokio::test]
    async fn test_archive_service_builder() {
        // 测试构建器的基本功能
        let builder = ArchiveServiceBuilder::new();
        assert!(builder.config.is_none());
        #[cfg(feature = "dbnexus")]
        assert!(builder.database_session.is_none());
    }

    #[tokio::test]
    #[cfg(all(not(feature = "aws"), feature = "dbnexus"))]
    async fn test_fetch_database_logs() {
        use crate::sink::database::LogEntity;
        use chrono::{Duration, Utc};
        use dbnexus::pool::DbPool;

        // Create in-memory SQLite for testing
        let pool = DbPool::new("sqlite::memory:").await.unwrap();
        let session = pool.get_session("").await.unwrap();

        // Setup schema
        let create_sql = r#"
            CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT
            )
        "#;
        session.execute_raw(create_sql, &[]).await.unwrap();

        // Insert mock logs
        let now = Utc::now();
        let insert_sql = r#"
            INSERT INTO logs (timestamp, level, target, message, fields, file, line, thread_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;

        for i in 0..3 {
            session
                .execute_raw(
                    insert_sql,
                    &[
                        &(now - Duration::hours(i as i64)),
                        &"INFO",
                        &"test_target",
                        &format!("Test message {}", i),
                        &r#"{}"#,
                        &"test.rs",
                        &(100 + i),
                        &"thread-1",
                    ],
                )
                .await
                .unwrap();
        }

        // Initialize service config (mocked, won't use S3)
        let config = S3ArchiveConfig {
            enabled: true,
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            archive_interval_days: 1,
            local_retention_days: 7,
            ..Default::default()
        };

        let service = ArchiveService::new(config.clone(), Some(session.clone()))
            .await
            .unwrap();

        // Fetch logs
        let start_date = now - Duration::hours(3);
        let end_date = now;
        let data = service
            .fetch_database_logs(&session, start_date, end_date)
            .await
            .unwrap();

        assert!(!data.is_empty());
    }

    #[tokio::test]
    #[cfg(all(not(feature = "aws"), feature = "dbnexus"))]
    async fn test_cleanup_old_database_logs() {
        use chrono::{Duration, Utc};
        use dbnexus::pool::DbPool;

        let pool = DbPool::new("sqlite::memory:").await.unwrap();
        let session = pool.get_session("").await.unwrap();

        // Setup schema
        let create_sql = r#"
            CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                fields TEXT,
                file TEXT,
                line INTEGER,
                thread_id TEXT
            )
        "#;
        session.execute_raw(create_sql, &[]).await.unwrap();

        let now = Utc::now();
        let old_date = now - Duration::days(10);

        let insert_sql = r#"
            INSERT INTO logs (timestamp, level, target, message, fields, file, line, thread_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;

        // Insert old log
        session
            .execute_raw(
                insert_sql,
                &[
                    &old_date,
                    &"INFO",
                    &"test",
                    &"old log",
                    &r#"{}"#,
                    &"test.rs",
                    &100,
                    &"thread-1",
                ],
            )
            .await
            .unwrap();

        // Insert new log
        session
            .execute_raw(
                insert_sql,
                &[
                    &now,
                    &"INFO",
                    &"test",
                    &"new log",
                    &r#"{}"#,
                    &"test.rs",
                    &101,
                    &"thread-2",
                ],
            )
            .await
            .unwrap();

        let config = S3ArchiveConfig {
            enabled: true,
            local_retention_days: 7,
            ..Default::default()
        };

        let service = ArchiveService::new(config.clone(), Some(session.clone()))
            .await
            .unwrap();

        let cutoff_date = now - Duration::days(7);
        service
            .cleanup_old_database_logs(&session, cutoff_date)
            .await
            .unwrap();

        // Verify only 1 log remains
        let rows = session
            .execute_raw("SELECT COUNT(*) FROM logs", &[])
            .await
            .unwrap();
        let count: i64 = rows[0].get(0).and_then(|v| v.as_i64()).unwrap_or(0);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    #[cfg(not(feature = "aws"))]
    async fn test_cleanup_old_files() {
        use chrono::{Duration, Utc};
        use filetime::FileTime;

        let temp_dir = TempDir::new().unwrap();
        let retention_dir = temp_dir.path().join("retention");
        fs::create_dir_all(&retention_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let old_file = retention_dir.join("old.log");
        let new_file = retention_dir.join("new.log");

        fs::write(&old_file, "old content").unwrap();
        fs::write(&new_file, "new content").unwrap();

        let old_time =
            FileTime::from_unix_time((Utc::now().timestamp() - 10 * 24 * 3600) as i64, 0);
        filetime::set_file_mtime(&old_file, old_time).unwrap();

        let now = Utc::now();

        let config = S3ArchiveConfig {
            enabled: true,
            local_retention_days: 7,
            local_retention_path: retention_dir.clone(),
            ..Default::default()
        };

        let service = ArchiveService::new(config.clone(), None).await.unwrap();

        let cutoff = now - Duration::days(7);
        service.cleanup_old_files(cutoff).await.unwrap();

        assert!(!old_file.exists(), "Old file should be deleted");
        assert!(new_file.exists(), "New file should remain");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    #[cfg(not(feature = "aws"))]
    async fn test_save_to_local_retention() {
        let temp_dir = TempDir::new().unwrap();
        let retention_dir = temp_dir.path().join("logs/archive_failures");
        fs::create_dir_all(&retention_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();

        let config = S3ArchiveConfig {
            enabled: true,
            local_retention_days: 7,
            local_retention_path: retention_dir.clone(),
            ..Default::default()
        };
        let service = ArchiveService::new(config, None).await.unwrap();

        let data = b"dummy parquet data".to_vec();
        let now = Utc::now();
        let start_date = now - Duration::days(1);
        let end_date = now;

        service
            .save_to_local_retention(data.clone(), start_date, end_date)
            .await
            .unwrap();

        let date_path = retention_dir.join(format!(
            "{}/{:02}/{:02}",
            start_date.year(),
            start_date.month(),
            start_date.day()
        ));
        assert!(
            date_path.exists(),
            "Date directory should exist: {:?}",
            date_path
        );

        let entries: Vec<_> = fs::read_dir(&date_path)
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "Should have exactly one file in the date directory: found {} entries",
            entries.len()
        );

        let file_path = entries[0].path();
        assert!(file_path.exists(), "File should exist at {:?}", file_path);

        let saved_data = fs::read(&file_path).unwrap();
        assert_eq!(saved_data, data, "Saved data should match original data");

        std::env::set_current_dir(original_dir).unwrap();
    }
}
