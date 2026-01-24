//! Database sink implementation using dbnexus.
//!
//! This module provides database logging functionality with support for
//! PostgreSQL, MySQL, and SQLite, including batch writes, partitioning,
//! Parquet export, and S3 archival.

#[cfg(feature = "dbnexus")]
use std::fmt;
#[cfg(feature = "dbnexus")]
use std::path::PathBuf;
#[cfg(feature = "dbnexus")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "dbnexus")]
use std::sync::Arc;
#[cfg(feature = "dbnexus")]
use std::time::{Duration, Instant};

#[cfg(feature = "dbnexus")]
use anyhow::Result;
#[cfg(feature = "dbnexus")]
use chrono::{DateTime, Datelike, Utc};
#[cfg(feature = "dbnexus")]
use dbnexus::pool::{DbPool, Session};

#[cfg(feature = "dbnexus")]
use crate::config::{DatabaseDriver, DatabaseSinkConfig, FileSinkConfig};
#[cfg(feature = "dbnexus")]
use crate::error::InklogError;
#[cfg(feature = "dbnexus")]
use crate::log_record::LogRecord;
#[cfg(feature = "dbnexus")]
use crate::masking::DataMasker;
#[cfg(feature = "dbnexus")]
use crate::sink::circuit_breaker::CircuitBreaker;
#[cfg(feature = "dbnexus")]
use crate::sink::file::FileSink;

#[cfg(feature = "dbnexus")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LogEntity {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Option<serde_json::Value>,
    pub file: Option<String>,
    pub line: Option<i32>,
    pub thread_id: Option<String>,
}

#[cfg(feature = "dbnexus")]
impl LogEntity {
    pub fn from_log_record(record: &LogRecord) -> Self {
        Self {
            id: 0,
            timestamp: record.timestamp,
            level: record.level.to_string(),
            target: record.target.clone(),
            message: record.message.clone(),
            fields: if record.fields.is_empty() {
                None
            } else {
                Some(serde_json::json!(record.fields))
            },
            file: record.file.clone(),
            line: record.line.map(|i| i as i32),
            thread_id: Some(record.thread_id.clone()),
        }
    }
}

#[cfg(feature = "dbnexus")]
/// Type alias for backward compatibility with Sea-ORM tests
pub type Model = LogEntity;

#[cfg(feature = "dbnexus")]
pub struct DatabaseSink {
    config: DatabaseSinkConfig,
    buffer: Vec<LogRecord>,
    last_flush: Instant,
    rt: tokio::runtime::Runtime,
    pool: Option<Arc<DbPool>>,
    fallback_sink: Option<FileSink>,
    circuit_breaker: CircuitBreaker,
    masker: Arc<DataMasker>,
    stop: Arc<AtomicBool>,
}

#[cfg(feature = "dbnexus")]
impl DatabaseSink {
    pub fn new(config: DatabaseSinkConfig) -> Result<Self, InklogError> {
        // Initialize fallback sink
        let fallback_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("logs/db_fallback.log"),
            ..Default::default()
        };
        let fallback_sink = FileSink::new(fallback_config).ok();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(std::cmp::max(2, num_cpus::get()))
            .thread_name("inklog-db-worker")
            .enable_all()
            .build()
            .map_err(InklogError::IoError)?;

        let pool = rt.block_on(async {
            let pool = DbPool::new(&config.url)
                .await
                .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
            let session = pool
                .get_session("admin")
                .await
                .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
            Self::ensure_table_exists(&session, &config)
                .await
                .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
            Ok::<_, InklogError>(pool)
        })?;

