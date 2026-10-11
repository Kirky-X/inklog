// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Sink degraded机制示例
//!
//! 演示 inklog 的故障检测和自动degraded策略：
//!
//! ## degraded策略（三级保障）
//!
//! | 故障场景 | degraded策略 |
//! |----------|----------|
//! | Database 不可用 | degraded到 File Sink |
//! | File 不可用（磁盘满）| degraded到 Console Sink |
//! | Console 不可用 | 最终degraded（丢弃日志并告警）|
//!
//! ## 运行
//!
//! ```bash
//! cargo run --bin fallback
//! ```
//!
//! ## 示例内容
//!
//! 1. **multi_sink_config()**: 配置多个 Sink（Console + File）
//! 2. **simulate_failure()**: 模拟primary sink 故障场景
//! 3. **fallback_demo()**: 演示degraded到 File Sink

use async_trait::async_trait;
use inklog::LogTemplate;
use inklog::chrono::Utc;
use inklog::config::{ConsoleSinkConfig, FileSinkConfig};
use inklog::log_record::LogRecord;
use inklog::sink::LogSink;
use inklog::sink::console::ConsoleSink;
use inklog::sink::file::FileSink;
use inklog::tokio::sync::Mutex;
use inklog::{SinkHealth, SinkStatus};
use inklog_examples::common::{print_section, print_separator, temp_file_path};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// degraded状态追踪器
///
/// 用于模拟 Sink degraded过程中的状态变化
#[derive(Debug, Clone)]
struct FallbackState {
    /// 当前激活的 Sink 类型
    active_sink: String,
    /// degraded事件计数
    fallback_count: u32,
    /// 最后degraded时间
    last_fallback_time: Option<chrono::DateTime<Utc>>,
}

impl FallbackState {
    fn new(primary_name: &str) -> Self {
        Self {
            active_sink: primary_name.to_string(),
            fallback_count: 0,
            last_fallback_time: None,
        }
    }

    fn trigger_fallback(&mut self, new_sink: &str) {
        println!(
            "[{}] degrading: {} -> {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            self.active_sink,
            new_sink
        );
        self.active_sink = new_sink.to_string();
        self.fallback_count += 1;
        self.last_fallback_time = Some(Utc::now());
    }
}

/// 模拟故障的 Sink（包装器）
///
/// 用于演示当primary sink 发生故障时如何切换到fallback sink
///
/// 使用 `tokio::sync::Mutex` 实现内部可变性，以满足 `LogSink: &self` 的 trait 约束。
/// 内部 sink 以 `Arc` 持有，便于在 flush/shutdown 中先 clone 出句柄、
/// 释放锁之后再执行异步 I/O（避免持锁 await）。
struct FailingSink {
    inner: Arc<Mutex<Option<Arc<dyn LogSink>>>>,
    fail_after: usize,
    write_count: Arc<Mutex<usize>>,
    original_name: String,
}

impl FailingSink {
    fn new(sink: Box<dyn LogSink>, fail_after: usize, name: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Some(Arc::from(sink)))),
            fail_after,
            write_count: Arc::new(Mutex::new(0)),
            original_name: name.to_string(),
        }
    }
}

#[async_trait]
impl LogSink for FailingSink {
    async fn write(&self, record: &LogRecord) -> Result<(), inklog::InklogError> {
        // 原子地检查计数并获取 inner sink，避免 TOCTOU 竞态
        let mut count = self.write_count.lock().await;
        *count += 1;
        let current_count = *count;

        if current_count <= self.fail_after {
            // 在同一个临界区内完成计数检查和 sink 获取
            let inner = self.inner.lock().await;
            if let Some(ref sink) = *inner {
                return sink.write(record).await;
            }
        }
        drop(count); // 释放计数锁后再触发故障

        // 触发故障
        Err(inklog::InklogError::IoError(std::io::Error::other(
            format!(
                "{} sink failed after {} writes",
                self.original_name, self.fail_after
            ),
        )))
    }

    async fn flush(&self) -> Result<(), inklog::InklogError> {
        // 参照 write() 的模式：锁内只 clone 出 sink 的 Arc 句柄，
        // 释放锁之后再执行异步 I/O，避免持有 tokio Mutex 期间 await
        //（持锁 await 会阻塞其他尝试获取锁的任务，甚至造成死锁）
        let sink = {
            let inner = self.inner.lock().await;
            inner.as_ref().cloned()
        }; // 锁在此处已释放

        match sink {
            Some(sink) => sink.flush().await,
            None => Err(inklog::InklogError::IoError(std::io::Error::other(
                "Sink is failed",
            ))),
        }
    }

