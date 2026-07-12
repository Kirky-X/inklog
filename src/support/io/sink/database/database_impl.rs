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

use crate::FileSinkConfig;
use crate::InklogError;
use crate::LogRecord;
use crate::Metrics;
use crate::support::io::sink::circuit_breaker::CircuitBreaker;
use crate::support::io::sink::file::FileSink;

use super::{DatabaseSink, DatabaseSinkInner};

pub(super) const DEFAULT_BATCH_SIZE: usize = 100;
pub(super) const DEFAULT_FLUSH_INTERVAL_MS: u64 = 500;
pub(super) const MIN_BATCH_SIZE: usize = 10;
pub(super) const MAX_BATCH_SIZE: usize = 1000;
pub(super) const ADAPTIVE_WINDOW_SIZE: usize = 10;

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
            inner: tokio::sync::Mutex::new(inner),
            database,
            masker: Arc::new(crate::DataMasker::new()),
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub async fn set_metrics(&self, metrics: Arc<Metrics>) {
        let mut inner = self.inner.lock().await;
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
        let mut inner = self.inner.lock().await;

        if !inner.circuit_breaker.can_execute() {
            if let Some(ref sink) = inner.fallback_sink {
                let _ = sink.write(record).await;
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
            if let Err(e) = Self::flush_inner(self, &mut inner).await {
                inner.failure_count += 1;
                inner.circuit_breaker.record_failure();
                if let Some(ref sink) = inner.fallback_sink {
                    let _ = sink.write(record).await;
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

    async fn flush(&self) -> Result<(), InklogError> {
        let mut inner = self.inner.lock().await;
        Self::flush_inner(self, &mut inner).await
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        self.stop.store(true, Ordering::Relaxed);
        let mut inner = self.inner.lock().await;
        let _ = Self::flush_inner(self, &mut inner).await;
        tracing::info!("Database sink shutdown complete");
        Ok(())
    }
}

impl DatabaseSink {
    async fn flush_inner(&self, inner: &mut DatabaseSinkInner) -> Result<(), InklogError> {
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
        // 直接 await，不再通过 execute_async + block_in_place 包装（T010）
        match self.database.insert_batch(&records).await {
            Ok(written) => {
                if let Some(m) = &inner.metrics {
                    m.add_db_batch_records_total(written);
                    m.update_sink_health("database", true, None);
                }
                Ok(())
            }
            Err(e) => {
                if let Some(m) = &inner.metrics {
                    m.inc_sink_error();
                    m.update_sink_health("database", false, Some(e.to_string()));
                }
                // 恢复 buffer，让下次 flush 重试
                inner.buffer = records;
                Err(e)
            }
        }
    }
}

/// Convert LogRecord to Parquet format
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
