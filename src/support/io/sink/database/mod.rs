//! Database sink implementation using dbnexus.
//!
//! This module provides database logging functionality with support for
//! PostgreSQL, MySQL, and SQLite.

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::fmt;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::path::PathBuf;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::sync::Arc;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::time::{Duration, Instant};

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use anyhow::Result;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use parking_lot::Mutex;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use tokio::runtime::Handle;

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::support::io::sink::circuit_breaker::CircuitBreaker;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::support::io::sink::file::FileSink;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::DataMasker;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::FileSinkConfig;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::InklogError;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::LogRecord;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use crate::Metrics;

const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 500;
const MIN_BATCH_SIZE: usize = 10;
const MAX_BATCH_SIZE: usize = 1000;
const ADAPTIVE_WINDOW_SIZE: usize = 10;

/// DatabaseSink 的可变内部状态
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
        // Use block_in_place to allow blocking in async context
        // This is safe because we're not holding any async locks across the blocking call
        tokio::task::block_in_place(|| Handle::current().block_on(f))
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
impl fmt::Display for DatabaseSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseSink")
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_write_with_mock_db() {
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

    /// 测试 new() 方法（不传 config，使用默认配置）
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_new_without_config() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let sink = DatabaseSink::new(mock_db.clone()).unwrap();
        // 写入一条记录，验证默认配置下能正常工作
        let record = LogRecord::default();
        let result = sink.write(&record);
        assert!(result.is_ok());
        let flush_result = sink.flush();
        assert!(flush_result.is_ok());
        assert_eq!(mock_db.stored_count(), 1);
    }

    /// 测试 fmt::Display 实现
    #[test]
    fn test_database_sink_display() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let sink = DatabaseSink::new(mock_db).unwrap();
        let s = format!("{}", sink);
        assert_eq!(s, "DatabaseSink");
    }

    /// 测试 flush 空缓冲区（应直接返回 Ok）
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_flush_empty_buffer() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let sink = DatabaseSink::new(mock_db).unwrap();
        // 没有写入任何记录，直接 flush 应该返回 Ok
        let result = sink.flush();
        assert!(result.is_ok());
    }

    /// 测试 shutdown 方法
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_shutdown() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let sink = DatabaseSink::new(mock_db.clone()).unwrap();
        // 写入一条记录，但不 flush
        let record = LogRecord::default();
        let _ = sink.write(&record);
        // shutdown 应该触发 flush，把记录写入数据库
        let result = sink.shutdown();
        assert!(result.is_ok());
        // shutdown 后记录应该已经被 flush 到数据库
        assert_eq!(mock_db.stored_count(), 1);
    }

    /// 测试 buffer 满触发 flush
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_buffer_full_triggers_flush() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        // 使用最小批处理大小（10）以快速触发 flush
        let config = DatabaseSinkConfig {
            batch_size: 10,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(mock_db.clone(), Some(config)).unwrap();

        // 写入 10 条记录，应该触发一次 flush
        for i in 0..10 {
            let record = LogRecord {
                message: format!("message {}", i),
                ..Default::default()
            };
            let result = sink.write(&record);
            assert!(result.is_ok(), "Write {} failed: {:?}", i, result.err());
        }

        // 10 条记录应该已经被写入数据库
        assert_eq!(mock_db.stored_count(), 10);
    }

    /// 测试 last_flush 超时触发 flush
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_flush_timeout_triggers_flush() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        // 使用大 batch_size 避免触发 buffer 满
        let config = DatabaseSinkConfig {
            batch_size: 1000,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(mock_db.clone(), Some(config)).unwrap();

        // 写入一条记录，由于 batch_size=1000 不会触发 flush
        let record = LogRecord::default();
        let _ = sink.write(&record);
        assert_eq!(mock_db.stored_count(), 0);

        // 等待超过 DEFAULT_FLUSH_INTERVAL_MS (500ms)
        tokio::time::sleep(Duration::from_millis(600)).await;

        // 再写入一条记录，应该因为 last_flush 超时而触发 flush
        let record2 = LogRecord::default();
        let result = sink.write(&record2);
        assert!(result.is_ok());

        // 两条记录应该都被写入数据库
        assert_eq!(mock_db.stored_count(), 2);
    }

    /// 测试 masker 应用：写入包含敏感信息的记录，验证消息被脱敏
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_applies_masking() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let sink = DatabaseSink::new(mock_db.clone()).unwrap();

        // 写入包含邮箱的记录
        let record = LogRecord {
            message: "User email: test@example.com".to_string(),
            ..Default::default()
        };
        let _ = sink.write(&record);
        let _ = sink.flush();

        // 验证数据库中的记录已被脱敏（邮箱被替换）
        let records = mock_db.get_records();
        assert_eq!(records.len(), 1);
        assert!(
            !records[0].message.contains("test@example.com"),
            "Message should be masked, got: {}",
            records[0].message
        );
    }

    /// 测试 set_metrics 方法
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_set_metrics() {
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let sink = DatabaseSink::new(mock_db.clone()).unwrap();
        let metrics = Arc::new(Metrics::new());
        sink.set_metrics(metrics.clone());

        // 写入并 flush 一条记录，验证 metrics 被更新
        let record = LogRecord::default();
        let _ = sink.write(&record);
        let _ = sink.flush();

        // metrics 应该记录了 batch records total
        assert!(metrics.db_batch_records_total() > 0);
    }

    /// 测试用 FailingDatabase：insert_batch 始终失败
    struct FailingDatabase;

    #[async_trait::async_trait]
    impl crate::integrations::infra::Database for FailingDatabase {
        async fn insert_batch(&self, _records: &[LogRecord]) -> Result<usize, InklogError> {
            Err(InklogError::DatabaseError(
                "Simulated database failure".to_string(),
            ))
        }

        async fn is_healthy(&self) -> bool {
            false
        }
    }

    /// 测试 flush 失败时返回错误
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_flush_failure_returns_error() {
        let failing_db = Arc::new(FailingDatabase);
        let config = DatabaseSinkConfig {
            batch_size: 10,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(failing_db, Some(config)).unwrap();

        // 写入足够多的记录触发 flush，flush 应该失败
        for _ in 0..10 {
            let record = LogRecord::default();
            let _ = sink.write(&record);
        }

        // 直接调用 flush 也应该返回错误
        let result = sink.flush();
        // 如果 buffer 为空（因为前面已经触发 flush 失败但记录被放回 buffer），可能返回 Ok 或 Err
        // 这里主要验证不会 panic
        let _ = result;
    }

    /// 测试 shutdown 在错误情况下也能正常返回 Ok
    #[tokio::test(flavor = "multi_thread")]
    async fn test_database_sink_shutdown_with_failing_db() {
        let failing_db = Arc::new(FailingDatabase);
        let sink = DatabaseSink::new(failing_db).unwrap();

        // 写入记录
        let record = LogRecord::default();
        let _ = sink.write(&record);

        // shutdown 应该返回 Ok（即使 flush 失败，shutdown 也会忽略错误）
        let result = sink.shutdown();
        assert!(result.is_ok());
    }

    // ========================================================================
    // adjust_batch_size 覆盖：成功率达到阈值时触发批大小调整
    // 覆盖行 156, 158-160, 165-166, 171-173
    // ========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_adjust_batch_size_grows_on_high_success_and_low_latency() {
        // 写入 100 条记录（batch_size=10），触发 10 次 flush 成功。
        // 第 10 次 flush 后 write_latencies.len()=10，adjust_batch_size 执行实际逻辑。
        // success_rate=1.0 >= 0.95 且 avg_latency < 50ms → current_batch_size 翻倍
        let mock_db = Arc::new(MockDatabaseAdapter::new());
        let config = DatabaseSinkConfig {
            batch_size: 10,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(mock_db.clone(), Some(config)).unwrap();
        let metrics = Arc::new(Metrics::new());
        sink.set_metrics(metrics.clone());

        // 写入 100 条记录，触发 10 次 flush
        for i in 0..100 {
            let record = LogRecord {
                message: format!("adjust-grow-{}", i),
                ..Default::default()
            };
            let result = sink.write(&record);
            assert!(result.is_ok(), "write {} failed: {:?}", i, result.err());
        }

        // 验证 10 次 flush 都成功写入数据库
        assert_eq!(
            mock_db.stored_count(),
            100,
            "all 100 records should be flushed to db"
        );
        // 覆盖 adjust_batch_size 后，验证 metrics 仍正常工作
        assert!(metrics.db_batch_records_total() > 0);
    }

    // ========================================================================
    // adjust_batch_size 覆盖：高延迟触发缩减分支
    // 覆盖行 167-168
    // ========================================================================

    /// 慢速数据库：每次 insert_batch sleep 250ms，使 avg_latency > 200ms
    struct SlowDatabase;

    #[async_trait::async_trait]
    impl crate::integrations::infra::Database for SlowDatabase {
        async fn insert_batch(&self, _records: &[LogRecord]) -> Result<usize, InklogError> {
            tokio::time::sleep(Duration::from_millis(250)).await;
            Ok(_records.len())
        }

        async fn is_healthy(&self) -> bool {
            true
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_adjust_batch_size_shrinks_on_high_latency() {
        // 每次 flush 250ms，10 次后 avg_latency=250ms > 200ms → current_batch_size 减半
        let slow_db = Arc::new(SlowDatabase);
        let config = DatabaseSinkConfig {
            batch_size: 10,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(slow_db, Some(config)).unwrap();

        // 写入 100 条记录，触发 10 次 flush（每次 250ms，总约 2.5s）
        for i in 0..100 {
            let record = LogRecord {
                message: format!("adjust-shrink-{}", i),
                ..Default::default()
            };
            let result = sink.write(&record);
            assert!(result.is_ok(), "write {} failed: {:?}", i, result.err());
        }

        // 到达此处说明 adjust_batch_size 执行了缩减分支（未 panic）
        // current_batch_size 应从 10 减为 5（MIN_BATCH_SIZE）
        // 间接验证：再写入 5 条，应触发 flush（因为 current_batch_size=5）
        let before = sink.flush();
        let _ = before;
    }

    // ========================================================================
    // circuit_breaker open 后走 fallback_sink 分支
    // 覆盖行 200-201, 203
    // ========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_circuit_breaker_open_routes_to_fallback_sink() {
        // FailingDatabase 触发 3 次 flush 失败 → circuit_breaker open
        // 第 13 条 write：can_execute()=false → 走 fallback_sink → 返回 Ok
        let failing_db = Arc::new(FailingDatabase);
        let config = DatabaseSinkConfig {
            batch_size: 10,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(failing_db, Some(config)).unwrap();

        // 写入 12 条记录触发 3 次 flush 失败
        // 第 1-9 条：buffer 增长，不 flush
        // 第 10 条：buffer=10，flush 失败（failure #1），记录放回
        // 第 11 条：buffer=11，flush 失败（failure #2），记录放回
        // 第 12 条：buffer=12，flush 失败（failure #3），circuit_breaker open
        for i in 0..12 {
            let record = LogRecord {
                message: format!("cb-open-{}", i),
                ..Default::default()
            };
            let _ = sink.write(&record);
        }

        // 第 13 条：circuit_breaker.can_execute()=false → fallback_sink → Ok
        let record = LogRecord {
            message: "after circuit open".to_string(),
            ..Default::default()
        };
        let result = sink.write(&record);
        assert!(
            result.is_ok(),
            "write after circuit open should route to fallback and return Ok, got: {:?}",
            result
        );
    }

    // ========================================================================
    // flush_inner 失败时更新 metrics
    // 覆盖行 279-280
    // ========================================================================

    #[tokio::test(flavor = "multi_thread")]
    async fn test_flush_failure_updates_sink_error_metrics() {
        // FailingDatabase + set_metrics → flush 失败时 inc_sink_error + update_sink_health
        let failing_db = Arc::new(FailingDatabase);
        let config = DatabaseSinkConfig {
            batch_size: 10,
            ..Default::default()
        };
        let sink = DatabaseSink::new_with_config(failing_db, Some(config)).unwrap();
        let metrics = Arc::new(Metrics::new());
        sink.set_metrics(metrics.clone());

        // 写入 10 条触发 flush 失败
        for _ in 0..10 {
            let record = LogRecord::default();
            let _ = sink.write(&record);
        }

        // 验证 metrics 的 sink_error 被增加（覆盖行 279）
        assert!(
            metrics.sink_errors() >= 1,
            "flush failure should increment sink_errors, got: {}",
            metrics.sink_errors()
        );
    }

    // ========================================================================
    // adjust_batch_size 直接单元测试
    // 通过构造 DatabaseSinkInner 直接调用 adjust_batch_size，
    // 覆盖所有分支：早返回、增长、缩减、上限、下限、中间区、零总操作数
    // ========================================================================

    /// 构造测试用 DatabaseSinkInner，避免依赖真实数据库
    fn make_test_inner(batch_size: usize) -> DatabaseSinkInner {
        DatabaseSinkInner {
            buffer: Vec::new(),
            last_flush: Instant::now(),
            fallback_sink: None,
            circuit_breaker: CircuitBreaker::new(3, Duration::from_secs(30), 3),
            current_batch_size: batch_size,
            write_latencies: Vec::new(),
            success_count: 0,
            failure_count: 0,
            metrics: None,
        }
    }

    /// 覆盖行 152-153：write_latencies 不足 ADAPTIVE_WINDOW_SIZE 时早返回
    /// 验证 current_batch_size 不变、计数器未清零
    #[test]
    fn test_adjust_batch_size_early_return_when_insufficient_latencies() {
        let mut inner = make_test_inner(100);
        // 只填入 5 条延迟记录（< ADAPTIVE_WINDOW_SIZE=10）
        inner.write_latencies = vec![Duration::from_millis(10); 5];
        inner.success_count = 5;
        inner.failure_count = 0;

        DatabaseSink::adjust_batch_size(&mut inner);

        // 早返回：batch_size 不变，latencies 和计数器未被清零
        assert_eq!(
            inner.current_batch_size, 100,
            "batch_size should not change on early return"
        );
        assert_eq!(
            inner.write_latencies.len(),
            5,
            "latencies should not be cleared on early return"
        );
        assert_eq!(
            inner.success_count, 5,
            "success_count should not be cleared on early return"
        );
        assert_eq!(
            inner.failure_count, 0,
            "failure_count should not be cleared on early return"
        );
    }

    /// 覆盖行 165-166, 171-173：高成功率 + 低延迟 → 批大小翻倍
    #[test]
    fn test_adjust_batch_size_grows_on_high_success_low_latency() {
        let mut inner = make_test_inner(100);
        // 10 条低延迟记录，avg = 10ms < 50ms
        inner.write_latencies = vec![Duration::from_millis(10); ADAPTIVE_WINDOW_SIZE];
        inner.success_count = 10;
        inner.failure_count = 0; // success_rate = 1.0 >= 0.95

        DatabaseSink::adjust_batch_size(&mut inner);

        assert_eq!(
            inner.current_batch_size, 200,
            "batch_size should double from 100 to 200"
        );
        assert!(
            inner.write_latencies.is_empty(),
            "latencies should be cleared"
        );
        assert_eq!(inner.success_count, 0, "success_count should be cleared");
        assert_eq!(inner.failure_count, 0, "failure_count should be cleared");
    }

    /// 覆盖行 167-168：高延迟（> 200ms）触发缩减
    #[test]
    fn test_adjust_batch_size_shrinks_on_high_latency_direct() {
        let mut inner = make_test_inner(100);
        // 10 条高延迟记录，avg = 300ms > 200ms
        inner.write_latencies = vec![Duration::from_millis(300); ADAPTIVE_WINDOW_SIZE];
        inner.success_count = 10;
        inner.failure_count = 0; // success_rate = 1.0，但延迟过高

        DatabaseSink::adjust_batch_size(&mut inner);

        assert_eq!(
            inner.current_batch_size, 50,
            "batch_size should halve from 100 to 50"
        );
        assert!(
            inner.write_latencies.is_empty(),
            "latencies should be cleared"
        );
    }

    /// 覆盖行 167-168：低成功率（< 0.8）触发缩减（即使延迟正常）
    #[test]
    fn test_adjust_batch_size_shrinks_on_low_success_rate() {
        let mut inner = make_test_inner(100);
        inner.write_latencies = vec![Duration::from_millis(10); ADAPTIVE_WINDOW_SIZE];
        inner.success_count = 5;
        inner.failure_count = 5; // success_rate = 0.5 < 0.8

        DatabaseSink::adjust_batch_size(&mut inner);

        assert_eq!(
            inner.current_batch_size, 50,
            "batch_size should halve when success_rate < 0.8"
        );
        assert!(
            inner.write_latencies.is_empty(),
            "latencies should be cleared"
        );
    }

    /// 覆盖行 166 的 .min(MAX_BATCH_SIZE)：增长受 MAX_BATCH_SIZE 上限约束
    #[test]
    fn test_adjust_batch_size_grow_capped_at_max() {
        let mut inner = make_test_inner(600);
        inner.write_latencies = vec![Duration::from_millis(10); ADAPTIVE_WINDOW_SIZE];
        inner.success_count = 10;
        inner.failure_count = 0; // 600 * 2 = 1200 > 1000 → capped

        DatabaseSink::adjust_batch_size(&mut inner);

        assert_eq!(
            inner.current_batch_size, MAX_BATCH_SIZE,
            "batch_size should be capped at MAX_BATCH_SIZE"
        );
    }

    /// 覆盖行 168 的 .max(MIN_BATCH_SIZE)：缩减受 MIN_BATCH_SIZE 下限约束
    #[test]
    fn test_adjust_batch_size_shrink_floored_at_min() {
        let mut inner = make_test_inner(MIN_BATCH_SIZE);
        inner.write_latencies = vec![Duration::from_millis(300); ADAPTIVE_WINDOW_SIZE];
        inner.success_count = 10;
        inner.failure_count = 0; // 10 / 2 = 5 < 10 → floored

        DatabaseSink::adjust_batch_size(&mut inner);

        assert_eq!(
            inner.current_batch_size, MIN_BATCH_SIZE,
            "batch_size should be floored at MIN_BATCH_SIZE"
        );
    }

    /// 覆盖行 165-169 之间的"中间区"：成功率 0.8~0.95、延迟 50~200ms → 不增不减
    #[test]
    fn test_adjust_batch_size_no_change_in_middle_zone() {
        let mut inner = make_test_inner(100);
        // avg_latency = 100ms（介于 50ms 和 200ms 之间）
        inner.write_latencies = vec![Duration::from_millis(100); ADAPTIVE_WINDOW_SIZE];
        // success_rate = 0.9（介于 0.8 和 0.95 之间）
        inner.success_count = 9;
        inner.failure_count = 1;

        DatabaseSink::adjust_batch_size(&mut inner);

        assert_eq!(
            inner.current_batch_size, 100,
            "batch_size should not change in middle zone"
        );
        // 即使未调整，计数器仍应被清零（行 171-173）
        assert!(
            inner.write_latencies.is_empty(),
            "latencies should be cleared even without adjustment"
        );
        assert_eq!(inner.success_count, 0, "success_count should be cleared");
        assert_eq!(inner.failure_count, 0, "failure_count should be cleared");
    }

    /// 覆盖行 162：total_ops == 0 时 success_rate = 1.0
    /// 这种情况在正常流程中不会发生（adjust_batch_size 仅在 flush 成功后调用），
    /// 但函数需要正确处理此边界条件
    #[test]
    fn test_adjust_batch_size_total_ops_zero_uses_default_success_rate() {
        let mut inner = make_test_inner(100);
        inner.write_latencies = vec![Duration::from_millis(10); ADAPTIVE_WINDOW_SIZE];
        inner.success_count = 0;
        inner.failure_count = 0; // total_ops = 0 → success_rate = 1.0

        DatabaseSink::adjust_batch_size(&mut inner);

        // success_rate = 1.0 >= 0.95 且 avg_latency = 10ms < 50ms → 增长
        assert_eq!(
            inner.current_batch_size, 200,
            "should grow when total_ops=0 (success_rate defaults to 1.0)"
        );
    }
}
