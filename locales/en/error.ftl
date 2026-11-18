# Error type prefixes for localized_message()

error-config_error = Configuration error
error-io_error = IO error
error-serialization_error = Serialization error
error-database_error = Database error
error-cache_error = Cache error
error-encryption_error = Encryption error
error-secret_scan_limit = Secret scan limit exceeded
error-secret_scan_oversized_input = Secret scan oversized input
error-shutdown_error = Shutdown error
error-channel_error = Channel error
error-compression_error = Compression error
error-runtime_error = Runtime error
error-http_server_error = HTTP server error
error-unknown_error = Unknown error
query-decompression_limit_exceeded = Decompressed output exceeds the 1 GiB safety limit (possible decompression bomb); query aborted
masking-limit-exceeded = Masking skipped: input exceeds the 1 MiB limit; content fully redacted (not scanned means not provably safe)
file-retention-compliance-hint = encrypt is enabled with retention_days below 180: compliance (MLPS 2.0) requires at least 6 months of log retention; consider raising retention_days and keep_files
error-redaction_pattern_compile_failed = failed to compile sensitive redaction pattern; it is disabled and messages will NOT be redacted for it
log_record-non_finite_field = inklog: field '{ $field }' is non-finite ({ $sentinel }), stored as string sentinel

# Query / decrypt / decompress error detail (production code paths)

query-encrypted-file-needs-key-env = encrypted log file '{ $path }' requires a key env var (use --key-env, conventionally INKLOG_ENCRYPTION_KEY)
query-decrypted-not-utf8 = decrypted log file '{ $path }' is not valid UTF-8: { $err }
query-not-utf8 = log file '{ $path }' is not valid UTF-8: { $err }
query-decompressed-not-utf8 = decompressed log file '{ $path }' is not valid UTF-8: { $err }
query-compression-feature-missing = '{ $ext }' log files require the '{ $feature }' feature
query-decode-failed = { $codec } decode failed for '{ $path }': { $err }
query-encrypted-file-too-small = encrypted file '{ $path }' too small for a header
query-truncated-v1-header = truncated v1 header
query-truncated-legacy-header = truncated legacy header
query-decryption-failed-wrong-key = decryption failed (wrong key?)
query-unsupported-encryption-version = unsupported encryption version { $version } in '{ $path }'
