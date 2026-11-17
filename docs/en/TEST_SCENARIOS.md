# 🧪 inklog Test Scenarios

This document captures inklog's test pyramid baseline, test target registry, E2E scenario definitions, test-writing rules, combination matrix, and static gates. All verification uses `cargo test`.

> Related documents: [📖 User Guide](USER_GUIDE.md) · [🏗️ Architecture](ARCHITECTURE.md) · [📊 Performance Baseline](PERFORMANCE.md) · [🤝 Contributing Guide](CONTRIBUTING.md)

## 🗼 Test Pyramid Baseline

| Layer | Carrier | Count baseline |
| --- | --- | --- |
| L1 lib unit tests | `#[cfg(test)]` inside `src/**` | 1435 passed in the lib target (self-tests of manager/workers/sink/config/i18n and other modules); the src/main.rs bin target adds 115 and the examples crate lib target adds 35, all counting as inline src tests |
| L2 integration tests | `tests/**` (4 explicit `[[test]]` registrations + 4 top-level auto-discovered) | integration 103+1i / unit_tests 63 / combinations 20 / performance 11+3i / integration_tests 35+1i / docker 27 / cli_integration 22 |
| L3 E2E scenarios | `tests/e2e/e2e_advanced.rs` (directory-hosted, never bare at the top level) | 228 passed, isolated by mod domain |
| L4 container level | `tests/docker/` (auto-discovered by main.rs) | 27 passed (sqlite embedded path; pg/mysql cases gated to their driver groups) |