    fn is_healthy(&self) -> bool {
        // try_lock 避免在 sync 方法中 await；锁被占用说明写入正在进行，
        // 视为healthy（竞争 ≠ 故障），仅 inner 为 None（已故障/关闭）时unhealthy
        self.inner.try_lock().map(|g| g.is_some()).unwrap_or(true)
    }

    async fn shutdown(&self) -> Result<(), inklog::InklogError> {
        // 同 flush()：锁内 clone Arc，锁外执行异步 I/O
        let sink = {
            let inner = self.inner.lock().await;
            inner.as_ref().cloned()
        };

        match sink {
            Some(sink) => sink.shutdown().await,
            None => Ok(()),
        }
    }
}

/// 示例 1: 多 Sink 配置
///
/// 展示如何同时配置 Console 和 File Sink，实现日志的双重输出。
/// 这是在生产环境中推荐的配置方式，确保日志不会因单一 Sink 故障而丢失。
async fn multi_sink_config() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("Example 1: multi-sink setup (console + file)");

    let log_path = temp_file_path("multi_sink");
    println!("Log file path: {}", log_path);

    // 1. 配置 Console Sink
    let console_config = ConsoleSinkConfig {
        enabled: true,
        colored: true,
        stderr_levels: vec!["error".to_string(), "warn".to_string()],
        masking_enabled: false,
        output_format: Default::default(),
        secret_scan_enabled: false,
    };
    let console_sink = ConsoleSink::new(console_config, LogTemplate::new("[{level}] {message}"));
    println!("Console sink: configured");

    let file_config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&log_path),
        max_size: "10MB".to_string(),
        rotation_time: "daily".to_string(),
        keep_files: 5,
        compress: false,
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 7,
        max_total_size: "100MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 10,
        flush_interval_ms: 100,
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };
    let file_sink = FileSink::new(file_config)?;
    println!("File sink: configured");

    let records = vec![
        LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "application started successfully".to_string(),
            target: "fallback::multi_sink".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        },
        LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "database connection established".to_string(),
            target: "fallback::multi_sink".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        },
        LogRecord {
            timestamp: Utc::now(),
            level: "WARN".to_string(),
            message: "config parameters use defaults".to_string(),
            target: "fallback::multi_sink".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        },
    ];

    print_section("writing logs to multiple sinks");

    for record in &records {
        console_sink.write(record).await?;
        file_sink.write(record).await?;

        let level_marker = match record.level.as_str() {
            "ERROR" => "[ERROR]",
            "WARN" => "[WARN] ",
            "INFO" => "[INFO] ",
            _ => "[     ]",
        };
        println!("{} -> Console + File: {}", level_marker, record.message);
    }

    console_sink.flush().await?;
    file_sink.flush().await?;

    print_section("verifying the file sink");
    let content = fs::read_to_string(&log_path)?;
    println!("File content ({} bytes):\n{}", content.len(), content);

    // 6. 清理
    cleanup_files(&log_path, "inklog_example_multi_sink")?;

    println!("\n✓ Multi-sink setup completed");
    println!("  - Console sink: real-time output to the console");
    println!("  - File sink: persist to a file");
    println!("  - Both work simultaneously without interfering");

    Ok(())
}

