// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! DatabaseSink implementation details.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;

use super::CircuitBreaker;
use super::FileSink;
use crate::FileSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::Metrics;

use super::{DatabaseSink, DatabaseSinkInner};

pub(super) const DEFAULT_BATCH_SIZE: usize = 100;
pub(super) const DEFAULT_FLUSH_INTERVAL_MS: u64 = 500;
pub(super) const MIN_BATCH_SIZE: usize = 10;
pub(super) const MAX_BATCH_SIZE: usize = 1000;
pub(super) const ADAPTIVE_WINDOW_SIZE: usize = 10;

/// Maximum number of database worker connections, capped at `min(num_cpus, 4)`.
/// Prevents resource exhaustion when pool_size is set too high.
pub(super) const MAX_DB_WORKER_LIMIT: usize = 4;

/// Compute the effective upper bound for database worker threads/connections.
/// Returns `min(num_cpus::get().max(1), MAX_DB_WORKER_LIMIT)`.
pub(crate) fn effective_db_worker_limit() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, MAX_DB_WORKER_LIMIT)
}

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
            flush_buffer: Vec::with_capacity(batch_size),
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
            inner: parking_lot::Mutex::new(inner),
            database,
            masker: Arc::new(crate::DataMasker::new()),
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub async fn set_metrics(&self, metrics: Arc<Metrics>) {
        let mut inner = self.inner.lock();
        inner.metrics = Some(metrics);
    }

    pub(super) fn adjust_batch_size(inner: &mut DatabaseSinkInner) {
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
}

impl fmt::Display for DatabaseSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatabaseSink")
    }
}

#[async_trait]
impl crate::support::io::sink::LogSink for DatabaseSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        // Phase 1: under lock — check circuit breaker, push record, maybe swap buffers
        let (records_to_flush, should_flush, circuit_open) = {
            let mut inner = self.inner.lock();

            if !inner.circuit_breaker.can_execute() {
                (Vec::new(), false, true)
            } else {
                let masked_record = LogRecord {
                    message: self.masker.mask(&record.message),
                    ..record.clone()
                };
                inner.buffer.push(masked_record);

                let should = inner.buffer.len() >= inner.current_batch_size
                    || inner.last_flush.elapsed()
                        > Duration::from_millis(DEFAULT_FLUSH_INTERVAL_MS);

                if should {
                    // Double-buffer swap via mem::take (avoids split-borrow issue)
                    let tmp = std::mem::take(&mut inner.buffer);
                    inner.buffer = std::mem::take(&mut inner.flush_buffer);
                    inner.flush_buffer = tmp;
                    inner.last_flush = Instant::now();
                    let records = std::mem::take(&mut inner.flush_buffer);
                    (records, true, false)
                } else {
                    (Vec::new(), false, false)
                }
            }
        };
        // Lock released — no parking_lot MutexGuard held across await

        // Circuit breaker open → fallback to file sink
        if circuit_open {
            let fallback = self.inner.lock().fallback_sink.clone();
            if let Some(sink) = fallback
                && let Err(e) = sink.write(record).await
            {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set("err", e.to_string());
                tracing::warn!(
                    "{}",
                    crate::i18n::tr_args("warn-fallback_write_failed", args)
                );
            }
            return Ok(());
        }

        if should_flush {
            let start = Instant::now();
            let result = self.db_insert_batch(&records_to_flush).await;

            match result {
                Ok(written) => {
                    let mut inner = self.inner.lock();
                    if let Some(ref m) = inner.metrics {
                        m.add_db_batch_records_total(written);
                        m.update_sink_health("database", true, None);
                    }
                    inner.success_count += 1;
                    inner.write_latencies.push(start.elapsed());
                    Self::adjust_batch_size(&mut inner);
                    inner.circuit_breaker.record_success();
                }
                Err(e) => {
                    let fallback;
                    {
                        let mut inner = self.inner.lock();
                        inner.failure_count += 1;
                        inner.circuit_breaker.record_failure();
                        if let Some(ref m) = inner.metrics {
                            m.inc_sink_error();
                            m.update_sink_health("database", false, Some(e.to_string()));
                        }
                        // Re-queue records for retry
                        inner.buffer.extend(records_to_flush);
                        fallback = inner.fallback_sink.clone();
                    }
                    // Fallback write outside lock
                    if let Some(sink) = fallback
                        && let Err(e) = sink.write(record).await
                    {
                        let mut args = fluent_bundle::FluentArgs::new();
                        args.set("err", e.to_string());
                        tracing::warn!(
                            "{}",
                            crate::i18n::tr_args("warn-fallback_write_failed", args)
                        );
                    }
                    return Err(e);
                }
            }
        } else {
            self.inner.lock().circuit_breaker.record_success();
        }

        Ok(())
    }

    async fn flush(&self) -> Result<(), InklogError> {
        let records_to_flush;
        let metrics_clone;

        {
            let mut inner = self.inner.lock();
            if inner.buffer.is_empty() {
                return Ok(());
            }
            let tmp = std::mem::take(&mut inner.buffer);
            inner.buffer = std::mem::take(&mut inner.flush_buffer);
            inner.flush_buffer = tmp;
            inner.last_flush = Instant::now();
            records_to_flush = std::mem::take(&mut inner.flush_buffer);
            metrics_clone = inner.metrics.clone();
        }
        // Lock released

        self.finish_flush(records_to_flush, metrics_clone).await
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        self.stop.store(true, Ordering::Relaxed);
        let records_to_flush;
        let metrics_clone;

        {
            let mut inner = self.inner.lock();
            let tmp = std::mem::take(&mut inner.buffer);
            inner.buffer = std::mem::take(&mut inner.flush_buffer);
            inner.flush_buffer = tmp;
            inner.last_flush = Instant::now();
            records_to_flush = std::mem::take(&mut inner.flush_buffer);
            metrics_clone = inner.metrics.clone();
        }
        // Lock released

        let _ = self.finish_flush(records_to_flush, metrics_clone).await;
        tracing::info!("{}", crate::i18n::tr("info-db_shutdown_complete"));
        Ok(())
    }
}

