// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Compression strategies for log files.
//!
//! This module provides a strategy pattern implementation for log file compression,
//! supporting multiple compression algorithms (Zstd, Gzip, etc.).

use crate::InklogError;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tracing::error;

/// Trait for compression strategies.
///
/// Implement this trait to define custom compression algorithms.
pub trait CompressionStrategy: Send + Sync {
    /// Compress the given data.
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError>;

    /// Decompress the given data.
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError>;

    /// Get the file extension for this compression format.
    fn extension(&self) -> &'static str;

    /// Get the name of this compression algorithm.
    fn name(&self) -> &'static str;

    /// Compress a file at the given path.
    fn compress_file(&self, path: &Path, level: i32) -> Result<PathBuf, InklogError>;
}

/// Zstd compression strategy.
#[derive(Debug, Clone)]
pub struct ZstdCompression {
    level: i32,
}

impl ZstdCompression {
    /// Create a new Zstd compression strategy with the given level (0-22).
    pub fn new(level: i32) -> Self {
        let level = level.clamp(0, 22);
        Self { level }
    }

    /// Get the compression level.
    pub fn level(&self) -> i32 {
        self.level
    }
}

impl Default for ZstdCompression {
    fn default() -> Self {
        Self::new(3)
    }
}

impl CompressionStrategy for ZstdCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        zstd::encode_all(data, self.level).map_err(|e| InklogError::CompressionError(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        zstd::decode_all(data).map_err(|e| InklogError::CompressionError(e.to_string()))
    }

    fn extension(&self) -> &'static str {
        "zst"
    }

    fn name(&self) -> &'static str {
        "zstd"
    }

    fn compress_file(&self, path: &Path, level: i32) -> Result<PathBuf, InklogError> {
        compress_file_internal(path, level)
    }
}

/// No-op compression strategy (stores data uncompressed).
#[derive(Debug, Clone, Default)]
pub struct NoCompression;

impl CompressionStrategy for NoCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        Ok(data.to_vec())
    }

    fn extension(&self) -> &'static str {
        ""
    }

    fn name(&self) -> &'static str {
        "none"
    }

    fn compress_file(&self, path: &Path, _level: i32) -> Result<PathBuf, InklogError> {
        Ok(path.to_path_buf())
    }
}

/// Gzip compression strategy.
#[derive(Debug, Clone)]
pub struct GzipCompression {
    level: u32,
}

impl GzipCompression {
    /// Create a new Gzip compression strategy with the given level (0-9).
    pub fn new(level: u32) -> Self {
        let level = level.clamp(0, 9);
        Self { level }
    }

    /// Get the compression level.
    pub fn level(&self) -> u32 {
        self.level
    }
}

impl Default for GzipCompression {
    fn default() -> Self {
        Self::new(6)
    }
}

impl CompressionStrategy for GzipCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        std::io::Write::write_all(&mut encoder, data)
            .map_err(|e| InklogError::CompressionError(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| InklogError::CompressionError(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| InklogError::CompressionError(e.to_string()))?;
        Ok(decompressed)
    }

    fn extension(&self) -> &'static str {
        "gz"
    }

    fn name(&self) -> &'static str {
        "gzip"
    }

    fn compress_file(&self, path: &Path, level: i32) -> Result<PathBuf, InklogError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let compressed_path = path.with_extension("gz");

        let input_file = File::open(path).map_err(|e| {
            error!("Failed to open file for compression: {}", e);
            InklogError::IoError(e)
        })?;

        let mut reader = BufReader::new(input_file);
        let output_file = File::create(&compressed_path).map_err(|e| {
            error!("Failed to create compressed file: {}", e);
            InklogError::IoError(e)
        })?;

        let level = level.clamp(0, 9) as u32;
        let mut encoder = GzEncoder::new(output_file, Compression::new(level));

        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = Read::read(&mut reader, &mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            std::io::Write::write_all(&mut encoder, &buffer[..bytes_read])?;
        }

        encoder.finish().map_err(|e| {
            error!("Failed to finish compression: {}", e);
            InklogError::CompressionError(e.to_string())
        })?;

        let _ = std::fs::remove_file(path);

        Ok(compressed_path)
    }
}

