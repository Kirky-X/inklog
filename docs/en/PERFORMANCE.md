# ⚡ inklog Performance Baseline

This document records inklog's first official performance baseline (2026-09-11, criterion medians). The benchmarks cover the write, serialization, and encryption paths; the numbers and the test environment are preserved as measured, and later versions continue the record in this document.

> Related documents: [📖 User Guide](USER_GUIDE.md) · [🏗️ Architecture](ARCHITECTURE.md) · [🧪 Test Scenarios](TEST_SCENARIOS.md)

## 🖥️ Environment and Method

| Item | Value |
| --- | --- |
| Date | 2026-09-11 |
| Machine | WSL2 (linux 6.6.87.2-microsoft-standard-x64), development laptop |
| Tool | criterion 0.8 (`cargo bench --bench rc4_pipeline_bench`) |
| profile | release (`opt-level=3`, `lto=fat`, `codegen-units=1`) |
| Run parameters | `--warm-up-time 1 --measurement-time 2 --sample-size 10` (fast baseline) |

> **CI gate scope**: the CI `benchmark` job is a **manual switch** (`workflow_dispatch` with the `run_bench` checkbox, off by default, not part of the push/PR gates), with a loose ceiling threshold (`write_path/template_render_text` median ≤5 µs, ≈16x this local baseline — it only catches catastrophic regressions such as mistakenly wiring blocking I/O onto the hot path). Precise regression comparison is carried manually by the numbers in this document; machine-to-machine noise varies a lot, and the gate will be tightened to "median regression >10% flagged red" once CI gets fixed runners.

## 📊 Baseline Numbers (2026-09-11, medians)

### Write path (write_path)

| Benchmark | Median | Throughput |
| --- | --- | --- |
| `template_render_text` (default template + 2 fields) | **320 ns** | ~3.12 M rec/s |
| `mask_sensitive_fields` (DataMasker regex + sensitive keys) | **50.5 µs** | ~19.8 K rec/s |
| `logrecord_to_json` (single-record serialization) | **257 ns** | ~3.89 M rec/s |

Key points:

- Template rendering and JSON serialization sit in the same magnitude (~300 ns/record); the default write main path (template rendering) costs **< 0.5 µs** of observable processing per log record;
- Masking (`mask_sensitive_fields`) is the most expensive security step on the main path (regex set + recursive field traversal), costing about **160x** rendering. Enable it on demand at the sink layer (`masking_enabled`) only; do not call it unconditionally on the hot path;
- Encryption and rotation fire once per rotated file (see the encryption path below) and never sit on the per-record write path.

### Serialization path (serialization)

| Benchmark | Median | Throughput |
| --- | --- | --- |
| `logrecord_batch_100_to_json` (batch of 100) | **26.5 µs** | ~3.77 M rec/s |
| `otlp_body_100` (`otlp` feature, 100-record OTLP JSON body) | see the latest local criterion output | — |

The batch-to-single serialization ratio is about 1.06x — no superlinear amplification; OTLP body encoding is sampled separately under the `otlp` feature (the encoder is isomorphic to serde, same magnitude).

### Encryption path (encryption)

| Benchmark | Median | Throughput |
| --- | --- | --- |
| `pbkdf2_derive_600k` (PBKDF2-HMAC-SHA256, 600k iterations) | **63.3 ms** | ~15.8 ops/s |
| `aes256gcm_roundtrip_1kb` (1 KiB encrypt/decrypt round trip) | **506 ns** | ~1.88 GiB/s |

Key points:

- PBKDF2 with 600k iterations costs about 63 ms, **at most once per rotated file** (key derivation), negligible when amortized per record; 600,000 is the OWASP recommendation for PBKDF2-HMAC-SHA256 and the minimum iteration count accepted by security review — never lower it for performance;
- The AES-256-GCM data plane runs at about 1.9 GiB/s (ring backend), keeping the main-path cost of encrypting records under control.

## 📈 Existing Benchmarks (inklog_bench)

`benches/inklog_bench.rs` covers LogRecord creation, console sink latency, channel enqueue, sustained/burst throughput, FileSink throughput, no-op control, template rendering, masking, backpressure, concurrency, object pool, zero-allocation groups and more (parquet conversion is gated by the database feature). Run it with:

```bash
cargo bench --bench inklog_bench
```

