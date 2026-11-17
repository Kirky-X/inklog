// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 源头敏感字段改写 × sink 层正则掩码两层叠加集成测试
//! （change source-level-sensitive-fields R-source-redaction-003）。
//!
//! 两层语义：源头层（字段名精确匹配 → 占位串，订阅器扇出前）+
//! sink 层（PII 掩码/全局 sanitizer 正则扫 message/fields 兜底）。占位串
//! `<redacted:email>` 不含原始值，不被正则层二次破坏；message 内嵌
//! 的信用卡号仍被正则层替换为 `[CARD_NUM]`。
//!
//! 落盘形态与轮转语义（修正记录，见会话甄别结论）：
//! - NDJSON 由 `FileSinkConfig.output_format` 控制（`global.output_format`
//!   不进入文件管线，是控制台/出站层配置）；JSON 出站同时回落 FileSink
//!   路径（manager.rs `file_uses_channel_buffered`）。
//! - `max_size` 触发的轮转会把已写批次改名归档，活动文件清空后重新累积
//!   ——轮询断言不能只盯活动文件；`compress: false` 让归档保持明文
//!   `.log`，全文 grep 断言（无原始明文）才能覆盖归档段。
//! - 可靠的读取时点是 `shutdown()` 之后（worker drain flush_batch 保证
//!   落盘，comprehensive_validation_test 的核正同款口径）。
//!
//! 进程级注册经 `mark_sensitive_fields` 声明后不清理：集成测试二进制
//! 与其它测试二进制进程隔离，且字段名 `email` 不被本二进制其它用例
//! 用作事件字段（已 grep 验证），无串扰面。

use inklog::config::{FileSinkConfig, GlobalConfig};
use inklog::{InklogConfig, LoggerManager, OutputFormat, mark_sensitive_fields};
use serial_test::serial;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tracing_subscriber::prelude::*;

const AUDIT_KEY_ENV: &str = "INKLOG_AUDIT_KEY";
/// Luhn 有效 Visa 测试卡号（credit_card 规则命中后整段替换为 [CARD_NUM]）。
const TEST_CARD: &str = "4111111111111111";
const RAW_EMAIL: &str = "alice@example.com";
/// 源头层占位串（T001 默认模板 `<redacted:{field}>`）。
const EMAIL_PLACEHOLDER: &str = "<redacted:email>";

#[tokio::test]
#[serial]
async fn test_source_redaction_plus_sink_masking_two_layers() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("twolayer.log");
    let chain_manifest = temp_dir.path().join("twolayer.chain.jsonl");

    // audit chain 需要 INKLOG_AUDIT_KEY（file.rs 同判：缺失时 warn 后静默禁用）
    unsafe {
        std::env::set_var(AUDIT_KEY_ENV, "integration-audit-chain-key");
    }

    // 源头层声明：先 mark 后建 manager（subscriber 构建期冻结快照）
    mark_sensitive_fields(["email"]);

    let config = InklogConfig {
        global: GlobalConfig::default(),
        file_sink: Some(FileSinkConfig {
            enabled: true,
            path: log_file.clone(),
            // NDJSON 落盘：fields 以 JSON 对象序列化，占位串可 grep 核对。
            // 注意挂在 FileSinkConfig 而非 GlobalConfig（后者不进文件管线）。
            output_format: OutputFormat::Json,
            // 小尺寸触发轮转：归档审计链（chain.jsonl）随之写穿 manifest，
            // 断言覆盖活动文件 + 归档段 + 链文件三类落盘产物
            max_size: "1KB".into(),
            keep_files: 3,
            // 归档保持明文 .log（默认 zstd 压缩会使其不可 grep）
            compress: false,
            // sink 层正则掩码（pii_masking；默认 true，此处显式钉住）
            masking_enabled: true,
            audit_chain_enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let (logger, subscriber, filter) = LoggerManager::build_detached(
        config,
        #[cfg(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ))]
        None,
    )
    .await
    .unwrap();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry().with(subscriber).with(filter),
    );

    // email 只经事件字段（源头层域）；message 内嵌卡号（正则层域）
    for i in 0..20 {
        tracing::info!(
            target: "twolayer::pay",
            message = format!("payment attempt {i} with card {TEST_CARD}"),
            email = RAW_EMAIL
        );
    }

    // 轮询等待：至少一次轮转（chain manifest 落盘）且两类标记已落到任一
    // 文件段（活动文件会被轮转清空，标记可能只存在于归档段）
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let all_segments = read_all_segments(temp_dir.path());
        if chain_manifest.exists()
            && all_segments.contains(EMAIL_PLACEHOLDER)
            && all_segments.contains("[CARD_NUM]")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for sink flush/rotation; segments so far: {all_segments}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // shutdown 触发 drain flush_batch 确保余量落盘后再做终局断言
    // （仓库口径：comprehensive_validation_test "shutdown 触发 flush_batch
    // 确保落盘后再读"）
    let _ = logger.shutdown();

    let all_segments = read_all_segments(temp_dir.path());
    let chain = std::fs::read_to_string(&chain_manifest).unwrap_or_default();

    // 源头层验证：落盘 NDJSON 中 email 字段为占位串（字段值原文不出现在
    // 任何段，占位串出现在某段 ⇒ 改写先于落盘发生）
    assert!(
        all_segments.contains(EMAIL_PLACEHOLDER),
        "source-layer placeholder must survive to disk: {all_segments}"
    );

    // 正则层验证：message 内嵌卡号被替换（完整原文不得出现在任何段）
    assert!(
        all_segments.contains("[CARD_NUM]"),
        "sink-layer regex must mask the embedded card number: {all_segments}"
    );

    // 两层共同验证：全部落盘段无原始明文（email 明文 + 卡号完整原文）
    assert!(
        !all_segments.contains(RAW_EMAIL),
        "raw email must never hit disk: {all_segments}"
    );
    assert!(
        !all_segments.contains(TEST_CARD),
        "raw card number must never hit disk: {all_segments}"
    );

    // 链文件全文无原始明文（轮转事件只含归档路径与 hash，不透传记录
    // 内容——断言钉住该性质，防止未来把明文事件写进链）
    assert!(
        !chain.is_empty(),
        "chain manifest must exist after rotation"
    );
    assert!(!chain.contains(RAW_EMAIL), "chain leaked email");
    assert!(!chain.contains(TEST_CARD), "chain leaked card number");
    assert!(
        !chain.contains(EMAIL_PLACEHOLDER),
        "chain must not carry record content, only archive metadata"
    );

    unsafe {
        std::env::remove_var(AUDIT_KEY_ENV);
    }
}

/// 读取临时目录内全部落盘段：活动文件 + 归档（归档命名为
/// `<stem>_<timestamp>.log.N`，扩展名不固定，逐文件全读）。不可 UTF-8
/// 解码的文件（如误开压缩的 .zst 二进制）读取失败自然跳过。返回拼接
/// 全文，供"任一段含标记 / 全段无明文"两类断言复用。
fn read_all_segments(dir: &std::path::Path) -> String {
    let mut all = String::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            all.push_str(&content);
        }
    }
    all
}
