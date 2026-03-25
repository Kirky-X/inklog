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
use parking_lot::Mutex;
#[cfg(feature = "dbnexus")]
use tokio::runtime::Handle;

#[cfg(feature = "dbnexus")]
use crate::support::io::sink::circuit_breaker::CircuitBreaker;
#[cfg(feature = "dbnexus")]
use crate::support::io::sink::file::FileSink;
#[cfg(feature = "dbnexus")]
use crate::DataMasker;
#[cfg(feature = "dbnexus")]
use crate::FileSinkConfig;
#[cfg(feature = "dbnexus")]
use crate::InklogError;
#[cfg(feature = "dbnexus")]
use crate::LogRecord;
#[cfg(feature = "dbnexus")]
use crate::Metrics;

const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 500;
const MIN_BATCH_SIZE: usize = 10;
const MAX_BATCH_SIZE: usize = 1000;
const ADAPTIVE_WINDOW_SIZE: usize = 10;

/// DatabaseSink 的可变内部状态
#[cfg(feature = "dbnexus")]
struct DatabaseSinkInner {
    buffer: Vec<LogRecord>,
    last_flush: Instant,
    fallback_sink: Option<FileSink>,
    circuit_breaker: CircuitBreaker,
    current_batch_size: usize,
    write_latencies: Vec<Duration>,
    success_count: usize,
    failure_count: usize,
    metrics: Option<Arc<Metrics>>,
}

#[cfg(feature = "dbnexus")]
pub struct DatabaseSink {
    /// 可变内部状态
    inner: Mutex<DatabaseSinkInner>,
    /// 数据库实现（DI 模式）
    /// 所有数据库操作通过此 trait 进行，完全符合 DI 架构要求
    database: Arc<dyn crate::integrations::infra::Database>,
    /// 数据脱敏器（只读）
    masker: Arc<DataMasker>,
    /// 停止标志
    stop: Arc<AtomicBool>,
}

#[cfg(feature = "dbnexus")]
impl DatabaseSink {
    /// 创建 DatabaseSink（使用默认配置）
    ///
    /// # 参数
    ///
    /// * `database` - 必须提供数据库实现（DI 模式）
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 架构说明
    ///
    /// 此方法完全依赖 `Database` trait，不持有任何具体的数据库连接池。
    /// 这确保了代码完全符合 DI 架构要求，便于测试和替换实现。
    pub fn new(
        database: Arc<dyn crate::integrations::infra::Database>,
    ) -> Result<Self, InklogError> {
        Self::new_with_config(database, None)
    }

    /// 创建 DatabaseSink（带配置参数）
    ///
    /// # 参数
    ///
    /// * `database` - 必须提供数据库实现（DI 模式）
    /// * `config` - 可选的数据库配置，用于设置批处理参数
    ///
    /// # 返回
    ///
    /// 成功返回 `Ok(Self)`，失败返回 `Err(InklogError)`
    ///
    /// # 架构说明
    ///
    /// 此方法用于测试场景，允许传入配置参数。
    /// 在生产环境中，应使用 `new()` 方法遵循 DI 架构。
    pub fn new_with_config(
        database: Arc<dyn crate::integrations::infra::Database>,
        config: Option<crate::DatabaseSinkConfig>,
    ) -> Result<Self, InklogError> {
        let fallback_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("logs/db_fallback.log"),
            ..Default::default()
        };
        let fallback_sink = FileSink::new(fallback_config).ok();

        // 使用配置参数或默认值
        let batch_size = config
            .as_ref()
            .map(|c| c.batch_size)
            .unwrap_or(DEFAULT_BATCH_SIZE);

        let inner = DatabaseSinkInner {
            buffer: Vec::with_capacity(batch_size),
            last_flush: Instant::now(),
            fallback_sink,
            circuit_breaker: CircuitBreaker::new(3, Duration::from_secs(30), 3),
            current_batch_size: batch_size,
            write_latencies: Vec::with_capacity(ADAPTIVE_WINDOW_SIZE),
            success_count: 0,
            failure_count: 0,
            metrics: None,
        };

        Ok(Self {
            inner: Mutex::new(inner),
            database,
            masker: Arc::new(DataMasker::new()),
            stop: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn set_metrics(&self, metrics: Arc<Metrics>) {
        let mut inner = self.inner.lock();
        inner.metrics = Some(metrics);
    }

    fn adjust_batch_size(inner: &mut DatabaseSinkInner) {
        if inner.write_latencies.len() < ADAPTIVE_WINDOW_SIZE {
            return;
        }

        let avg_latency: Duration =
            inner.write_latencies.iter().sum::<Duration>() / inner.write_latencies.len() as u32;
        let total_ops = inner.success_count + inner.failure_count;
        let success_rate = if total_ops > 0 {
            inner.success_count as f64 / total_ops as f64
        } else {
            1.0
        };

        if success_rate >= 0.95 && avg_latency < Duration::from_millis(50) {
            inner.current_batch_size = (inner.current_batch_size * 2).min(MAX_BATCH_SIZE);
        } else if success_rate < 0.8 || avg_latency > Duration::from_millis(200) {
            inner.current_batch_size = (inner.current_batch_size / 2).max(MIN_BATCH_SIZE);
        }

        inner.write_latencies.clear();
        inner.success_count = 0;
        inner.failure_count = 0;
    }

    fn execute_async<F, T>(&self, f: F) -> T
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        Handle::current().block_on(f)
    }
}

#[cfg(feature = "dbnexus")]
impl fmt::Display for DatabaseSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseSink")
    }
}

