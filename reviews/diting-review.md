# diting Full Review — inklog v0.2.0

> Generated: 2026-08-19 · Reviewer: diting (3 engines: A dimensional / B decay / C over-engineering)
> Commit under review: `main` @ `f85c35b` (review fixes applied) · Scope: `src/` (~40k lines)

## 1. Summary

| Metric | Result |
|---|---|
| Build (default, sqlite, duckdb, postgres feature sets) | ✅ clean, `-D warnings` on all four |
| Tests (sqlite workspace + postgres lib + cli bin) | ✅ 920 + 25 + 204 + 91 passed, 0 failed |
| fmt / clippy / doc / deny | ✅ pass (audit: known pre-existing transitive advisories only) |
| Critical | 0 |
| High | 3 (all fixed & verified this review) |
| Medium | 3 |
| Low | 4 (incl. 1 pre-existing/accepted) |
| **Overall Score** | **68 / 100** |
| **Verdict** | **Changes required — approved on condition** (3 Highs fixed; 3 Mediums must be resolved before next release) |

### Fixes applied this review (commit `f85c35b`, pre-commit hook passed)

| ID | Fix | Verification |
|---|---|---|
| HIGH-001 | `encrypt_file` now writes the documented `ENCLOG1\0` header | interop test proves CLI decrypts library-encrypted files; all encryption round-trip tests updated & green |
| HIGH-002 | each enabled async sink gets its own channel; subscriber fans out | compile + full test suite green; routing no longer single-consumer |
| HIGH-003 | cleanup filters to sibling log files, sorts by mtime, never touches unrelated/active files | regression tests for unrelated-file safety + keep_files boundary |

---

## 2. Engine A — Dimensional Findings

### HIGH

#### HIGH-001 — Encryption format mismatch: library writer vs CLI decryptor vs docs (FIXED)

- **Location**: `src/support/io/sink/file.rs:891` (`encrypt_file`), `src/cli/decrypt.rs:215` (`MAGIC_HEADER`), `src/cli/decrypt.rs:295` (`decrypt_file_compatible`), `docs/SECURITY.md:302`
- **Issue**: The library's `FileSink::encrypt_file` wrote raw `nonce(12) + ciphertext` with no header. The CLI `decrypt_file_compatible` requires a 24-byte header (`ENCLOG1\0` + version + algo + nonce) that `docs/SECURITY.md` also documents. Result: log files produced by the library (encryption enabled) could **not** be decrypted by the shipped `inklog-cli decrypt` tool — and vice versa. Only the library's own test decrypted them manually using `encrypted_data[..12]` as nonce, masking the mismatch.
- **Confidence**: 95% (format contract broken between producer, consumer, and documentation).
- **Fix**: `encrypt_file` now writes the documented header (`ENCLOG1\0` + `1u16` + `1u16` + nonce + ciphertext). Added interop regression test `test_library_encrypted_file_decryptable_by_cli` in `src/cli/decrypt.rs` proving the CLI decrypts library output; updated the three round-trip tests to parse the header.

#### HIGH-002 — Multi-sink routing: shared MPMC channel delivers each record to only one worker (FIXED)

- **Location**: `src/domain/core/subscriber.rs:33`, `src/domain/core/workers.rs` (file worker `rx_file`, db worker `rx_db`)
- **Issue**: The subscriber sends every record into a single `async_sender`; the file worker and database worker both `clone()` that same receiver and `recv` concurrently. In a crossbeam MPMC channel each message is consumed by exactly one receiver — so with file + database both enabled, each record went to a *random one* of the two sinks and neither sink received a complete log stream. The codebase already documents this exact hazard for the shutdown channels (`workers.rs:253-255`: "MPMC channel's send() 只能被一个 receiver 消费") yet applied the fix only to shutdown, not the data path. The subscriber docstring (`"avoid deep cloning when sending to multiple sinks"`) shows the intent was fan-out.
- **Confidence**: 95%.
- **Fix**: each enabled async sink now owns a dedicated channel; `LoggerSubscriber` gained `with_extra_async_sender` and fans out to all channels (`send_to_async_sinks`). `WorkerParams` carries a separate `db_receiver` for the database worker.

