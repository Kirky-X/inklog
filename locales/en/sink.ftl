# Worker and sink internal tracing messages

# File sink worker messages
sink-file_recovery_received = File sink: Received recovery command
sink-file_recovered = File sink: Successfully recovered
sink-file_recovery_failed = File sink: Recovery failed
sink-file_auto_recovery = File sink: Triggering auto-recovery due to consecutive failures
sink-file_auto_recovery_ok = File sink: Auto-recovery successful

# Database sink worker messages
sink-db_recovery_received = Database sink: Received recovery command
sink-db_recovered = Database sink: Successfully recovered
sink-db_recovery_failed = Database sink: Recovery failed
sink-db_auto_recovery = Database sink: Triggering auto-recovery due to consecutive failures
sink-db_auto_recovery_ok = Database sink: Auto-recovery successful

# Health check messages
sink-health_unhealthy = Health Check: Sink '{ $name }' is unhealthy. Last error: { $error }
sink-health_attempting_recovery = Health Check: Attempting recovery for sink '{ $name }'
sink-health_send_failed = Health Check: Failed to send recovery command for '{ $name }': { $err }

# File sink operation messages
sink-file_reject_path = Rejecting unsafe log path { $path }: { $reason }
sink-file_mkdir_failed = Failed to create log directory { $dir }: { $err }
sink-file_cleanup_panic = cleanup_timer thread panicked: { $msg }
sink-file_rotation_panic = rotation_timer thread panicked: { $msg }

# Database adapter messages
db-pool_create_failed = Failed to create connection pool: { $err }
db-session_failed = Failed to get session: { $err }
db-batch_insert_failed = Batch insert failed: { $err }
db-table_empty = Table name must not be empty
db-table_invalid_start = Invalid table name '{ $name }': must start with a letter or underscore
db-table_invalid_char = Invalid table name '{ $name }': contains forbidden character '{ $char }'
db-ensure_table_failed = Failed to ensure table exists: { $err }
db-duckdb_ddl_not_create_table = DuckDB DDL channel only allows CREATE TABLE templates, got: { $sql }

# Cache adapter messages
cache-get_failed = Failed to get cache key '{ $key }': { $err }
cache-set_failed = Failed to set cache key '{ $key }': { $err }
cache-delete_failed = Failed to delete cache key '{ $key }': { $err }
cache-check_failed = Failed to check existence of cache key '{ $key }': { $err }
cache-build_failed = Failed to build oxcache: { $err }
cache-capacity_zero = OxCacheAdapterBuilder: capacity must be > 0

# Config validation warnings
warn-db_batch_size_zero = database_sink.batch_size is 0, resetting to default 100
warn-db_flush_interval_zero = database_sink.flush_interval_ms is 0, resetting to default 500
warn-db_compression_level_clamp = parquet_config.compression_level out of range 1-22, clamping to 3
warn-fallback_retries_zero = fallback_max_retries is 0, resetting to 1
warn-rate_limit_zero = rate_limit = 0 is invalid, resetting to None (unlimited)
warn-threshold_reset = shrink_threshold >= expand_threshold, resetting to defaults
warn-delay_clamp = fallback_initial_delay_ms > fallback_max_delay_ms, clamping
warn-weak_password = Weak encryption password (< 16 chars). Consider using a longer passphrase or a random 32-byte key
warn-db_health_check_failed = Database health check failed: { $err }
warn-fallback_write_failed = Fallback sink write failed: { $err }
info-db_shutdown_complete = Database sink shutdown complete
warn-cache_ttl_zero = OxCacheAdapterBuilder: TTL is zero, using default TTL

# Subscriber messages
subscriber-drop-fallback-pending = LoggerSubscriber dropped with { $count } unflushed fallback records

# File sink batch write messages
sink-batch_flush_failed = Batch flush error: { $err }
sink-fsync_failed = fsync after batch write failed: { $err }
sink-rotate_postprocess_failed = Failed to post-process rotated log: { $err }
sink-rotate_postprocess_panicked = Rotation post-processing thread panicked: { $msg }
sink-audit_manifest_write_failed = Failed to write audit chain manifest { $path }: { $err }
sink-plaintext_residue = plaintext residue: remove after encryption failed