#[cfg(feature = "dbnexus")]
impl crate::support::io::sink::LogSink for DatabaseSink {
    fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        let mut inner = self.inner.lock();

        if !inner.circuit_breaker.can_execute() {
            if let Some(ref sink) = inner.fallback_sink {
                let _ = sink.write(record);
            }
            return Ok(());
        }

        let masked_record = LogRecord {
            message: self.masker.mask(&record.message),
            ..record.clone()
        };

        inner.buffer.push(masked_record);

        if inner.buffer.len() >= inner.current_batch_size
            || inner.last_flush.elapsed() > Duration::from_millis(DEFAULT_FLUSH_INTERVAL_MS)
        {
            let start = Instant::now();
            if let Err(e) = Self::flush_inner(self, &mut inner) {
                inner.failure_count += 1;
                inner.circuit_breaker.record_failure();
                if let Some(ref sink) = inner.fallback_sink {
                    let _ = sink.write(record);
                }
                return Err(e);
            }
            inner.success_count += 1;
            inner.write_latencies.push(start.elapsed());
            Self::adjust_batch_size(&mut inner);
        }

        inner.circuit_breaker.record_success();
        Ok(())
    }

    fn flush(&self) -> Result<(), InklogError> {
        let mut inner = self.inner.lock();
        Self::flush_inner(self, &mut inner)
    }

    fn shutdown(&self) -> Result<(), InklogError> {
        self.stop.store(true, Ordering::Relaxed);
        let mut inner = self.inner.lock();
        let _ = Self::flush_inner(self, &mut inner);
        tracing::info!("Database sink shutdown complete");
        Ok(())
    }
}

#[cfg(feature = "dbnexus")]
impl DatabaseSink {
    fn flush_inner(&self, inner: &mut DatabaseSinkInner) -> Result<(), InklogError> {
        if inner.buffer.is_empty() {
            return Ok(());
        }

        let records = std::mem::take(&mut inner.buffer);
        inner.last_flush = Instant::now();
        let batch_size = records.len();
        if let Some(metrics) = &inner.metrics {
            metrics.set_db_batch_size(batch_size);
        }

        // 使用注入的 database 实现
        // 所有数据库操作通过 Database trait 进行，完全符合 DI 架构要求
        let database_clone = self.database.clone();
        let metrics_ref = inner.metrics.clone();
        let records_clone = records.clone();

        let result = self.execute_async(async move {
            match database_clone.insert_batch(&records_clone).await {
                Ok(written) => {
                    if let Some(m) = &metrics_ref {
                        m.add_db_batch_records_total(written);
                        m.update_sink_health("database", true, None);
                    }
                    Ok(())
                }
                Err(e) => {
                    if let Some(m) = &metrics_ref {
                        m.inc_sink_error();
                        m.update_sink_health("database", false, Some(e.to_string()));
                    }
                    Err(e)
                }
            }
        });

        if let Err(e) = result {
            inner.buffer = records;
            return Err(e);
        }

        Ok(())
    }
}

/// Convert LogRecord to Parquet format
#[cfg(feature = "dbnexus")]
pub fn convert_logs_to_parquet(
    logs: &[crate::LogRecord],
    _config: &crate::ParquetConfig,
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
        .map(|l| l.timestamp.timestamp_millis())
        .collect();
    let levels: Vec<&str> = logs.iter().map(|l| l.level.as_str()).collect();
    let targets: Vec<&str> = logs.iter().map(|l| l.target.as_str()).collect();
    let messages: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();
    let fields: Vec<Option<String>> = logs
        .iter()
        .map(|l| Some(serde_json::to_string(&l.fields).unwrap_or_default()))
        .collect();
    let files: Vec<Option<&str>> = logs.iter().map(|l| l.file.as_deref()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::infra::database::MockDatabaseAdapter;
    use crate::support::io::sink::LogSink;
    use crate::DatabaseSinkConfig;
    use crate::LogRecord;
    use crate::Metrics;
    use std::sync::Arc;

    #[test]
    fn test_convert_logs_to_parquet_empty() {
        let logs: Vec<LogRecord> = vec![];
        let config = DatabaseSinkConfig::default().parquet_config;
        let result = convert_logs_to_parquet(&logs, &config);
        assert!(result.is_ok());
        // Empty log list produces a Parquet file with schema/metadata but no records
        let bytes = result.unwrap();
        assert!(!bytes.is_empty()); // Parquet writer adds schema overhead even for empty data
    }

    #[test]
    fn test_convert_logs_to_parquet_non_empty() {
        let log1 = LogRecord::default();
        let log2 = LogRecord {
            message: "warn message".into(),
            ..Default::default()
        };

        let config = DatabaseSinkConfig::default().parquet_config;
        let result = convert_logs_to_parquet(&[log1, log2], &config);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    #[ignore = "Requires multi-threaded tokio runtime for DatabaseSink construction"]
    fn test_database_sink_write_with_mock_db() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let config = DatabaseSinkConfig::default();

        let sink = DatabaseSink::new_with_config(mock_db.clone(), Some(config)).unwrap();

        let metrics = Arc::new(Metrics::new());
        sink.set_metrics(metrics.clone());

        let record = LogRecord::default();
        let result = sink.write(&record);
        assert!(result.is_ok());

        let flush_result = sink.flush();
        assert!(flush_result.is_ok());

        // Verify the mock DB received the record
        assert_eq!(mock_db.stored_count(), 1);
    }
}