impl DatabaseSink {
    /// Insert a batch of records into the database (no lock held).
    async fn db_insert_batch(&self, records: &[LogRecord]) -> Result<usize, InklogError> {
        let written = self.database.insert_batch(records).await?;
        Ok(written)
    }

    /// Complete a flush operation: insert batch + update metrics/state.
    /// Called after the lock has been released (double-buffer pattern).
    async fn finish_flush(
        &self,
        records: Vec<LogRecord>,
        metrics: Option<Arc<Metrics>>,
    ) -> Result<(), InklogError> {
        if records.is_empty() {
            return Ok(());
        }

        let batch_size = records.len();
        if let Some(ref m) = metrics {
            m.set_db_batch_size(batch_size);
        }

        match self.database.insert_batch(&records).await {
            Ok(written) => {
                if let Some(ref m) = metrics {
                    m.add_db_batch_records_total(written);
                    m.update_sink_health("database", true, None);
                }
                Ok(())
            }
            Err(e) => {
                if let Some(ref m) = metrics {
                    m.inc_sink_error();
                    m.update_sink_health("database", false, Some(e.to_string()));
                }
                // Re-queue records for retry
                let mut inner = self.inner.lock();
                let mut retry_records = records;
                retry_records.append(&mut inner.buffer);
                inner.buffer = retry_records;
                Err(e)
            }
        }
    }
}

/// Convert LogRecord to Parquet format
#[cfg(feature = "parquet")]
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
        .map(|l| {
            serde_json::to_string(&l.fields)
                .inspect_err(|e| {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("err", e.to_string());
                    tracing::warn!(
                        "{}",
                        crate::i18n::tr_args("config-json_serialize_failed", args)
                    );
                })
                .ok()
        })
        .collect();
    let files: Vec<Option<&str>> = logs.iter().map(|l| l.file.as_deref()).collect();
    let lines: Vec<Option<i32>> = logs
        .iter()
        .map(|l| {
            l.line.and_then(|line_num| {
                line_num.try_into().ok().or_else(|| {
                    tracing::warn!(line_num, "line_num exceeds i32 range, clamping");
                    Some(i32::MAX)
                })
            })
        })
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

/// Convert LogRecord to Parquet format fallback (parquet feature not enabled).
///
/// Returns an explicit error when the `parquet` feature is disabled, rather than
/// silently producing no output.
#[cfg(not(feature = "parquet"))]
pub fn convert_logs_to_parquet(
    _logs: &[crate::LogRecord],
    _config: &crate::ParquetConfig,
) -> Result<Vec<u8>, String> {
    Err("parquet feature not enabled: rebuild inklog with `parquet` feature to export logs as Parquet".to_string())
}
