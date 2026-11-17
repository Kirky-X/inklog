// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 源头敏感字段改写（SensitiveRedaction）热路径基准。
//!
//! 对比命中（3 字段全部替换）与未命中（0 字段跳过）两组吞吐：
//! 未命中路径即"未声明敏感字段的部署"在链头的固定开销上界。
//! 基线数字记录方式与 rc4_pipeline_bench 一致（本机记录，不在本轮
//! 开启 CI 红绿灯）。

use std::collections::HashSet;
use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use inklog::{LogRecord, RecordMiddleware, SensitiveRedaction};

/// 命中组记录：email/phone/api_key 三个字段全部命中注册集。
fn hit_record() -> LogRecord {
    let mut record = LogRecord::new(
        tracing::Level::INFO,
        "app::handlers::auth".to_string(),
        "user login".to_string(),
    );
    for (key, value) in [
        ("email", "alice@example.com"),
        ("phone", "13812345678"),
        ("api_key", "AKIAIOSFODNN7EXAMPLE"),
    ] {
        record.fields.insert(
            key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
    record
}

/// 未命中组记录：session/user/duration 三个字段均不在注册集。
fn miss_record() -> LogRecord {
    let mut record = LogRecord::new(
        tracing::Level::INFO,
        "app::handlers::auth".to_string(),
        "request handled".to_string(),
    );
    for (key, value) in [
        ("session", "s-123"),
        ("user", "u-456"),
        ("duration_ms", "42"),
    ] {
        record.fields.insert(
            key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
    record
}

fn bench_sensitive_redaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("sensitive_redaction");

    let registered: HashSet<String> = ["email", "phone", "api_key"]
        .into_iter()
        .map(String::from)
        .collect();
    let redaction = SensitiveRedaction::new(registered);
    assert_eq!(redaction.field_count(), 3);

    group.throughput(Throughput::Elements(1));
    // 命中 3 字段：3 次占位替换（zeroize 原串 + 重建占位串）
    group.bench_function("hit_3_fields", |b| {
        b.iter(|| {
            let mut record = hit_record();
            redaction.process(black_box(&mut record));
            record
        })
    });

    // 未命中 0 字段：仅集合查找开销（未声明敏感字段的部署成本上界）
    group.bench_function("miss_0_fields", |b| {
        b.iter(|| {
            let mut record = miss_record();
            redaction.process(black_box(&mut record));
            record
        })
    });

    group.finish();
}

criterion_group!(benches, bench_sensitive_redaction);
criterion_main!(benches);