# Fallback journal messages
journal-replay-skipped-corrupt = fallback journal replay skipped corrupt lines
journal-replay-no-durable-sink = fallback journal replay skipped: no durable sink configured
journal-push-spawn-failed = journal push worker spawn failed: { $err }
journal-push-empty-addr = empty addr
journal-undecryptable-preserved = fallback journal: undecryptable segments present; file preserved
journal-plaintext-append-refused = fallback journal: plaintext instance refused to append to an encrypted journal (config rollback)
journal-encrypted-spill-failed = fallback journal: encrypted spill failed
journal-aesgcm-encrypt-failed = fallback journal: AES-GCM encryption failed
journal-unsupported-format = fallback journal: unsupported encrypted format, nothing replayed
journal-push-failed-kept = fallback journal push failed; record kept on disk

# Audit chain messages
audit-chain-key-missing = audit_chain_enabled but INKLOG_AUDIT_KEY is not set; audit chain disabled

# Secret scan / masking internal tracing messages
secret-entropy-threshold-not-finite = EntropyScanner: threshold_bits must be finite; NaN/infinity would silently disable entropy detection
secret-scan-oversized-rejected = oversized input rejected by the fail-closed secret scan exit
secret-scan-limit-withheld = secret scan limit exceeded; output withheld (fail-closed)
secret-pattern-scan-match-error = regex match error during secret pattern scan; skipping this match
masking-detect-match-error = regex match error during detect; skipping this match
masking-from-rules-duplicate-names = from_rules: duplicate rule names make detect attribution ambiguous

# Config validation warnings
warn-http_feature_not_compiled = http_server.enabled = true but the 'http' feature is not compiled in; the monitoring server will not start

# Channel-buffered file sink maintenance (cleanup thread & rotation)
sink-channel_buffered_cleanup_failed = ChannelBufferedFileSink cleanup failed: { $err }
sink-channel_buffered_cleanup_panic = Cleanup thread panicked: { $msg }
sink-rotate_rename_copy_both_failed = Rotation rename and copy both failed for { $path }

# Fallback flush maintenance
sink-fallback_flush_ticker_spawn_failed = failed to spawn fallback flush ticker; ERROR/FATAL fallback only drains on exit

# File sink worker & operation failure messages
sink-file_open_failed = Failed to open log file: { $err }
sink-file-rename_failed = Failed to rename log file: { $err }
sink-cleanup_failed = Cleanup failed: { $err }
sink-cleanup_dir_read_failed = Failed to read directory '{ $dir }' for cleanup: { $err }
sink-size_cleanup_remove_failed = Failed to remove { $path } during size cleanup: { $err }
sink-expiry_cleanup_remove_failed = Failed to remove { $path } during expiry cleanup: { $err }
sink-record_lost = Log record lost: { $reason }
sink-batch_write_failed = Batch write error: { $err }
sink-batch_write_requeued = Batch write failed: { $unsent } unsent records re-queued for retry
sink-rotate_copy_then_remove_failed = Failed to remove original file after copy during rotation: { $err }
sink-encrypt_read_input_failed = Failed to read file for encryption: { $err }
sink-encryption_failed = Encryption failed: { $err }
sink-encrypted_file_create_failed = Failed to create encrypted file: { $err }
sink-plaintext_residue_remove_failed = Failed to remove plaintext source after encryption { $path }: { $err }
sink-original_after_compression_remove_failed = Failed to remove original file after compression { $path }: { $err }
sink-original_after_encryption_remove_failed = Failed to remove original file after encryption { $path }: { $err }
sink-no_compression_backend = Compression requested but no compression backend feature is enabled (enable "gzip" or "compression"); leaving the file uncompressed
sink-low_disk_space_check = Low disk space - checking before write
sink-rotation_failed = Rotation failed: { $err }
sink-rotation_timer_shutdown_timeout = Warning: rotation timer shutdown timeout after { $timeout_ms }ms
sink-cleanup_timer_shutdown_timeout = Warning: cleanup timer shutdown timeout after { $timeout_ms }ms

# Channel-buffered file sink write/flush failure messages
sink-channel_write_failed = ChannelBufferedFileSink: Write error: { $err }
sink-channel_flush_failed = ChannelBufferedFileSink: Flush error: { $err }
sink-channel_drain_write_failed = ChannelBufferedFileSink: Write error during drain: { $err }
sink-channel_final_flush_failed = ChannelBufferedFileSink: Final flush error: { $err }
sink-channel_periodic_flush_failed = ChannelBufferedFileSink: Periodic flush error: { $err }
sink-channel_reopen_after_rotation_failed = ChannelBufferedFileSink: reopen after rotation failed: { $err }

