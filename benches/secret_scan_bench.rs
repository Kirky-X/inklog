// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! secret-scan 出站脱敏门热路径基准（feature `secret-scan`）。
//!
//! 与 rc4_pipeline_bench 同口径：criterion 基线 + 文档化，不在 CI 设红绿灯；
//! 基线数字由本基准产出后记入 docs/PERFORMANCE.md。
//!
//! 覆盖面：注册表纯扫描、门替换（值形态/含熵）、全 masker 出站路径
//! （门 + fast-masking + 规则集）与无门基线的差值即出站门净成本。

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use inklog::support::processing::{
    DataMasker, EntropyScanner, SecretPatternRegistry, SecretScanGate,
};

/// 典型出站日志行：含一个裸 sk- 密钥、一个邮箱与常规正文。
const HOT_LINE: &str = "login ok user=u-123 email=ops@example.com \
                        token=sk-proj-abcdefghijklmnopqrstuvwxyz123456 request_done";

/// 干净出站日志行（无任何裸 secret/PII）：生产日志的主导场景，
/// 门的每行固定开销由本行与命中行的差值呈现。
const CLEAN_LINE: &str = "login ok user=u-123 session=550e8400e29b \
                          request_done handler=auth.login latency_ms=42";

/// token 密集行：熵扫描用（无前缀裸高熵串 + 常规正文）。
const ENTROPY_LINE: &str = "payload blob=aB3xK9mP2qR7sT5uV1wX8yZ4nC6jM0xY7z \
                            session=550e8400e29b41d4a716446655440000 done";

fn bench_secret_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("secret_scan");
    group.throughput(Throughput::Bytes(HOT_LINE.len() as u64));

    let registry = SecretPatternRegistry::with_builtins();

    group.bench_function("registry_scan_hit", |b| {
        b.iter(|| registry.scan(black_box(HOT_LINE)))
    });

    group.bench_function("registry_scan_clean", |b| {
        b.iter(|| registry.scan(black_box(CLEAN_LINE)))
    });

    let gate = SecretScanGate::new(SecretPatternRegistry::with_builtins());
    group.bench_function("gate_mask_value_shapes", |b| {
        b.iter(|| gate.mask(black_box(HOT_LINE)))
    });

    // 干净行：无命中路径的门固定成本（分配优化后应接近零中间分配）
    group.bench_function("gate_mask_clean", |b| {
        b.iter(|| gate.mask(black_box(CLEAN_LINE)))
    });

    let gate_entropy = SecretScanGate::new(SecretPatternRegistry::with_builtins())
        .with_entropy(EntropyScanner::new());
    group.bench_function("gate_mask_with_entropy", |b| {
        b.iter(|| gate_entropy.mask(black_box(HOT_LINE)))
    });

    group.bench_function("entropy_scan_only", |b| {
        let scanner = EntropyScanner::new();
        b.iter(|| scanner.scan(black_box(ENTROPY_LINE)))
    });

    // 全出站路径：门 + 内置规则集（+ fast-masking AC 路径，若启用）
    let masker = DataMasker::builder()
        .with_secret_scan(SecretScanGate::new(SecretPatternRegistry::with_builtins()))
        .build();
    group.bench_function("mask_fast_with_gate", |b| {
        b.iter(|| masker.mask(black_box(HOT_LINE)))
    });

    // 无门基线：出站门净成本 = mask_fast_with_gate - 本项
    let baseline = DataMasker::new();
    group.bench_function("mask_baseline_without_gate", |b| {
        b.iter(|| baseline.mask(black_box(HOT_LINE)))
    });

    group.finish();
}

criterion_group!(benches, bench_secret_scan);
criterion_main!(benches);
