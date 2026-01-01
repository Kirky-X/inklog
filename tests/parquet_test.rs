//! Parquet功能验证测试
//!
//! 测试Parquet导出功能的正确性、性能和兼容性

use arrow_array::RecordBatchReader;
use bytes::Bytes;
use inklog::sink::database::convert_logs_to_parquet;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::time::Instant;

/// 创建测试日志数据
fn create_test_logs(count: usize) -> Vec<inklog::sink::database::Model> {
    (0..count)
        .map(|i| inklog::sink::database::Model {
            id: i as i64,
            timestamp: chrono::Utc::now(),
            level: match i % 5 {
                0 => "trace".to_string(),
                1 => "debug".to_string(),
                2 => "info".to_string(),
                3 => "warn".to_string(),
                _ => "error".to_string(),
            },
            target: format!("test_module::function_{}", i % 10),
            message: format!("Test log message number {}", i),
            fields: Some(serde_json::json!({
                "user_id": i,
                "request_id": format!("req-{:010x}", i),
                "duration_ms": i * 10,
            })),
            file: Some(format!("src/test_{}.rs", i % 5)),
            line: Some((i % 100) as i32),
            thread_id: format!("thread-{}", i % 4),
        })
        .collect()
}

/// 验证Parquet文件可读性和Schema正确性
fn verify_parquet_file(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = Bytes::copy_from_slice(data);
    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;

    // 验证Schema
    let schema = reader.schema();
    assert_eq!(schema.fields().len(), 9, "Schema should have 9 fields");

    let field_names: Vec<String> = schema
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect();
    assert_eq!(
        field_names,
        vec![
            "id",
            "timestamp",
            "level",
            "target",
            "message",
            "fields",
            "file",
            "line",
            "thread_id"
        ],
        "Schema field names should match"
    );

    // 验证数据
    let mut total_rows = 0;
    for batch in reader {
        let batch = batch?;
        assert!(batch.num_rows() > 0, "Batch should have rows");
        total_rows += batch.num_rows();
    }

    assert!(total_rows > 0, "Parquet file should contain data");

    Ok(())
}

#[test]
fn test_parquet_basic_conversion() {
    let logs = create_test_logs(100);
    let result = convert_logs_to_parquet(&logs);

    assert!(result.is_ok(), "Parquet conversion should succeed");
    let parquet_data = result.unwrap();

    assert!(!parquet_data.is_empty(), "Parquet data should not be empty");

    // 验证文件可读性
    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_small_dataset() {
    let logs = create_test_logs(1_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs);
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Parquet conversion should succeed for 1K records"
    );
    let parquet_data = result.unwrap();

    println!("1K records conversion time: {:?}", duration);
    println!("1K records Parquet size: {} bytes", parquet_data.len());

    // 验证压缩率（假设原始JSON约为每条记录200字节）
    let estimated_original_size = logs.len() * 200;
    let compression_ratio = estimated_original_size as f64 / parquet_data.len() as f64;
    println!("Estimated compression ratio: {:.2}x", compression_ratio);

    assert!(
        compression_ratio > 1.5,
        "Compression ratio should be > 1.5x, got {:.2}x",
        compression_ratio
    );

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_medium_dataset() {
    let logs = create_test_logs(10_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs);
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Parquet conversion should succeed for 10K records"
    );
    let parquet_data = result.unwrap();

    println!("10K records conversion time: {:?}", duration);
    println!("10K records Parquet size: {} bytes", parquet_data.len());

    // 验证性能（10K记录应该在5秒内完成）
    assert!(
        duration.as_secs() < 5,
        "10K records conversion should complete in < 5 seconds, took {:?}",
        duration
    );

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_large_dataset() {
    let logs = create_test_logs(100_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs);
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Parquet conversion should succeed for 100K records"
    );
    let parquet_data = result.unwrap();

    println!("100K records conversion time: {:?}", duration);
    println!("100K records Parquet size: {} bytes", parquet_data.len());

    // 验证性能（100K记录应该在30秒内完成）
    assert!(
        duration.as_secs() < 30,
        "100K records conversion should complete in < 30 seconds, took {:?}",
        duration
    );

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_compression_ratio() {
    let logs = create_test_logs(10_000);
    let result = convert_logs_to_parquet(&logs).expect("Parquet conversion should succeed");

    // 计算原始JSON大小
    let json_data = serde_json::to_vec(&logs).expect("JSON serialization should succeed");
    let original_size = json_data.len();
    let compressed_size = result.len();

    let compression_ratio = original_size as f64 / compressed_size as f64;

    println!("Original JSON size: {} bytes", original_size);
    println!("Compressed Parquet size: {} bytes", compressed_size);
    println!("Actual compression ratio: {:.2}x", compression_ratio);

    // 验证压缩率 > 50%
    assert!(
        compression_ratio > 2.0,
        "Compression ratio should be > 2.0x, got {:.2}x",
        compression_ratio
    );
}

#[test]
fn test_parquet_empty_dataset() {
    let logs: Vec<inklog::sink::database::Model> = vec![];
    let result = convert_logs_to_parquet(&logs);

    assert!(
        result.is_ok(),
        "Parquet conversion should succeed for empty dataset"
    );
    let parquet_data = result.unwrap();

    // 空数据集应该产生一个有效的Parquet文件（即使没有数据行）
    assert!(
        !parquet_data.is_empty(),
        "Parquet file should have metadata even for empty data"
    );
}

#[test]
fn test_parquet_schema_compatibility() {
    let logs = create_test_logs(100);
    let result = convert_logs_to_parquet(&logs).expect("Parquet conversion should succeed");

    let bytes = Bytes::from(result);
    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)
        .expect("Should create Parquet reader")
        .build()
        .expect("Should build reader");

    let schema = reader.schema();

    // 验证每个字段的类型
    let fields = schema.fields();
    assert_eq!(fields[0].name(), "id");
    assert_eq!(fields[0].data_type(), &arrow_schema::DataType::Int64);

    assert_eq!(fields[1].name(), "timestamp");
    assert_eq!(fields[1].data_type(), &arrow_schema::DataType::Utf8);

    assert_eq!(fields[2].name(), "level");
    assert_eq!(fields[2].data_type(), &arrow_schema::DataType::Utf8);

    assert_eq!(fields[3].name(), "target");
    assert_eq!(fields[3].data_type(), &arrow_schema::DataType::Utf8);

    assert_eq!(fields[4].name(), "message");
    assert_eq!(fields[4].data_type(), &arrow_schema::DataType::Utf8);

    assert_eq!(fields[5].name(), "fields");
    assert_eq!(fields[5].data_type(), &arrow_schema::DataType::Utf8);

    assert_eq!(fields[6].name(), "file");
    assert_eq!(fields[6].data_type(), &arrow_schema::DataType::Utf8);

    assert_eq!(fields[7].name(), "line");
    assert_eq!(fields[7].data_type(), &arrow_schema::DataType::Int64);

    assert_eq!(fields[8].name(), "thread_id");
    assert_eq!(fields[8].data_type(), &arrow_schema::DataType::Utf8);
}
