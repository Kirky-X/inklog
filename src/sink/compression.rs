// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 压缩相关工具模块
//!
//! 提供文件压缩功能，支持 ZSTD 压缩算法

use crate::error::InklogError;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use tracing::error;

/// 压缩单个文件
///
/// # 参数
///
/// * `path` - 要压缩的文件路径
/// * `compression_level` - 压缩级别 (0-22)
///
/// # 返回值
///
/// 返回压缩后的文件路径
pub fn compress_file(path: &PathBuf, compression_level: i32) -> Result<PathBuf, InklogError> {
    let compressed_path = path.with_extension("zst");

    let input_file = File::open(path).map_err(|e| {
        error!("Failed to open file for compression: {}", e);
        InklogError::IoError(e)
    })?;

    let mut reader = BufReader::new(input_file);
    let output_file = File::create(&compressed_path).map_err(|e| {
        error!("Failed to create compressed file: {}", e);
        InklogError::IoError(e)
    })?;

    let mut encoder = zstd::stream::Encoder::new(output_file, compression_level).map_err(|e| {
        error!("Failed to create zstd encoder: {}", e);
        InklogError::CompressionError(e.to_string())
    })?;

    {
        let mut writer = BufWriter::new(encoder.by_ref());

        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = Read::read(&mut reader, &mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            Write::write_all(&mut writer, &buffer[..bytes_read])?;
        }
    }

    encoder.finish().map_err(|e| {
        error!("Failed to finish compression: {}", e);
        InklogError::CompressionError(e.to_string())
    })?;

    let _ = std::fs::remove_file(path);

    Ok(compressed_path)
}

/// 批量压缩数据
///
/// # 参数
///
/// * `data` - 要压缩的数据
/// * `compression_level` - 压缩级别 (0-22)
///
/// # 返回值
///
/// 返回压缩后的数据
pub fn compress_data(data: &[u8], compression_level: i32) -> Result<Vec<u8>, InklogError> {
    zstd::encode_all(data, compression_level)
        .map_err(|e| InklogError::CompressionError(e.to_string()))
}

/// 压缩字符串数据
///
/// # 参数
///
/// * `data` - 要压缩的字符串数据
/// * `compression_level` - 压缩级别 (0-22)
///
/// # 返回值
///
/// 返回压缩后的数据
pub fn compress_string(data: &str, compression_level: i32) -> Result<Vec<u8>, InklogError> {
    compress_data(data.as_bytes(), compression_level)
}