Full results for the CI main scope (`--features "sqlite http cli kit compression gzip parquet fast-masking test-utils"`, matching ci.yml): **lib 1435 / cli-bin 115 / examples-bin 35 / unit_tests 63 / integration 103+1i / combinations 20 / integration_tests 35+1i / docker 27 / cli_integration 22 / performance 11+3i / e2e_advanced 228 / examples integration_test 6, 0 failed** (2100 passed / 5 ignored in total; the 6 examples-crate integration_test entries belong to the examples crate and are not counted in the inklog tests/** baseline).

> **Scope note**: the table above lists actual `cargo test` execution counts under the corresponding feature combination; the current total-size scope counted statically by test functions lives in the [📐 Test Suite Size](#-test-suite-size) section at the end of this document (HEAD 4e4233c: 1,625 inline in src + 571 in tests + 41 in examples = 2,237 test functions).

## 🗂️ Test Target Registration and Functional Domain Mapping

Explicit `[[test]]` registrations (tests aggregated in deep directories must be registered explicitly, otherwise cargo has no compile entry point and will not run them):

- **integration** (required-features = sqlite,http,cli,compression,parquet,test-utils): additional/comprehensive (all sink combinations and dynamic configuration) / batch (batch writes) / verification (file_sink: compressed + encrypted archive verification) / log (native log crate + tracing dual path) / config (env overrides) / http (server lifecycle) / cli (inklog-cli binary) / parquet (export) / compression_ratio / recovery (auto recovery) / stability (long_running, manual)
- **combinations** (required-features = sqlite): encryption_file (encrypted file integration + key management) / multi_sink_fallback (SinkHealthMonitor state machine + fallback) / complex_features (encryption + database stacking, 16KB threshold forces size rotation and asserts the archive is v2 ciphertext)
- **performance**: benchmark (single-thread/multi-thread/latency/batch/pooling ×7) / large_volume (1GB compressed rotation) / long_running + resource_monitor (memory/CPU sampling, manual)
- **e2e_advanced**: see the next section

Top-level auto-discovery: **unit_tests** (mod unit: config/sink/cli/archive layered unit tests) / **integration_tests** / **cli_integration** (assert_cmd driving inklog-cli) / **docker** (database_lifecycle/database_sink/dbnexus_adapter).

Directory hosting convention: e2e_* files must live under `tests/e2e/`; when `required-features` is not satisfied cargo silently skips the target (no error, no execution), so the CI feature combination must cover every target's required-features (including `test-utils`).

## 🎬 E2E Scenario Definitions (tests/e2e/e2e_advanced.rs, 228 tests × 21 mod domains)

| Domain | Scenario highlights |
| --- | --- |
| log_level_e2e | FromStr round-trip for all variants/reject invalid input/short_str/Default=info |
| console_sink_e2e | stdout buffered writes/stderr level routing/masking enabled/shutdown health |
| rotation_strategy_e2e | size rotation trigger/no trigger/exact boundaries/size string parsing/next_path; time rotation interval/should_rotate; composite strategy fires on any match |
| sink_registry_e2e | Sink registration/lookup/default sink semantics |
| security_e2e | PathValidator path checks/LogSanitizer sanitization/CircuitBreaker interplay |
| data_masker_e2e | DataMasker phone/email/ID card/custom patterns/boundaries |
| log_record_masking_e2e | LogRecord-level masking pipeline (effective at field level) |
| inklog_error_e2e | InklogError Display for all variants/classification |
| log_template_e2e | LogTemplate rendering/variable substitution/defaults |
| circuit_breaker_e2e | breaker open/half-open/closed state transitions |
| object_pool_e2e | object pool checkout/check-in/exhaustion/recycling |
| config_validation_e2e | Config invalid-value validation matrix |
| sink_health_monitor_e2e | health collection/unhealthy detection/recovery |
| metrics_e2e | Metrics counters/latency recording/sink health |
| file_sink_e2e | FileSink write/rotation/flush persistence + tracing→FileSink, log→FileSink multi-component integration |
| compression_e2e | compression strategies NoCompression/Zstd/Gzip: compress-decompress round trip/file compression/level clamping/extensions (`compression` gated) |
| i18n_e2e | LogI18nFormatter: locale parsing/number·timestamp·level formatting/sort comparison (`i18n` gated) |
| log_adapter_e2e | `log` crate bridge: LogLevel ↔ log crate level mapping |
| multi_component_e2e | multi-component integration: tracing → template rendering/masking → FileSink → on-disk verification |
| custom_sink_e2e | dynamic sink registration: custom in-memory sink plugged in via `add_sink` with zero core changes |
| ops_event_e2e | internal audit event stream: publish_ops_event persisted through the unified sink channel |

## 📌 Test-Writing Rules

The following are real runtime behaviors that test writing and assertions must follow (solidified while repairing the test infrastructure; all are the library's current semantics):

1. **shutdown uses a 2-second timeout send**: workers run on `spawn_blocking`, and their internal writes depend on the caller's runtime driver thread making progress; `shutdown` uses `send_timeout(2s)` on the stop signal and keeps polling worker handles after timeout, so it never hangs. Shutdown assertions in tests must allow the driver thread to participate.
2. **LoggerSubscriber needs a manual Clone impl**: its fields include `AtomicU64` (std does not derive Clone); the test harness installs the subscriber in child threads with thread-local semantics (`tracing::with_default` does not propagate across threads).
3. **runtime-in-runtime avoidance**: the default `DbNexusAdapter` is created in `build_detached` (async context), not in `start_workers` (which may run on a runtime thread where `Handle::current().block_on` panics).
4. **Global subscriber single-install semantics**: `set_global_default` can be installed once per process and never swapped; the unified scope for concurrent multi-case tests is `build_detached` + thread-level `set_default` + `tracing::!` (`additional_tests.rs` is the template).
5. **FileSink weak-key validation**: encryption keys must base64-decode to exactly 32 bytes with Shannon entropy ≥ 4.0 (low-entropy strings such as 32 repeated digits are rejected and no `.enc` file is produced). Tests always use high-entropy keys.
6. **Encryption applies only to rotated archives**: keeping the active log file in plaintext is by design (`rotate_inner` compresses to `.zst` first, then a background task encrypts to `.enc`); the comprehensive assertions are aligned accordingly (the active file contains plaintext, archives do not).
7. **LoggerBuilder deferred validation**: invalid level and other configuration problems uniformly return `ConfigError` at `build()` (the builder returns a `Result`).
8. **Adaptive channel scaling has transients**: after scale-up, waiting for the drain window (`shrink_wait_seconds`) shrinks it again; polling assertions need 20ms-level peak sampling, break as soon as scale-up is observed, and assert `peak_capacity ≥ initial`.
9. **The log macros' entry point is the process-wide global logger** (`LogLogger::install` binds first and cannot be swapped): concurrent tests cannot bind it deterministically, so the log macro path only gets smoke verification, while real persistence assertions go through the tracing path.

## 🧮 Combination Matrix

| Combination | Coverage | Result |
| --- | --- | --- |
| sqlite+http+cli+kit+compression+gzip+parquet+fast-masking+test-utils (CI main scope) | all 12 targets | 2100 passed / 0 failed / 5 ignored |
| performance re-run alone ×2 | stable re-verification of the performance target | 11+3i ×2 all green |
| integration serial (--test-threads=1) | shutdown regression verification | 103+1i all green (re-measured locally 2026-11-16, 225.63s) |
| default features (no db) | lib/e2e_advanced/performance compile | compile + run pass |
| driver exclusivity | sqlite/postgres/mysql/duckdb are mutually exclusive (dbnexus forbids mixing) | `--all-features` not applicable |

## 🚧 Static Gates

| Gate | Command scope | Result |
| --- | --- | --- |
| fmt | `cargo fmt --all -- --check` | clean |
| clippy | `cargo clippy --all-targets --features "sqlite http cli kit compression gzip parquet fast-masking test-utils" -- -D warnings` | zero warnings |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "sqlite http cli kit compression parquet fast-masking"` | zero warnings |
| deny | `cargo deny check` | 4 checks ok (licenses bound to the LICENSE hash via clarify: inklog/oxcache/oxcache_macros/trait-kit/dbnexus) |
| audit | `cargo audit` | rc=0 (548 crates, no hits) |
| MSRV | rust-version = 1.97.1 (workspace-wide, CI dtolnay/rust-toolchain@1.97.1) | consistent |
| coverage | `cargo llvm-cov --features "sqlite http cli kit zstd gzip parquet fast-masking" --lib --fail-under-lines 95` (same scope for CI/pre-push) | 95.79% (recorded line-coverage measurement, re-measured locally 2026-11-16; the 85/90 stages have no separate commits or recorded measurements) |

## 📉 Coverage Scope and Untestable Combinations

The gate scope is `--lib` (inline src tests) + an explicit feature combination; `tests/` integration tests and docker container-level cases (postgres/mysql need container services) run independently and do not count toward the line-coverage gate. The four database backends are mutually exclusive (the dbnexus red line), so there is no `--all-features` scope; compile-time protection for exclusive combinations is provided by the lib.rs `compile_error!` guards. Bare default (no explicit features) is likewise not a valid scope: with `tests/cli_integration.rs` fully disabled by `#![cfg(feature = "cli")]` it becomes a zero-coverage empty target and llvm-cov crashes at the reporting stage — the pre-push coverage gate therefore uses the same explicit-features scope as CI (lefthook.yml).

The following behaviors are untestable or defensively retained under the line-coverage scope and are excluded from coverage targets (as of v0.3.0, measured by llvm-cov):

| Location | Nature |
| --- | --- |
| `ring_buffered_file.rs` background io thread write/flush/final-flush error branches | Requires injecting writer failures at the lower layer; currently no test injection point; error counting and logging are fallback paths |
| `ring_buffered_file.rs` rename+copy double failure and cleanup thread panic capture | Requires a deterministic environment with directory permission errors or thread crashes; double failure is explicit reporting for a persistently failing environment |
| `compression.rs` `remove_file` failure warn after compression | Source and artifact share a directory; the "artifact writable but source deletion fails" scenario cannot be constructed deterministically |
| `object_pool.rs` oxcache cache build/get/set failure mapping | Defensive error mapping; the underlying cache never returns errors on its normal path |
| `query.rs` filesystem changes between the two `expand_paths` calls (TOCTOU guard) | Requires deleting an input file mid-query; a race-defense branch |
| `metrics.rs` degraded-recovery `max_retries` cap branch | The state machine resets the retry counter on every new degradation, so the cap is currently unreachable (incidental finding, see the governance review record) |
| Message formatting arguments of `assert!`/`panic!` in test modules | Lazily evaluated; execute only when the assertion fails |

Constraint statement: this coverage raise deleted no existing tests and relaxed no assertions (the R-inklog-007 red line); all growth came from new tests (error branches, boundary values, Debug/Display diagnostics, async lifecycle forwarding, degradation state machine).

## 📐 Test Suite Size

**Test suite size** (measured at HEAD, counted via `grep -rEc '#\[(tokio::)?test\b' --include='*.rs'`): 1,625 inline in `src/` + 571 in `tests/` + 41 in `examples/` for a total of **2,237 test functions**, plus 48 criterion benchmark functions (33 in `benches/inklog_bench.rs`, 7 in `benches/rc4_pipeline_bench.rs`, 8 in `benches/secret_scan_bench.rs`, counted by `.bench_function(` registrations; 1 of them gated behind the `otlp` feature, and `secret_scan_bench` requires the `secret-scan` feature). The baseline lives in [PERFORMANCE.md](PERFORMANCE.md).
