// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 自适应 Channel 策略示例
//!
//! 演示 `inklog::ChannelStrategy` 枚举和 `PerformanceConfig` 的自适应阈值参数：
//!
//! 1. `ChannelStrategy::Fixed` vs `ChannelStrategy::Adaptive` 对比
//! 2. FromStr / Display trait 实现
//! 3. 自适应阈值参数（expand/shrink threshold、min/max capacity）
//! 4. 不同负载场景下的配置推荐
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin channel_strategy
//! ```

use inklog::config::{ChannelStrategy, PerformanceConfig};
use inklog_examples::common::{print_section, print_separator};
use std::str::FromStr;

fn main() {
    print_separator("inklog 自适应 Channel 策略示例");

    show_strategy_variants();
    show_from_str_display();
    show_adaptive_thresholds();
    show_performance_profiles();
    show_config_compatibility();

    println!("\n所有 ChannelStrategy 示例展示完毕。");
}

/// 演示 ChannelStrategy 两个变体
fn show_strategy_variants() {
    print_section("示例 1：ChannelStrategy 变体");

    let fixed = ChannelStrategy::Fixed;
    let adaptive = ChannelStrategy::Adaptive;

    println!("枚举变体：");
    println!("  {:?} - 静态容量，可预测内存使用", fixed);
    println!("  {:?} - 动态调整，更好的突发流量处理", adaptive);

    println!("\n默认策略：");
    println!(
        "  ChannelStrategy::default() = {:?}",
        ChannelStrategy::default()
    );

    println!("\n行为对比：");
    println!("{:<15} {:<25} {:<25}", "场景", "Fixed", "Adaptive");
    println!("{}", "-".repeat(65));
    println!(
        "{:<15} {:<25} {:<25}",
        "正常负载", "稳定", "稳定（动态变化）"
    );
    println!(
        "{:<15} {:<25} {:<25}",
        "流量突增", "可能丢弃日志", "扩容以处理"
    );
    println!(
        "{:<15} {:<25} {:<25}",
        "低流量", "容量恒定", "缩容以节省内存"
    );
    println!(
        "{:<15} {:<25} {:<25}",
        "内存使用", "可预测", "可变（通常更低）"
    );
}

/// 演示 FromStr 和 Display trait
fn show_from_str_display() {
    print_section("示例 2：FromStr / Display trait");

    println!("FromStr 解析：");
    for input in ["fixed", "adaptive", "FIXED", "Adaptive"] {
        match ChannelStrategy::from_str(input) {
            Ok(strategy) => println!("  {:>10}.parse() → Ok({:?})", input, strategy),
            Err(e) => println!("  {:>10}.parse() → Err(\"{}\")", input, e),
        }
    }

    println!("\n无效输入：");
    for input in ["dynamic", "flexible", "", "invalid"] {
        let result: Result<ChannelStrategy, String> = input.parse();
        match result {
            Ok(s) => println!("  {:>10}.parse() → Ok({:?})", input, s),
            Err(e) => println!("  {:>10}.parse() → Err(\"{}\")", input, e),
        }
    }

    println!("\nDisplay 输出：");
    for strategy in [ChannelStrategy::Fixed, ChannelStrategy::Adaptive] {
        println!("  format!(\"{{}}\", {:?}) → \"{}\"", strategy, strategy);
    }
}

/// 演示自适应阈值参数（PerformanceConfig）
fn show_adaptive_thresholds() {
    print_section("示例 3：自适应阈值参数");

    let default = PerformanceConfig::default();
    println!("默认 PerformanceConfig：");
    println!("  channel_capacity        = {}", default.channel_capacity);
    println!("  worker_threads          = {}", default.worker_threads);
    println!("  channel_strategy        = {:?}", default.channel_strategy);
    println!(
        "  expand_threshold_percent= {}",
        default.expand_threshold_percent
    );
    println!(
        "  shrink_threshold_percent= {}",
        default.shrink_threshold_percent
    );
    println!(
        "  shrink_wait_seconds     = {}",
        default.shrink_wait_seconds
    );
    println!("  min_capacity            = {}", default.min_capacity);
    println!("  max_capacity            = {}", default.max_capacity);

    println!("\n阈值参数语义（仅 Adaptive 策略生效）：");
    println!("  expand_threshold_percent: 容量使用率达到此百分比时触发扩容");
    println!("    默认 80% → 8000/10000 时扩容");
    println!("  shrink_threshold_percent: 容量使用率降至此百分比时考虑缩容");
    println!("    默认 20% → 2000/10000 时缩容");
    println!("  shrink_wait_seconds: 低负载持续此秒数后才缩容（防抖动）");
    println!("    默认 30 秒");
    println!("  min_capacity / max_capacity: 缩容/扩容的上下限");

    println!("\n自定义自适应配置（激进扩容，保守缩容）：");
    let aggressive = PerformanceConfig {
        channel_capacity: 20000,
        worker_threads: 4,
        channel_strategy: ChannelStrategy::Adaptive,
        expand_threshold_percent: 60,
        shrink_threshold_percent: 30,
        shrink_wait_seconds: 60,
        min_capacity: 5000,
        max_capacity: 100000,
    };
    println!(
        "  channel_capacity        = {}",
        aggressive.channel_capacity
    );
    println!(
        "  channel_strategy        = {:?}",
        aggressive.channel_strategy
    );
    println!(
        "  expand_threshold_percent= {} (更早扩容)",
        aggressive.expand_threshold_percent
    );
    println!(
        "  shrink_threshold_percent= {} (更晚缩容)",
        aggressive.shrink_threshold_percent
    );
    println!(
        "  shrink_wait_seconds     = {} (更长防抖)",
        aggressive.shrink_wait_seconds
    );
    println!("  min_capacity            = {}", aggressive.min_capacity);
    println!("  max_capacity            = {}", aggressive.max_capacity);
}

