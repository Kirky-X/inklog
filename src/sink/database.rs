use crate::config::{DatabaseDriver, DatabaseSinkConfig, FileSinkConfig};
use crate::error::InklogError;
use crate::log_record::LogRecord;
use crate::sink::file::FileSink;
use crate::sink::{CircuitBreaker, LogSink};
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait, QueryFilter,
    QuerySelect, Schema, Set, Statement,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

use chrono::{Datelike, Timelike};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timestamp: DateTimeUtc,
    pub level: String,
    pub target: String,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    #[sea_orm(column_type = "Json", nullable)]
    pub fields: Option<serde_json::Value>,
    pub file: Option<String>,
    pub line: Option<i32>,
    pub thread_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// Archive Metadata Entity Module
pub mod archive_metadata {
    use sea_orm::entity::prelude::*;
    use serde::Serialize;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "archive_metadata")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub archive_date: DateTimeUtc,
        pub s3_key: String,
        pub record_count: i64,
        pub file_size: i64,
        pub status: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

use archive_metadata::ActiveModel as ArchiveMetadataActiveModel;
use archive_metadata::Entity as ArchiveMetadataEntity;

pub struct DatabaseSink {
    config: DatabaseSinkConfig,
    buffer: Vec<LogRecord>,
    last_flush: Instant,
    last_archive_check: chrono::DateTime<chrono::Utc>,
    last_partition_check: chrono::DateTime<chrono::Utc>,
    rt: Runtime,
    db: Option<DatabaseConnection>,
    fallback_sink: Option<FileSink>,
    circuit_breaker: CircuitBreaker,
}

impl DatabaseSink {
    pub fn new(config: DatabaseSinkConfig) -> Result<Self, InklogError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(InklogError::IoError)?;

        // Initialize fallback sink
        let fallback_config = FileSinkConfig {
            enabled: true,
            path: PathBuf::from("logs/db_fallback.log"),
            ..Default::default()
        };
        let fallback_sink = FileSink::new(fallback_config).ok();

