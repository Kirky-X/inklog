// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 环境变量覆盖加载示例
//!
//! 演示 `InklogConfig::load_with_env_overrides()` 的使用：
//!
//! 1. 设置 INKLOG_* 环境变量覆盖配置
//! 2. 加载配置并验证覆盖生效
//! 3. 演示支持的环境变量列表
//!
//! # 运行
//!
//! ```bash
//! # 直接运行（使用默认配置）
//! cargo run --bin env_overrides
//!
//! # 设置环境变量后运行
//! INKLOG_GLOBAL_LEVEL=debug \
//! INKLOG_PERFORMANCE_CHANNEL_CAPACITY=20000 \
//! INKLOG_PERFORMANCE_WORKER_THREADS=4 \
//! cargo run --bin env_overrides
//! ```

use inklog::InklogConfig;
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    print_separator(&tr("example-env-overrides-inklog-env-var-override-loading"));

    show_supported_env_vars();
    show_load_with_env_overrides();
    show_env_override_demo();
    show_priority_order();

    println!(
        "\n{}",
        tr("example-env-overrides-all-env-var-override-examples")
    );
}

/// 演示支持的环境变量列表
fn show_supported_env_vars() {
    print_section(&tr("example-env-overrides-example-1-supported-environment"));

    println!("{}", tr("example-env-overrides-global-config"));
    println!("  INKLOG_GLOBAL_LEVEL=debug|info|warn|error|trace|fatal");
    println!("  INKLOG_GLOBAL_FORMAT=json|text");
    println!("  INKLOG_GLOBAL_AUTO_FALLBACK=true|false");

    println!("\n{}", tr("example-env-overrides-performance-config"));
    println!("  INKLOG_PERFORMANCE_CHANNEL_CAPACITY=10000");
    println!("  INKLOG_PERFORMANCE_WORKER_THREADS=4");
    println!("  INKLOG_PERFORMANCE_CHANNEL_STRATEGY=fixed|adaptive");

    println!("\n{}", tr("example-env-overrides-file-sink-config"));
    println!("  INKLOG_FILE_SINK_ENABLED=true");
    println!("  INKLOG_FILE_SINK_PATH=logs/app.log");
    println!("  INKLOG_FILE_SINK_MAX_SIZE=104857600");

    println!("\n{}", tr("example-env-overrides-http-server-config"));
    println!("  INKLOG_HTTP_SERVER_ENABLED=true");
    println!("  INKLOG_HTTP_SERVER_HOST=0.0.0.0");
    println!("  INKLOG_HTTP_SERVER_PORT=9090");
    println!("  INKLOG_HTTP_SERVER_ERROR_MODE=warn|strict");
    println!("  INKLOG_HTTP_SERVER_METRICS_PATH=/metrics");
    println!("  INKLOG_HTTP_SERVER_HEALTH_PATH=/health");

    println!("\n{}", tr("example-env-overrides-database-sink-config"));
    println!("  INKLOG_DATABASE_SINK_DRIVER=postgres|mysql|sqlite");
    println!("  INKLOG_DATABASE_SINK_URL=postgres://user:pass@host/db");
}

/// 演示 load_with_env_overrides() 实际调用
fn show_load_with_env_overrides() {
    print_section(&tr("example-env-overrides-example-2-calling-load-with-env"));

    println!(
        "{}\n",
        tr("example-env-overrides-calling-inklogconfig-load-with")
    );

    match InklogConfig::load_with_env_overrides() {
        Ok(config) => {
            println!("{}", tr("example-env-overrides-config-loaded-successfully"));
            println!("\n{}", tr("example-env-overrides-loaded-config"));
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-global-level",
                    &[("level", &config.global.level)]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-global-format",
                    &[("format", &config.global.format)]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-global-auto-fallback",
                    &[("auto_fallback", &config.global.auto_fallback)]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-performance-channel-capacity",
                    &[("channel_capacity", &config.performance.channel_capacity)]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-performance-worker-threads",
                    &[("worker_threads", &config.performance.worker_threads)]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-performance-channel-strategy",
                    &[(
                        "channel_strategy",
                        &format!("{:?}", config.performance.channel_strategy)
                    )]
                )
            );
            println!(
                "\n  {}",
                tr_args(
                    "example-env-overrides-file-sink-enabled",
                    &[(
                        "value",
                        &(config.file_sink.as_ref().is_some_and(|c| c.enabled))
                    )]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-http-server-enabled",
                    &[(
                        "value",
                        &(config.http_server.as_ref().is_some_and(|c| c.enabled))
                    )]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-env-overrides-database-sink-enabled",
                    &[(
                        "value",
                        &(config.database_sink.as_ref().is_some_and(|c| c.enabled))
                    )]
                )
            );
        }
        Err(e) => {
            println!(
                "{}",
                tr_args("example-env-overrides-failed-to-load-config", &[("e", &e)])
            );
            println!(
                "\n{}",
                tr("example-env-overrides-this-usually-means-a-config")
            );
        }
    }
}

