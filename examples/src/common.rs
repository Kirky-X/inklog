// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 共享辅助函数
//!
//! 提供示例中常用的辅助功能。

use anyhow::Result;
use std::time::Duration;

/// 从 i18n catalog 查询无参文案（示例叙述性输出统一走 catalog，默认英文）。
pub fn tr(id: &str) -> String {
    inklog::i18n::tr(id)
}

/// 从 i18n catalog 查询带命名参数的文案；参数按 `{ $key }` 占位插值。
pub fn tr_args(id: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut msg = inklog::i18n::MsgArgs::new();
    for (key, value) in args {
        msg.set(key, *value);
    }
    inklog::i18n::tr_args(id, msg)
}

/// 等待用户按 Ctrl+C
pub async fn wait_for_ctrl_c() -> Result<()> {
    tokio::signal::ctrl_c().await?;
    Ok(())
}

/// 打印分隔线
pub fn print_separator(title: &str) {
    println!("\n{}\n", "=".repeat(60));
    println!("  {}", title);
    println!("{}\n", "=".repeat(60));
}

/// 打印子标题
pub fn print_section(title: &str) {
    println!("\n--- {} ---\n", title);
}

/// 格式化持续时间
pub fn format_duration(duration: Duration) -> String {
    let micros = duration.as_micros();
    if micros < 1000 {
        format!("{}μs", micros)
    } else if micros < 1_000_000 {
        format!("{:.2}ms", duration.as_secs_f64() * 1000.0)
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

/// 生成临时文件路径
pub fn temp_file_path(prefix: &str) -> String {
    format!(
        "/tmp/inklog_example_{}_{}.log",
        prefix,
        uuid::Uuid::new_v4()
    )
}