        let mut sink = Self {
            config: config.clone(),
            buffer: Vec::with_capacity(config.batch_size),
            last_flush: Instant::now(),
            last_archive_check: Utc::now(),
            last_partition_check: Utc::now() - chrono::Duration::days(1),
            rt,
            db: None,
            fallback_sink,
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(30)),
        };

        let _ = sink.init_db(); // 不要因为初始化失败而导致整个系统崩溃，断路器会处理
        Ok(sink)
    }

    fn init_db(&mut self) -> Result<(), InklogError> {
        let url = self.config.url.clone();
        let pool_size = self.config.pool_size;
        let db = self
            .rt
            .block_on(async {
                let mut opt = ConnectOptions::new(url);
                opt.max_connections(pool_size)
                    .min_connections(2)
                    .connect_timeout(Duration::from_secs(5))
                    .idle_timeout(Duration::from_secs(8));

                Database::connect(opt).await
            })
            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

        self.rt
            .block_on(async {
                let builder = db.get_database_backend();
                let schema = Schema::new(builder);

                match self.config.driver {
                    DatabaseDriver::PostgreSQL => {
                        let stmt =
                            builder.build(schema.create_table_from_entity(Entity).if_not_exists());
                        db.execute_unprepared(&stmt.sql)
                            .await
                            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
                    }
                    DatabaseDriver::MySQL => {
                        let create_table_sql = r#"
                            CREATE TABLE IF NOT EXISTS `logs` (
                                `id` BIGINT AUTO_INCREMENT PRIMARY KEY,
                                `timestamp` DATETIME(3) NOT NULL,
                                `level` VARCHAR(20) NOT NULL,
                                `target` VARCHAR(255) NOT NULL,
                                `message` TEXT NOT NULL,
                                `fields` JSON,
                                `file` VARCHAR(512),
                                `line` INT,
                                `thread_id` VARCHAR(100) NOT NULL,
                                INDEX `idx_timestamp` (`timestamp`),
                                INDEX `idx_level` (`level`),
                                INDEX `idx_target` (`target`)
                            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
                        "#;
                        let stmt = Statement::from_string(
                            sea_orm::DatabaseBackend::MySql,
                            create_table_sql,
                        );
                        db.execute_unprepared(&stmt.sql)
                            .await
                            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
                    }
                    DatabaseDriver::SQLite => {
                        let create_table_sql = r#"
                            CREATE TABLE IF NOT EXISTS "logs" (
                                "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                                "timestamp" TEXT NOT NULL,
                                "level" TEXT NOT NULL,
                                "target" TEXT NOT NULL,
                                "message" TEXT NOT NULL,
                                "fields" TEXT,
                                "file" TEXT,
                                "line" INTEGER,
                                "thread_id" TEXT NOT NULL
                            )
                        "#;
                        let stmt = Statement::from_string(
                            sea_orm::DatabaseBackend::Sqlite,
                            create_table_sql,
                        );
                        db.execute_unprepared(&stmt.sql)
                            .await
                            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

                        let create_index_sql = r#"
                            CREATE INDEX IF NOT EXISTS "idx_logs_timestamp" ON "logs" ("timestamp")
                        "#;
                        let stmt_index = Statement::from_string(
                            sea_orm::DatabaseBackend::Sqlite,
                            create_index_sql,
                        );
                        db.execute_unprepared(&stmt_index.sql)
                            .await
                            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
                    }
                }

                let stmt_archive = builder.build(
                    schema
                        .create_table_from_entity(ArchiveMetadataEntity)
                        .if_not_exists(),
                );
                db.execute_unprepared(&stmt_archive.sql)
                    .await
                    .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

                Ok::<(), InklogError>(())
            })
            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

        self.db = Some(db);
        Ok(())
    }

    fn flush_buffer(&mut self) -> Result<(), InklogError> {
        eprintln!(
            "DEBUG: flush_buffer called, buffer.len()={}",
            self.buffer.len()
        );
        if self.buffer.is_empty() {
            eprintln!("DEBUG: flush_buffer - buffer is empty, returning early");
            return Ok(());
        }

        // 动态调整批大小：如果最近有失败，减小批大小以提高成功率
        let current_batch_size =
            if self.circuit_breaker.state() == crate::sink::CircuitState::HalfOpen {
                self.config.batch_size / 2
            } else {
                self.config.batch_size
            };

        eprintln!(
            "DEBUG: flush_buffer - current_batch_size={}, flush_interval_ms={}, elapsed={}ms",
            current_batch_size,
            self.config.flush_interval_ms,
            self.last_flush.elapsed().as_millis()
        );

        // 只有在缓冲区大小小于当前批次大小且距离上次刷新时间小于刷新间隔时才跳过刷新
        if self.buffer.len() < current_batch_size
            && self.last_flush.elapsed() < Duration::from_millis(self.config.flush_interval_ms)
        {
            eprintln!("DEBUG: flush_buffer - skipping due to conditions");
            return Ok(());
        }

        // 检查断路器
        eprintln!("DEBUG: flush_buffer - checking circuit breaker");
        if !self.circuit_breaker.can_execute() {
            eprintln!("DEBUG: flush_buffer - circuit breaker blocked");
            self.fallback_to_file()?;
            self.buffer.clear();
            self.last_flush = Instant::now();
            return Ok(());
        }

        // Partition check
        let now = Utc::now();
        let should_check_partition = now.date_naive() != self.last_partition_check.date_naive();
        if should_check_partition {
            self.last_partition_check = now;
        }

        let mut success = false;
        eprintln!(
            "DEBUG: flush_buffer - preparing to insert {} records",
            self.buffer.len()
        );
        if let Some(db) = &self.db {
            let logs: Vec<ActiveModel> = self
                .buffer
                .iter()
                .map(|r| ActiveModel {
                    timestamp: Set(r.timestamp),
                    level: Set(r.level.clone()),
                    target: Set(r.target.clone()),
                    message: Set(r.message.clone()),
                    fields: Set(Some(
                        serde_json::to_value(&r.fields).unwrap_or(serde_json::Value::Null),
                    )),
                    file: Set(r.file.clone()),
                    line: Set(r.line.map(|l| l as i32)),
                    thread_id: Set(r.thread_id.clone()),
                    ..Default::default()
                })
                .collect();

            eprintln!("DEBUG: flush_buffer - logs prepared, entering async block");
            let res = self.rt.block_on(async {
                eprintln!("DEBUG: flush_buffer - inside async block");
                match self.config.driver {
                    DatabaseDriver::PostgreSQL => {
                        if should_check_partition {
                            let partition_name = format!("logs_{}", now.format("%Y_%m"));
                            let start_date = now.format("%Y-%m-01").to_string();
                            let next_month = if now.month() == 12 {
                                format!("{}-01-01", now.year() + 1)
                            } else {
                                format!("{}-{:02}-01", now.year(), now.month() + 1)
                            };
                            let sql = format!(
                                "CREATE TABLE IF NOT EXISTS {} PARTITION OF {} FOR VALUES FROM ('{}') TO ('{}')",
                                partition_name, self.config.table_name, start_date, next_month
                            );
                            let stmt = Statement::from_string(db.get_database_backend(), sql);
                            let _ = db.execute_unprepared(&stmt.sql).await;
                        }
                    }
                    DatabaseDriver::MySQL => {
                        if should_check_partition {
                            let partition_sql = format!(
                                "CREATE TABLE IF NOT EXISTS `logs_{}` PARTITION OF `logs` FOR VALUES IN (TO_DAYS('{}'))",
                                now.format("%Y_%m"),
                                now.format("%Y-%m-01")
                            );
                            let stmt = Statement::from_string(sea_orm::DatabaseBackend::MySql, partition_sql);
                            let _ = db.execute_unprepared(&stmt.sql).await;
                        }
                    }
                    DatabaseDriver::SQLite => {}
                }
                eprintln!("DEBUG: flush_buffer - about to insert {} records", logs.len());
                let insert_result = Entity::insert_many(logs).exec(db).await;
                eprintln!("DEBUG: flush_buffer - insert result: {:?}", insert_result);
                insert_result
            });

            match res {
                Ok(_) => {
                    self.circuit_breaker.record_success();
                    success = true;
                }
                Err(e) => {
                    eprintln!("Database insert error: {}", e);
                    self.circuit_breaker.record_failure();
                    // 尝试重新连接（如果是半开启状态或连接丢失）
                    let _ = self.init_db();
                }
            }
        }

        if !success {
            self.fallback_to_file()?;
        }

        self.buffer.clear();
        self.last_flush = Instant::now();
        Ok(())
    }

    fn fallback_to_file(&mut self) -> Result<(), InklogError> {
        if let Some(sink) = &mut self.fallback_sink {
            for record in &self.buffer {
                let _ = sink.write(record);
            }
        }
        Ok(())
    }

    // S3 Archive Logic - Moved to write() to avoid borrow checker issues
}