/// 演示环境变量覆盖效果（设置后加载对比）
fn show_env_override_demo() {
    print_section(&tr("example-env-overrides-example-3-env-var-override"));

    // 保存原始值用于恢复
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    println!(
        "{}\n",
        tr_args(
            "example-env-overrides-test-id-isolates-concurrent-runs",
            &[("test_id", &test_id)]
        )
    );

    let baseline = InklogConfig::load_with_env_overrides().expect("baseline load");
    println!("{}", tr("example-env-overrides-baseline-config-no-env-var"));
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-global-level-2",
            &[("level", &baseline.global.level)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-performance-channel-capacity-2",
            &[("channel_capacity", &baseline.performance.channel_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-performance-worker-threads-2",
            &[("worker_threads", &baseline.performance.worker_threads)]
        )
    );

    println!(
        "\n{}",
        tr("example-env-overrides-setting-environment-variables")
    );
    let env_vars = [
        ("INKLOG_GLOBAL_LEVEL", "debug"),
        ("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "20000"),
        ("INKLOG_PERFORMANCE_WORKER_THREADS", "4"),
    ];
    for (key, value) in env_vars {
        println!(
            "  {}",
            tr_args(
                "example-env-overrides-export",
                &[("key", &key), ("value", &value)]
            )
        );
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(env_vars[0].0, env_vars[0].1) };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(env_vars[1].0, env_vars[1].1) };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(env_vars[2].0, env_vars[2].1) };
    }

    let overridden = InklogConfig::load_with_env_overrides().expect("overridden load");
    println!("\n{}", tr("example-env-overrides-config-after-overrides"));
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-global-level-expected-debug",
            &[("level", &overridden.global.level)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-performance-channel-capacity-3",
            &[("channel_capacity", &overridden.performance.channel_capacity)]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-performance-worker-threads-3",
            &[("worker_threads", &overridden.performance.worker_threads)]
        )
    );

    let level_ok = overridden.global.level == "debug";
    let capacity_ok = overridden.performance.channel_capacity == 20000;
    let workers_ok = overridden.performance.worker_threads == 4;

    println!("\n{}", tr("example-env-overrides-verification-results"));
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-global-level-override",
            &[
                ("value", &(baseline.global.level != overridden.global.level)),
                ("value2", &(if level_ok { "✓" } else { "✗" }))
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-channel-capacity-override",
            &[
                (
                    "value",
                    &(baseline.performance.channel_capacity
                        != overridden.performance.channel_capacity)
                ),
                ("value2", &(if capacity_ok { "✓" } else { "✗" }))
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-env-overrides-worker-threads-override",
            &[
                (
                    "value",
                    &(baseline.performance.worker_threads != overridden.performance.worker_threads)
                ),
                ("value2", &(if workers_ok { "✓" } else { "✗" }))
            ]
        )
    );

    for (key, _) in env_vars {
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(key) };
    }
    println!(
        "\n{}",
        tr("example-env-overrides-test-environment-variables")
    );
}

/// 演示配置优先级
fn show_priority_order() {
    print_section(&tr("example-env-overrides-example-4-config-precedence"));

    println!("{}", tr("example-env-overrides-inklog-config-loading"));
    println!(
        "  {}",
        tr("example-env-overrides-1-code-defaults-default-default")
    );
    println!(
        "  {}",
        tr("example-env-overrides-2-config-file-inklog-toml")
    );
    println!(
        "  {}",
        tr("example-env-overrides-3-env-var-overrides-inklog")
    );

    println!(
        "\n{}",
        tr("example-env-overrides-load-with-env-overrides-flow")
    );
    println!("  {}", tr("example-env-overrides-1-call-load-sync-to-load"));
    println!(
        "  {}",
        tr("example-env-overrides-2-call-apply-env-overrides-to")
    );
    println!(
        "  {}",
        tr("example-env-overrides-3-return-the-final-config")
    );

    println!("\n{}", tr("example-env-overrides-recommendations"));
    println!(
        "  - Development: manage base config in the config file, use env vars for temporary tweaks"
    );
    println!(
        "  - Containers: use env-var overrides for easy Kubernetes ConfigMap/Secret injection"
    );
    println!(
        "  {}",
        tr("example-env-overrides-ci-cd-use-env-vars-to-switch")
    );

    println!("\n{}", tr("example-env-overrides-code-example"));
    println!("  // Option 1: env-var overrides only (defaults apply when no config file exists)");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!();
    println!("  // Option 2: initialize directly with LoggerManager");
    println!("  let logger = LoggerManager::with_config(");
    println!(
        "      {}",
        tr("example-env-overrides-inklogconfig-load-with-env")
    );
    println!("  ).await?;");
}
