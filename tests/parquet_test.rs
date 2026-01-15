//! Parquet功能验证测试
//!
//! 测试Parquet导出功能的正确性、性能和兼容性

use arrow_array::RecordBatchReader;
use arrow_schema::DataType;
use bytes::Bytes;
use inklog::sink::database::convert_logs_to_parquet;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::time::Instant;

// ============ Test Data Helper Functions ============

/// Creates test log data with specified count
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

// ============ Parquet Verification Helper Functions ============

/// Expected schema field names
const EXPECTED_FIELD_NAMES: &[&str] = &[
    "id",
    "timestamp",
    "level",
    "target",
    "message",
    "fields",
    "file",
    "line",
    "thread_id",
];

/// Expected schema field types
const EXPECTED_FIELD_TYPES: &[DataType] = &[
    DataType::Int64, // id
    DataType::Utf8,  // timestamp
    DataType::Utf8,  // level
    DataType::Utf8,  // target
    DataType::Utf8,  // message
    DataType::Utf8,  // fields
    DataType::Utf8,  // file
    DataType::Int64, // line
    DataType::Utf8,  // thread_id
];

/// Verifies Parquet file schema (names and types)
fn verify_parquet_schema(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = Bytes::copy_from_slice(data);
    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;

    let schema = reader.schema();
    let fields = schema.fields();

    // Verify field count
    assert_eq!(fields.len(), 9, "Schema should have 9 fields");

    // Verify field names and types
    for (i, (name, dtype)) in EXPECTED_FIELD_NAMES
        .iter()
        .zip(EXPECTED_FIELD_TYPES.iter())
        .enumerate()
    {
        assert_eq!(fields[i].name(), *name);
        assert_eq!(fields[i].data_type(), dtype);
    }

    Ok(())
}

/// Verifies Parquet file data content
fn verify_parquet_data(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = Bytes::copy_from_slice(data);
    let reader = ParquetRecordBatchReaderBuilder::try_new(bytes)?.build()?;

    let mut total_rows = 0;
    for batch in reader {
        let batch = batch?;
        assert!(batch.num_rows() > 0, "Batch should have rows");
        total_rows += batch.num_rows();
    }

    assert!(total_rows > 0, "Parquet file should contain data");

    Ok(())
}

/// Complete Parquet file verification (schema + data)
fn verify_parquet_file(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    verify_parquet_schema(data)?;
    verify_parquet_data(data)?;
    Ok(())
}

// ============ Parquet Tests ============

#[test]
fn test_parquet_basic_conversion() {
    let logs = create_test_logs(100);
    let result = convert_logs_to_parquet(&logs, &Default::default());

    assert!(result.is_ok(), "Parquet conversion should succeed");
    let parquet_data = result.expect("Parquet conversion should succeed");

    assert!(!parquet_data.is_empty(), "Parquet data should not be empty");

    verify_parquet_file(&parquet_data).expect("Parquet file should be valid");
}

#[test]
fn test_parquet_small_dataset() {
    let logs = create_test_logs(1_000);
    let start = Instant::now();
    let result = convert_logs_to_parquet(&logs, &Default::default());
    let duration = start.elapsed();

    let parquet_data = result.expect("Parquet conversion should succeed for 1K records");

    println!("1K records conversion time: {:?}", duration);
    println!("1K records Parquet size: {} bytes", parquet_data.len());

    // Verify compression ratio (assuming ~200 bytes per record in JSON)
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
    let result = convert_logs_to_parquet(&logs, &Default::default());
    let duration = start.elapsed();

    let parquet_data = result.expect("Parquet conversion should succeed for 10K records");

    println!("10K records conversion time: {:?}", duration);
    println!("10K records Parquet size: {} bytes", parquet_data.len());

    // Verify performance (10K records should complete in < 5 seconds)
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
    let result = convert_logs_to_parquet(&logs, &Default::default());
    let duration = start.elapsed();

    let parquet_data = result.expect("Parquet conversion should succeed for 100K records");

    println!("100K records conversion time: {:?}", duration);
    println!("100K records Parquet size: {} bytes", parquet_data.len());

    // Verify performance (100K records should complete in < 30 seconds)
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
    let result = convert_logs_to_parquet(&logs, &Default::default())
        .expect("Parquet conversion should succeed");

    // Calculate original JSON size
    let json_data = serde_json::to_vec(&logs).expect("JSON serialization should succeed");
    let original_size = json_data.len();
    let compressed_size = result.len();

    let compression_ratio = original_size as f64 / compressed_size as f64;

    println!("Original JSON size: {} bytes", original_size);
    println!("Compressed Parquet size: {} bytes", compressed_size);
    println!("Actual compression ratio: {:.2}x", compression_ratio);

    // Verify compression ratio > 50%
    assert!(
        compression_ratio > 2.0,
        "Compression ratio should be > 2.0x, got {:.2}x",
        compression_ratio
    );
}

#[test]
fn test_parquet_empty_dataset() {
    let logs: Vec<inklog::sink::database::Model> = vec![];
    let result = convert_logs_to_parquet(&logs, &Default::default());

    let parquet_data = result.expect("Parquet conversion should succeed for empty dataset");

    // Empty dataset should produce a valid Parquet file (even without data rows)
    assert!(
        !parquet_data.is_empty(),
        "Parquet file should have metadata even for empty data"
    );
}

#[test]
fn test_parquet_schema_compatibility() {
    let logs = create_test_logs(100);
    let result = convert_logs_to_parquet(&logs, &Default::default())
        .expect("Parquet conversion should succeed");

    // Use the consolidated schema verification
    verify_parquet_schema(&result).expect("Schema verification should pass");
}