/// 演示不同负载场景的配置推荐
fn show_performance_profiles() {
    print_section("示例 4：性能配置推荐");

    let profiles = [
        (
            "高吞吐量 (>10k logs/s)",
            PerformanceConfig {
                channel_capacity: 50000,
                worker_threads: 8,
                channel_strategy: ChannelStrategy::Fixed,
                expand_threshold_percent: 80,
                shrink_threshold_percent: 20,
                shrink_wait_seconds: 30,
                min_capacity: 1000,
                max_capacity: 50000,
            },
        ),
        (
            "低延迟 (<1ms p99)",
            PerformanceConfig {
                channel_capacity: 5000,
                worker_threads: 4,
                channel_strategy: ChannelStrategy::Adaptive,
                expand_threshold_percent: 60,
                shrink_threshold_percent: 20,
                shrink_wait_seconds: 30,
                min_capacity: 1000,
                max_capacity: 20000,
            },
        ),
        (
            "资源受限 (有限 RAM/CPU)",
            PerformanceConfig {
                channel_capacity: 2000,
                worker_threads: 2,
                channel_strategy: ChannelStrategy::Fixed,
                expand_threshold_percent: 80,
                shrink_threshold_percent: 20,
                shrink_wait_seconds: 30,
                min_capacity: 500,
                max_capacity: 5000,
            },
        ),
        (
            "突发流量 (Adaptive 推荐)",
            PerformanceConfig {
                channel_capacity: 10000,
                worker_threads: 4,
                channel_strategy: ChannelStrategy::Adaptive,
                expand_threshold_percent: 70,
                shrink_threshold_percent: 25,
                shrink_wait_seconds: 45,
                min_capacity: 2000,
                max_capacity: 80000,
            },
        ),
    ];

    for (name, config) in profiles {
        println!("{}:", name);
        println!(
            "  channel_capacity={}, workers={}, strategy={:?}",
            config.channel_capacity, config.worker_threads, config.channel_strategy
        );
        if config.channel_strategy == ChannelStrategy::Adaptive {
            println!(
                "  expand@{}%, shrink@{}%, wait={}s, range=[{},{}]",
                config.expand_threshold_percent,
                config.shrink_threshold_percent,
                config.shrink_wait_seconds,
                config.min_capacity,
                config.max_capacity
            );
        }
        println!();
    }
}

/// 演示配置兼容性和序列化
fn show_config_compatibility() {
    print_section("示例 5：配置兼容性（serde 序列化）");

    let config = PerformanceConfig {
        channel_strategy: ChannelStrategy::Adaptive,
        expand_threshold_percent: 75,
        shrink_threshold_percent: 25,
        ..Default::default()
    };

    let toml_str = format!(
        r#"[performance]
channel_capacity = {}
worker_threads = {}
channel_strategy = "{}"
expand_threshold_percent = {}
shrink_threshold_percent = {}
shrink_wait_seconds = {}
min_capacity = {}
max_capacity = {}"#,
        config.channel_capacity,
        config.worker_threads,
        config.channel_strategy,
        config.expand_threshold_percent,
        config.shrink_threshold_percent,
        config.shrink_wait_seconds,
        config.min_capacity,
        config.max_capacity,
    );
    println!("序列化为 TOML：");
    println!("{}", toml_str);

    println!("\n环境变量覆盖：");
    println!("  export INKLOG_PERFORMANCE_CHANNEL_STRATEGY=adaptive");
    println!("  export INKLOG_PERFORMANCE_CHANNEL_CAPACITY=20000");
    println!("  export INKLOG_PERFORMANCE_WORKER_THREADS=4");

    let _ = config; // 实际使用配置
}
