// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Parquet 测试数据校验的共享实现。
//!
//! 收编自 `integration` 目标（tests/integration/parquet/parquet_test.rs）与
//! `integration_tests` 目标（tests/integration_tests.rs）中两份逐字的
//! `verify_parquet_data`。函数体逐字迁移，行为不变。

use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

/// Verifies Parquet file data content
pub(crate) fn verify_parquet_data(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
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
