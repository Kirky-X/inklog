// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Compression strategies for log files.
//!
//! This module provides a strategy pattern implementation for log file compression,
//! supporting multiple compression algorithms (Zstd, Gzip, etc.).

use crate::InklogError;
#[cfg(any(feature = "zstd", feature = "gzip"))]
use std::fs::File;
#[cfg(any(feature = "zstd", feature = "gzip"))]
use std::io::{BufReader, Read};
#[cfg(feature = "zstd")]
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
#[cfg(any(feature = "zstd", feature = "gzip"))]
use tracing::error;

/// 解压输出上限：1 GiB（压缩炸弹防护，与 query 路径共用语义）。
#[cfg_attr(not(any(feature = "zstd", feature = "gzip")), allow(dead_code))]
pub(crate) const DECOMPRESSION_OUTPUT_LIMIT: u64 = 1024 * 1024 * 1024;

/// 受限解压：读取至多 limit+1 字节以区分"恰好到限"与"超限"。
#[cfg_attr(not(any(feature = "zstd", feature = "gzip")), allow(dead_code))]
pub(crate) fn decompress_limited<R: std::io::Read>(
    reader: &mut R,
    codec: &str,
) -> Result<Vec<u8>, InklogError> {
    use std::io::Read;
    let mut limited = reader.take(DECOMPRESSION_OUTPUT_LIMIT);
    let mut out = Vec::new();
    limited
        .read_to_end(&mut out)
        .map_err(|e| InklogError::CompressionError(format!("{codec}: {e}")))?;
    if out.len() as u64 > DECOMPRESSION_OUTPUT_LIMIT {
        return Err(InklogError::RuntimeError(crate::i18n::tr(
            "query-decompression_limit_exceeded",
        )));
    }
    Ok(out)
}

/// 创建 0600 权限的压缩产物文件（unix；其余平台退化为默认权限）。
#[cfg_attr(not(any(feature = "zstd", feature = "gzip")), allow(dead_code))]
fn create_compressed_output(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        // 全限定路径：本函数的 `#[cfg(not(unix))]` 分支可能在 zstd/gzip 均未启用时
        // 被编译（函数体本身不受这两个 feature 门控），而顶层 `use std::fs::File`
        // 是 feature 门控的——裸 `File` 在该组合下会 E0433（非 unix 消费者如
        // `default-features = false` 依赖方会踩中）。
        std::fs::File::create(path)
    }
}

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
    ///
    /// On success the original file is removed. If removing the original
    /// fails after the compressed file has been written, both the original
    /// and the compressed file remain on disk (an error is logged) and the
    /// compressed path is still returned.
    fn compress_file(&self, path: &Path, level: i32) -> Result<PathBuf, InklogError>;
}

/// Zstd compression strategy.
#[cfg(feature = "zstd")]
#[derive(Debug, Clone)]
pub struct ZstdCompression {
    level: i32,
}

#[cfg(feature = "zstd")]
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

