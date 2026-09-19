# Sink health monitor (SinkHealthMonitor) runtime messages

# Fallback & recovery tracing events
metrics-sink_recovering = Sink { $sink } is recovering from { $target }
metrics-sink_recovery_max_retries = Sink { $sink } exceeded max recovery attempts ({ $max_retries })
metrics-recovery_attempt_delay = Attempting recovery, delay { $delay_ms }ms
metrics-sink_failed_fallback_disabled = Sink { $sink } failed but automatic fallback is disabled
metrics-sink_fallback_triggered = Sink { $sink } fell back to { $target }, reason: { $error }
metrics-sink_failure_warning = Sink { $sink } failed { $count } time(s) in a row
metrics-encryption_error_fallback = Encryption key error, falling back to plaintext writes
metrics-sink_recovery_confirmed = Sink { $sink } recovered, switched back to normal mode

# Fallback reason strings (stored in FallbackState/FallbackEvent records)
metrics-fallback_reason_database = Database failure: { $error }
metrics-fallback_reason_disk_full = Insufficient disk space: { $error }
metrics-fallback_reason_file = FileSink failure: { $error }
metrics-fallback_reason_unknown = Unknown failure: { $error }
metrics-fallback_reason_encryption = Encryption key error: { $error }
metrics-fallback_reason_plaintext = Plaintext writes (encryption error): { $error }
metrics-recovery_succeeded = Recovery succeeded
metrics-unknown_error = Unknown error
