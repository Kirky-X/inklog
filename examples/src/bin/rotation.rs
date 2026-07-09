// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use chrono::Utc;
use inklog::config::FileSinkConfig;
use inklog::sink::rotation::{
    CompositeRotation, RotationContext, RotationStrategy, SizeBasedRotation, TimeBasedRotation,
};
use inklog_examples::common::{print_section, print_separator};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog 日志轮转示例 ===\n");

    show_size_based_rotation();
    show_time_based_rotation();
    show_rotation_file_naming();
    show_composite_rotation();
    show_file_sink_config_rotation();

    println!("\n✓ 所有轮转示例演示完成");
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
    print_separator("1. 按大小轮转（SizeBasedRotation）");

    print_section("1.1 from_size_string(\"100MB\") 构造");
    let strategy = SizeBasedRotation::from_size_string("100MB").expect("解析失败");
    println!(
        "max_size = {} 字节 ({} MB)",
        strategy.max_size(),
        strategy.max_size() / 1024 / 1024
    );
    assert_eq!(strategy.max_size(), 100 * 1024 * 1024);
    assert_eq!(strategy.name(), "size_based");

    print_section("1.2 文件大小未超限 → 不轮转");
    let ctx = make_context(50 * 1024 * 1024, 0, 0);
    let result = strategy.should_rotate(&ctx);
    println!(
        "current_size = {} MB, 限制 = 100 MB",
        ctx.current_size / 1024 / 1024
    );
    println!("should_rotate = {}", result.should_rotate);
    assert!(!result.should_rotate);
    assert!(result.reason.is_none());

    print_section("1.3 文件大小超限 → 触发轮转");
    let ctx = make_context(150 * 1024 * 1024, 0, 0);
    let result = strategy.should_rotate(&ctx);
    println!(
        "current_size = {} MB, 限制 = 100 MB",
        ctx.current_size / 1024 / 1024
    );
    println!("should_rotate = {}", result.should_rotate);
    println!("reason        = {:?}", result.reason);
    assert!(result.should_rotate);
    assert!(result.reason.as_ref().unwrap().contains("exceeds limit"));
}

/// 展示按时间轮转
fn show_time_based_rotation() {
    print_separator("2. 按时间轮转（TimeBasedRotation）");

    print_section("2.1 四种内置间隔");
    for interval in ["hourly", "daily", "weekly", "monthly"] {
        let s = TimeBasedRotation::from_interval_string(interval).expect("解析失败");
        println!(
            "{:<10} interval_secs = {} 秒 ({} 小时)",
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

    print_section("2.2 未知间隔返回错误");
    let err = TimeBasedRotation::from_interval_string("yearly").unwrap_err();
    println!("错误信息: {}", err);
    assert!(err.contains("Unknown rotation interval"));

    print_section("2.3 未到间隔 → 不轮转（daily，已过 1 小时）");
    let strategy = TimeBasedRotation::from_interval_string("daily").unwrap();
    let ctx = make_context(100, 3600, 0); // 距上次轮转 1 小时
    let result = strategy.should_rotate(&ctx);
    println!("已过 {} 秒, 间隔 {} 秒", 3600, strategy.interval_secs());
    println!("should_rotate = {}", result.should_rotate);
    assert!(!result.should_rotate);

    print_section("2.4 到达间隔 → 触发轮转（daily，已过 24 小时）");
    let ctx = make_context(100, 86400, 0); // 距上次轮转 24 小时
    let result = strategy.should_rotate(&ctx);
    println!("已过 {} 秒, 间隔 {} 秒", 86400, strategy.interval_secs());
    println!("should_rotate = {}", result.should_rotate);
    println!("reason        = {:?}", result.reason);
    assert!(result.should_rotate);
    assert!(result.reason.as_ref().unwrap().contains("daily"));
}

/// 展示轮转后的文件命名规则
fn show_rotation_file_naming() {
    print_separator("3. 轮转后的文件命名规则");

    print_section("3.1 generate_next_path 默认格式");
    let strategy = SizeBasedRotation::new(1024);
    let ctx = make_context(2048, 0, 1);
    let base = PathBuf::from("/var/log/app.log");
    let next = strategy.generate_next_path(&base, &ctx);
    println!("base path : {}", base.display());
    println!("next path : {}", next.display());
    // 命名格式: {stem}_{timestamp}_{seq}.{ext}
    assert!(next.to_string_lossy().contains("app_"));
    assert!(next.to_string_lossy().ends_with(".log"));

    print_section("3.2 不同 sequence 生成不同文件名");
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

    print_section("3.3 无扩展名路径回退为 .log");
    let base_no_ext = PathBuf::from("/var/log/app");
    let next = strategy.generate_next_path(&base_no_ext, &ctx);
    println!("base path : {}", base_no_ext.display());
    println!("next path : {}", next.display());
    assert!(next.to_string_lossy().ends_with(".log"));
}

/// 展示组合轮转策略
fn show_composite_rotation() {
    print_separator("4. 组合轮转策略（CompositeRotation）");

    print_section("4.1 组合 size + time，两个条件均不满足");
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

    print_section("4.2 size 条件满足 → 触发轮转");
    let ctx = make_context(2048, 3600, 0); // 2048 字节 > 1024
    let result = composite.should_rotate(&ctx);
    println!(
        "size=2048B(>1KB), elapsed=1h(<24h) → should_rotate={}",
        result.should_rotate
    );
    assert!(result.should_rotate);

    print_section("4.3 time 条件满足 → 触发轮转");
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
    print_separator("5. FileSinkConfig 轮转配置字段");

    print_section("5.1 默认配置");
    let default_cfg = FileSinkConfig::default();
    println!("max_size      = {:?}", default_cfg.max_size);
    println!("rotation_time = {:?}", default_cfg.rotation_time);
    println!("keep_files    = {}", default_cfg.keep_files);
    println!("compress      = {}", default_cfg.compress);
    assert_eq!(default_cfg.max_size, "100MB");
    assert_eq!(default_cfg.rotation_time, "daily");

    print_section("5.2 自定义轮转配置（小文件 + 每小时轮转）");
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

    println!("\n说明: FileSink 内部使用 CompositeRotation 同时检查 size 和 time，");
    println!(
        "      任一条件满足即触发轮转，轮转后文件按 {{stem}}_{{timestamp}}_{{seq}}.{{ext}} 命名。"
    );
}