        Ok(Self {
            config,
            buffer: Vec::with_capacity(100),
            last_flush: Instant::now(),
            rt,
            pool: Some(Arc::new(pool)),
            fallback_sink,
            circuit_breaker: CircuitBreaker::new(3, Duration::from_secs(30), 3),
            masker: Arc::new(DataMasker::new()),
            stop: Arc::new(AtomicBool::new(false)),
        })
    }

    async fn ensure_table_exists(session: &Session, config: &DatabaseSinkConfig) -> Result<()> {
        let table_name = &config.table_name;

        match config.driver {
            DatabaseDriver::PostgreSQL => {
                let create_sql = format!(
                    r#"
                    CREATE TABLE IF NOT EXISTS {table_name} (
                        id BIGSERIAL,
                        timestamp TIMESTAMPTZ NOT NULL,
                        level TEXT NOT NULL,
                        target TEXT NOT NULL,
                        message TEXT NOT NULL,
                        fields JSONB,
                        file TEXT,
                        line INTEGER,
                        thread_id TEXT,
                        PRIMARY KEY (id, timestamp)
                    ) PARTITION BY RANGE (timestamp);

                    CREATE INDEX IF NOT EXISTS {table_name}_timestamp_idx ON {table_name} (timestamp DESC);
                    CREATE INDEX IF NOT EXISTS {table_name}_level_idx ON {table_name} (level);
                    CREATE INDEX IF NOT EXISTS {table_name}_target_idx ON {table_name} (target);
                    "#
                );
                session.execute_raw_ddl(&create_sql).await?;
            }
            DatabaseDriver::MySQL => {
                let create_sql = format!(
                    r#"
                    CREATE TABLE IF NOT EXISTS {table_name} (
                        id BIGINT AUTO_INCREMENT,
                        timestamp DATETIME(3) NOT NULL,
                        level VARCHAR(20) NOT NULL,
                        target VARCHAR(255) NOT NULL,
                        message TEXT NOT NULL,
                        fields JSON,
                        file VARCHAR(512),
                        line INT,
                        thread_id VARCHAR(128),
                        PRIMARY KEY (id, timestamp),
                        INDEX idx_timestamp (timestamp),
                        INDEX idx_level (level),
                        INDEX idx_target (target)
                    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
                    "#
                );
                session.execute_raw_ddl(&create_sql).await?;
            }
            DatabaseDriver::SQLite => {
                // Create table
                let create_table_sql = format!(
                    r#"
                    CREATE TABLE IF NOT EXISTS {table_name} (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp TEXT NOT NULL,
                        level TEXT NOT NULL,
                        target TEXT NOT NULL,
                        message TEXT NOT NULL,
                        fields TEXT,
                        file TEXT,
                        line INTEGER,
                        thread_id TEXT
                    );
                    "#
                );
                session.execute_raw_ddl(&create_table_sql).await?;

                // Create indexes (separate statements for SQLite)
                let idx_timestamp = format!("CREATE INDEX IF NOT EXISTS {table_name}_timestamp_idx ON {table_name} (timestamp DESC)");
                session.execute_raw_ddl(&idx_timestamp).await?;

                let idx_level = format!(
                    "CREATE INDEX IF NOT EXISTS {table_name}_level_idx ON {table_name} (level)"
                );
                session.execute_raw_ddl(&idx_level).await?;

                let idx_target = format!(
                    "CREATE INDEX IF NOT EXISTS {table_name}_target_idx ON {table_name} (target)"
                );
                session.execute_raw_ddl(&idx_target).await?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "dbnexus")]
impl crate::sink::LogSink for DatabaseSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError> {
        if !self.circuit_breaker.can_execute() {
            if let Some(ref mut sink) = self.fallback_sink {
                let _ = sink.write(record);
            }
            return Ok(());
        }

        let masked_record = LogRecord {
            message: self.masker.mask(&record.message),
            ..record.clone()
        };

        self.buffer.push(masked_record.clone());

        if self.buffer.len() >= self.config.batch_size
            || self.last_flush.elapsed() > Duration::from_millis(self.config.flush_interval_ms)
        {
            if let Err(e) = self.flush() {
                self.circuit_breaker.record_failure();
                if let Some(ref mut sink) = self.fallback_sink {
                    let _ = sink.write(record);
                }
                return Err(e);
            }
        }

        self.circuit_breaker.record_success();
        Ok(())
    }

    fn flush(&mut self) -> Result<(), InklogError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let records = std::mem::take(&mut self.buffer);
        self.last_flush = Instant::now();

        if let Some(ref pool) = self.pool {
            let pool_clone = pool.clone();
            self.rt.block_on(async {
                if let Err(e) = write_batch_to_db(&pool_clone, &self.config, &records).await {
                    self.buffer = records;
                    Err(e)
                } else {
                    Ok(())
                }
            })?;
        }

        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), InklogError> {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.flush();
        tracing::info!("Database sink shutdown complete");
        Ok(())
    }
}

#[cfg(feature = "dbnexus")]
async fn write_batch_to_db(
    pool: &DbPool,
    config: &DatabaseSinkConfig,
    records: &[LogRecord],
) -> Result<usize, InklogError> {
    if records.is_empty() {
        return Ok(0);
    }

    let session = pool
        .get_session("admin")
        .await
        .map_err(|e: dbnexus::error::DbError| InklogError::DatabaseError(e.to_string()))?;
    let mut inserted = 0;

    let target_table = match config.driver {
        DatabaseDriver::PostgreSQL => {
            let first_ts = records
                .first()
                .map(|r| r.timestamp)
                .unwrap_or_else(chrono::Utc::now);
            get_partition_for_timestamp(&first_ts, config)
        }
        _ => config.table_name.clone(),
    };

    for record in records {
        let sql = format!(
            "INSERT INTO {} (timestamp, level, target, message, fields, file, line, thread_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            target_table
        );

        let params: Vec<sea_orm::Value> = vec![
            sea_orm::Value::from(record.timestamp.to_rfc3339()),
            sea_orm::Value::from(record.level.to_string()),
            sea_orm::Value::from(record.target.clone()),
            sea_orm::Value::from(record.message.clone()),
            sea_orm::Value::from(serde_json::to_string(&record.fields).ok()),
            sea_orm::Value::from(record.file.clone()),
            sea_orm::Value::from(record.line.map(|l| l as i64)),
            sea_orm::Value::from(record.thread_id.clone()),
        ];

        if session.execute_paramized(&sql, params).await.is_ok() {
            inserted += 1;
        }
    }

    Ok(inserted)
}