/// Internal function to compress a file using Zstd.
fn compress_file_internal(path: &Path, compression_level: i32) -> Result<PathBuf, InklogError> {
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

/// Compress a single file (legacy function for backward compatibility).
pub fn compress_file(path: &Path, compression_level: i32) -> Result<PathBuf, InklogError> {
    compress_file_internal(path, compression_level)
}

/// Batch compress data.
pub fn compress_data(data: &[u8], compression_level: i32) -> Result<Vec<u8>, InklogError> {
    zstd::encode_all(data, compression_level)
        .map_err(|e| InklogError::CompressionError(e.to_string()))
}

/// Compress string data.
pub fn compress_string(data: &str, compression_level: i32) -> Result<Vec<u8>, InklogError> {
    compress_data(data.as_bytes(), compression_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zstd_compression() {
        let strategy = ZstdCompression::new(3);
        let data = b"Hello, World! This is a test message for compression.";

        let compressed = strategy.compress(data).unwrap();
        assert!(!compressed.is_empty());

        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_zstd_level_clamping() {
        let strategy = ZstdCompression::new(100);
        assert_eq!(strategy.level(), 22);

        let strategy = ZstdCompression::new(-10);
        assert_eq!(strategy.level(), 0);
    }

    #[test]
    fn test_no_compression() {
        let strategy = NoCompression;
        let data = b"Hello, World!";

        let compressed = strategy.compress(data).unwrap();
        assert_eq!(data.to_vec(), compressed);

        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_extension() {
        let zstd = ZstdCompression::default();
        assert_eq!(zstd.extension(), "zst");

        let none = NoCompression;
        assert_eq!(none.extension(), "");
    }

    #[test]
    fn test_compress_data() {
        let data = b"Test data for compression";
        let compressed = compress_data(data, 3).unwrap();
        assert!(!compressed.is_empty());

        // Verify decompression works
        let decompressed = zstd::decode_all(&compressed[..]).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_gzip_compression() {
        let strategy = GzipCompression::new(6);
        let data = b"Hello, World! This is a test message for gzip compression.";

        let compressed = strategy.compress(data).unwrap();
        assert!(!compressed.is_empty());

        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_gzip_level_clamping() {
        let strategy = GzipCompression::new(100);
        assert_eq!(strategy.level(), 9);

        let strategy = GzipCompression::new(0);
        assert_eq!(strategy.level(), 0);
    }

    #[test]
    fn test_gzip_extension() {
        let gzip = GzipCompression::default();
        assert_eq!(gzip.extension(), "gz");
        assert_eq!(gzip.name(), "gzip");
    }

    #[test]
    fn test_zstd_default_level() {
        let zstd = ZstdCompression::default();
        assert_eq!(zstd.level(), 3);
        assert_eq!(zstd.name(), "zstd");
    }

    #[test]
    fn test_zstd_compress_empty_data() {
        let strategy = ZstdCompression::new(3);
        let compressed = strategy.compress(b"").unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_zstd_decompress_invalid_data_errors() {
        let strategy = ZstdCompression::new(3);
        let invalid_data = b"not valid zstd data";
        let result = strategy.decompress(invalid_data);
        assert!(matches!(result, Err(InklogError::CompressionError(_))));
    }

    #[test]
    fn test_no_compression_default_and_compress_file() {
        let strategy = NoCompression;
        assert_eq!(strategy.name(), "none");
        let path = Path::new("/tmp/nonexistent_file_for_test");
        let result = strategy.compress_file(path, 3);
        assert_eq!(result.unwrap(), path.to_path_buf());
    }

    #[test]
    fn test_gzip_compress_empty_data() {
        let strategy = GzipCompression::new(6);
        let compressed = strategy.compress(b"").unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_gzip_decompress_invalid_data_errors() {
        let strategy = GzipCompression::new(6);
        let invalid_data = b"not valid gzip data";
        let result = strategy.decompress(invalid_data);
        assert!(matches!(result, Err(InklogError::CompressionError(_))));
    }

    #[test]
    fn test_gzip_default_level() {
        let gzip = GzipCompression::default();
        assert_eq!(gzip.level(), 6);
    }

    #[test]
    fn test_compress_string_function() {
        let data = "Hello, compression!";
        let compressed = compress_string(data, 3).unwrap();
        assert!(!compressed.is_empty());
        let decompressed = zstd::decode_all(&compressed[..]).unwrap();
        assert_eq!(data.as_bytes(), decompressed);
    }

    #[test]
    fn test_compress_data_empty() {
        let compressed = compress_data(b"", 3).unwrap();
        let decompressed = zstd::decode_all(&compressed[..]).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_compress_file_legacy_function() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("legacy_input.log");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test log line 1").unwrap();
        writeln!(file, "test log line 2").unwrap();
        drop(file);

        let compressed_path = compress_file(&file_path, 3).unwrap();
        assert!(compressed_path.extension().unwrap_or_default() == "zst");
        assert!(!file_path.exists(), "Original file should be removed");
        assert!(compressed_path.exists(), "Compressed file should exist");

        let compressed_bytes = std::fs::read(&compressed_path).unwrap();
        let decompressed = zstd::decode_all(&compressed_bytes[..]).unwrap();
        let text = String::from_utf8(decompressed).unwrap();
        assert!(text.contains("test log line 1"));
        assert!(text.contains("test log line 2"));
    }

    #[test]
    fn test_zstd_compress_file_via_strategy() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("strategy_input.log");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"strategy compression test data").unwrap();
        drop(file);

        let strategy = ZstdCompression::new(3);
        let compressed_path = strategy.compress_file(&file_path, 5).unwrap();
        assert_eq!(compressed_path.extension().unwrap_or_default(), "zst");
        assert!(!file_path.exists(), "Original should be removed");
        assert!(compressed_path.exists());

        let compressed_bytes = std::fs::read(&compressed_path).unwrap();
        let decompressed = zstd::decode_all(&compressed_bytes[..]).unwrap();
        assert_eq!(decompressed, b"strategy compression test data");
    }

    #[test]
    fn test_zstd_compress_file_open_missing_errors() {
        let strategy = ZstdCompression::new(3);
        let result = strategy.compress_file(Path::new("/nonexistent/path/file.log"), 3);
        assert!(matches!(result, Err(InklogError::IoError(_))));
    }

    #[test]
    fn test_gzip_compress_file_via_strategy() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("gzip_input.log");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"gzip strategy compression test").unwrap();
        drop(file);

        let strategy = GzipCompression::new(6);
        let compressed_path = strategy.compress_file(&file_path, 9).unwrap();
        assert_eq!(compressed_path.extension().unwrap_or_default(), "gz");
        assert!(!file_path.exists(), "Original should be removed");
        assert!(compressed_path.exists());

        let compressed_bytes = std::fs::read(&compressed_path).unwrap();
        let mut decoder = flate2::read::GzDecoder::new(&compressed_bytes[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, b"gzip strategy compression test");
    }

    #[test]
    fn test_gzip_compress_file_missing_input_errors() {
        let strategy = GzipCompression::new(6);
        let result = strategy.compress_file(Path::new("/nonexistent/gzip_input.log"), 6);
        assert!(matches!(result, Err(InklogError::IoError(_))));
    }

    #[test]
    fn test_zstd_large_data_roundtrip() {
        let strategy = ZstdCompression::new(9);
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let compressed = strategy.compress(&data).unwrap();
        assert!(
            compressed.len() < data.len(),
            "Should compress repetitive data"
        );
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_gzip_with_level_zero() {
        let strategy = GzipCompression::new(0);
        let data = b"data at level 0";
        let compressed = strategy.compress(data).unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_zstd_with_max_level() {
        let strategy = ZstdCompression::new(22);
        let data = b"max level compression test";
        let compressed = strategy.compress(data).unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }
}
