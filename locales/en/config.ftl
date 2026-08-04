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
