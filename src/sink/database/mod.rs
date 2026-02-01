//! Database sink implementation using dbnexus.
//!
//! This module provides database logging functionality with support for
//! PostgreSQL, MySQL, and SQLite.

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
use chrono::{DateTime, Utc};
#[cfg(feature = "dbnexus")]
use dbnexus::pool::DbPool;

#[cfg(feature = "dbnexus")]
use crate::config::FileSinkConfig;
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

const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 500;

#[cfg(feature = "dbnexus")]
pub struct DatabaseSink {
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
    pub fn new(config: &crate::config::DatabaseSinkConfig) -> Result<Self, InklogError> {
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
            let db_config = dbnexus::DbConfigBuilder::new()
                .url(&config.url)
                .max_connections(config.pool_size)
                .build()
                .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
            let pool = DbPool::with_config(db_config)
                .await
                .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
            Ok::<_, InklogError>(pool)
        })?;

        Ok(Self {
            buffer: Vec::with_capacity(DEFAULT_BATCH_SIZE),
            last_flush: Instant::now(),
            rt,
            pool: Some(Arc::new(pool)),
            fallback_sink,
            circuit_breaker: CircuitBreaker::new(3, Duration::from_secs(30), 3),
            masker: Arc::new(DataMasker::new()),
            stop: Arc::new(AtomicBool::new(false)),
        })
    }
}

#[cfg(feature = "dbnexus")]
impl fmt::Display for DatabaseSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseSink")
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

        if self.buffer.len() >= DEFAULT_BATCH_SIZE
            || self.last_flush.elapsed() > Duration::from_millis(DEFAULT_FLUSH_INTERVAL_MS)
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
                if let Err(e) = write_batch_to_db(&pool_clone, &records).await {
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
async fn write_batch_to_db(pool: &DbPool, records: &[LogRecord]) -> Result<usize, InklogError> {
    if records.is_empty() {
        return Ok(0);
    }

    let session = pool
        .get_session("admin")
        .await
        .map_err(|e: dbnexus::error::DbError| InklogError::DatabaseError(e.to_string()))?;
    let mut inserted = 0;

    // Use default table name "logs" - dbnexus manages the actual table
    let target_table = "logs";

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
fn get_partition_for_timestamp(_ts: &DateTime<Utc>) -> String {
    // Partition management is handled by dbnexus
    "logs".to_string()
}

/// Convert LogRecord to Parquet format
#[cfg(feature = "dbnexus")]
pub fn convert_logs_to_parquet(
    logs: &[crate::log_record::LogRecord],
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
        Field::new("message", DataType::Utf8, true),
        Field::new("fields", DataType::Utf8, true),
        Field::new("file", DataType::Utf8, true),
        Field::new("line", DataType::Int32, true),
        Field::new("thread_id", DataType::Utf8, true),
    ]);

    let ids: Vec<i64> = (1..=logs.len() as i64).collect();
    let timestamps: Vec<i64> = logs
        .iter()
        .map(|l| l.timestamp.timestamp_millis() as i64)
        .collect();
    let levels: Vec<&str> = logs.iter().map(|l| l.level.as_str()).collect();
    let targets: Vec<&str> = logs.iter().map(|l| l.target.as_str()).collect();
    let messages: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();
    let fields: Vec<Option<String>> = logs
        .iter()
        .map(|l| Some(serde_json::to_string(&l.fields).unwrap_or_default()))
        .collect();
    let files: Vec<Option<&str>> = logs
        .iter()
        .map(|l| l.file.as_ref().map(|s| s.as_str()))
        .collect();
    let lines: Vec<Option<i32>> = logs
        .iter()
        .map(|l| l.line.map(|line_num| line_num as i32))
        .collect();
    let thread_ids: Vec<Option<&str>> = logs.iter().map(|l| Some(l.thread_id.as_str())).collect();

    let batch = RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(Int64Array::from(ids)) as Arc<dyn arrow_array::Array>,
            Arc::new(Date64Array::from(timestamps)) as Arc<dyn arrow_array::Array>,
            Arc::new(StringArray::from(levels)) as Arc<dyn arrow_array::Array>,
            Arc::new(StringArray::from(targets)) as Arc<dyn arrow_array::Array>,
            Arc::new(StringArray::from(messages)) as Arc<dyn arrow_array::Array>,
            Arc::new(StringArray::from(fields)) as Arc<dyn arrow_array::Array>,
            Arc::new(StringArray::from(files)) as Arc<dyn arrow_array::Array>,
            Arc::new(Int32Array::from(lines)) as Arc<dyn arrow_array::Array>,
            Arc::new(StringArray::from(thread_ids)) as Arc<dyn arrow_array::Array>,
        ],
    )
    .map_err(|e| e.to_string())?;

    let mut bytes = Vec::new();
    let mut writer = parquet::arrow::ArrowWriter::try_new(&mut bytes, batch.schema(), None)
        .map_err(|e| e.to_string())?;
    writer.write(&batch).map_err(|e| e.to_string())?;
    writer.close().map_err(|e| e.to_string())?;

    Ok(bytes)
}