The `sampling_stress_overhead` group pins the per-record overhead of sampling decisions on the rate-limit pressure path: `RateLimiter::new(0)` forces every record into the pressure branch, comparing `no_policy_builtin_fallback` (built-in fallback: drop non-critical levels) against `policy_per_level_and_prefix` (a `SamplingPolicy` with per_level rates + target prefix rules) — i.e. the per-record incremental cost of the sampling policy.

The `target_rate_limiter_lookup` group pins the lookup-path overhead of per-target tiered rate limiting: `lookup_longest_prefix_hit` (longest-prefix hit) and `lookup_no_rule_fallthrough` (miss, full-table scan) are the pure prefix-lookup baselines (measured ≈6ns / ≈4ns, far below the 100ns magnitude), and `evaluate_governed_pass` is the end-to-end verdict including group-bucket token deduction (measured ≈41ns, the single-threaded contention-free floor; the group bucket kernel is limiteron's `TokenBucketLimiter` with atomic CAS, so multi-producer scenarios include CAS contention). The 2026-10-01 re-measurement on the same environment (≈8ns / ≈4ns / ≈39ns) appears in the official baseline table below.

### inklog_bench official baseline (2026-10-01, criterion medians)

Same environment, fast scope: `--warm-up-time 1 --measurement-time 3 --sample-size 10` (the `parquet_conversion` group is gated by the database feature and was not enabled this run, so it is not in the baseline; `otlp_body_100` belongs to the rc4 group and was not enabled either). Numbers are the point estimates from criterion `estimates.json` medians.

| Group / benchmark | Median | Notes |
| --- | --- | --- |
| `create_log_record` | 93 ns | LogRecord construction (including thread_id formatting) |
| `template_rendering/render_simple_template` | 242 ns | simple template rendering |
| `template_rendering/render_complex_template` | 268 ns | complex template rendering |
| `masking/mask_email` | 99 ns | single email rule hit |
| `masking/mask_phone` | 852 ns | single phone rule hit |
| `masking/mask_data_masher` | 44.1 µs | DataMasker full rule-set traversal |
| `console_sink_latency/console_sync_latency` | 390 µs | console sink synchronous write latency (including TTY detection and formatting; this machine pipes output instead of a real TTY) |
| `channel_enqueue_latency/async_channel_enqueue` | 2.94 µs | async channel enqueue (bounded crossbeam channel) |
| `file_sink/async_file_log` | 3.01 µs | FileSink async write (channel dispatch path) |
| `noop_sink/async_noop_log` | 3.88 µs | no-op control (upper bound of fixed pipe overhead) |
| `throughput_sustained/sustained_5_logs_per_sec` | 190.8 ms | 100 records written continuously at 5 rec/s (includes fixed waiting, not per-record cost) |
| `throughput_burst/burst_500_logs_per_sec` | 3.27 µs | 500-record burst enqueue (≈6.5 ns/record) |
| `backpressure/backpressure_100_capacity` | 73.0 µs | backpressure triggered on a capacity-100 channel |
| `backpressure/backpressure_burst_10k` | 65.9 ms | 10k-record burst, full pipeline |
| `concurrency/concurrent_4_threads` | 7.25 ms | concurrent writes on 4 threads |
| `concurrency/concurrent_async_tasks` | 1.02 ms | concurrent tokio task writes |
| `memory_usage/steady_state_memory` | 4.83 µs | steady-state memory patrol path |
| `object_pool/pool_get_put_log_record` | 72 ns | object pool LogRecord checkout/check-in |
| `object_pool/pool_get_put_string_buffer` | 3.8 ns | object pool String buffer checkout/check-in |
| `object_pool/pool_hit_rate_after_warmup` | 57 ns | pool hit after warm-up |
| `object_pool/pool_vs_allocation_log_record` | 156 ns | pooled vs direct allocation (LogRecord) |
| `object_pool/pool_vs_allocation_string` | ≈0 (ps-level) | pooled vs direct allocation (String; difference below timing resolution) |
| `zero_allocation/hot_path_with_pool` | 150 ns | hot path (object pool enabled) |
| `zero_allocation/hot_path_without_pool` | 96 ns | hot path (object pool disabled) |
| `zero_allocation/string_pool_reuse` | 4.1 ns | string pool reuse |
| `sampling_stress_overhead/no_policy_builtin_fallback` | 264 ns | pressure-branch built-in fallback decision |
| `sampling_stress_overhead/policy_per_level_and_prefix` | 573 ns | pressure-branch sampling-policy decision (increment ≈309 ns/record) |
| `target_rate_limiter_lookup/lookup_longest_prefix_hit` | 7.6 ns | longest-prefix hit |
| `target_rate_limiter_lookup/lookup_no_rule_fallthrough` | 4.2 ns | miss, full-table scan |
| `target_rate_limiter_lookup/evaluate_governed_pass` | 39 ns | group-bucket verdict end-to-end |

**2026-10-11 re-measurement** (after the selfdev-lib-adoption-r1 changes: the group-bucket kernel switched from a hand-written `Mutex<f64>` bucket to limiteron's `TokenBucketLimiter`; same environment, same scope `--warm-up-time 1 --measurement-time 3 --sample-size 10`):

| Group / benchmark | Re-measured median | Baseline (10-01) | Change |
| --- | --- | --- | --- |
| `target_rate_limiter_lookup/lookup_longest_prefix_hit` | 6.56 ns | 7.6 ns | −14% |
| `target_rate_limiter_lookup/lookup_no_rule_fallthrough` | 4.06 ns | 4.2 ns | −3% |
| `target_rate_limiter_lookup/evaluate_governed_pass` | 41.5 ns | 39 ns | +6.4% |

Group mean 17.4 ns vs baseline 16.9 ns (+2.5%), satisfying the acceptance rule that the `target_rate_limiter_lookup` group mean degrades by ≤10%; the two pure prefix-lookup baselines show no degradation (the allocation-free linear scan of the rule index is unchanged).

Key points:

- Both the sampling policy and the per-target rate-limit lookup are bypass overheads (ns-level) and never stack onto the sink write path;
- Burst throughput ≈6.5 ns/record (`throughput_burst`, amortized over 500 records), and backpressure turns into explicit cost at the capacity boundary (`backpressure_100_capacity` 73 µs);
- Object-pool checkout/check-in is itself ns-level (LogRecord 72 ns / String 3.8 ns); the `zero_allocation` hot-path comparison shows no advantage at this scale with the pool enabled (150 ns vs 96 ns — pool maintenance overhead is visible) — the pool targets larger objects and high allocation frequency; the hot path is not pooled by default.

## 🔍 Comparison Against New 0.3.0 Capabilities

The capabilities added in 0.3.0 (dynamic sink registration, runtime level hot-reload, sampler, token-bucket rate limiting, middleware chain) are all sink-side decorators or bypass logic:

- Decorator costs before a record enters a sink land on the **per-sink thread**, never stacking onto the subscriber hot path;
- `Sampler::should_emit` and `SinkRateLimit::try_acquire` are atomic counters/CAS, lock-free;
- `reload` swapping happens only when `set_level` is called (event-driven), with zero impact on `on_event`.

Conclusion: the write main path (encryption/rotation/batching) does not regress; `write_path/template_render_text` keeps the same template-rendering semantics as before 0.3.0 and can be compared directly.

## 🔁 Reproduction

```bash
# Fast baseline (the scope of this document)
cargo bench --bench rc4_pipeline_bench -- --warm-up-time 1 --measurement-time 2 --sample-size 10
cargo bench --bench inklog_bench -- --warm-up-time 1 --measurement-time 3 --sample-size 10

# Full baseline (default parameters; measured on the same machine on 2026-10-10 ≈11 minutes:
# rc4 ≈1 min + inklog ≈9 min + secret_scan ≈1.3 min)
cargo bench --bench rc4_pipeline_bench
cargo bench --bench inklog_bench
cargo bench --bench secret_scan_bench --features secret-scan
```

> `inklog_bench`'s `console_sink_latency/console_sync_latency` writes about 1.45 million lines to stdout
> (the console sink is the measured path; about 110 MB under default parameters). For a full run, redirect the output:
> `cargo bench --bench inklog_bench > /tmp/inklog_bench.log 2>&1`.

### Benchmark scope correction record (2026-10-10)

The three logger helpers in `inklog_bench` previously used `file_sink: None` / `console_sink: None`
to mean "disabled", but under manager semantics `None` always means **enabled by default** (file:
`is_none_or(|cfg| cfg.enabled)` = true; console: `ConsoleSinkConfig::default().enabled` = true).
Consequences and the fix (`benches/inklog_bench.rs` now sets `enabled: false` explicitly to disable non-measured sinks):

| Item | Before fix | After fix |
| --- | --- | --- |
| `noop_sink/async_noop_log` (single benchmark, default parameters) | 10 min 42 s (measured) | 41.8 s |
| `console_sink_latency/console_sync_latency` (single benchmark, default parameters) | 10 min 42 s (measured) | 24.9 s |
| `cargo bench --bench inklog_bench` full run, default parameters | 28 min 42 s (measured) | 9 min 01 s (measured) |
| `noop_sink/async_noop_log` median | 1.06 µs (includes stdout/file channel components) | 1.40 µs (pure subscription overhead) |
| `console_sink_latency/console_sync_latency` median | 3.51 ms (dominated by file-channel backpressure) | 4.30 µs (console path intrinsic) |

Before the fix, `noop_sink` actually flooded stdout (about 1.45 million lines measured over a full run) and enabled the file channel for the default `logs/app.log` path: the stdout flood blocked the console worker, and every sampling teardown had to drain a huge backlog — the two benchmarks together took 74% of the full run; `setup_file_logger` likewise enabled the console because of `console_sink: None` (each file benchmark emitted an extra million lines). The **measured region of the file/channel benchmarks did not change**, but their historical numbers (`console_sink_latency` 390 µs, `noop_sink` 3.88 µs, etc. in the table above) use the pre-fix scope — take care when comparing across versions.

### rc4_pipeline_bench continuation (2026-10-01, same environment, same scope)

Re-measured on the same machine as the 2026-09-11 baseline (fast scope); all groups are of the same magnitude with no regression:

| Benchmark | 2026-09-11 | 2026-10-01 |
| --- | --- | --- |
| `write_path/template_render_text` | 320 ns | 311 ns |
| `write_path/mask_sensitive_fields` | 50.5 µs | 38.4 µs |
| `serialization/logrecord_to_json` | 257 ns | 259 ns |
| `serialization/logrecord_batch_100_to_json` | 26.5 µs | 26.1 µs |
| `encryption/pbkdf2_derive_600k` | 63.3 ms | 46.4 ms |
| `encryption/aes256gcm_roundtrip_1kb` | 506 ns (≈1.9 GiB/s) | 496 ns (≈1.9 GiB/s) |

(`otlp_body_100` is gated by the `otlp` feature; it was not enabled this run and is not in this table.)

### secret_scan_bench baseline (2026-10-04, criterion medians, `--features secret-scan`)

The outbound masking gate hot path (`benches/secret_scan_bench.rs`, WSL2 x64 / 16 threads, same machine as the other baselines). All 8 medians were produced in a single session; two consecutive full runs showed about ±15% overall drift (machine noise, not a code regression) — treat the reproduction numbers as authoritative for absolute values:

| Benchmark | Median | Notes |
| --- | --- | --- |
| `registry_scan_hit` | 16.2 µs | 9-pattern scan, hitting line (sk- + email body) |
| `registry_scan_clean` | 14.2 µs | same as above, clean line — the scan itself dominates, small gap vs the hit line |
| `gate_mask_value_shapes` | 14.7 µs | gate replacement (9 value-shape patterns) |
| `gate_mask_clean` | 14.2 µs | gate replacement on a clean line (zero intermediate allocations on no hit) |
| `gate_mask_with_entropy` | 14.7 µs | gate replacement + entropy scan |
| `entropy_scan_only` | 544 ns | entropy scan on a token-dense line (fixed count-array fast path; previous run measured 464 ns) |
| `mask_fast_with_gate` | 42.2 µs | full outbound path (gate + fast-masking + rule set) |
| `mask_baseline_without_gate` | 31.2 µs | baseline without the gate; the gate's net cost ≈ the difference to the row above |

Key points: the gate's fixed per-line cost is dominated by 9 fancy-regex per-pattern scans (clean ≈ hit), almost independent of whether the content hits; allocation-side optimizations (zero reassembly on no hit, counting only non-zero hits, fixed array for entropy scanning) show up in allocation counts rather than the usual timing metrics. `registry_scan_hit` once showed a bimodal distribution within a single run (std_dev ≈ 29 µs); for reproduction, prefer the median interval across multiple full runs.
