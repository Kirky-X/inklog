// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 日志轮转示例（Layer 1 本地资源）
//!
//! 演示 inklog 的轮转策略 API，使用临时目录自动清理：
//! - `SizeBasedRotation` 按大小轮转（max_size）
//! - `TimeBasedRotation` 按时间轮转（daily / hourly / weekly / monthly）
//! - `RotationStrategy::generate_next_path` 轮转后的文件命名规则
//! - `CompositeRotation` 组合策略（满足任一条件即轮转）
//! - `FileSinkConfig` 中 `max_size` / `rotation_time` 字段配置
//!
//! # 运行
//! ```bash
//! cargo run --bin rotation
//! ```

use inklog::chrono::Utc;
use inklog::config::FileSinkConfig;
use inklog::sink::rotation::{
    CompositeRotation, RotationContext, RotationStrategy, SizeBasedRotation, TimeBasedRotation,
};
use inklog_examples::common::{print_section, print_separator};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog log rotation examples ===\n");

    show_size_based_rotation();
    show_time_based_rotation();
    show_rotation_file_naming();
    show_composite_rotation();
    show_file_sink_config_rotation();

    println!("\n✓ all rotation examples completed");
    Ok(())
}

/// 构造一个 RotationContext（last_rotation 距今 elapsed_secs 秒）
fn make_context(current_size: u64, elapsed_secs: u64, sequence: u32) -> RotationContext {
    let now = Utc::now();
    let last_rotation = Instant::now() - Duration::from_secs(elapsed_secs);
    RotationContext {
        current_path: PathBuf::from("/var/log/app.log"),
        current_size,
        max_size: None,
        file_opened_at: last_rotation,
        last_rotation,
        now,
        sequence,
    }
}

/// 展示按大小轮转
fn show_size_based_rotation() {
    print_separator("1. Size-based rotation (SizeBasedRotation)");

    print_section("1.1 from_size_string(\"100MB\") construction");
    let strategy = SizeBasedRotation::from_size_string("100MB").expect("failed to parse");
    println!(
        "max_size = {} bytes ({} MB)",
        strategy.max_size(),
        strategy.max_size() / 1024 / 1024
    );
    assert_eq!(strategy.max_size(), 100 * 1024 * 1024);
    assert_eq!(strategy.name(), "size_based");

    print_section("1.2 file size within the limit → no rotation");
    let ctx = make_context(50 * 1024 * 1024, 0, 0);
    let result = strategy.should_rotate(&ctx);
    println!(
        "current_size = {} MB, limit = 100 MB",
        ctx.current_size / 1024 / 1024
    );
    println!("should_rotate = {}", result.should_rotate);
    assert!(!result.should_rotate);
    assert!(result.reason.is_none());

    print_section("1.3 file size over the limit → rotation triggers");
    let ctx = make_context(150 * 1024 * 1024, 0, 0);
    let result = strategy.should_rotate(&ctx);
    println!(
        "current_size = {} MB, limit = 100 MB",
        ctx.current_size / 1024 / 1024
    );
    println!("should_rotate = {}", result.should_rotate);
    println!("reason        = {:?}", result.reason);
    assert!(result.should_rotate);
    assert!(result.reason.as_ref().unwrap().contains("exceeds limit"));
}

/// 展示按时间轮转
fn show_time_based_rotation() {
    print_separator("2. Time-based rotation (TimeBasedRotation)");

    print_section("2.1 the four built-in intervals");
    for interval in ["hourly", "daily", "weekly", "monthly"] {
        let s = TimeBasedRotation::from_interval_string(interval).expect("failed to parse");
        println!(
            "{:<10} interval_secs = {} sec ({} hours)",
            interval,
            s.interval_secs(),
            s.interval_secs() / 3600
        );
    }
    assert_eq!(
        TimeBasedRotation::from_interval_string("hourly")
            .unwrap()
            .interval_secs(),
        3600
    );
    assert_eq!(
        TimeBasedRotation::from_interval_string("daily")
            .unwrap()
            .interval_secs(),
        86400
    );

    print_section("2.2 an unknown interval returns an error");
    let err = TimeBasedRotation::from_interval_string("yearly").unwrap_err();
    println!("Error message: {}", err);
    assert!(err.contains("Unknown rotation interval"));

    print_section("2.3 interval not reached → no rotation (daily, 1 hour elapsed)");
    let strategy = TimeBasedRotation::from_interval_string("daily").unwrap();
    let ctx = make_context(100, 3600, 0); // 距上次轮转 1 小时
    let result = strategy.should_rotate(&ctx);
    println!(
        "elapsed {} sec, interval {} sec",
        3600,
        strategy.interval_secs()
    );
    println!("should_rotate = {}", result.should_rotate);
    assert!(!result.should_rotate);

    print_section("2.4 interval reached → rotation triggers (daily, 24 hours elapsed)");
    let ctx = make_context(100, 86400, 0); // 距上次轮转 24 小时
    let result = strategy.should_rotate(&ctx);
    println!(
        "elapsed {} sec, interval {} sec",
        86400,
        strategy.interval_secs()
    );
    println!("should_rotate = {}", result.should_rotate);
    println!("reason        = {:?}", result.reason);
    assert!(result.should_rotate);
    assert!(result.reason.as_ref().unwrap().contains("daily"));
}

