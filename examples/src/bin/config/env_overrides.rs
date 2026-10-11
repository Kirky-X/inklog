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
use inklog_examples::common::{print_section, print_separator};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    print_separator("inklog env-var override loading example");

    show_supported_env_vars();
    show_load_with_env_overrides();
    show_env_override_demo();
    show_priority_order();

    println!("\nAll env-var override examples completed.");
}

/// 演示支持的环境变量列表
fn show_supported_env_vars() {
    print_section("Example 1: supported environment variables");

    println!("Global config:");
    println!("  INKLOG_GLOBAL_LEVEL=debug|info|warn|error|trace|fatal");
    println!("  INKLOG_GLOBAL_FORMAT=json|text");
    println!("  INKLOG_GLOBAL_AUTO_FALLBACK=true|false");

    println!("\nPerformance config:");
    println!("  INKLOG_PERFORMANCE_CHANNEL_CAPACITY=10000");
    println!("  INKLOG_PERFORMANCE_WORKER_THREADS=4");
    println!("  INKLOG_PERFORMANCE_CHANNEL_STRATEGY=fixed|adaptive");

    println!("\nFile sink config:");
    println!("  INKLOG_FILE_SINK_ENABLED=true");
    println!("  INKLOG_FILE_SINK_PATH=logs/app.log");
    println!("  INKLOG_FILE_SINK_MAX_SIZE=104857600");

    println!("\nHTTP server config:");
    println!("  INKLOG_HTTP_SERVER_ENABLED=true");
    println!("  INKLOG_HTTP_SERVER_HOST=0.0.0.0");
    println!("  INKLOG_HTTP_SERVER_PORT=9090");
    println!("  INKLOG_HTTP_SERVER_ERROR_MODE=warn|strict");
    println!("  INKLOG_HTTP_SERVER_METRICS_PATH=/metrics");
    println!("  INKLOG_HTTP_SERVER_HEALTH_PATH=/health");

    println!("\nDatabase sink config:");
    println!("  INKLOG_DATABASE_SINK_DRIVER=postgres|mysql|sqlite");
    println!("  INKLOG_DATABASE_SINK_URL=postgres://user:pass@host/db");
}

/// 演示 load_with_env_overrides() 实际调用
fn show_load_with_env_overrides() {
    print_section("Example 2: calling load_with_env_overrides() for real");

    println!("Calling InklogConfig::load_with_env_overrides()...\n");

    match InklogConfig::load_with_env_overrides() {
        Ok(config) => {
            println!("✓ Config loaded successfully");
            println!("\nLoaded config:");
            println!("  global.level          = \"{}\"", config.global.level);
            println!("  global.format         = \"{}\"", config.global.format);
            println!("  global.auto_fallback  = {}", config.global.auto_fallback);
            println!(
                "  performance.channel_capacity  = {}",
                config.performance.channel_capacity
            );
            println!(
                "  performance.worker_threads    = {}",
                config.performance.worker_threads
            );
            println!(
                "  performance.channel_strategy  = {:?}",
                config.performance.channel_strategy
            );
            println!(
                "\n  file_sink.enabled     = {}",
                config.file_sink.as_ref().is_some_and(|c| c.enabled)
            );
            println!(
                "  http_server.enabled   = {}",
                config.http_server.as_ref().is_some_and(|c| c.enabled)
            );
            println!(
                "  database_sink.enabled = {}",
                config.database_sink.as_ref().is_some_and(|c| c.enabled)
            );
        }
        Err(e) => {
            println!("✗ Failed to load config: {}", e);
            println!("\n(This usually means a config file parse error; check the TOML syntax)");
        }
    }
}

/// 演示环境变量覆盖效果（设置后加载对比）
fn show_env_override_demo() {
    print_section("Example 3: env-var override effect demo");

    // 保存原始值用于恢复
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    println!("Test ID: {} (isolates concurrent runs)\n", test_id);

    let baseline = InklogConfig::load_with_env_overrides().expect("baseline load");
    println!("Baseline config (no env-var overrides):");
    println!("  global.level = \"{}\"", baseline.global.level);
    println!(
        "  performance.channel_capacity = {}",
        baseline.performance.channel_capacity
    );
    println!(
        "  performance.worker_threads = {}",
        baseline.performance.worker_threads
    );

    println!("\nSetting environment variables:");
    let env_vars = [
        ("INKLOG_GLOBAL_LEVEL", "debug"),
        ("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "20000"),
        ("INKLOG_PERFORMANCE_WORKER_THREADS", "4"),
    ];
    for (key, value) in env_vars {
        println!("  export {}={}", key, value);
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(env_vars[0].0, env_vars[0].1) };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(env_vars[1].0, env_vars[1].1) };
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(env_vars[2].0, env_vars[2].1) };
    }

    let overridden = InklogConfig::load_with_env_overrides().expect("overridden load");
    println!("\nConfig after overrides:");
    println!(
        "  global.level = \"{}\" (expected: \"debug\")",
        overridden.global.level
    );
    println!(
        "  performance.channel_capacity = {} (expected: 20000)",
        overridden.performance.channel_capacity
    );
    println!(
        "  performance.worker_threads = {} (expected: 4)",
        overridden.performance.worker_threads
    );

    let level_ok = overridden.global.level == "debug";
    let capacity_ok = overridden.performance.channel_capacity == 20000;
    let workers_ok = overridden.performance.worker_threads == 4;

    println!("\nVerification results:");
    println!(
        "  global.level override: {} → {}",
        baseline.global.level != overridden.global.level,
        if level_ok { "✓" } else { "✗" }
    );
    println!(
        "  channel_capacity override: {} → {}",
        baseline.performance.channel_capacity != overridden.performance.channel_capacity,
        if capacity_ok { "✓" } else { "✗" }
    );
    println!(
        "  worker_threads override: {} → {}",
        baseline.performance.worker_threads != overridden.performance.worker_threads,
        if workers_ok { "✓" } else { "✗" }
    );

    for (key, _) in env_vars {
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(key) };
    }
    println!("\n(Test environment variables cleaned up)");
}

/// 演示配置优先级
fn show_priority_order() {
    print_section("Example 4: config precedence");

    println!("inklog config loading precedence (low to high):");
    println!("  1. Code defaults (Default::default())");
    println!("  2. Config file (inklog.toml / INKLOG_CONFIG_PATH)");
    println!("  3. Env-var overrides (INKLOG_*)");

    println!("\nload_with_env_overrides() flow:");
    println!("  1. Call load_sync() to load defaults + config file");
    println!("  2. Call apply_env_overrides() to apply INKLOG_* env vars");
    println!("  3. Return the final config");

    println!("\nRecommendations:");
    println!(
        "  - Development: manage base config in the config file, use env vars for temporary tweaks"
    );
    println!(
        "  - Containers: use env-var overrides for easy Kubernetes ConfigMap/Secret injection"
    );
    println!("  - CI/CD: use env vars to switch dev/staging/prod configs");

    println!("\nCode example:");
    println!("  // Option 1: env-var overrides only (defaults apply when no config file exists)");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!();
    println!("  // Option 2: initialize directly with LoggerManager");
    println!("  let logger = LoggerManager::with_config(");
    println!("      InklogConfig::load_with_env_overrides()?");
    println!("  ).await?;");
}