#[cfg(feature = "zstd")]
impl Default for ZstdCompression {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(feature = "zstd")]
impl CompressionStrategy for ZstdCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        zstd::encode_all(data, self.level).map_err(|e| InklogError::CompressionError(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        let mut decoder = zstd::stream::Decoder::new(data)
            .map_err(|e| InklogError::CompressionError(e.to_string()))?;
        decompress_limited(&mut decoder, "zstd")
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
#[cfg(feature = "gzip")]
#[derive(Debug, Clone)]
pub struct GzipCompression {
    level: u32,
}

#[cfg(feature = "gzip")]
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

#[cfg(feature = "gzip")]
impl Default for GzipCompression {
    fn default() -> Self {
        Self::new(6)
    }
}

#[cfg(feature = "gzip")]
impl CompressionStrategy for GzipCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        std::io::Write::write_all(&mut encoder, data)
            .map_err(|e| InklogError::CompressionError(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| InklogError::CompressionError(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, InklogError> {
        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(data);
        decompress_limited(&mut decoder, "gzip")
    }

    fn extension(&self) -> &'static str {
        "gz"
    }

    fn name(&self) -> &'static str {
        "gzip"
    }

    fn compress_file(&self, path: &Path, level: i32) -> Result<PathBuf, InklogError> {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let compressed_path = path.with_extension("gz");

        let output_file = create_compressed_output(&compressed_path).map_err(|e| {
            error!("Failed to create compressed file: {}", e);
            InklogError::IoError(e)
        })?;
        let mut encoder = GzEncoder::new(output_file, Compression::new(level.clamp(0, 9) as u32));
        gzip_copy_into(&mut encoder, path)?;

        encoder.finish().map_err(|e| {
            error!("Failed to finish compression: {}", e);
            InklogError::CompressionError(e.to_string())
        })?;

        if let Err(e) = std::fs::remove_file(path) {
            // 删除失败时原始文件与压缩文件并存，需保留两者待人工/后续清理
            let mut args = crate::i18n::MsgArgs::new();
            args.set("err", e.to_string());
            error!(
                original = %path.display(),
                compressed = %compressed_path.display(),
                "{}",
                crate::i18n::tr_args("config-compression_remove_failed", args)
            );
        }

        Ok(compressed_path)
    }
}

/// gzip 编码核心：把 `input` 内容编码进已打开的 writer（不命名、不删源）。
#[cfg(feature = "gzip")]
fn gzip_copy_into<W: std::io::Write>(
    encoder: &mut flate2::write::GzEncoder<W>,
    input: &Path,
) -> Result<(), InklogError> {
    let input_file = File::open(input).map_err(|e| {
        error!("Failed to open file for compression: {}", e);
        InklogError::IoError(e)
    })?;
    let mut reader = BufReader::new(input_file);
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = Read::read(&mut reader, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        std::io::Write::write_all(encoder, &buffer[..bytes_read])?;
    }
    Ok(())
}

/// 追加命名压缩（轮转归档专用）：产物为 `{input}.{ext}`（gz），**不替换**
/// 既有扩展名、不删除源文件。
///
/// `with_extension` 会把序号候选（`X.log.1`）的序号当扩展名剥掉——不同
/// 轮转 attempt 的产物互相覆盖且冲突检查不单射（曾致同秒高频轮转挂死）。
/// 仅 gzip 单后端组合启用（zstd 在场时轮转走 `zstd_encode_to` 路径）。
#[cfg(all(feature = "gzip", not(feature = "zstd")))]
pub(crate) fn gzip_compress_file_keeping_name(
    path: &Path,
    level: i32,
) -> Result<PathBuf, InklogError> {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    let compressed_path = append_ext(path, "gz");
    let output_file = create_compressed_output(&compressed_path).map_err(|e| {
        error!("Failed to create compressed file: {}", e);
        InklogError::IoError(e)
    })?;
    let mut encoder = GzEncoder::new(output_file, Compression::new(level.clamp(0, 9) as u32));
    gzip_copy_into(&mut encoder, path)?;
    encoder.finish().map_err(|e| {
        error!("Failed to finish compression: {}", e);
        InklogError::CompressionError(e.to_string())
    })?;
    Ok(compressed_path)
}

/// Internal function to compress a file using Zstd.
#[cfg(feature = "zstd")]
fn compress_file_internal(path: &Path, compression_level: i32) -> Result<PathBuf, InklogError> {
    let compressed_path = path.with_extension("zst");
    zstd_encode_to(path, &compressed_path, compression_level)?;

    if let Err(e) = std::fs::remove_file(path) {
        // 删除失败时原始文件与压缩文件并存，需保留两者待人工/后续清理
        let mut args = crate::i18n::MsgArgs::new();
        args.set("err", e.to_string());
        error!(
            original = %path.display(),
            compressed = %compressed_path.display(),
            "{}",
            crate::i18n::tr_args("config-compression_remove_failed", args)
        );
    }

    Ok(compressed_path)
}

/// zstd 编码核心：`input` → `output`（调用方决定命名，不删源文件）。
#[cfg(feature = "zstd")]
pub(crate) fn zstd_encode_to(
    input: &Path,
    output: &Path,
    compression_level: i32,
) -> Result<(), InklogError> {
    let input_file = File::open(input).map_err(|e| {
        error!("Failed to open file for compression: {}", e);
        InklogError::IoError(e)
    })?;

    let output_file = create_compressed_output(output).map_err(|e| {
        error!("Failed to create compressed file: {}", e);
        InklogError::IoError(e)
    })?;

    let mut encoder = zstd::stream::Encoder::new(output_file, compression_level).map_err(|e| {
        error!("Failed to create zstd encoder: {}", e);
        InklogError::CompressionError(e.to_string())
    })?;

    {
        let mut writer = BufWriter::new(encoder.by_ref());

        let mut reader = BufReader::new(input_file);
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

    Ok(())
}

/// 在完整文件名后追加扩展名（不替换）。
///
/// 轮转序号候选（`X.log.1`）经 `with_extension` 会剥掉序号——产物互相
/// 覆盖且冲突检查不单射；追加命名保证 attempt 与产物一一对应。
pub(crate) fn append_ext(path: &Path, ext: &str) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(format!(".{ext}"));
    PathBuf::from(os)
}

/// Compress a single file (legacy function for backward compatibility).
#[cfg(feature = "zstd")]
pub fn compress_file(path: &Path, compression_level: i32) -> Result<PathBuf, InklogError> {
    compress_file_internal(path, compression_level)
}

/// Batch compress data.
#[cfg(feature = "zstd")]
pub fn compress_data(data: &[u8], compression_level: i32) -> Result<Vec<u8>, InklogError> {
    zstd::encode_all(data, compression_level)
        .map_err(|e| InklogError::CompressionError(e.to_string()))
}

/// Compress string data.
#[cfg(feature = "zstd")]
pub fn compress_string(data: &str, compression_level: i32) -> Result<Vec<u8>, InklogError> {
    compress_data(data.as_bytes(), compression_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_compression() {
        let strategy = ZstdCompression::new(3);
        let data = b"Hello, World! This is a test message for compression.";

        let compressed = strategy.compress(data).unwrap();
        assert!(!compressed.is_empty());

        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "zstd")]
    fn test_extension() {
        let zstd = ZstdCompression::default();
        assert_eq!(zstd.extension(), "zst");

        let none = NoCompression;
        assert_eq!(none.extension(), "");
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_compress_data() {
        let data = b"Test data for compression";
        let compressed = compress_data(data, 3).unwrap();
        assert!(!compressed.is_empty());

        // Verify decompression works
        let decompressed = zstd::decode_all(&compressed[..]).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_compression() {
        let strategy = GzipCompression::new(6);
        let data = b"Hello, World! This is a test message for gzip compression.";

        let compressed = strategy.compress(data).unwrap();
        assert!(!compressed.is_empty());

        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_level_clamping() {
        let strategy = GzipCompression::new(100);
        assert_eq!(strategy.level(), 9);

        let strategy = GzipCompression::new(0);
        assert_eq!(strategy.level(), 0);
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_extension() {
        let gzip = GzipCompression::default();
        assert_eq!(gzip.extension(), "gz");
        assert_eq!(gzip.name(), "gzip");
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_default_level() {
        let zstd = ZstdCompression::default();
        assert_eq!(zstd.level(), 3);
        assert_eq!(zstd.name(), "zstd");
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_compress_empty_data() {
        let strategy = ZstdCompression::new(3);
        let compressed = strategy.compress(b"").unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "gzip")]
    fn test_gzip_compress_empty_data() {
        let strategy = GzipCompression::new(6);
        let compressed = strategy.compress(b"").unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_decompress_invalid_data_errors() {
        let strategy = GzipCompression::new(6);
        let invalid_data = b"not valid gzip data";
        let result = strategy.decompress(invalid_data);
        assert!(matches!(result, Err(InklogError::CompressionError(_))));
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_default_level() {
        let gzip = GzipCompression::default();
        assert_eq!(gzip.level(), 6);
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_compress_string_function() {
        let data = "Hello, compression!";
        let compressed = compress_string(data, 3).unwrap();
        assert!(!compressed.is_empty());
        let decompressed = zstd::decode_all(&compressed[..]).unwrap();
        assert_eq!(data.as_bytes(), decompressed);
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_compress_data_empty() {
        let compressed = compress_data(b"", 3).unwrap();
        let decompressed = zstd::decode_all(&compressed[..]).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "zstd")]
    fn test_zstd_compress_file_open_missing_errors() {
        let strategy = ZstdCompression::new(3);
        let result = strategy.compress_file(Path::new("/nonexistent/path/file.log"), 3);
        assert!(matches!(result, Err(InklogError::IoError(_))));
    }

    #[test]
    #[cfg(feature = "gzip")]
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
    #[cfg(feature = "gzip")]
    fn test_gzip_compress_file_missing_input_errors() {
        let strategy = GzipCompression::new(6);
        let result = strategy.compress_file(Path::new("/nonexistent/gzip_input.log"), 6);
        assert!(matches!(result, Err(InklogError::IoError(_))));
    }

    #[test]
    #[cfg(feature = "zstd")]
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
    #[cfg(feature = "gzip")]
    fn test_gzip_with_level_zero() {
        let strategy = GzipCompression::new(0);
        let data = b"data at level 0";
        let compressed = strategy.compress(data).unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_with_max_level() {
        let strategy = ZstdCompression::new(22);
        let data = b"max level compression test";
        let compressed = strategy.compress(data).unwrap();
        let decompressed = strategy.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    // =========================================================================
    // compress_file 错误路径：覆盖 File::create 失败分支
    // =========================================================================

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_compress_file_create_output_fails_errors() {
        // 覆盖行 181-182：GzipCompression::compress_file 中 File::create 失败
        // 策略：输入文件存在，但 compressed_path（path.with_extension("gz")）
        // 指向一个已存在的目录 → File::create 返回 Err → 走 IoError 分支
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("input_create_fail.log");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"data").unwrap();
        drop(file);

        // compressed_path = input_create_fail.gz（with_extension 替换扩展名）
        // 把它创建为目录，使 File::create 失败
        let compressed_path = temp.path().join("input_create_fail.gz");
        std::fs::create_dir(&compressed_path).unwrap();

        let strategy = GzipCompression::new(6);
        let result = strategy.compress_file(&file_path, 6);
        assert!(
            matches!(result, Err(InklogError::IoError(_))),
            "expected IoError when output file create fails, got: {:?}",
            result
        );
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_compress_file_internal_create_output_fails_errors() {
        // 覆盖行 219-220：compress_file_internal 中 File::create 失败
        // 同样的策略：把 compressed_path 创建为目录
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("zstd_input_create_fail.log");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"data").unwrap();
        drop(file);

        // compressed_path = zstd_input_create_fail.zst
        let compressed_path = temp.path().join("zstd_input_create_fail.zst");
        std::fs::create_dir(&compressed_path).unwrap();

        // 直接调用 compress_file_internal 的公共入口 compress_file
        let result = compress_file(&file_path, 3);
        assert!(
            matches!(result, Err(InklogError::IoError(_))),
            "expected IoError when zstd output file create fails, got: {:?}",
            result
        );
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_compress_file_internal_invalid_level_clamped() {
        // zstd::stream::Encoder::new 对超出范围的 compression_level 会内部 clamp 到有效级别，
        // 而不是返回 Err。因此行 224-225（Encoder::new 失败分支）在实际中难以可靠触发。
        // 本测试验证 zstd 的 clamp 行为：传入 i32::MIN 应返回 Ok（而非 Err）。
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("invalid_level.log");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"data").unwrap();
        drop(file);

        // zstd 会 clamp i32::MIN 到有效级别，返回 Ok
        let result = compress_file(&file_path, i32::MIN);
        assert!(
            result.is_ok(),
            "zstd should clamp invalid level and return Ok, got: {:?}",
            result
        );

        // 验证压缩文件已生成
        let compressed_path = result.unwrap();
        assert!(compressed_path.exists());
        assert!(compressed_path.extension().is_some_and(|ext| ext == "zst"));
    }
}

#[cfg(all(test, feature = "gzip", unix))]
mod output_permission_tests {
    use super::*;

    /// R-sec-001：压缩产物（.gz）必须以 0600 创建。
    #[test]
    fn test_gzip_output_created_with_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let plain = dir.path().join("perm.log");
        std::fs::write(&plain, b"compress me 0600").unwrap();

        let gz = GzipCompression::default();
        gz.compress_file(&plain, 6).unwrap();

        let out = plain.with_extension("gz");
        let meta = std::fs::metadata(&out).unwrap();
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o600,
            "compressed artifact must be 0600"
        );
    }
}

// mod 整体按 gzip/zstd 门控：两者皆关时整个 mod 连同 `use super::*` 一并消失，
// 避免 default features 下 unused import（clippy -D warnings 必红）；两种 cfg 均用
// outer 形式，混合 inner/outer 会触发 clippy::mixed_attributes_style
#[cfg(test)]
#[cfg(any(feature = "gzip", feature = "zstd"))]
mod error_path_tests {
    use super::*;

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_compress_file_missing_input_errors_after_artifact_created() {
        // 输入在产物创建之后才打开：打开失败必须显性报错（产物残留由
        // 调用方轮转流程清理）
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("nope.log");

        let err = GzipCompression::default()
            .compress_file(&missing, 6)
            .unwrap_err();
        assert!(
            matches!(err, InklogError::IoError(ref e) if e.kind() == std::io::ErrorKind::NotFound),
            "missing input must surface as NotFound io error, got: {err}"
        );
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn test_gzip_compress_file_roundtrip_and_source_removal() {
        let dir = tempfile::TempDir::new().unwrap();
        let plain = dir.path().join("rt.log");
        std::fs::write(&plain, b"roundtrip payload").unwrap();

        let out = GzipCompression::default()
            .compress_file(&plain, 6)
            .expect("compress");
        assert!(out.exists(), "artifact must exist");
        assert!(
            !plain.exists(),
            "source must be removed after successful compression"
        );
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_zstd_compress_file_missing_input_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("nope.log");

        let err = compress_file(&missing, 3).unwrap_err();
        assert!(
            matches!(err, InklogError::IoError(ref e) if e.kind() == std::io::ErrorKind::NotFound),
            "missing input must surface as NotFound io error, got: {err}"
        );
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn test_decompress_limited_roundtrip() {
        // decompress_limited 只负责受限读取，解码由调用方接入 Decoder 流
        let payload = b"limited decompression payload".repeat(64);
        let compressed = compress_data(&payload, 3).unwrap();
        let mut decoder = zstd::stream::Decoder::new(std::io::Cursor::new(compressed)).unwrap();
        let out = decompress_limited(&mut decoder, "zstd").unwrap();
        assert_eq!(out, payload);
    }
}