impl LogSink for DatabaseSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError> {
        self.buffer.push(record.clone());

        if self.buffer.len() >= self.config.batch_size
            || self.last_flush.elapsed() >= Duration::from_millis(self.config.flush_interval_ms)
        {
            eprintln!(
                "DEBUG: write - buffer.len()={}, batch_size={}, elapsed={}ms",
                self.buffer.len(),
                self.config.batch_size,
                self.last_flush.elapsed().as_millis()
            );
            eprintln!("DEBUG: write - db.is_some()={}", self.db.is_some());
            if let Err(e) = self.flush_buffer() {
                eprintln!("DEBUG: flush_buffer error: {:?}", e);
            }
        }

        // Periodically check for archive
        let now = Utc::now();
        // Check if it's 2 AM and we haven't checked today
        if now.hour() == 2 && self.last_archive_check.date_naive() != now.date_naive() {
            self.last_archive_check = now;
            let db_opt = self.db.clone();
            let config = self.config.clone();

            if let Some(db) = db_opt {
                if config.archive_to_s3 {
                    let res = self.rt.block_on(async move {
                        // Logic from archive_logs adapted to not use self
                        let days = config.archive_after_days as i64;
                        let cutoff = Utc::now() - chrono::Duration::days(days);

                        let logs = Entity::find()
                            .filter(Column::Timestamp.lt(cutoff))
                            .limit(1000)
                            .all(&db)
                            .await
                            .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

                        if logs.is_empty() {
                            return Ok(());
                        }

                        // Convert logs to Parquet format
                        let parquet_data = convert_logs_to_parquet(&logs).map_err(|e| {
                            InklogError::SerializationError(serde_json::Error::io(
                                std::io::Error::other(e.to_string()),
                            ))
                        })?;

                        let file_size = parquet_data.len() as i64;

                        #[cfg(feature = "aws")]
                        {
                            if let (Some(bucket), Some(region)) =
                                (&config.s3_bucket, &config.s3_region)
                            {
                                let aws_config = aws_config::from_env()
                                    .region(aws_types::region::Region::new(region.clone()))
                                    .load()
                                    .await;
                                let client = aws_sdk_s3::Client::new(&aws_config);
                                let key = format!(
                                    "{}/{}/logs_{}.parquet",
                                    Utc::now().format("%Y"),
                                    Utc::now().format("%m"),
                                    Utc::now().format("%d_%H%M%S")
                                );

                                client
                                    .put_object()
                                    .bucket(bucket)
                                    .key(&key)
                                    .body(parquet_data.into())
                                    .storage_class(aws_sdk_s3::types::StorageClass::Glacier)
                                    .send()
                                    .await
                                    .map_err(|e| InklogError::S3Error(e.to_string()))?;

                                let meta = ArchiveMetadataActiveModel {
                                    archive_date: Set(Utc::now()),
                                    s3_key: Set(key),
                                    record_count: Set(logs.len() as i64),
                                    file_size: Set(file_size),
                                    status: Set("SUCCESS".to_string()),
                                    ..Default::default()
                                };
                                ArchiveMetadataEntity::insert(meta)
                                    .exec(&db)
                                    .await
                                    .map_err(|e| InklogError::DatabaseError(e.to_string()))?;

                                let ids: Vec<i64> = logs.iter().map(|l| l.id).collect();
                                Entity::delete_many()
                                    .filter(Column::Id.is_in(ids))
                                    .exec(&db)
                                    .await
                                    .map_err(|e| InklogError::DatabaseError(e.to_string()))?;
                            }
                        }

                        #[cfg(not(feature = "aws"))]
                        {
                            // 本地归档：保存Parquet文件到本地目录
                            let archive_dir = std::path::Path::new("logs/archive");
                            if let Err(e) = std::fs::create_dir_all(archive_dir) {
                                eprintln!("Failed to create archive directory: {}", e);
                            } else {
                                let filename =
                                    format!("logs_{}.parquet", Utc::now().format("%Y%m%d_%H%M%S"));
                                let filepath = archive_dir.join(&filename);
                                if let Err(e) = std::fs::write(&filepath, &parquet_data) {
                                    eprintln!("Failed to write archive file: {}", e);
                                } else {
                                    let meta = ArchiveMetadataActiveModel {
                                        archive_date: Set(Utc::now()),
                                        s3_key: Set(format!("local/{}", filename)),
                                        record_count: Set(logs.len() as i64),
                                        file_size: Set(file_size),
                                        status: Set("LOCAL_SUCCESS".to_string()),
                                        ..Default::default()
                                    };
                                    if let Err(e) = ArchiveMetadataEntity::insert(meta)
                                        .exec(&db)
                                        .await
                                        .map_err(|e| InklogError::DatabaseError(e.to_string()))
                                    {
                                        eprintln!("Failed to insert archive metadata: {}", e);
                                    }

                                    let ids: Vec<i64> = logs.iter().map(|l| l.id).collect();
                                    if let Err(e) = Entity::delete_many()
                                        .filter(Column::Id.is_in(ids))
                                        .exec(&db)
                                        .await
                                        .map_err(|e| InklogError::DatabaseError(e.to_string()))
                                    {
                                        eprintln!("Failed to delete archived logs: {}", e);
                                    }
                                }
                            }
                        }
                        Ok::<(), InklogError>(())
                    });

                    if let Err(e) = res {
                        eprintln!("Archive error: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), InklogError> {
        self.flush_buffer()
    }

    fn is_healthy(&self) -> bool {
        self.db.is_some()
    }

    fn shutdown(&mut self) -> Result<(), InklogError> {
        self.flush_buffer()?;
        if let Some(db) = self.db.take() {
            self.rt.block_on(async move {
                let _ = db.close().await;
            });
        }
        Ok(())
    }
}

/// Convert logs to Parquet format using Arrow schema
pub fn convert_logs_to_parquet(
    logs: &[Model],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use parquet::basic::{Compression, Encoding};
    use parquet::file::properties::WriterProperties;
    use std::io::Cursor;
    use std::sync::Arc;

    // Create Arrow schema for log data
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("timestamp", DataType::Utf8, false),
        Field::new("level", DataType::Utf8, false),
        Field::new("target", DataType::Utf8, false),
        Field::new("message", DataType::Utf8, false),
        Field::new("fields", DataType::Utf8, true),
        Field::new("file", DataType::Utf8, true),
        Field::new("line", DataType::Int64, true),
        Field::new("thread_id", DataType::Utf8, false),
    ]));

    // Convert Model data to Arrow arrays
    let mut id_builder = Vec::with_capacity(logs.len());
    let mut timestamp_builder = Vec::with_capacity(logs.len());
    let mut level_builder = Vec::with_capacity(logs.len());
    let mut target_builder = Vec::with_capacity(logs.len());
    let mut message_builder = Vec::with_capacity(logs.len());
    let mut fields_builder = Vec::with_capacity(logs.len());
    let mut file_builder = Vec::with_capacity(logs.len());
    let mut line_builder = Vec::with_capacity(logs.len());
    let mut thread_id_builder = Vec::with_capacity(logs.len());

    for log in logs {
        id_builder.push(log.id);
        timestamp_builder.push(log.timestamp.to_rfc3339());
        level_builder.push(log.level.clone());
        target_builder.push(log.target.clone());
        message_builder.push(log.message.clone());
        fields_builder.push(serde_json::to_string(&log.fields).ok());
        file_builder.push(log.file.clone());
        line_builder.push(log.line.map(|l| l as i64));
        thread_id_builder.push(log.thread_id.clone());
    }

    // Create Arrow arrays
    let id_array = Arc::new(Int64Array::from(id_builder)) as ArrayRef;
    let timestamp_array = Arc::new(StringArray::from(timestamp_builder)) as ArrayRef;
    let level_array = Arc::new(StringArray::from(level_builder)) as ArrayRef;
    let target_array = Arc::new(StringArray::from(target_builder)) as ArrayRef;
    let message_array = Arc::new(StringArray::from(message_builder)) as ArrayRef;
    let fields_array = Arc::new(StringArray::from(fields_builder)) as ArrayRef;
    let file_array = Arc::new(StringArray::from(file_builder)) as ArrayRef;
    let line_array = Arc::new(Int64Array::from(line_builder)) as ArrayRef;
    let thread_id_array = Arc::new(StringArray::from(thread_id_builder)) as ArrayRef;

    // Create RecordBatch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            id_array,
            timestamp_array,
            level_array,
            target_array,
            message_array,
            fields_array,
            file_array,
            line_array,
            thread_id_array,
        ],
    )?;

    // Write to Parquet format
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);

    let writer_properties = WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .set_encoding(Encoding::PLAIN)
        .build();

    let mut writer = ArrowWriter::try_new(cursor, schema, Some(writer_properties))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(buffer)
}