/// 示例 2: 模拟故障与degraded切换
///
/// 演示当primary sink 发生故障时，系统如何检测故障并切换到fallback sink。
/// 这是 inklog 高可用性的核心机制。
async fn simulate_failure() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("Example 2: failure simulation and fallback switch");

    let backup_path = temp_file_path("fallback_backup");
    println!("Fallback log file: {}", backup_path);

    let mut state = FallbackState::new("Console");
    println!("\nInitial state: primary sink = console");

    // 2. 配置备用 File Sink
    let backup_config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&backup_path),
        max_size: "10MB".to_string(),
        rotation_time: "daily".to_string(),
        keep_files: 3,
        compress: false,
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 1,
        max_total_size: "50MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 1,
        flush_interval_ms: 10,
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };
    let backup_sink = FileSink::new(backup_config)?;
    println!("Fallback file sink: ready");

    // 3. 配置主 Console Sink（包装为会故障的 Sink）
    let primary_config = ConsoleSinkConfig {
        enabled: true,
        colored: true,
        stderr_levels: vec![],
        masking_enabled: false,
        output_format: Default::default(),
        secret_scan_enabled: false,
    };
    let console_sink = ConsoleSink::new(primary_config, LogTemplate::new("[{level}] {message}"));

    // 包装为会故障的 Sink（在第 5 次写入后故障）
    let primary_sink = FailingSink::new(Box::new(console_sink), 5, "Console");

    let test_records: Vec<LogRecord> = (1..=8)
        .map(|i| LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: format!("test log message #{}", i),
            target: "fallback::simulate".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
            trace_id: None,
            span_id: None,
        })
        .collect();

    print_section("writing logs (triggering failure)");

    let mut success_count = 0;
    let mut fallback_triggered = false;

    for record in &test_records {
        match primary_sink.write(record).await {
            Ok(()) => {
                success_count += 1;
                println!(
                    "[{}] [primary sink] write succeeded #{}",
                    Utc::now().format("%H:%M:%S%.3f"),
                    success_count
                );
            }
            Err(e) => {
                // primary sink 故障，触发degraded
                if !fallback_triggered {
                    println!("\n*** Primary sink failure: {} ***", e);
                    state.trigger_fallback("File");
                    fallback_triggered = true;
                }

                println!(
                    "[{}] [fallback sink] took over writing",
                    Utc::now().format("%H:%M:%S%.3f")
                );
                backup_sink.write(record).await?;
                backup_sink.flush().await?;
            }
        }
    }

    print_section("degradation event statistics");
    println!("degradation count: {}", state.fallback_count);
    println!(
        "last degradation time: {}",
        state
            .last_fallback_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    println!("active sinks: {}", state.active_sink);

    print_section("verifying the fallback file");
    let content = fs::read_to_string(&backup_path)?;
    println!("Fallback file content ({} bytes):", content.len());
    for line in content.lines() {
        println!("  {}", line);
    }

    // 8. 清理
    cleanup_files(&backup_path, "inklog_example_fallback_backup")?;

    println!("\n✓ Failure simulation and fallback switch completed");
    println!("  - Primary sink fails after write #{}", 5);
    println!("  - Fallback sink took over the next {} logs", 8 - 5);

    Ok(())
}

