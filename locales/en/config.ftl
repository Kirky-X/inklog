# Config validation and loading messages

config-load_failed = Failed to load config: { $err }
config-read_failed = Failed to read config file '{ $path }': { $err }
config-parse_failed = Failed to parse config file '{ $path }': { $err }
config-exceeds_max_size = Config file '{ $path }' exceeds maximum size ({ $size } bytes > { $max } bytes)

# Validation errors
config-channel_capacity_zero = channel_capacity cannot be 0
config-worker_threads_zero = worker_threads cannot be 0
config-max_capacity_lt_min = performance.max_capacity ({ $max }) must be >= min_capacity ({ $min })
config-invalid_log_level = Invalid log level '{ $level }'. Valid levels: { $valid }
config-file_path_empty = file_sink.path must not be empty when file sink is enabled
config-file_batch_size_zero = file_sink.batch_size must be > 0
config-http_port_zero = http_server.port must not be 0 when enabled

# Environment override warnings
config-env_invalid_level = Invalid INKLOG_GLOBAL_LEVEL '{ $val }', keeping current value '{ $current }'
config-env_unsafe_path = INKLOG_FILE_SINK_PATH '{ $path }' contains unsafe characters, ignoring
config-env_invalid_size = INKLOG_FILE_SINK_MAX_SIZE '{ $val }' is not a valid size format (e.g., '100MB'), ignoring
config-env_unknown_error_mode = Unknown INKLOG_HTTP_SERVER_ERROR_MODE '{ $val }', keeping current value
config-env_unsafe_http_path = HTTP path '{ $path }' (env: { $env_var }) is not a valid URL path (must start with '/' and contain no traversal), ignoring
config-invalid_stderr_level = Invalid stderr_levels entry '{ $level }'. Valid levels: { $valid }
config-env_invalid_format = INKLOG_GLOBAL_FORMAT is empty, ignoring
config-env_invalid_db_url = INKLOG_DATABASE_SINK_URL contains unsafe characters, ignoring

# Database config parsing errors
config-unknown_db_driver = Unknown database driver '{ $driver }'. Valid drivers: { $valid }
config-unknown_partition_strategy = Unknown partition strategy: '{ $strategy }'. Valid: monthly, yearly
config-unknown_archive_format = Unknown archive format: '{ $format }'. Valid: json, parquet, csv

# Performance config parsing errors
config-unknown_channel_strategy = Unknown channel strategy: '{ $strategy }'. Valid: fixed, adaptive

# Rotation config parsing errors
config-unknown_rotation_interval = Unknown rotation interval: '{ $interval }'. Valid: hourly, daily, weekly, monthly
config-invalid_size_number = Invalid size number: '{ $num }'

# Sink registry errors
config-unknown_sink_type = Unknown sink type: '{ $type }'

# Masking rule errors
config-mask_rule_requires_pattern = Masking rule '{ $name }' requires a pattern
config-invalid_regex_in_rule = Invalid regex in rule '{ $name }': { $err }
config-rule_already_registered = Masking rule '{ $name }' already registered
config-failed_parse_toml = Failed to parse TOML: { $err }
config-toml_missing_masking_rules = TOML missing '[[masking_rules]]' array
config-masking_missing_name = masking_rules entry missing 'name' field
config-masking_missing_pattern = masking_rules entry '{ $name }' missing 'pattern' field

# HTTP config errors
config-http_auth_token_empty = HTTP auth enabled but token env var '{ $env }' is empty
config-http_auth_token_not_set = HTTP auth enabled but token env var '{ $env }' is not set
config-invalid_http_address = Invalid HTTP server address: '{ $addr }': { $err }

# Encryption config errors
config-encryption_key_not_set = Encryption key environment variable '{ $env }' not set
config-encryption_base64_wrong_length = Encryption key from Base64 must be exactly 32 bytes (256 bits), got { $got } bytes
config-encryption_key_wrong_length = Encryption key must be exactly 32 bytes (256 bits) for raw keys, got { $got } bytes
config-encryption_password_too_short = Encryption password must be at least 12 characters, got { $got }

# Manager/builder errors
config-failed_read_config = Failed to read config file: { $err }
config-failed_parse_config = Failed to parse config file: { $err }
config-failed_load_config = Failed to load config: { $err }
config-failed_send_recovery = Failed to send recovery command: { $err }
config-builder_validation_failed = Builder validation failed with { $count } error(s)
config-ring_buffer_dropped = Record dropped by ring buffer (dropped_count: { $count })