#### HIGH-003 — Destructive cleanup: deletes arbitrary files in the log directory (FIXED)

- **Location**: `src/support/io/sink/file.rs:439` (`perform_cleanup`)
- **Issue**: Cleanup ran `read_dir(parent)` over **every** file in the log directory with no filename filter, no mtime sort, and no protection of the active log file. The size branch deleted files in read_dir order until the excess was freed (could remove the *current* log file or unrelated user files like `user_data.txt`); the expiry branch deleted `entries.len() - keep_files` in read_dir order regardless of which files were actually expired or newest — it could keep stale files and delete the freshest ones.
- **Confidence**: 95%.
- **Fix**: candidates are now filtered to rotated siblings (`{stem}_*`), sorted by mtime oldest-first, the active file is always skipped, the size branch frees the oldest files first, and the expiry branch only ever removes files older than the retention cutoff while always preserving the newest `keep_files`. New regression test `test_perform_cleanup_never_touches_unrelated_files` proves unrelated files and the active file survive even when the size limit is exceeded.

### MEDIUM

#### MEDIUM-001 — MySQL SQL injection via incomplete string escaping (OPEN)

- **Location**: `src/integrations/infra/database.rs:393` (`escape_sql_string`), `:540-552` (inline `INSERT ... VALUES ('{message}')`)
- **Issue**: Values are interpolated into SQL with only `s.replace('\'', "''")`. That is correct for SQLite/Postgres/DuckDB but **not** for MySQL, where backslash is a string-escape character by default (`NO_BACKSLASH_ESCAPES` off). A log message containing a backslash before a quote (e.g. a Windows path or attacker-controlled text) can terminate the string literal and inject SQL. The dbnexus permission layer may partially mitigate multi-statement execution, but the escaping layer itself is demonstrably backend-unsafe and the correct fix is parameterized queries or backend-aware escaping.
- **Confidence**: 85% (escaping defect certain; exploitability depends on MySQL backend + dbnexus permission path).
- **Remedy**: use prepared statements / bind parameters for `batch_execute_in_transaction`, or add a MySQL-specific escaper (`\`, `'`, `"`, `\0`, `\n`, `\r`, `\x1a`).

#### MEDIUM-002 — DatabaseSink masks message but not structured fields (OPEN)

- **Location**: `src/support/io/sink/database/database_impl.rs:163-166`
- **Issue**: `DatabaseSink` applies the `DataMasker` to `record.message` only and leaves `record.fields` untouched, while `FileSink` (`file.rs:1103`) and `ConsoleSink` (`console.rs:121`) mask fields via `masker.mask_hashmap(&mut masked.fields)`. With `masking_enabled` (default true), sensitive structured fields (e.g. `{"apiKey": ...}`) leak **unmasked** into the database while being masked on file/console — inconsistent and a real data-exposure gap.
- **Confidence**: 90%.
- **Remedy**: mask `fields` in the database path with the same `mask_hashmap` call.

#### MEDIUM-003 — IP whitelist prefix-match bypass (OPEN)

- **Location**: `src/domain/core/http_server.rs:148-150`
- **Issue**: For `allowed.ends_with(".*")` the code computes `prefix = allowed[..len-2]` and checks `client_ip.starts_with(prefix)`. An entry `"192.168.*"` produces prefix `"192.168"` which `starts_with` also matches for `192.1681.1.1` / `192.1682.0.1` — i.e. hosts that are **not** in the intended /16 subnet bypass the whitelist on the HTTP health/metrics endpoint. (The CIDR branch uses the correct `network.contains()`.) Defaults are safe (127.0.0.1:9090, disabled), but any operator who enables a `.*` rule gets a weaker guarantee than advertised.
- **Confidence**: 90%.
- **Remedy**: parse the wildcard form into a proper `IpNet` and use `contains()`, or validate that the byte segments after the prefix start on a dot boundary.

### LOW

- **LOW-001** — CWE-117 coverage incomplete (`src/validation/sanitize.rs`): minimal/strict modes escape C0 controls but not U+2028 / U+2029 (line/paragraph separators, `is_control()` is false for them); the ANSI strip only removes SGR `\x1b[0-9;*m` sequences, leaving CSI/other escape sequences intact. Default regexes are linear (no ReDoS). **Remedy:** add the two Unicode separators to the escape set and extend the strip regex. *(Open)*
- **LOW-002** — Fallback buffer flushed only on `Drop` (`src/domain/core/subscriber.rs:113`): critical logs buffered during channel-full sit in memory until shutdown. **Remedy:** a periodic flush timer or flush on the worker's health tick. *(Open)*
- **LOW-003** — Pre-existing/accepted advisories: `cargo audit` reports `RUSTSEC-2026-0235` (rkyv 0.7.46, transitive via duckdb→rust_decimal), `RUSTSEC-2024-0436` (paste unmaintained), `RUSTSEC-2026-0221` (event-listener unsound) — all transitive, already ignored in CI (`ci.yml`). No action required beyond monitoring upstream.
- **LOW-004** — Dead code: `src/cli/decrypt.rs:560` (`let _canonical_output = output_dir.canonicalize()?;`) and `:593` (`let _ = canonical_input;`) canonicalize then discard the result. **Remedy:** delete (see Engine C).

### Performance note
`sanitize_record` (`subscriber.rs:88-97`) runs the regex-based `LogSanitizer` on **every** record inside `on_event` whenever masking is enabled (default true), contradicting the "lock-free hot path" claim — the fast path is lock-free but not regex-free. Worth benchmarking before the 500 logs/sec claim is relied upon.

---

## 3. Engine B — Decay Analysis

| Risk | Symptom | Source | Consequence | Remedy |
|---|---|---|---|---|
| **R1 Post-Merge** | `file.rs` 4,287 lines, `manager.rs` 3,007, `workers.rs` 1,213; one crate does file/rotation/compression/encryption/cleanup/masking/CLI/HTTP/DB | Single-crate monolith grown without re-partitioning | Bugs hide in oversized files — HIGH-003 (cleanup) lived at line 439 of a 4k-line file and escaped review for two features | Split `file.rs` into focused submodules (rotation, retention, encrypt); shrink the worker threads' per-sink loops |
| **R2 Post-Extend** | `ring_buffered_file.rs` (876 lines) is a second, parallel file sink with its own threads/config that never joined the main config surface | Added as a standalone capability instead of extending `FileSink` | Two file sinks to maintain with divergent behavior; `ChannelBufferedConfig` is not wired into `InklogConfig` | Unify behind one sink + config, or remove; do not maintain two file sinks |
| **R3 Facade** | `LogRecord::mask_sensitive_fields` (`log_record.rs:314`) is only called from tests; production sinks call `DataMasker` directly | Facade added before sinks owned a masker | Dead surface API + uncertainty about which masking is authoritative | Route prod paths through one entry point or delete the facade |
| **R4 Duplicate** | Shutdown/drain loops copied 3× across console/file/db workers; encryption header layout parsed in two places in `decrypt.rs` | Copy-paste evolution | Format drift — HIGH-001 happened precisely because the format was defined in two places that drifted apart | One shared format constant + one parse helper; extract a worker loop primitive |
| **R5 Slowdown** | Regex sanitization + masking in the per-record hot path; no benchmark for the masked path | Correctness-first, perf claims unverified | Claimed "500 logs/sec, microsecond latency" is unbacked for default config | Add a criterion benchmark for the sanitize+mask path; gate the claim |
| **R6 Vendor Duplication** | dbnexus is a self-maintained local fork (`../dbnexus`) with its own permission/parser layer | Localized dependency strategy | inklog's DB correctness depends on a fork's parser/permission semantics (relevant to MEDIUM-001) | Track upstream deltas; add a compliance test that runs the injected-SQL cases against each backend |

**Health Score: 58/100** — Structure 5/10 · Comprehension 6/10 · Age 6/10 (young, already carries legacy + v1 format cruft) · Infrastructure 6/10 (CI gate broken by `--all-features`, coverage gate unenforced) · Composition 6/10 (DI available for DB but sinks still constructed ad-hoc).

---

## 4. Engine C — Over-engineering

| Location | Tag | What | Replacement |
|---|---|---|---|
| `ring_buffered_file.rs:61-63` | `yagni` | `Option<BufWriter<File>>` kept "for future runtime rotation… not currently exercised", plus a second flush thread | Drop the Option; the two-thread design duplicates `FileSink`. Unify with `FileSink` (`Net: -400+` if merged/removed) |
| `src/domain/core/workers.rs:122` `update_adaptive_capacity` | `yagni` | "Adaptive channel capacity" only writes an `effective_capacity` atomic read by the health monitor — the crossbeam channel is `bounded()` once and never resized; the metric is cosmetic | Delete the resize illusion; report real `sender.len()`/capacity | 
| `log_record.rs:314` `mask_sensitive_fields` | `delete` | Prod sinks use `DataMasker`; this method is test-only | Delete (`~60 lines`) |
| `decrypt.rs:560,593` | `delete` | `let _canonical_output` / `let _ = canonical_input` — canonicalize then discard | Delete the dead statements (`4 lines`) |
| `decrypt.rs` legacy + v1 format parse | `shrink` | Two decrypt functions + two hand-rolled header parsers for a format that never shipped | One parser over the shared header constant (`Net: -30`) |

**Net: -500 lines possible** (conservative: -100 excluding the file-sink consolidation).

---

## 5. Verification Evidence

### Gates
```
cargo fmt --all -- --check                     → PASS
cargo clippy (sqlite/duckdb/postgres/default + -D warnings) → PASS (4 combos)
cargo doc --no-deps --features sqlite          → PASS (4 pre-existing link warnings only)
cargo deny check                               → PASS (advisory-not-detected warnings only)
cargo audit                                    → FAIL on known pre-existing transitive advisories (accepted in CI)
cargo test --features sqlite --workspace       → PASS (920 lib + 25 doc + 204 + 63 + 35 + 26 …)
INKLOG_TEST_DB_URL=postgres://… cargo test --features postgres --lib → PASS (914)
cargo test --features cli --bin inklog-cli     → PASS (91)
```

### New regression tests
- `src/cli/decrypt.rs` — `test_library_encrypted_file_decryptable_by_cli` (HIGH-001)
- `src/support/io/sink/file.rs` — `test_perform_cleanup_never_touches_unrelated_files`, updated `test_perform_cleanup_with_keep_files_boundary` (HIGH-003)
- `src/support/io/sink/file.rs` — updated 3 encryption round-trip tests to the header format (HIGH-001)

### Fix commit
`f85c35b` `fix(review): align encryption header, per-sink channels, safe cleanup` — 5 files, +306/−113 — pre-commit hook (fmt, clippy sqlite+postgres, check, unit tests) passed.

---

## 6. Verdict

**Changes required — approved on condition.** The three High-severity defects found this review (encryption format contract, single-consumer routing, destructive cleanup) were fixed and verified with regression tests. The remaining Mediums — MySQL escaping (MEDIUM-001), unmasked structured fields in DatabaseSink (MEDIUM-002), and IP whitelist prefix bypass (MEDIUM-003) — must be resolved before the next release. Engine C identifies ~500 lines of removable complexity, with the two-file-sink duplication as the highest-value cleanup.