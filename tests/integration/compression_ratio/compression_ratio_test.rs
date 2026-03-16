// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 压缩比验证测试
//!
//! 验证zstd压缩算法的压缩效率

#[cfg(test)]
mod tests {
    use std::io::Write;

    fn calculate_compression_ratio(original_size: u64, compressed_size: u64) -> f64 {
        if original_size == 0 {
            return 0.0;
        }
        ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
    }

    fn generate_log_data(count: usize, pattern: &str) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..count {
            let line = format!(
                "[2026-03-16T10:00:00.000Z] INFO [thread-{}] {}: test log message with some data\n",
                i % 10,
                pattern
            );
            data.extend_from_slice(line.as_bytes());
        }
        data
    }

    fn generate_json_log_data(count: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..count {
            let line = format!(
                r#"{{"timestamp":"2026-03-16T10:00:00.000Z","level":"INFO","thread":"thread-{}","message":"test log message with structured data","user_id":{},"request_id":"req-{}","metadata":{{"key":"value","count":{}}}}}"#,
                i % 10,
                i,
                i,
                i * 2
            );
            data.extend_from_slice(line.as_bytes());
            data.push(b'\n');
        }
        data
    }

    fn generate_repetitive_data(count: usize) -> Vec<u8> {
        let pattern = b"This is a repetitive log message that should compress very well. ";
        let mut data = Vec::new();
        for _ in 0..count {
            data.extend_from_slice(pattern);
        }
        data
    }

    fn compress_with_zstd(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let mut encoder = zstd::Encoder::new(Vec::new(), 3)?;
        encoder.write_all(data)?;
        encoder.finish()
    }

    #[test]
    fn test_zstd_compression_ratio_text_logs() {
        let min_ratio: f64 = std::env::var("MIN_COMPRESSION_RATIO")
            .map(|r| r.parse().unwrap_or(70.0))
            .unwrap_or(70.0);

        let log_data = generate_log_data(10000, "standard text log pattern");
        let original_size = log_data.len() as u64;

        let compressed_data = compress_with_zstd(&log_data).expect("压缩失败");
        let compressed_size = compressed_data.len() as u64;

        let ratio = calculate_compression_ratio(original_size, compressed_size);

        println!("文本日志压缩测试:");
        println!("原始大小: {} bytes", original_size);
        println!("压缩后大小: {} bytes", compressed_size);
        println!("压缩比: {:.2}%", ratio);

        assert!(
            ratio >= min_ratio,
            "压缩比低于阈值: {:.2}% < {:.2}%",
            ratio,
            min_ratio
        );
    }

    #[test]
    fn test_zstd_compression_ratio_json_logs() {
        let min_ratio: f64 = 70.0;

        let log_data = generate_json_log_data(5000);
        let original_size = log_data.len() as u64;

        let compressed_data = compress_with_zstd(&log_data).expect("压缩失败");
        let compressed_size = compressed_data.len() as u64;

        let ratio = calculate_compression_ratio(original_size, compressed_size);

        println!("JSON日志压缩测试:");
        println!("原始大小: {} bytes", original_size);
        println!("压缩后大小: {} bytes", compressed_size);
        println!("压缩比: {:.2}%", ratio);

        assert!(
            ratio >= min_ratio,
            "JSON日志压缩比低于阈值: {:.2}% < {:.2}%",
            ratio,
            min_ratio
        );
    }

    #[test]
    fn test_zstd_compression_ratio_repetitive_data() {
        let min_ratio: f64 = 90.0;

        let log_data = generate_repetitive_data(10000);
        let original_size = log_data.len() as u64;

        let compressed_data = compress_with_zstd(&log_data).expect("压缩失败");
        let compressed_size = compressed_data.len() as u64;

        let ratio = calculate_compression_ratio(original_size, compressed_size);

        println!("重复数据压缩测试:");
        println!("原始大小: {} bytes", original_size);
        println!("压缩后大小: {} bytes", compressed_size);
        println!("压缩比: {:.2}%", ratio);

        assert!(
            ratio >= min_ratio,
            "重复数据压缩比低于阈值: {:.2}% < {:.2}%",
            ratio,
            min_ratio
        );
    }

    #[test]
    fn test_zstd_compression_ratio_mixed_content() {
        let min_ratio: f64 = 60.0;

        let mut log_data = Vec::new();
        log_data.extend_from_slice(&generate_log_data(3000, "text"));
        log_data.extend_from_slice(&generate_json_log_data(3000));
        log_data.extend_from_slice(&generate_repetitive_data(3000));

        let original_size = log_data.len() as u64;

        let compressed_data = compress_with_zstd(&log_data).expect("压缩失败");
        let compressed_size = compressed_data.len() as u64;

        let ratio = calculate_compression_ratio(original_size, compressed_size);

        println!("混合内容压缩测试:");
        println!("原始大小: {} bytes", original_size);
        println!("压缩后大小: {} bytes", compressed_size);
        println!("压缩比: {:.2}%", ratio);

        assert!(
            ratio >= min_ratio,
            "混合内容压缩比低于阈值: {:.2}% < {:.2}%",
            ratio,
            min_ratio
        );
    }

    #[test]
    fn test_zstd_compression_levels() {
        let log_data = generate_log_data(5000, "compression level test");

        println!("压缩级别对比测试:");

        for level in [1, 3, 9, 19] {
            let mut encoder = zstd::Encoder::new(Vec::new(), level).unwrap();
            encoder.write_all(&log_data).unwrap();
            let compressed = encoder.finish().unwrap();

            let ratio = calculate_compression_ratio(log_data.len() as u64, compressed.len() as u64);
            println!(
                "级别 {}: {} bytes -> {} bytes, 压缩比: {:.2}%",
                level,
                log_data.len(),
                compressed.len(),
                ratio
            );
        }
    }

    #[test]
    fn test_compression_ratio_calculation() {
        assert_eq!(calculate_compression_ratio(1000, 200), 80.0);
        assert_eq!(calculate_compression_ratio(1000, 500), 50.0);
        assert_eq!(calculate_compression_ratio(1000, 1000), 0.0);
        assert_eq!(calculate_compression_ratio(0, 0), 0.0);
    }
}