#[cfg(feature = "dbnexus")]
fn get_partition_for_timestamp(ts: &DateTime<Utc>, config: &DatabaseSinkConfig) -> String {
    match config.partition {
        crate::config::PartitionStrategy::Monthly => {
            format!("{}_{:04}_{:02}", config.table_name, ts.year(), ts.month())
        }
        crate::config::PartitionStrategy::Yearly => {
            format!("{}_{:04}", config.table_name, ts.year())
        }
    }
}

/// 将 LogEntity 转换为 Parquet 格式
#[cfg(feature = "dbnexus")]
pub fn convert_logs_to_parquet(
    logs: &[LogEntity],
    _config: &crate::config::ParquetConfig,
) -> Result<Vec<u8>, String> {
    use arrow_array::{Date64Array, Int32Array, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("timestamp", DataType::Date64, false),
        Field::new("level", DataType::Utf8, false),
        Field::new("target", DataType::Utf8, false),
        Field::new("message", DataType::Utf8, false),
        Field::new("fields", DataType::Utf8, true),
        Field::new("file", DataType::Utf8, true),
        Field::new("line", DataType::Int32, true),
        Field::new("thread_id", DataType::Utf8, true),
    ]);

    let ids: Vec<i64> = logs.iter().map(|l| l.id).collect();
    let timestamps: Vec<i64> = logs
        .iter()
        .map(|l| l.timestamp.timestamp_millis())
        .collect();
    let levels: Vec<&str> = logs.iter().map(|l| l.level.as_str()).collect();
    let targets: Vec<&str> = logs.iter().map(|l| l.target.as_str()).collect();
    let messages: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();
    let fields: Vec<Option<String>> = logs
        .iter()
        .map(|l| l.fields.as_ref().map(|v| v.to_string()))
        .collect();
    let files: Vec<Option<&str>> = logs.iter().map(|l| l.file.as_deref()).collect();
    let lines: Vec<Option<i32>> = logs.iter().map(|l| l.line).collect();
    let thread_ids: Vec<Option<&str>> = logs.iter().map(|l| l.thread_id.as_deref()).collect();

    let batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(Int64Array::from(ids)),
            Arc::new(Date64Array::from(timestamps)),
            Arc::new(StringArray::from(levels)),
            Arc::new(StringArray::from(targets)),
            Arc::new(StringArray::from(messages)),
            Arc::new(StringArray::from(fields)),
            Arc::new(StringArray::from(files)),
            Arc::new(Int32Array::from(lines)),
            Arc::new(StringArray::from(thread_ids)),
        ],
    )
    .map_err(|e| e.to_string())?;

    let mut buffer = Vec::new();
    let mut writer = parquet::arrow::ArrowWriter::try_new(
        std::io::Cursor::new(&mut buffer),
        batch.schema(),
        None,
    )
    .map_err(|e| e.to_string())?;
    writer.write(&batch).map_err(|e| e.to_string())?;
    writer.close().map_err(|e| e.to_string())?;

    Ok(buffer)
}

#[cfg(feature = "dbnexus")]
impl fmt::Display for DatabaseSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseSink({})", self.config.name)
    }
}

#[cfg(not(feature = "dbnexus"))]
mod db_nexus_disabled {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct DatabaseSink;

    impl DatabaseSink {
        pub fn new(_config: DatabaseSinkConfig) -> Result<Self, InklogError> {
            Ok(Self)
        }
    }

    #[derive(Clone, Debug)]
    pub struct LogEntity;

    impl LogEntity {
        pub fn from_log_record(_record: &LogRecord) -> Self {
            Self
        }
    }

    pub fn convert_logs_to_parquet(
        _logs: &[LogEntity],
        _config: &crate::config::ParquetConfig,
    ) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }

    /// Type alias for backward compatibility
    pub type Model = LogEntity;
}

#[cfg(not(feature = "dbnexus"))]
pub use db_nexus_disabled::*;
