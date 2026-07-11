// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 集成测试：验证各 lib 模块端到端协同工作。
//!
//! 这些测试与单元测试的区别：单元测试覆盖单个函数，集成测试覆盖多个模块的协同
//! （例如：file_ops 写入文件 → crypto_ops 加密 → crypto_ops 解密 → 验证内容）。

use inklog::support::io::sink::console::ConsoleSink;
use inklog::support::io::sink::file::FileSink;
use inklog::LogTemplate;
use inklog_examples::console_ops::{create_console_config, write_test_cases};
use inklog_examples::crypto_ops::{
    decrypt_file, encrypt_log_file, generate_temp_key, parse_encrypted_format,
};
use inklog_examples::file_ops::{
    cleanup_files, create_file_config, create_log_record, write_level_records,
};
use inklog_examples::perf_ops::{calculate_percentiles, format_throughput};
use inklog_examples::template_ops::{
    create_record_with_fields, create_sample_record, render_formats,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

// 环境变量是进程级状态；集成测试与单元测试可能并行运行，需加锁。
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 端到端：file_ops 创建文件 → 写入多级别 → 读取验证 → 清理。
#[tokio::test]
async fn integration_file_ops_end_to_end() {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let log_path = dir.path().join("integration.log");
    let path_str = log_path.to_str().unwrap();

    // 1. 构造配置 + 创建 sink
    let config = create_file_config(path_str, "10MB", false);
    let sink = FileSink::new(config).expect("创建 FileSink 失败");

    // 2. 写入多级别日志
    let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    let written = write_level_records(&sink, &levels).await.expect("写入失败");
    assert_eq!(written, 5);

    // 3. 验证文件存在且包含所有级别
    assert!(log_path.exists());
    let content = std::fs::read_to_string(&log_path).expect("读取失败");
    for level in &levels {
        assert!(content.contains(level), "应包含 {}", level);
    }

    // 4. 用 cleanup_files 清理（按 prefix）
    let prefix = "integration";
    let deleted = cleanup_files(path_str, prefix).expect("cleanup 失败");
    assert!(deleted >= 1, "至少应删除 1 个文件");
    assert!(!log_path.exists(), "清理后文件应不存在");
}

/// 端到端：file_ops 写入明文 → crypto_ops 加密 → crypto_ops 解密 → 内容一致。
#[tokio::test]
async fn integration_file_then_crypto_roundtrip() {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let plain_path = dir.path().join("plain.log");
    let enc_path = dir.path().join("plain.log.enc");
    let plain_str = plain_path.to_str().unwrap();
    let enc_str = enc_path.to_str().unwrap();

    // 1. 用 file_ops 写入明文日志
    let config = create_file_config(plain_str, "10MB", false);
    let sink = FileSink::new(config).expect("创建 FileSink 失败");
    let levels = ["INFO", "WARN", "ERROR"];
    write_level_records(&sink, &levels).await.expect("写入失败");

    // 2. 用 crypto_ops 加密（持有锁以保护环境变量）
    let _guard = ENV_LOCK.lock().unwrap();
    let key = generate_temp_key();
    std::env::set_var("LOG_ENCRYPTION_KEY", &key);
    encrypt_log_file(plain_str, enc_str, "LOG_ENCRYPTION_KEY").expect("加密失败");

    // 3. 验证加密文件存在且文件头正确
    assert!(enc_path.exists());
    let header = parse_encrypted_format(enc_str).expect("解析失败");
    assert!(header.is_valid_magic());
    assert!(header.is_aes_gcm());

    // 4. 用 crypto_ops 解密
    let decrypted = decrypt_file(enc_str, "LOG_ENCRYPTION_KEY").expect("解密失败");

    // 5. 验证解密内容包含原日志信息（注意：FileSink 写入会附加模板格式）
    for level in &levels {
        assert!(
            decrypted.contains(level),
            "解密内容应包含级别 {}，实际: {}",
            level,
            decrypted
        );
    }

    std::env::remove_var("LOG_ENCRYPTION_KEY");
}

/// 端到端：template_ops 构造记录 → render_formats 渲染 → 验证输出。
#[test]
fn integration_template_render_pipeline() {
    // 1. 构造带字段的记录
    let mut fields = HashMap::new();
    fields.insert(
        "request_id".to_string(),
        Value::String("abc-123".to_string()),
    );
    fields.insert("status".to_string(), Value::Number(200.into()));
    let record = create_record_with_fields(fields);

    // 2. 渲染多种格式
    let results = render_formats(&record);
    assert_eq!(results.len(), 5);

    // 3. 验证每种格式输出非空且不含未替换占位符
    for (name, output) in &results {
        assert!(!output.is_empty(), "格式 [{}] 输出不应为空", name);
        // 检查常见占位符都被替换
        for placeholder in [
            "{level}",
            "{timestamp}",
            "{message}",
            "{target}",
            "{file}",
            "{line}",
        ] {
            assert!(
                !output.contains(placeholder),
                "格式 [{}] 仍含未替换占位符 {}",
                name,
                placeholder
            );
        }
    }

    // 4. 验证简洁格式包含级别 INFO 和消息
    let simple = results
        .iter()
        .find(|(n, _)| n == "简洁格式")
        .map(|(_, v)| v.as_str())
        .unwrap();
    assert!(simple.contains("INFO"));
    assert!(simple.contains("用户登录成功"));
}

/// 端到端：console_ops 写入测试用例 + perf_ops 计算延迟百分位。
#[tokio::test]
async fn integration_console_write_with_perf_stats() {
    // 1. 用 console_ops 写入 5 个用例（捕获延迟）
    let config = create_console_config(false, vec![]);
    let sink = ConsoleSink::new(config, LogTemplate::default());

    let cases = [
        ("INFO", "msg 1"),
        ("INFO", "msg 2"),
        ("INFO", "msg 3"),
        ("INFO", "msg 4"),
        ("INFO", "msg 5"),
    ];

    let mut latencies = Vec::with_capacity(cases.len());
    for (level, msg) in cases {
        let start = std::time::Instant::now();
        let single = [(level, msg)];
        write_test_cases(&sink, &single).await.expect("写入失败");
        latencies.push(start.elapsed());
    }

    // 2. 用 perf_ops 计算百分位
    let (p50, p95, p99) = calculate_percentiles(latencies);
    // 5 个元素，p50 = sorted[2], p95_idx = 4, p99_idx = 4
    assert!(p50 <= p95, "P50 应 <= P95");
    assert!(p95 <= p99, "P95 应 <= P99");

    // 3. 用 perf_ops 格式化吞吐量
    let total = std::time::Instant::now() - std::time::Instant::now() + Duration::from_millis(10);
    let _ = format_throughput(total, 5); // 仅验证不 panic
}

/// 端到端：file_ops 构造单条记录 → template_ops 渲染 → 验证级别正确。
#[test]
fn integration_record_construction_and_template() {
    // 1. 用 file_ops 构造记录
    let record = create_log_record("ERROR", "disk full", "disk_monitor");
    assert_eq!(record.level, "ERROR");
    assert_eq!(record.message, "disk full");

    // 2. 用 template_ops 的 sample record 验证模板渲染（混合调用）
    let sample = create_sample_record();
    let template = LogTemplate::new("[{level}] {message}");
    let rendered = template.render(&sample);
    assert!(rendered.contains("[INFO]"));
    assert!(rendered.contains("用户登录成功"));
}

/// 端到端：crypto_ops 加密 → 修改密文 1 字节 → 解密应失败（GCM 完整性）。
#[test]
fn integration_crypto_tamper_detection() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let plain = dir.path().join("plain.log");
    let enc = dir.path().join("plain.log.enc");

    std::fs::write(&plain, "敏感内容：用户密码 = hunter2").unwrap();
    let key = generate_temp_key();
    std::env::set_var("LOG_ENCRYPTION_KEY", &key);
    encrypt_log_file(
        plain.to_str().unwrap(),
        enc.to_str().unwrap(),
        "LOG_ENCRYPTION_KEY",
    )
    .expect("加密失败");

    // 篡改密文（翻转第 30 字节，位于 ciphertext 区域）
    let mut bytes = std::fs::read(&enc).unwrap();
    assert!(bytes.len() > 30, "加密文件应足够长");
    bytes[30] ^= 0xFF;
    std::fs::write(&enc, &bytes).unwrap();

    // 解密应失败（GCM 认证标签不匹配）
    let result = decrypt_file(enc.to_str().unwrap(), "LOG_ENCRYPTION_KEY");
    assert!(result.is_err(), "篡改密文后解密应失败");

    std::env::remove_var("LOG_ENCRYPTION_KEY");
}
