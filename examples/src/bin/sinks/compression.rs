// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Zstd 压缩/解压缩示例（Layer 1 本地资源）
//!
//! 演示 inklog 的日志压缩能力，使用临时目录自动清理：
//! - `ZstdCompression` 压缩/解压内存数据
//! - `compress_file` 压缩整个文件（原文件会被删除）
//! - 不同压缩级别对压缩比的影响
//! - `GzipCompression` 与 `NoCompression` 策略对比
//!
//! # 运行
//! ```bash
//! cargo run --bin compression
//! ```

use inklog::sink::compression::{
    CompressionStrategy, GzipCompression, NoCompression, ZstdCompression, compress_data,
    compress_file,
};
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-compression-inklog-zstd-compression-examples")
    );

    show_in_memory_compression();
    show_file_compression().await?;
    show_compression_levels_comparison();
    show_strategy_variants();

    println!("\n{}", tr("example-compression-all-compression-examples"));
    Ok(())
}

/// 展示内存数据压缩/解压
fn show_in_memory_compression() {
    print_separator(&tr("example-compression-1-in-memory-zstd-compress"));

    print_section(&tr("example-compression-1-1-zstdcompression-default"));
    let strategy = ZstdCompression::default();
    println!(
        "{}",
        tr_args(
            "example-compression-algorithm-name",
            &[("value", &(strategy.name()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-compression-extension",
            &[("value", &(strategy.extension()))]
        )
    );
    assert_eq!(strategy.name(), "zstd");
    assert_eq!(strategy.extension(), "zst");

    print_section(&tr("example-compression-1-2-compress-a-string"));
    let original =
        "inklog enterprise log infrastructure — a log message used to test compression.".repeat(8);
    let compressed = strategy
        .compress(original.as_bytes())
        .expect("compression failed");
    println!(
        "{}",
        tr_args(
            "example-compression-original-size-bytes",
            &[("value", &(original.len()))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-compression-compressed-size-bytes",
            &[("value", &(compressed.len()))]
        )
    );
    assert!(!compressed.is_empty());

    print_section(&tr("example-compression-1-3-decompress-back-to-the"));
    let decompressed = strategy
        .decompress(&compressed)
        .expect("decompression failed");
    println!(
        "{}",
        tr_args(
            "example-compression-decompressed-size-bytes",
            &[("value", &(decompressed.len()))]
        )
    );
    assert_eq!(decompressed, original.as_bytes());
    println!(
        "{}",
        tr("example-compression-decompressed-data-matches-the")
    );

    print_section(&tr("example-compression-1-4-the-compress-data"));
    let compressed2 = compress_data(b"compress_data convenience function", 3)
        .expect("the convenience function failed to compress");
    println!(
        "Convenience function result size: {} bytes",
        compressed2.len()
    );
    assert!(!compressed2.is_empty());
}

/// 展示文件级压缩
async fn show_file_compression() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("2. File-level Zstd compression");

    let temp_dir = TempDir::new()?;
    let log_path = temp_dir.path().join("app.log");

    print_section("2.1 prepare the raw log file");
    let mut file = fs::File::create(&log_path)?;
    for i in 0..50 {
        writeln!(
            file,
            "2026-06-30 INFO  inklog::sink - log message #{:03} demonstrates file compression",
            i
        )?;
    }
    drop(file);
    let original_size = fs::metadata(&log_path)?.len();
    println!("Raw file: {}", log_path.display());
    println!("Original size: {} bytes", original_size);

    print_section("2.2 compress_file() the file");
    let compressed_path = compress_file(&log_path, 3)?;
    println!("Compressed file: {}", compressed_path.display());
    println!(
        "Compressed size: {} bytes",
        fs::metadata(&compressed_path)?.len()
    );
    assert_eq!(compressed_path.extension().unwrap(), "zst");
    assert!(
        !log_path.exists(),
        "compress_file should remove the original file"
    );
    assert!(compressed_path.exists(), "the compressed file should exist");

    print_section("2.3 verify the compressed file can be restored");
    let compressed_bytes = fs::read(&compressed_path)?;
    // 使用 ZstdCompression::decompress 还原，避免示例直接依赖 zstd crate
    let strategy = ZstdCompression::default();
    let decompressed = strategy.decompress(&compressed_bytes)?;
    let text = String::from_utf8(decompressed)?;
    assert!(text.contains("log message #000"));
    assert!(text.contains("log message #049"));
    println!("✓ decompressed content contains all 50 log records");

    Ok(())
}

/// 展示不同压缩级别的压缩比对比
fn show_compression_levels_comparison() {
    print_separator("3. How the compression level affects the ratio");

    // 重复内容占比越高，压缩比越好
    let data: Vec<u8> = (0..2000).map(|i| (i % 64) as u8).collect();
    let original_size = data.len();
    println!("Raw data: {} bytes (highly repetitive)\n", original_size);

    println!("{:<10}{:<15}{:<15}", "level", "size(B)", "ratio");
    println!("{}", "-".repeat(40));

    for level in [1, 3, 9, 19, 22] {
        let strategy = ZstdCompression::new(level);
        let compressed = strategy.compress(&data).expect("compression failed");
        let ratio = compressed.len() as f64 / original_size as f64;
        println!("{:<10}{:<15}{:<15.4}", level, compressed.len(), ratio);
    }

    println!(
        "\nNote: a higher level gives a better ratio but costs more CPU; 3-9 is recommended for production."
    );
}

/// 展示不同压缩策略对比
fn show_strategy_variants() {
    print_separator("4. Compression strategy comparison (Zstd / Gzip / NoCompression)");

    let data = b"inklog compression strategy comparison test data for zstd and gzip.";

    print_section("4.1 ZstdCompression (level=3)");
    let zstd = ZstdCompression::new(3);
    let zstd_compressed = zstd.compress(data).unwrap();
    println!(
        "extension: {}, compressed size: {} bytes",
        zstd.extension(),
        zstd_compressed.len()
    );
    assert_eq!(zstd.decompress(&zstd_compressed).unwrap(), data);

    print_section("4.2 GzipCompression (level=6)");
    let gzip = GzipCompression::new(6);
    let gzip_compressed = gzip.compress(data).unwrap();
    println!(
        "extension: {}, compressed size: {} bytes",
        gzip.extension(),
        gzip_compressed.len()
    );
    assert_eq!(gzip.decompress(&gzip_compressed).unwrap(), data);

    print_section("4.3 NoCompression (pass-through)");
    let none = NoCompression;
    let none_compressed = none.compress(data).unwrap();
    println!(
        "extension: {:?}, output size: {} bytes",
        none.extension(),
        none_compressed.len()
    );
    assert_eq!(none_compressed.as_slice(), data);
    assert_eq!(none.decompress(&none_compressed).unwrap(), data);
}
