// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! confers 审计桥示例（feature `confers-audit`）
//!
//! 演示把 confers 的配置审计事件接入 inklog 落盘管道：
//! - 实现 confers `AuditSink` 端口的 [`ConfersAuditSink`] 注入 `AuditWriter`
//! - 审计事件（本地 HMAC 链文件照旧）同步批量转发到 inklog 结构化 sink
//! - 级别映射：confers Durable → WARN，BestEffort → INFO
//! - 端口契约「不得无限阻塞」：通道满/无 runtime 时丢弃计数
//!
//! # 运行
//! ```bash
//! cargo run --features confers-audit --bin confers_audit
//! ```

use std::sync::Arc;
use std::time::Duration;

use confers::audit::{AuditEvent, AuditWriter};
use inklog::chrono::Utc;
use inklog::config::ConsoleSinkConfig;
use inklog::integrations::ConfersAuditSink;
use inklog::sink::console::ConsoleSink;
use inklog_examples::common::{print_section, print_separator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&inklog::i18n::tr("example-confers-audit-title"));

    // 1. inklog 侧结构化 sink（控制台；换成 FileSink/DatabaseSink 即改落盘目标）
    let console = ConsoleSink::new(
        ConsoleSinkConfig {
            enabled: true,
            colored: false,
            stderr_levels: vec![],
            masking_enabled: false,
            output_format: Default::default(),
        },
        inklog::LogTemplate::default(),
    );

    // 2. confers AuditSink 端口的 inklog 实现
    let bridge = Arc::new(ConfersAuditSink::new(Arc::new(console)));

    // 3. 注入 confers AuditWriter：本地审计链文件照旧，事件同步转发 inklog
    let log_dir = tempfile::tempdir()?;
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(log_dir.path().to_path_buf())
        .sink(bridge.clone())
        .build();

    print_section(&inklog::i18n::tr("example-confers-audit-write-events"));
    let events = vec![
        AuditEvent::KeyAccess {
            key: "db.password".to_string(),
            timestamp: Utc::now(),
        },
        AuditEvent::Decrypt {
            field: "api_key".to_string(),
            success: true,
            timestamp: Utc::now(),
        },
        AuditEvent::LoadSuccess {
            source: "app.toml".to_string(),
            timestamp: Utc::now(),
        },
    ];
    for event in &events {
        writer.write(event.clone())?;
    }

    // 4. 等待异步 writer task 落盘后展示观测计数
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!();
    let mut result_args = inklog::i18n::MsgArgs::new();
    result_args.set("accepted", bridge.accepted());
    result_args.set("dropped", bridge.dropped());
    result_args.set("failures", bridge.write_failures());
    println!(
        "{}",
        inklog::i18n::tr_args("example-confers-audit-result", result_args)
    );
    let mut dir_args = inklog::i18n::MsgArgs::new();
    dir_args.set("path", log_dir.path().display().to_string());
    println!(
        "{}",
        inklog::i18n::tr_args("example-confers-audit-chain-dir", dir_args)
    );

    Ok(())
}
