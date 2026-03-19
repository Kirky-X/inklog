// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Sink 降级机制示例
//!
//! 演示 inklog 的故障检测和自动降级策略：
//!
//! ## 降级策略（三级保障）
//!
//! | 故障场景 | 降级策略 |
//! |----------|----------|
//! | Database 不可用 | 降级到 File Sink |
//! | File 不可用（磁盘满）| 降级到 Console Sink |
//! | Console 不可用 | 最终降级（丢弃日志并告警）|
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
//! 2. **simulate_failure()**: 模拟主 Sink 故障场景
//! 3. **fallback_demo()**: 演示降级到 File Sink

use chrono::Utc;
use inklog::config::{ConsoleSinkConfig, FileSinkConfig};
use inklog::log_record::LogRecord;
use inklog::metrics::{SinkHealth, SinkStatus};
use inklog::sink::console::ConsoleSink;
use inklog::sink::file::FileSink;
use inklog::sink::LogSink;
use inklog::template::LogTemplate;
use inklog_examples::common::{print_section, print_separator, temp_file_path};
use std::fs;
use std::path::PathBuf;

/// 降级状态追踪器
///
/// 用于模拟 Sink 降级过程中的状态变化
#[derive(Debug, Clone)]
struct FallbackState {
    /// 当前激活的 Sink 类型
    active_sink: String,
    /// 备用 Sink 是否就绪
    backup_ready: bool,
    /// 降级事件计数
    fallback_count: u32,
    /// 最后降级时间
    last_fallback_time: Option<chrono::DateTime<Utc>>,
}

impl FallbackState {
    fn new(primary_name: &str) -> Self {
        Self {
            active_sink: primary_name.to_string(),
            backup_ready: true,
            fallback_count: 0,
            last_fallback_time: None,
        }
    }

    fn trigger_fallback(&mut self, new_sink: &str) {
        println!(
            "[{}] 触发降级: {} -> {}",
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
/// 用于演示当主 Sink 发生故障时如何切换到备用 Sink
struct FailingSink {
    inner: Option<Box<dyn LogSink>>,
    fail_after: usize,
    write_count: usize,
    original_name: String,
}

impl FailingSink {
    fn new(sink: Box<dyn LogSink>, fail_after: usize, name: &str) -> Self {
        Self {
            inner: Some(sink),
            fail_after,
            write_count: 0,
            original_name: name.to_string(),
        }
    }
}

impl LogSink for FailingSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), inklog::InklogError> {
        self.write_count += 1;

        // 在达到故障点之前正常工作
        if self.write_count <= self.fail_after {
            if let Some(ref mut sink) = self.inner {
                return sink.write(record);
            }
        }

        // 触发故障
        Err(inklog::InklogError::IoError(std::io::Error::other(format!(
            "{} sink failed after {} writes",
            self.original_name, self.fail_after
        ))))
    }

    fn flush(&mut self) -> Result<(), inklog::InklogError> {
        if let Some(ref mut sink) = self.inner {
            sink.flush()
        } else {
            Err(inklog::InklogError::IoError(std::io::Error::other(
                "Sink is failed",
            )))
        }
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_some()
    }

    fn shutdown(&mut self) -> Result<(), inklog::InklogError> {
        if let Some(ref mut sink) = self.inner {
            sink.shutdown()
        } else {
            Ok(())
        }
    }
}

/// 示例 1: 多 Sink 配置
///
/// 展示如何同时配置 Console 和 File Sink，实现日志的双重输出。
/// 这是在生产环境中推荐的配置方式，确保日志不会因单一 Sink 故障而丢失。
fn multi_sink_config() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("示例 1: 多 Sink 配置（Console + File）");

    // 生成临时文件路径
    let log_path = temp_file_path("multi_sink");
    println!("日志文件路径: {}", log_path);

    // 1. 配置 Console Sink
    let console_config = ConsoleSinkConfig {
        enabled: true,
        colored: true,
        stderr_levels: vec!["error".to_string(), "warn".to_string()],
        masking_enabled: false,
    };
    let mut console_sink = ConsoleSink::new(
        console_config,
        LogTemplate::new("[{level}] {message}"),
    );
    println!("Console Sink: 已配置");