/// 展示轮转后的文件命名规则
fn show_rotation_file_naming() {
    print_separator("3. Rotated file naming rules");

    print_section("3.1 generate_next_path default format");
    let strategy = SizeBasedRotation::new(1024);
    let ctx = make_context(2048, 0, 1);
    let base = PathBuf::from("/var/log/app.log");
    let next = strategy.generate_next_path(&base, &ctx);
    println!("base path : {}", base.display());
    println!("next path : {}", next.display());
    // 命名格式: {stem}_{timestamp}_{seq}.{ext}
    assert!(next.to_string_lossy().contains("app_"));
    assert!(next.to_string_lossy().ends_with(".log"));

    print_section("3.2 different sequence values produce different file names");
    let mut paths = Vec::new();
    for seq in 0..3u32 {
        let ctx = make_context(2048, 0, seq);
        let p = strategy.generate_next_path(&base, &ctx);
        paths.push(p.to_string_lossy().to_string());
    }
    for (i, p) in paths.iter().enumerate() {
        println!("seq={} → {}", i, p);
    }
    // 每个路径应互不相同（sequence 不同）
    assert_ne!(paths[0], paths[1]);
    assert_ne!(paths[1], paths[2]);

    print_section("3.3 a path without an extension falls back to .log");
    let base_no_ext = PathBuf::from("/var/log/app");
    let next = strategy.generate_next_path(&base_no_ext, &ctx);
    println!("base path : {}", base_no_ext.display());
    println!("next path : {}", next.display());
    assert!(next.to_string_lossy().ends_with(".log"));
}

/// 展示组合轮转策略
fn show_composite_rotation() {
    print_separator("4. Composite rotation strategy (CompositeRotation)");

    print_section("4.1 composite size + time, neither condition met");
    let mut composite = CompositeRotation::new(vec![]);
    composite.add(SizeBasedRotation::new(1024));
    composite.add(TimeBasedRotation::from_interval_string("daily").unwrap());
    println!("strategy name = {}", composite.name());
    assert_eq!(composite.name(), "composite");

    let ctx = make_context(100, 3600, 0); // 100 字节，1 小时
    let result = composite.should_rotate(&ctx);
    println!(
        "size=100B(<1KB), elapsed=1h(<24h) → should_rotate={}",
        result.should_rotate
    );
    assert!(!result.should_rotate);

    print_section("4.2 the size condition is met → rotation triggers");
    let ctx = make_context(2048, 3600, 0); // 2048 字节 > 1024
    let result = composite.should_rotate(&ctx);
    println!(
        "size=2048B(>1KB), elapsed=1h(<24h) → should_rotate={}",
        result.should_rotate
    );
    assert!(result.should_rotate);

    print_section("4.3 the time condition is met → rotation triggers");
    let ctx = make_context(100, 86400, 0); // 100 字节，24 小时
    let result = composite.should_rotate(&ctx);
    println!(
        "size=100B(<1KB), elapsed=24h(>=24h) → should_rotate={}",
        result.should_rotate
    );
    assert!(result.should_rotate);
}

/// 展示 FileSinkConfig 中的轮转配置字段
fn show_file_sink_config_rotation() {
    print_separator("5. FileSinkConfig rotation fields");

    print_section("5.1 default configuration");
    let default_cfg = FileSinkConfig::default();
    println!("max_size      = {:?}", default_cfg.max_size);
    println!("rotation_time = {:?}", default_cfg.rotation_time);
    println!("keep_files    = {}", default_cfg.keep_files);
    println!("compress      = {}", default_cfg.compress);
    assert_eq!(default_cfg.max_size, "100MB");
    assert_eq!(default_cfg.rotation_time, "daily");

    print_section("5.2 custom rotation configuration (small file + hourly)");
    let cfg = FileSinkConfig {
        enabled: true,
        path: PathBuf::from("/tmp/inklog_rotation_demo/app.log"),
        max_size: "10KB".to_string(),
        rotation_time: "hourly".to_string(),
        keep_files: 24,
        compress: true,
        compression_level: 9,
        ..Default::default()
    };
    println!("path          = {}", cfg.path.display());
    println!("max_size      = {}", cfg.max_size);
    println!("rotation_time = {}", cfg.rotation_time);
    println!("keep_files    = {}", cfg.keep_files);
    println!("compress      = {}", cfg.compress);
    println!("compression_level = {}", cfg.compression_level);
    assert_eq!(cfg.max_size, "10KB");
    assert_eq!(cfg.rotation_time, "hourly");

    println!("\nNote: FileSink uses CompositeRotation internally to check size and time;");
    println!(
        "      when either condition is met it rotates, and the file is named {{stem}}_{{timestamp}}_{{seq}}.{{ext}}."
    );
}
