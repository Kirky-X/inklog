// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! rc4 轮新增/受影响路径的性能基准（写入 / 序列化 / 加密）。
//!
//! 基线数字见 `docs/PERFORMANCE.md`（本机记录；CI 门禁阈值待机型稳定后启用，
//! 与设计 D4 口径一致：criterion 基线 + 文档化，不在本轮开启 CI 红绿灯）。

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use inklog::support::processing::LogTemplate;
use inklog::LogRecord;

/// 构造一条带字段的典型记录。
fn sample_record(message: &str) -> LogRecord {
    let mut record = LogRecord::new(
        tracing::Level::INFO,
        "app::handlers::auth".to_string(),
        message.to_string(),
    );
    record.fields.insert(
        "user".to_string(),
        serde_json::Value::String("u-123".to_string()),
    );
    record.fields.insert(
        "duration_ms".to_string(),
        serde_json::Value::Number(serde_json::Number::from(42)),
    );
    record
}

// ============================================================================
// 写入路径：模板渲染（text）与脱敏（masking）
// ============================================================================

fn bench_write_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_path");

    let template = LogTemplate::new("{timestamp} [{level}] {target} - {message} {fields}");
    let record = sample_record("user logged in from 10.0.0.1");
    group.throughput(Throughput::Elements(1));
    group.bench_function("template_render_text", |b| {
        b.iter(|| template.render(black_box(&record)))
    });

    // 脱敏路径（DataMasker 正则 + 敏感键替换）——写入主链路的安全成本
    group.bench_function("mask_sensitive_fields", |b| {
        b.iter(|| {
            let mut r = sample_record("login user@example.com password=hunter2000");
            r.mask_sensitive_fields();
            r
        })
    });

    group.finish();
}

// ============================================================================
// 序列化路径：LogRecord JSON / OTLP 批量体
// ============================================================================

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    let record = sample_record("structured export record");
    group.throughput(Throughput::Elements(1));
    group.bench_function("logrecord_to_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&record)).unwrap())
    });

    let batch: Vec<LogRecord> = (0..100)
        .map(|i| sample_record(&format!("batch record {i}")))
        .collect();
    group.throughput(Throughput::Elements(100));
    group.bench_function("logrecord_batch_100_to_json", |b| {
        b.iter(|| {
            for r in &batch {
                black_box(serde_json::to_string(r).unwrap());
            }
        })
    });

    #[cfg(feature = "otlp")]
    group.bench_function("otlp_body_100", |b| {
        b.iter(|| {
            inklog::support::io::sink::otlp::encode_otlp_body(&batch, "bench-svc")
        })
    });

    group.finish();
}

// ============================================================================
// 加密路径：PBKDF2 派生与 AES-256-GCM roundtrip
// ============================================================================

fn bench_encryption(c: &mut Criterion) {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit};

    let mut group = c.benchmark_group("encryption");

    // PBKDF2-HMAC-SHA256 600k 迭代（每次轮转文件一次，非每条记录）
    group.sample_size(10);
    group.bench_function("pbkdf2_derive_600k", |b| {
        b.iter(|| {
            inklog::sink::encryption::derive_key_from_password(
                black_box("bench-password-abc-12345"),
                black_box(Some(&[0u8; 16])),
            )
            .unwrap()
        })
    });

    // AES-256-GCM 加解密 roundtrip（1KB 记录批次）
    group.sample_size(100);
    group.throughput(Throughput::Bytes(1024));
    let key = [0x42u8; 32];
    let cipher = Aes256Gcm::new((&key).into());
    let nonce = aes_gcm::Nonce::from([7u8; 12]);
    let payload = vec![0xA5u8; 1024];
    group.bench_function("aes256gcm_roundtrip_1kb", |b| {
        b.iter(|| {
            let ciphertext = cipher.encrypt(&nonce, black_box(payload.as_slice())).unwrap();
            cipher.decrypt(&nonce, ciphertext.as_slice()).unwrap()
        })
    });

    group.finish();
}

criterion_group!(benches, bench_write_path, bench_serialization, bench_encryption);
criterion_main!(benches);