    // 2. 配置 File Sink
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
        retention_days: 7,
        max_total_size: "100MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 10,
        flush_interval_ms: 100,
        masking_enabled: false,
    };
    let mut file_sink = FileSink::new(file_config)?;
    println!("File Sink: 已配置");

    // 3. 创建日志记录
    let records = vec![
        LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "应用启动成功".to_string(),
            target: "fallback::multi_sink".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
        },
        LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: "数据库连接已建立".to_string(),
            target: "fallback::multi_sink".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
        },
        LogRecord {
            timestamp: Utc::now(),
            level: "WARN".to_string(),
            message: "配置参数使用默认值".to_string(),
            target: "fallback::multi_sink".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
        },
    ];

    // 4. 同时写入两个 Sink
    print_section("写入日志到多个 Sink");

    for record in &records {
        // 写入 Console
        console_sink.write(record)?;
        // 写入 File
        file_sink.write(record)?;

        let level_marker = match record.level.as_str() {
            "ERROR" => "[ERROR]",
            "WARN" => "[WARN] ",
            "INFO" => "[INFO] ",
            _ => "[     ]",
        };
        println!("{} -> Console + File: {}", level_marker, record.message);
    }

    console_sink.flush()?;
    file_sink.flush()?;

    // 5. 验证文件内容
    print_section("验证 File Sink");
    let content = fs::read_to_string(&log_path)?;
    println!("文件内容 ({} 字节):\n{}", content.len(), content);

    // 6. 清理
    cleanup_files(&log_path, "inklog_example_multi_sink")?;

    println!("\n✓ 多 Sink 配置完成");
    println!("  - Console Sink: 实时输出到控制台");
    println!("  - File Sink: 持久化到文件");
    println!("  - 两者同时工作，互不影响");

    Ok(())
}

/// 示例 2: 模拟故障与降级切换
///
/// 演示当主 Sink 发生故障时，系统如何检测故障并切换到备用 Sink。
/// 这是 inklog 高可用性的核心机制。
fn simulate_failure() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("示例 2: 故障模拟与降级切换");

    // 生成临时文件路径
    let backup_path = temp_file_path("fallback_backup");
    println!("备用日志文件: {}", backup_path);

    // 1. 初始化状态追踪器
    let mut state = FallbackState::new("Console");
    println!("\n初始状态: 主 Sink = Console");

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
        retention_days: 1,
        max_total_size: "50MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 1,
        flush_interval_ms: 10,
        masking_enabled: false,
    };
    let mut backup_sink = FileSink::new(backup_config)?;
    println!("备用 File Sink: 已就绪");

    // 3. 配置主 Console Sink（包装为会故障的 Sink）
    let primary_config = ConsoleSinkConfig {
        enabled: true,
        colored: true,
        stderr_levels: vec![],
        masking_enabled: false,
    };
    let console_sink = ConsoleSink::new(
        primary_config,
        LogTemplate::new("[{level}] {message}"),
    );

    // 包装为会故障的 Sink（在第 5 次写入后故障）
    let mut primary_sink = FailingSink::new(Box::new(console_sink), 5, "Console");

    // 4. 创建测试日志
    let test_records: Vec<LogRecord> = (1..=8)
        .map(|i| LogRecord {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: format!("测试日志消息 #{}", i),
            target: "fallback::simulate".to_string(),
            fields: Default::default(),
            file: Some(file!().to_string()),
            line: Some(line!()),
            thread_id: "main".to_string(),
        })
        .collect();

    // 5. 写入日志并模拟故障
    print_section("写入日志（触发故障）");

    let mut success_count = 0;
    let mut fallback_triggered = false;

    for record in &test_records {
        // 尝试写入主 Sink
        match primary_sink.write(record) {
            Ok(()) => {
                success_count += 1;
                println!(
                    "[{}] [主 Sink] 写入成功 #{}",
                    Utc::now().format("%H:%M:%S%.3f"),
                    success_count
                );
            }
            Err(e) => {
                // 主 Sink 故障，触发降级
                if !fallback_triggered {
                    println!("\n*** 主 Sink 故障: {} ***", e);
                    state.trigger_fallback("File");
                    fallback_triggered = true;
                }

                // 写入备用 Sink
                println!(
                    "[{}] [备用 Sink] 接管写入",
                    Utc::now().format("%H:%M:%S%.3f")
                );
                backup_sink.write(record)?;
                backup_sink.flush()?;
            }
        }
    }

    // 6. 展示降级统计
    print_section("降级事件统计");
    println!("降级次数: {}", state.fallback_count);
    println!(
        "最后降级时间: {}",
        state
            .last_fallback_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "无".to_string())
    );
    println!("当前活跃 Sink: {}", state.active_sink);
    println!("备用 Sink 就绪: {}", state.backup_ready);

    // 7. 验证备用文件内容
    print_section("验证备用文件");
    let content = fs::read_to_string(&backup_path)?;
    println!("备用文件内容 ({} 字节):", content.len());
    for line in content.lines() {
        println!("  {}", line);
    }

    // 8. 清理
    cleanup_files(&backup_path, "inklog_example_fallback_backup")?;

    println!("\n✓ 故障模拟与降级切换完成");
    println!("  - 主 Sink 在第 {} 次写入后故障", 5);
    println!("  - 备用 Sink 成功接管后续 {} 条日志", 8 - 5);

    Ok(())
}

