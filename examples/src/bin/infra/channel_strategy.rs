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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::str::FromStr;

fn main() {
    print_separator(&tr("example-channel-strategy-inklog-adaptive-channel"));

    show_strategy_variants();
    show_from_str_display();
    show_adaptive_thresholds();
    show_performance_profiles();
    show_config_compatibility();

    println!(
        "\n{}",
        tr("example-channel-strategy-all-channelstrategy-examples")
    );
}

/// 演示 ChannelStrategy 两个变体
fn show_strategy_variants() {
    print_section(&tr("example-channel-strategy-example-1-channelstrategy"));

    let fixed = ChannelStrategy::Fixed;
    let adaptive = ChannelStrategy::Adaptive;

    println!("{}", tr("example-channel-strategy-enum-variants"));
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-static-capacity-predictable",
            &[("fixed", &format!("{:?}", fixed))]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-dynamic-sizing-better-burst",
            &[("adaptive", &format!("{:?}", adaptive))]
        )
    );

    println!("\n{}", tr("example-channel-strategy-default-strategy"));
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-channelstrategy-default",
            &[("value", &format!("{:?}", ChannelStrategy::default()))]
        )
    );

    println!("\n{}", tr("example-channel-strategy-behavior-comparison"));
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
    print_section(&tr(
        "example-channel-strategy-example-2-fromstr-display-traits",
    ));

    println!("{}", tr("example-channel-strategy-fromstr-parsing"));
    for input in ["fixed", "adaptive", "FIXED", "Adaptive"] {
        match ChannelStrategy::from_str(input) {
            Ok(strategy) => println!(
                "  {}",
                tr_args(
                    "example-channel-strategy-parse-ok",
                    &[
                        ("input", &format!("{:>10}", input)),
                        ("strategy", &format!("{:?}", strategy))
                    ]
                )
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-channel-strategy-parse-err",
                    &[("input", &format!("{:>10}", input)), ("e", &e)]
                )
            ),
        }
    }

    println!("\n{}", tr("example-channel-strategy-invalid-input"));
    for input in ["dynamic", "flexible", "", "invalid"] {
        let result: Result<ChannelStrategy, String> = input.parse();
        match result {
            Ok(s) => println!(
                "  {}",
                tr_args(
                    "example-channel-strategy-parse-ok-2",
                    &[
                        ("input", &format!("{:>10}", input)),
                        ("s", &format!("{:?}", s))
                    ]
                )
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-channel-strategy-parse-err",
                    &[("input", &format!("{:>10}", input)), ("e", &e)]
                )
            ),
        }
    }

    println!("\n{}", tr("example-channel-strategy-display-output"));
    for strategy in [ChannelStrategy::Fixed, ChannelStrategy::Adaptive] {
        println!("  format!(\"{{}}\", {:?}) → \"{}\"", strategy, strategy);
    }
}

/// 演示自适应阈值参数（PerformanceConfig）
fn show_adaptive_thresholds() {
    print_section(&tr("example-channel-strategy-example-3-adaptive-threshold"));

    let default = PerformanceConfig::default();
    println!(
        "{}",
        tr("example-channel-strategy-default-performanceconfig")
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-channel-capacity",
            &[("channel_capacity", &default.channel_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-worker-threads",
            &[("worker_threads", &default.worker_threads)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-channel-strategy",
            &[(
                "channel_strategy",
                &format!("{:?}", default.channel_strategy)
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-expand-threshold-percent",
            &[(
                "expand_threshold_percent",
                &default.expand_threshold_percent
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-shrink-threshold-percent",
            &[(
                "shrink_threshold_percent",
                &default.shrink_threshold_percent
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-shrink-wait-seconds",
            &[("shrink_wait_seconds", &default.shrink_wait_seconds)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-min-capacity",
            &[("min_capacity", &default.min_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-max-capacity",
            &[("max_capacity", &default.max_capacity)]
        )
    );

    println!(
        "\n{}",
        tr("example-channel-strategy-threshold-parameter-semantics")
    );
    println!(
        "  {}",
        tr("example-channel-strategy-expand-threshold-percent-expand")
    );
    println!(
        "    {}",
        tr("example-channel-strategy-default-80-expand-at-8000-10000")
    );
    println!(
        "  {}",
        tr("example-channel-strategy-shrink-threshold-percent-2")
    );
    println!(
        "    {}",
        tr("example-channel-strategy-default-20-shrink-at-2000-10000")
    );
    println!(
        "  shrink_wait_seconds: shrink only after low load persists this many seconds (debounce)"
    );
    println!("    {}", tr("example-channel-strategy-default-30-seconds"));
    println!(
        "  {}",
        tr("example-channel-strategy-min-capacity-max-capacity-lower")
    );

    println!(
        "\n{}",
        tr("example-channel-strategy-custom-adaptive-config")
    );
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
        "  {}",
        tr_args(
            "example-channel-strategy-channel-capacity",
            &[("channel_capacity", &aggressive.channel_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-channel-strategy",
            &[(
                "channel_strategy",
                &format!("{:?}", aggressive.channel_strategy)
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-expand-threshold-percent-2",
            &[(
                "expand_threshold_percent",
                &aggressive.expand_threshold_percent
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-shrink-threshold-percent-3",
            &[(
                "shrink_threshold_percent",
                &aggressive.shrink_threshold_percent
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-shrink-wait-seconds-longer",
            &[("shrink_wait_seconds", &aggressive.shrink_wait_seconds)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-min-capacity",
            &[("min_capacity", &aggressive.min_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-channel-strategy-max-capacity",
            &[("max_capacity", &aggressive.max_capacity)]
        )
    );
}

/// 演示不同负载Scenario的配置推荐
fn show_performance_profiles() {
    print_section(&tr("example-channel-strategy-example-4-performance-config"));

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
            "  {}",
            tr_args(
                "example-channel-strategy-channel-capacity-workers",
                &[
                    ("channel_capacity", &config.channel_capacity),
                    ("worker_threads", &config.worker_threads),
                    (
                        "channel_strategy",
                        &format!("{:?}", config.channel_strategy)
                    )
                ]
            )
        );
        if config.channel_strategy == ChannelStrategy::Adaptive {
            println!(
                "  {}",
                tr_args(
                    "example-channel-strategy-expand-shrink-wait-s-range",
                    &[
                        ("expand_threshold_percent", &config.expand_threshold_percent),
                        ("shrink_threshold_percent", &config.shrink_threshold_percent),
                        ("shrink_wait_seconds", &config.shrink_wait_seconds),
                        ("min_capacity", &config.min_capacity),
                        ("max_capacity", &config.max_capacity)
                    ]
                )
            );
        }
        println!();
    }
}

/// 演示配置兼容性和序列化
fn show_config_compatibility() {
    print_section(&tr("example-channel-strategy-example-5-formatted-config"));

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
    println!("{}", tr("example-channel-strategy-serialized-to-toml"));
    println!("{}", toml_str);

    println!("\n{}", tr("example-channel-strategy-env-var-overrides"));
    println!("  export INKLOG_PERFORMANCE_CHANNEL_STRATEGY=adaptive");
    println!("  export INKLOG_PERFORMANCE_CHANNEL_CAPACITY=20000");
    println!("  export INKLOG_PERFORMANCE_WORKER_THREADS=4");

    let _ = config; // 实际使用配置
}
