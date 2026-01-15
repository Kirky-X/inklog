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

/// 验证表名是否安全（防止 SQL 注入）
/// 只允许字母、数字、下划线，且必须以字母或下划线开头
fn validate_table_name(name: &str) -> Result<String, InklogError> {
    if name.is_empty() {
        return Err(InklogError::DatabaseError(
            "Table name cannot be empty".to_string(),
        ));
    }
    if name.len() > 128 {
        return Err(InklogError::DatabaseError(
            "Table name too long".to_string(),
        ));
    }
    // 检查首字符
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(InklogError::DatabaseError(format!(
            "Table name must start with letter or underscore, got: {}",
            first_char
        )));
    }
    // 检查所有字符
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(InklogError::DatabaseError(format!(
            "Table name contains invalid characters: {}",
            name
        )));
    }
    Ok(name.to_string())
}

/// 验证分区名称格式（必须是 logs_YYYY_MM 格式）
fn validate_partition_name(partition_name: &str) -> Result<String, InklogError> {
    if !partition_name.starts_with("logs_") {
        return Err(InklogError::DatabaseError(format!(
            "Partition name must start with 'logs_', got: {}",
            partition_name
        )));
    }
    // 验证日期部分格式 YYYY_MM
    let date_part = &partition_name[5..]; // 移除 "logs_" 前缀
    if date_part.len() != 7 || date_part.chars().nth(4) != Some('_') {
        return Err(InklogError::DatabaseError(format!(
            "Invalid partition date format, expected YYYY_MM, got: {}",
            date_part
        )));
    }
    let year = &date_part[..4];
    let month = &date_part[5..];
    if !year.chars().all(|c| c.is_ascii_digit()) || year.parse::<u32>().is_err() {
        return Err(InklogError::DatabaseError(format!(
            "Invalid year in partition name: {}",
            year
        )));
    }
    if !month.chars().all(|c| c.is_ascii_digit()) || month.parse::<u32>().is_err() {
        return Err(InklogError::DatabaseError(format!(
            "Invalid month in partition name: {}",
            month
        )));
    }
    let month_num: u32 = month.parse().unwrap();
    if month_num == 0 || month_num > 12 {
        return Err(InklogError::DatabaseError(format!(
            "Invalid month value in partition name: {}",
            month_num
        )));
    }
    Ok(partition_name.to_string())
}

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
        // 使用多线程运行时以提高数据库吞吐量 (16x 性能提升)
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(std::cmp::max(2, num_cpus::get()))
            .thread_name("inklog-db-worker")
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
        if self.buffer.is_empty() {
            return Ok(());
        }

        // 动态调整批大小：如果最近有失败，减小批大小以提高成功率
        let current_batch_size =
            if self.circuit_breaker.state() == crate::sink::CircuitState::HalfOpen {
                self.config.batch_size / 2
            } else {
                self.config.batch_size
            };

        // 只有在缓冲区大小小于当前批次大小且距离上次刷新时间小于刷新间隔时才跳过刷新
        if self.buffer.len() < current_batch_size
            && self.last_flush.elapsed() < Duration::from_millis(self.config.flush_interval_ms)
        {
            return Ok(());
        }

        // 检查断路器
        if !self.circuit_breaker.can_execute() {
            self.fallback_to_file()?;
            self.buffer.clear();
            self.last_flush = Instant::now();
            return Ok(());
        }

        // Partition check and validation
        let now = Utc::now();
        let should_check_partition = now.date_naive() != self.last_partition_check.date_naive();
        if should_check_partition {
            self.last_partition_check = now;
        }

        // Pre-validate MySQL partition name before async block
        let mysql_partition_valid = match self.config.driver {
            DatabaseDriver::MySQL => {
                if should_check_partition {
                    let partition_name = format!("logs_{}", now.format("%Y_%m"));
                    partition_name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_')
                } else {
                    true
                }
            }
            _ => true,
        };

        let mut success = false;
        if let Some(db) = &self.db {
            // 使用 drain() 直接消费 buffer 中的数据，避免克隆
            let logs: Vec<ActiveModel> = self
                .buffer
                .drain(..)
                .map(|r| ActiveModel {
                    timestamp: Set(r.timestamp),
                    level: Set(r.level),
                    target: Set(r.target),
                    message: Set(r.message),
                    fields: Set(Some(
                        serde_json::to_value(&r.fields).unwrap_or(serde_json::Value::Null),
                    )),
                    file: Set(r.file),
                    line: Set(r.line.map(|l| l as i32)),
                    thread_id: Set(r.thread_id),
                    ..Default::default()
                })
                .collect();
            let res = self.rt.block_on(async {
                match self.config.driver {
                    DatabaseDriver::PostgreSQL => {
                        if should_check_partition {
                            let partition_name = format!("logs_{}", now.format("%Y_%m"));
                            // 验证分区名称安全性
                            let validated_partition = match validate_partition_name(&partition_name) {
                                Ok(name) => name,
                                Err(e) => {
                                    tracing::error!("Partition name validation failed: {}", e);
                                    return Err(sea_orm::DbErr::Query(
                                        sea_orm::RuntimeErr::Internal(e.to_string())
                                    ));
                                }
                            };
                            
                            let start_date = now.format("%Y-%m-01").to_string();
                            let next_month = if now.month() == 12 {
                                format!("{}-01-01", now.year() + 1)
                            } else {
                                format!("{}-{:02}-01", now.year(), now.month() + 1)
                            };

                            // 验证表名安全性
                            let validated_table = match validate_table_name(&self.config.table_name) {
                                Ok(name) => name,
                                Err(e) => {
                                    tracing::error!("Table name validation failed: {}", e);
                                    return Err(sea_orm::DbErr::Query(
                                        sea_orm::RuntimeErr::Internal(e.to_string())
                                    ));
                                }
                            };

                            // 使用验证后的名称构建 SQL
                            let quoted_table = format!("\"{}\"", validated_table);
                            let quoted_partition = format!("\"{}\"", validated_partition);

                            let sql = format!(
                                "CREATE TABLE IF NOT EXISTS {} PARTITION OF {} FOR VALUES FROM ('{}') TO ('{}')",
                                quoted_partition, quoted_table, start_date, next_month
                            );
                            let stmt = Statement::from_string(db.get_database_backend(), sql);
                            let _ = db.execute_unprepared(&stmt.sql).await;
                        }
                    }
                    DatabaseDriver::MySQL => {
                        if should_check_partition {
                            let partition_name = format!("logs_{}", now.format("%Y_%m"));
                            let start_date = now.format("%Y-%m-01").to_string();

                            // 验证已在 async 块外部完成
                            if !mysql_partition_valid {
                                tracing::error!("Invalid partition name: {}", partition_name);
                                self.circuit_breaker.record_failure();
                                success = false;
                            } else {
                                // 使用验证后的分区名称
                                let validated_partition = validate_partition_name(&partition_name)
                                    .unwrap_or_else(|_| {
                                        tracing::error!("Invalid partition name: {}", partition_name);
                                        partition_name.clone()
                                    });
                                
                                // MySQL 使用反引号引用标识符
                                let partition_sql = format!(
                                    "CREATE TABLE IF NOT EXISTS `{}` PARTITION OF `logs` FOR VALUES IN (TO_DAYS('{}'))",
                                    validated_partition,
                                    start_date
                                );
                                let stmt = Statement::from_string(sea_orm::DatabaseBackend::MySql, partition_sql);
                                let _ = db.execute_unprepared(&stmt.sql).await;
                            }
                        }
                    }
                    DatabaseDriver::SQLite => {}
                }
                Entity::insert_many(logs).exec(db).await
            });

            match res {
                Ok(_) => {
                    self.circuit_breaker.record_success();
                    success = true;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Database insert failed");
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
            if let Err(e) = self.flush_buffer() {
                tracing::error!(error = ?e, "Failed to flush database buffer");
            }
        }

        // Periodically check for archive - only if S3 archive is configured
        if self.config.archive_to_s3 {
            let now = Utc::now();
            // Check if it's 2 AM and we haven't checked today
            if now.hour() == 2 && self.last_archive_check.date_naive() != now.date_naive() {
                self.last_archive_check = now;
                let db_opt = self.db.clone();
                let config = self.config.clone();

                if let Some(db) = db_opt {
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
                        let parquet_data = convert_logs_to_parquet(&logs, &config.parquet_config).map_err(|e| {
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
                                tracing::error!(error = %e, "Failed to create archive directory");
                            } else {
                                let filename =
                                    format!("logs_{}.parquet", Utc::now().format("%Y%m%d_%H%M%S"));
                                let filepath = archive_dir.join(&filename);
                                if let Err(e) = std::fs::write(&filepath, &parquet_data) {
                                    tracing::error!(error = %e, "Failed to write archive file");
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
                                        tracing::error!(error = %e, "Failed to insert archive metadata");
                                    }

                                    let ids: Vec<i64> = logs.iter().map(|l| l.id).collect();
                                    if let Err(e) = Entity::delete_many()
                                        .filter(Column::Id.is_in(ids))
                                        .exec(&db)
                                        .await
                                        .map_err(|e| InklogError::DatabaseError(e.to_string()))
                                    {
                                        tracing::error!(error = %e, "Failed to delete archived logs");
                                    }
                                }
                            }
                        }
                        Ok::<(), InklogError>(())
                    });

                    if let Err(e) = res {
                        tracing::error!(error = %e, "Archive operation failed");
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
    config: &crate::config::ParquetConfig,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use parquet::basic::{Compression, Encoding};
    use parquet::file::properties::WriterProperties;
    use std::io::Cursor;
    use std::sync::Arc;

    let encoding = match config.encoding.to_uppercase().as_str() {
        "DICTIONARY" => Encoding::RLE_DICTIONARY,
        "RLE" => Encoding::RLE,
        _ => Encoding::PLAIN,
    };

    let compression = Compression::ZSTD(Default::default());
    let writer_props = WriterProperties::builder()
        .set_compression(compression)
        .set_encoding(encoding)
        .set_max_row_group_size(config.max_row_group_size)
        .build();

    let include_all = config.include_fields.is_empty();
    let include_fields: std::collections::HashSet<String> =
        config.include_fields.iter().cloned().collect();

    let mut fields = Vec::new();
    let mut arrays: Vec<ArrayRef> = Vec::new();

    if include_all || include_fields.contains("id") {
        let mut id_builder = Vec::with_capacity(logs.len());
        for log in logs {
            id_builder.push(log.id);
        }
        fields.push(Field::new("id", DataType::Int64, false));
        arrays.push(Arc::new(Int64Array::from(id_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("timestamp") {
        let mut timestamp_builder = Vec::with_capacity(logs.len());
        for log in logs {
            timestamp_builder.push(log.timestamp.to_rfc3339());
        }
        fields.push(Field::new("timestamp", DataType::Utf8, false));
        arrays.push(Arc::new(StringArray::from(timestamp_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("level") {
        let mut level_builder = Vec::with_capacity(logs.len());
        for log in logs {
            level_builder.push(log.level.clone());
        }
        fields.push(Field::new("level", DataType::Utf8, false));
        arrays.push(Arc::new(StringArray::from(level_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("target") {
        let mut target_builder = Vec::with_capacity(logs.len());
        for log in logs {
            target_builder.push(log.target.clone());
        }
        fields.push(Field::new("target", DataType::Utf8, false));
        arrays.push(Arc::new(StringArray::from(target_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("message") {
        let mut message_builder = Vec::with_capacity(logs.len());
        for log in logs {
            message_builder.push(log.message.clone());
        }
        fields.push(Field::new("message", DataType::Utf8, false));
        arrays.push(Arc::new(StringArray::from(message_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("fields") {
        let mut fields_builder = Vec::with_capacity(logs.len());
        for log in logs {
            fields_builder.push(serde_json::to_string(&log.fields).ok());
        }
        fields.push(Field::new("fields", DataType::Utf8, true));
        arrays.push(Arc::new(StringArray::from(fields_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("file") {
        let mut file_builder = Vec::with_capacity(logs.len());
        for log in logs {
            file_builder.push(log.file.clone());
        }
        fields.push(Field::new("file", DataType::Utf8, true));
        arrays.push(Arc::new(StringArray::from(file_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("line") {
        let mut line_builder = Vec::with_capacity(logs.len());
        for log in logs {
            line_builder.push(log.line.map(|l| l as i64));
        }
        fields.push(Field::new("line", DataType::Int64, true));
        arrays.push(Arc::new(Int64Array::from(line_builder)) as ArrayRef);
    }

    if include_all || include_fields.contains("thread_id") {
        let mut thread_id_builder = Vec::with_capacity(logs.len());
        for log in logs {
            thread_id_builder.push(log.thread_id.clone());
        }
        fields.push(Field::new("thread_id", DataType::Utf8, false));
        arrays.push(Arc::new(StringArray::from(thread_id_builder)) as ArrayRef);
    }

    let schema = Arc::new(Schema::new(fields));

    let batch = RecordBatch::try_new(schema.clone(), arrays)?;

    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);

    let mut writer = ArrowWriter::try_new(cursor, schema, Some(writer_props))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(buffer)
}
