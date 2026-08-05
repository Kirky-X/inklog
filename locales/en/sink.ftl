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

# Cache adapter messages
cache-get_failed = Failed to get cache key '{ $key }': { $err }
cache-set_failed = Failed to set cache key '{ $key }': { $err }
cache-delete_failed = Failed to delete cache key '{ $key }': { $err }
cache-exists_failed = exists check failed for '{ $key }': { $err }
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