# Database sink operational messages
sink-database_buffer_overflow = database sink buffer overflow, dropped oldest buffered records
sink-database_line_num_clamp = line_num exceeds i32 range, clamping
db-batch_insert_warn_failed = Database batch insert failed

# Assembly invariants and worker health (panic/health surface)
sink-db_sender = db sender
sink-db_sender_present_when_db_enabled = db sender present when db_enabled
sink-worker_handles_lock_poisoned = Worker handles lock poisoned: { $err }
manager-level_state_mutex_poisoned = level_state mutex poisoned
sink-high_error_count = CRITICAL: Sink '{ $name }' has high error count ({ $failures })

# Masking / secret-scan engine warnings and regex compile invariants
masking-duplicate_rule_name = duplicate masking rule name in TOML config; keeping the first definition and skipping the later one
masking-cjk_name_shape_invariant = Invalid CJK name shape regex
masking-email_invariant = Invalid email regex
masking-phone_invariant = Invalid phone regex
masking-id_card_invariant = Invalid ID card regex
masking-bank_card_invariant = Invalid bank card regex
masking-api_key_invariant = Invalid API key regex
masking-aws_key_invariant = Invalid AWS key regex
masking-jwt_invariant = Invalid JWT regex
masking-generic_secret_invariant = Invalid generic secret regex
masking-international_phone_invariant = Invalid international phone regex
masking-credit_card_invariant = Invalid credit card regex
masking-ipv4_invariant = Invalid IPv4 regex
masking-ipv6_invariant = Invalid IPv6 regex
masking-mac_address_invariant = Invalid MAC address regex
masking-passport_invariant = Invalid passport regex
masking-ssn_invariant = Invalid SSN regex
masking-db_connection_invariant = Invalid DB connection regex
masking-github_token_invariant = Invalid GitHub token regex
masking-slack_token_invariant = Invalid Slack token regex
masking-stripe_key_invariant = Invalid Stripe key regex
masking-google_api_key_invariant = Invalid Google API key regex
masking-private_key_invariant = Invalid private key regex
secret-token_regex_invariant = token regex must compile
secret-builtin_pattern_invariant = builtin secret pattern must compile
secret-extended_pattern_invariant = extended secret pattern must compile
template-unclosed_placeholder = template contains an unclosed placeholder; it is rendered as literal text

# confers audit writer invariants (panic surface) and module build observer
audit-serialize-invariant = AuditEvent serialization must not fail: all variant payloads are serializable basic types
audit-writer-rx-lock-not-poisoned = writer receiver lock is not poisoned (no panic point in the critical section)
audit-writer-rx-present-at-startup = writer receiver must be present on first startup
kit-observer-module_build_failed = inklog build observer: module build failed

# Runtime log messages: sink worker lifecycle and file sink operations
log-sink_factory_failed_on_startup = { $sink } sink factory failed on startup; entering degraded retry mode (exponential backoff: 1s doubling up to 30s, retrying indefinitely); records arriving during retry are forwarded to the error sink and counted as failed
log-sink_factory_retry_failed = { $sink } sink factory retry failed; keeping the worker alive and retrying with exponential backoff
log-sink_factory_recovered = { $sink } sink factory succeeded after retry; worker resumed normal writes
log-sink_factory_registered = Registering sink factory: { $type }
log-sink_recovery_sent = Health Check: Recovery command sent for sink '{ $name }'
log-record_lost_stderr = LOST log record (sink: { $path }, target: { $target }, level: { $level }): { $reason }
log-file_rotating = Rotating log file: { $path }
log-file_rotated_to = Log rotated to: { $path }
log-file_opened = Opened log file: { $path } (size: { $size } bytes)
log-file_encrypted = Encrypted log file: { $path }
log-encryption_raw_key_warning = 32-byte input used directly as a raw encryption key; prefer a Base64-encoded random key or a password whose length is not 32
log-kit-module_ready = InklogModule: on_ready — database dependency verified
log-kit-module_shutdown = InklogModule: on_shutdown — releasing database adapter
log-kit_observer_module_build_started = inklog build observer: module build started
log-kit_observer_module_built = inklog build observer: module built successfully
log-db_batch_insert_succeeded = Database batch insert succeeded
log-db_batch_insert_succeeded_parameterized = Database batch insert succeeded (parameterized)
log-custom_sink_write_failed = custom sink '{ $name }' write failed (attempt { $attempt }/{ $max_attempts }): { $err }
sink-parquet_feature_not_enabled = parquet feature not enabled: rebuild inklog with `parquet` feature to export logs as Parquet