/// Example 3: full fallback strategy demo
///
/// 展示完整的degraded策略流程，包括：
/// - healthy状态检测
/// - degraded条件判断
/// - degraded执行和日志记录
async fn fallback_demo() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("Example 3: full fallback strategy demo");

    let primary_path = temp_file_path("primary");
    let fallback_path = temp_file_path("fallback");
    println!("Primary log file: {}", primary_path);
    println!("Fallback log file: {}", fallback_path);

    // 1. 展示 Sink healthy状态枚举
    print_section("Sink health status types");

    println!("\nSinkStatus enum:");
    println!("  - Healthy: sink fully healthy");
    println!("  - Degraded: sink degraded, may need fallback");
    println!("  - Unhealthy: sink unavailable, fallback required");

    println!("\nSinkHealth struct:");
    println!("  - is_healthy: bool - whether healthy");
    println!("  - status: SinkStatus - detailed status");
    println!("  - last_error: Option<String> - error info");

    let health_checks = vec![
        ("Console Sink", SinkHealth::healthy()),
        ("File Sink", SinkHealth::healthy()),
        (
            "Database Sink",
            SinkHealth::unhealthy("Connection timeout".to_string()),
        ),
    ];

    print_section("simulated health check result");
    for (name, health) in &health_checks {
        let is_healthy = matches!(health.status, SinkStatus::Healthy);
        let status_icon = if is_healthy { "✓" } else { "✗" };
        let status_text = match &health.status {
            SinkStatus::Healthy => "healthy",
            SinkStatus::Degraded { .. } => "degraded",
            SinkStatus::Unhealthy { .. } => "unhealthy",
            SinkStatus::NotStarted => "not started",
        };
        println!(
            "  {} {}: {} ({})",
            status_icon,
            name,
            status_text,
            health.last_error.as_deref().unwrap_or("no error")
        );

        if !is_healthy {
            println!("     -> Advice: degrade to the fallback sink");
        }
    }

    let primary_config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&primary_path),
        max_size: "10MB".to_string(),
        rotation_time: "daily".to_string(),
        keep_files: 3,
        compress: false,
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 1,
        max_total_size: "50MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 10,
        flush_interval_ms: 100,
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };
    let primary_sink = FileSink::new(primary_config)?;

    let fallback_config = FileSinkConfig {
        enabled: true,
        path: PathBuf::from(&fallback_path),
        max_size: "10MB".to_string(),
        rotation_time: "daily".to_string(),
        keep_files: 3,
        compress: false,
        compression_level: 3,
        encrypt: false,
        encryption_key_env: None,
        encryption_key_file: None,
        retention_days: 1,
        max_total_size: "50MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 10,
        flush_interval_ms: 100,
        fsync: false,
        audit_chain_enabled: false,
        masking_enabled: false,
        output_format: Default::default(),
    };
    let fallback_sink = FileSink::new(fallback_config)?;

    print_section("degradation flow simulation");

    let records: Vec<LogRecord> = (1..=10)
        .map(|i| {
            let level = if i <= 3 {
                "INFO"
            } else if i <= 7 {
                "WARN"
            } else {
                "ERROR"
            };
            LogRecord {
                timestamp: Utc::now(),
                level: level.to_string(),
                message: format!("log message #{} [{}]", i, level),
                target: "fallback::demo".to_string(),
                fields: Default::default(),
                file: Some(file!().to_string()),
                line: Some(line!()),
                thread_id: "main".to_string(),
                trace_id: None,
                span_id: None,
            }
        })
        .collect();

    // 模拟在第 6 条日志时primary sink 故障
    let failure_point = 6;
    let mut current_sink = "primary";

    for (i, record) in records.iter().enumerate() {
        let log_num = i + 1;

        if log_num == failure_point {
            println!(
                "\n[{}] *** Primary sink failed ***",
                Utc::now().format("%H:%M:%S%.3f")
            );
            println!("     Degrading: File(primary) -> File(fallback)");
            current_sink = "fallback";
        }

        let sink_name = if current_sink == "primary" {
            "primary sink"
        } else {
            "fallback sink"
        };

        if current_sink == "primary" {
            primary_sink.write(record).await?;
            primary_sink.flush().await?;
        } else {
            // 写入时标记这是degraded后的日志
            let fallback_record = LogRecord {
                timestamp: record.timestamp,
                level: record.level.clone(),
                message: format!("[fallback takeover] {}", record.message),
                target: record.target.clone(),
                fields: record.fields.clone(),
                file: record.file.clone(),
                line: record.line,
                thread_id: record.thread_id.clone(),
                trace_id: None,
                span_id: None,
            };
            fallback_sink.write(&fallback_record).await?;
            fallback_sink.flush().await?;
        }

        println!(
            "[{}] [{}] #{}: {}",
            Utc::now().format("%H:%M:%S%.3f"),
            sink_name,
            log_num,
            record.message
        );
    }

    print_section("file content after degradation");

    println!("\n--- Primary file (first {} logs) ---", failure_point - 1);
    let primary_content = fs::read_to_string(&primary_path)?;
    for line in primary_content.lines().take(10) {
        println!("  {}", line);
    }

    println!("\n--- Fallback file (logs after takeover) ---");
    let fallback_content = fs::read_to_string(&fallback_path)?;
    for line in fallback_content.lines().take(10) {
        println!("  {}", line);
    }

    // 6. 清理
    cleanup_files(&primary_path, "inklog_example_primary")?;
    cleanup_files(&fallback_path, "inklog_example_fallback")?;

    print_section("fallback strategy summary");

    println!("\nThree-tier fallback:");
    println!("  1. Database -> File (when the database is unavailable)");
    println!("     - Trigger: connection timeout, SQL errors");
    println!("     - Degradation action: write to a local file");
    println!();
    println!("  2. File -> Console (when the file is unavailable)");
    println!("     - Trigger: disk full, permission errors");
    println!("     - Degradation action: output to the console");
    println!();
    println!("  3. Console (final fallback)");
    println!("     - Trigger: all sinks unavailable");
    println!("     - Degradation action: log to syslog + alert");

    println!("\nDegradation event log:");
    println!("  - All degradation events are logged");
    println!("  - Includes degradation reason, time, and recovery state");
    println!("  - Degradation state queryable via the HTTP endpoint");

    println!("\n✓ Full fallback strategy demo completed");

    Ok(())
}

/// 清理临时文件
///
/// 删除指定路径相关的所有文件
fn cleanup_files(log_path: &str, prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = PathBuf::from(log_path)
        .parent()
        .ok_or_else(|| format!("path has no parent directory: {}", log_path))?
        .to_path_buf();
    let mut deleted_count = 0;

    for entry in fs::read_dir(&log_dir)? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name.contains(prefix) {
            fs::remove_file(entry.path())?;
            deleted_count += 1;
        }
    }

    if deleted_count > 0 {
        println!("  Cleaned up {} temp files", deleted_count);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog sink fallback mechanism example ===\n");

    multi_sink_config().await?;

    simulate_failure().await?;

    fallback_demo().await?;

    println!("\n=== All examples completed ===");
    Ok(())
}
