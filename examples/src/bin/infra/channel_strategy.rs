// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 自适应 Channel 策略示例
//!
//! 演示 `inklog::ChannelStrategy` 枚举和 `PerformanceConfig` 的自适应阈值参数：
//!
//! 1. `ChannelStrategy::Fixed` vs `ChannelStrategy::Adaptive` 对比
//! 2. FromStr / Display trait 实现
//! 3. 自适应阈值参数（expand/shrink threshold、min/max capacity）
//! 4. 不同负载Scenario下的配置推荐
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
    print_separator("inklog adaptive channel strategy example");

    show_strategy_variants();
    show_from_str_display();
    show_adaptive_thresholds();
    show_performance_profiles();
    show_config_compatibility();

    println!("\nAll ChannelStrategy examples completed.");
}

/// 演示 ChannelStrategy 两个变体
fn show_strategy_variants() {
    print_section("Example 1: ChannelStrategy variants");

    let fixed = ChannelStrategy::Fixed;
    let adaptive = ChannelStrategy::Adaptive;

    println!("Enum variants:");
    println!("  {:?} - static capacity, predictable memory use", fixed);
    println!("  {:?} - dynamic sizing, better burst handling", adaptive);

    println!("\nDefault strategy:");
    println!(
        "  ChannelStrategy::default() = {:?}",
        ChannelStrategy::default()
    );

    println!("\nBehavior comparison:");
    println!("{:<15} {:<25} {:<25}", "Scenario", "Fixed", "Adaptive");
    println!("{}", "-".repeat(65));
    println!(
        "{:<15} {:<25} {:<25}",
        "Normal load", "Stable", "Stable (changes dynamically)"
    );
    println!(
        "{:<15} {:<25} {:<25}",
        "Traffic spike", "May drop logs", "expand to handle"
    );
    println!(
        "{:<15} {:<25} {:<25}",
        "Low traffic", "Constant capacity", "shrink to save memory"
    );
    println!(
        "{:<15} {:<25} {:<25}",
        "Memory use", "Predictable", "Variable (usually lower)"
    );
}

/// 演示 FromStr 和 Display trait
fn show_from_str_display() {
    print_section("Example 2: FromStr / Display traits");

    println!("FromStr parsing:");
    for input in ["fixed", "adaptive", "FIXED", "Adaptive"] {
        match ChannelStrategy::from_str(input) {
            Ok(strategy) => println!("  {:>10}.parse() → Ok({:?})", input, strategy),
            Err(e) => println!("  {:>10}.parse() → Err(\"{}\")", input, e),
        }
    }

    println!("\nInvalid input:");
    for input in ["dynamic", "flexible", "", "invalid"] {
        let result: Result<ChannelStrategy, String> = input.parse();
        match result {
            Ok(s) => println!("  {:>10}.parse() → Ok({:?})", input, s),
            Err(e) => println!("  {:>10}.parse() → Err(\"{}\")", input, e),
        }
    }

    println!("\nDisplay output:");
    for strategy in [ChannelStrategy::Fixed, ChannelStrategy::Adaptive] {
        println!("  format!(\"{{}}\", {:?}) → \"{}\"", strategy, strategy);
    }
}

/// 演示自适应阈值参数（PerformanceConfig）
fn show_adaptive_thresholds() {
    print_section("Example 3: adaptive threshold parameters");

    let default = PerformanceConfig::default();
    println!("Default PerformanceConfig:");
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

    println!("\nThreshold parameter semantics (only effective for the Adaptive strategy):");
    println!("  expand_threshold_percent: expand when capacity usage reaches this percent");
    println!("    default 80% → expand at 8000/10000");
    println!("  shrink_threshold_percent: consider shrinking when usage drops to this percent");
    println!("    default 20% → shrink at 2000/10000");
    println!(
        "  shrink_wait_seconds: shrink only after low load persists this many seconds (debounce)"
    );
    println!("    default 30 seconds");
    println!("  min_capacity / max_capacity: lower/upper bounds for shrink/expand");

    println!("\nCustom adaptive config (aggressive expansion, conservative shrinking):");
    let aggressive = PerformanceConfig {
        channel_capacity: 20000,
        worker_threads: 4,
        channel_strategy: ChannelStrategy::Adaptive,
        expand_threshold_percent: 60,
        shrink_threshold_percent: 30,
        shrink_wait_seconds: 60,
        min_capacity: 5000,
        max_capacity: 100000,
        rate_limit: None,
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
        "  expand_threshold_percent= {} (expands earlier)",
        aggressive.expand_threshold_percent
    );
    println!(
        "  shrink_threshold_percent= {} (shrinks later)",
        aggressive.shrink_threshold_percent
    );
    println!(
        "  shrink_wait_seconds     = {} (longer debounce)",
        aggressive.shrink_wait_seconds
    );
    println!("  min_capacity            = {}", aggressive.min_capacity);
    println!("  max_capacity            = {}", aggressive.max_capacity);
}

/// 演示不同负载Scenario的配置推荐
fn show_performance_profiles() {
    print_section("Example 4: performance config recommendations");

    let profiles = [
        (
            "High throughput (>10k logs/s)",
            PerformanceConfig {
                channel_capacity: 50000,
                worker_threads: 8,
                channel_strategy: ChannelStrategy::Fixed,
                expand_threshold_percent: 80,
                shrink_threshold_percent: 20,
                shrink_wait_seconds: 30,
                min_capacity: 1000,
                max_capacity: 50000,
                rate_limit: None,
            },
        ),
        (
            "Low latency (<1ms p99)",
            PerformanceConfig {
                channel_capacity: 5000,
                worker_threads: 4,
                channel_strategy: ChannelStrategy::Adaptive,
                expand_threshold_percent: 60,
                shrink_threshold_percent: 20,
                shrink_wait_seconds: 30,
                min_capacity: 1000,
                max_capacity: 20000,
                rate_limit: None,
            },
        ),
        (
            "Resource-constrained (limited RAM/CPU)",
            PerformanceConfig {
                channel_capacity: 2000,
                worker_threads: 2,
                channel_strategy: ChannelStrategy::Fixed,
                expand_threshold_percent: 80,
                shrink_threshold_percent: 20,
                shrink_wait_seconds: 30,
                min_capacity: 500,
                max_capacity: 5000,
                rate_limit: None,
            },
        ),
        (
            "Bursty traffic (Adaptive recommended)",
            PerformanceConfig {
                channel_capacity: 10000,
                worker_threads: 4,
                channel_strategy: ChannelStrategy::Adaptive,
                expand_threshold_percent: 70,
                shrink_threshold_percent: 25,
                shrink_wait_seconds: 45,
                min_capacity: 2000,
                max_capacity: 80000,
                rate_limit: None,
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
    print_section("Example 5: formatted config display");

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
    println!("Serialized to TOML:");
    println!("{}", toml_str);

    println!("\nEnv-var overrides:");
    println!("  export INKLOG_PERFORMANCE_CHANNEL_STRATEGY=adaptive");
    println!("  export INKLOG_PERFORMANCE_CHANNEL_CAPACITY=20000");
    println!("  export INKLOG_PERFORMANCE_WORKER_THREADS=4");

    let _ = config; // 实际使用配置
}