/// 示例 3: 降级策略完整演示
///
/// 展示完整的降级策略流程，包括：
/// - 健康状态检测
/// - 降级条件判断
/// - 降级执行和日志记录
fn fallback_demo() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("示例 3: 降级策略完整演示");

    // 生成临时文件路径
    let primary_path = temp_file_path("primary");
    let fallback_path = temp_file_path("fallback");
    println!("主日志文件: {}", primary_path);
    println!("备用日志文件: {}", fallback_path);

    // 1. 展示 Sink 健康状态枚举
    print_section("Sink 健康状态类型");

    println!("\nSinkStatus 枚举:");
    println!("  - Healthy: Sink 完全健康");
    println!("  - Degraded: Sink 性能下降，可能需要降级");
    println!("  - Unhealthy: Sink 不可用，需要降级");

    println!("\nSinkHealth 结构:");
    println!("  - is_healthy: bool - 是否健康");
    println!("  - status: SinkStatus - 详细状态");
    println!("  - last_error: Option<String> - 错误信息");

    // 2. 创建模拟的健康检查结果
    let health_checks = vec![
        ("Console Sink", SinkHealth::healthy()),
        ("File Sink", SinkHealth::healthy()),
        (
            "Database Sink",
            SinkHealth::unhealthy("Connection timeout".to_string()),
        ),
    ];

    print_section("模拟健康检查结果");
    for (name, health) in &health_checks {
        let is_healthy = matches!(health.status, SinkStatus::Healthy);
        let status_icon = if is_healthy { "✓" } else { "✗" };
        let status_text = match &health.status {
            SinkStatus::Healthy => "健康",
            SinkStatus::Degraded { .. } => "降级",
            SinkStatus::Unhealthy { .. } => "不健康",
            SinkStatus::NotStarted => "未启动",
        };
        println!(
            "  {} {}: {} ({})",
            status_icon,
            name,
            status_text,
            health.last_error.as_deref().unwrap_or("无错误")
        );

        if !is_healthy {
            println!("     -> 建议: 降级到备用 Sink");
        }
    }

    // 3. 配置主和备用 Sink
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
        retention_days: 1,
        max_total_size: "50MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 10,
        flush_interval_ms: 100,
        masking_enabled: false,
    };
    let mut primary_sink = FileSink::new(primary_config)?;

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
        retention_days: 1,
        max_total_size: "50MB".to_string(),
        cleanup_interval_minutes: 60,
        batch_size: 10,
        flush_interval_ms: 100,
        masking_enabled: false,
    };
    let mut fallback_sink = FileSink::new(fallback_config)?;

    // 4. 模拟降级流程
    print_section("降级流程模拟");

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
                message: format!("日志消息 #{} [{}]", i, level),
                target: "fallback::demo".to_string(),
                fields: Default::default(),
                file: Some(file!().to_string()),
                line: Some(line!()),
                thread_id: "main".to_string(),
            }
        })
        .collect();

    // 模拟在第 6 条日志时主 Sink 故障
    let failure_point = 6;
    let mut current_sink = "primary";

    for (i, record) in records.iter().enumerate() {
        let log_num = i + 1;

        if log_num == failure_point {
            // 模拟故障
            println!(
                "\n[{}] *** 主 Sink 发生故障 ***",
                Utc::now().format("%H:%M:%S%.3f")
            );
            println!(
                "     触发降级: File(primary) -> File(fallback)"
            );
            current_sink = "fallback";
        }

        let sink_name = if current_sink == "primary" {
            "主 Sink"
        } else {
            "备用 Sink"
        };

        if current_sink == "primary" {
            primary_sink.write(record)?;
            primary_sink.flush()?;
        } else {
            // 写入时标记这是降级后的日志
            let fallback_record = LogRecord {
                timestamp: record.timestamp,
                level: record.level.clone(),
                message: format!("[降级接管] {}", record.message),
                target: record.target.clone(),
                fields: record.fields.clone(),
                file: record.file.clone(),
                line: record.line,
                thread_id: record.thread_id.clone(),
            };
            fallback_sink.write(&fallback_record)?;
            fallback_sink.flush()?;
        }

        println!(
            "[{}] [{}] #{}: {}",
            Utc::now().format("%H:%M:%S%.3f"),
            sink_name,
            log_num,
            record.message
        );
    }

    // 5. 展示降级后的文件内容
    print_section("降级后文件内容");

    println!("\n--- 主文件 (前 {} 条日志) ---", failure_point - 1);
    let primary_content = fs::read_to_string(&primary_path)?;
    for line in primary_content.lines().take(10) {
        println!("  {}", line);
    }

    println!("\n--- 备用文件 (接管后的日志) ---");
    let fallback_content = fs::read_to_string(&fallback_path)?;
    for line in fallback_content.lines().take(10) {
        println!("  {}", line);
    }

    // 6. 清理
    cleanup_files(&primary_path, "inklog_example_primary")?;
    cleanup_files(&fallback_path, "inklog_example_fallback")?;

    // 7. 展示降级策略总结
    print_section("降级策略总结");

    println!("\n三级降级保障:");
    println!("  1. Database -> File (数据库不可用时)");
    println!("     - 触发条件: 连接超时、SQL 错误");
    println!("     - 降级动作: 写入本地文件");
    println!();
    println!("  2. File -> Console (文件不可用时)");
    println!("     - 触发条件: 磁盘满、权限错误");
    println!("     - 降级动作: 输出到控制台");
    println!();
    println!("  3. Console (最终降级)");
    println!("     - 触发条件: 所有 Sink 都不可用");
    println!("     - 降级动作: 记录到系统日志 + 告警");

    println!("\n降级事件记录:");
    println!("  - 所有降级事件都会记录到日志");
    println!("  - 包括降级原因、时间和恢复状态");
    println!("  - 可通过 HTTP 端点查询降级状态");

    println!("\n✓ 降级策略完整演示完成");

    Ok(())
}

/// 清理临时文件
///
/// 删除指定路径相关的所有文件
fn cleanup_files(log_path: &str, prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = PathBuf::from(log_path).parent().unwrap().to_path_buf();
    let mut deleted_count = 0;

    for entry in fs::read_dir(&log_dir)? {
        let entry = entry?;
        let file_name = entry.file_name().to_str().unwrap().to_string();

        if file_name.contains(prefix) {
            fs::remove_file(entry.path())?;
            deleted_count += 1;
        }
    }

    if deleted_count > 0 {
        println!("  已清理 {} 个临时文件", deleted_count);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog Sink 降级机制示例 ===\n");

    // 示例 1: 多 Sink 配置
    multi_sink_config()?;

    // 示例 2: 故障模拟与降级切换
    simulate_failure()?;

    // 示例 3: 降级策略完整演示
    fallback_demo()?;

    println!("\n=== 所有示例完成 ===");
    Ok(())
}
