// Copyright (c) 2026 Kirky.X
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
    print_separator("inklog 环境变量覆盖加载示例");

    show_supported_env_vars();
    show_load_with_env_overrides();
    show_env_override_demo();
    show_priority_order();

    println!("\n所有环境变量覆盖示例展示完毕。");
}

/// 演示支持的环境变量列表
fn show_supported_env_vars() {
    print_section("示例 1：支持的环境变量列表");

    println!("Global 配置：");
    println!("  INKLOG_GLOBAL_LEVEL=debug|info|warn|error|trace|fatal");
    println!("  INKLOG_GLOBAL_FORMAT=json|text");
    println!("  INKLOG_GLOBAL_AUTO_FALLBACK=true|false");

    println!("\nPerformance 配置：");
    println!("  INKLOG_PERFORMANCE_CHANNEL_CAPACITY=10000");
    println!("  INKLOG_PERFORMANCE_WORKER_THREADS=4");
    println!("  INKLOG_PERFORMANCE_CHANNEL_STRATEGY=fixed|adaptive");

    println!("\nFile Sink 配置：");
    println!("  INKLOG_FILE_SINK_ENABLED=true");
    println!("  INKLOG_FILE_SINK_PATH=logs/app.log");
    println!("  INKLOG_FILE_SINK_MAX_SIZE=104857600");

    println!("\nHTTP Server 配置：");
    println!("  INKLOG_HTTP_SERVER_ENABLED=true");
    println!("  INKLOG_HTTP_SERVER_HOST=0.0.0.0");
    println!("  INKLOG_HTTP_SERVER_PORT=9090");
    println!("  INKLOG_HTTP_SERVER_ERROR_MODE=warn|strict");
    println!("  INKLOG_HTTP_SERVER_METRICS_PATH=/metrics");
    println!("  INKLOG_HTTP_SERVER_HEALTH_PATH=/health");

    println!("\nDatabase Sink 配置：");
    println!("  INKLOG_DATABASE_SINK_DRIVER=postgres|mysql|sqlite");
    println!("  INKLOG_DATABASE_SINK_URL=postgres://user:pass@host/db");
}

/// 演示 load_with_env_overrides() 实际调用
fn show_load_with_env_overrides() {
    print_section("示例 2：load_with_env_overrides() 实际调用");

    println!("调用 InklogConfig::load_with_env_overrides()...\n");

    match InklogConfig::load_with_env_overrides() {
        Ok(config) => {
            println!("✓ 配置加载成功");
            println!("\n加载后的配置：");
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
            println!("✗ 配置加载失败: {}", e);
            println!("\n（这通常表示配置文件解析错误，检查 TOML 语法）");
        }
    }
}

/// 演示环境变量覆盖效果（设置后加载对比）
fn show_env_override_demo() {
    print_section("示例 3：环境变量覆盖效果演示");

    // 保存原始值用于恢复
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    println!("测试 ID: {}（隔离并发运行）\n", test_id);

    // 1. 加载默认配置作为基线
    let baseline = InklogConfig::load_with_env_overrides().expect("baseline load");
    println!("基线配置（无环境变量覆盖）：");
    println!("  global.level = \"{}\"", baseline.global.level);
    println!(
        "  performance.channel_capacity = {}",
        baseline.performance.channel_capacity
    );
    println!(
        "  performance.worker_threads = {}",
        baseline.performance.worker_threads
    );

    // 2. 设置环境变量
    println!("\n设置环境变量：");
    let env_vars = [
        ("INKLOG_GLOBAL_LEVEL", "debug"),
        ("INKLOG_PERFORMANCE_CHANNEL_CAPACITY", "20000"),
        ("INKLOG_PERFORMANCE_WORKER_THREADS", "4"),
    ];
    for (key, value) in env_vars {
        println!("  export {}={}", key, value);
        std::env::set_var(env_vars[0].0, env_vars[0].1);
        std::env::set_var(env_vars[1].0, env_vars[1].1);
        std::env::set_var(env_vars[2].0, env_vars[2].1);
    }

    // 3. 重新加载并验证覆盖生效
    let overridden = InklogConfig::load_with_env_overrides().expect("overridden load");
    println!("\n覆盖后配置：");
    println!(
        "  global.level = \"{}\" (期望: \"debug\")",
        overridden.global.level
    );
    println!(
        "  performance.channel_capacity = {} (期望: 20000)",
        overridden.performance.channel_capacity
    );
    println!(
        "  performance.worker_threads = {} (期望: 4)",
        overridden.performance.worker_threads
    );

    // 验证
    let level_ok = overridden.global.level == "debug";
    let capacity_ok = overridden.performance.channel_capacity == 20000;
    let workers_ok = overridden.performance.worker_threads == 4;

    println!("\n验证结果：");
    println!(
        "  global.level 覆盖: {} → {}",
        baseline.global.level != overridden.global.level,
        if level_ok { "✓" } else { "✗" }
    );
    println!(
        "  channel_capacity 覆盖: {} → {}",
        baseline.performance.channel_capacity != overridden.performance.channel_capacity,
        if capacity_ok { "✓" } else { "✗" }
    );
    println!(
        "  worker_threads 覆盖: {} → {}",
        baseline.performance.worker_threads != overridden.performance.worker_threads,
        if workers_ok { "✓" } else { "✗" }
    );

    // 4. 清理环境变量
    for (key, _) in env_vars {
        std::env::remove_var(key);
    }
    println!("\n（已清理测试环境变量）");
}

/// 演示配置优先级
fn show_priority_order() {
    print_section("示例 4：配置优先级");

    println!("inklog 配置加载优先级（从低到高）：");
    println!("  1. 代码默认值（Default::default()）");
    println!("  2. 配置文件（inklog.toml / INKLOG_CONFIG_PATH）");
    println!("  3. 环境变量覆盖（INKLOG_*）");

    println!("\nload_with_env_overrides() 流程：");
    println!("  1. 调用 load_sync() 加载默认值 + 配置文件");
    println!("  2. 调用 apply_env_overrides() 应用 INKLOG_* 环境变量");
    println!("  3. 返回最终配置");

    println!("\n使用建议：");
    println!("  - 开发环境：用配置文件管理基础配置，环境变量做临时调整");
    println!("  - 容器部署：用环境变量覆盖，便于 Kubernetes ConfigMap/Secret 注入");
    println!("  - CI/CD：用环境变量区分 dev/staging/prod 配置");

    println!("\n代码示例：");
    println!("  // 方式 1：仅环境变量覆盖（无配置文件时用默认值）");
    println!("  let config = InklogConfig::load_with_env_overrides()?;");
    println!();
    println!("  // 方式 2：结合 LoggerManager 直接初始化");
    println!("  let logger = LoggerManager::with_config(");
    println!("      InklogConfig::load_with_env_overrides()?");
    println!("  ).await?;");
}
