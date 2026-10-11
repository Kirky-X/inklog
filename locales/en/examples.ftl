# Example program output (one key per printed message; see src/i18n for the runtime).
# Arguments use { $name } placeholders filled through inklog::i18n::tr_args.

# config-file
example-config-file-inklog-config-file-loading = === inklog config file loading example ===
example-config-file-all-config-file-loading = ✓ All config file loading examples completed
example-config-file-1-toml-config-file-format = 1. TOML config file format
example-config-file-the-config-file-contains-the = --- The config file contains the following sections ---
example-config-file-global-global-log-level-format = - [global]            Global log level, format, masking, degradation
example-config-file-console-sink-console-output = - [console_sink]      Console output (color, stderr routing)
example-config-file-file-sink-file-output-path = - [file_sink]         File output (path, rotation, compression, encryption)
example-config-file-database-sink-database-output = - [database_sink]     Database output (driver, connection, batching)
example-config-file-performance-performance-tuning = - [performance]       Performance tuning (channel capacity, worker threads)
example-config-file-http-server-http-health-check = - [http_server]       HTTP health check and Prometheus metrics endpoint
example-config-file-2-inklogconfig-sync-parsing = 2. InklogConfig sync parsing (from_str)
example-config-file-2-1-parsing-config-with-toml = 2.1 Parsing config with toml::from_str
example-config-file-global-level = global.level           = { $level }
example-config-file-global-format = global.format          = { $format }
example-config-file-global-masking-enabled = global.masking_enabled = { $masking_enabled }
example-config-file-console-sink-enabled = console_sink.enabled   = { $value }
example-config-file-console-sink-colored = console_sink.colored   = { $value }
example-config-file-2-2-file-sink-fields = 2.2 file_sink fields
example-config-file-file-sink-enabled = file_sink.enabled          = { $enabled }
example-config-file-file-sink-path = file_sink.path             = { $value }
example-config-file-file-sink-max-size = file_sink.max_size         = { $max_size }
example-config-file-file-sink-rotation-time = file_sink.rotation_time    = { $rotation_time }
example-config-file-file-sink-compress = file_sink.compress         = { $compress }
example-config-file-file-sink-compression-level = file_sink.compression_level = { $compression_level }
example-config-file-2-3-database-sink-fields = 2.3 database_sink fields
example-config-file-database-sink-enabled = database_sink.enabled    = { $enabled }
example-config-file-database-sink-driver = database_sink.driver     = { $driver }
example-config-file-database-sink-url = database_sink.url        = { $url }
example-config-file-2-4-performance-fields = 2.4 performance fields
example-config-file-channel-capacity = channel_capacity = { $channel_capacity }
example-config-file-worker-threads = worker_threads   = { $worker_threads }
example-config-file-2-5-http-server-fields = 2.5 http_server fields
example-config-file-http-server-enabled = http_server.enabled = { $enabled }
example-config-file-http-server-host = http_server.host    = { $host }
example-config-file-http-server-port = http_server.port    = { $port }
example-config-file-2-6-validate-check = 2.6 validate() check
example-config-file-config-validation-passed = ✓ Config validation passed
example-config-file-3-from-search-paths-auto-search = 3. from_search_paths auto search (config source of load)
example-config-file-3-1-setting-the-inklog-config = 3.1 Setting the INKLOG_CONFIG_PATH env var
example-config-file-inklog-config-path = INKLOG_CONFIG_PATH = { $value }
example-config-file-3-2-reading-and-parsing-with = 3.2 Reading and parsing with from_search_paths
example-config-file-loaded-from-inklog-config-path = ✓ Loaded from INKLOG_CONFIG_PATH: level = { $level }
example-config-file-inklog-config-path-env-var = INKLOG_CONFIG_PATH env var removed
example-config-file-note-loggermanager-load-calls = Note: LoggerManager::load() calls InklogConfig::load_sync(),
example-config-file-searching-for-the-config-file = searching for the config file in this order (first existing file wins):
example-config-file-1-inklog-config-path = 1. $INKLOG_CONFIG_PATH
example-config-file-2-inklog-config-toml-current = 2. ./inklog_config.toml (current directory)
example-config-file-3-config-inklog-config-toml = 3. ~/.config/inklog/config.toml
example-config-file-4-etc-inklog-config-toml = 4. /etc/inklog/config.toml
example-config-file-if-none-exist-default-default = If none exist, Default::default() is used.
example-config-file-4-initializing-the-logger-with = 4. Initializing the logger with LoggerManager::from_file
example-config-file-temporary-config-file = Temporary config file: { $value }
example-config-file-4-1-loggermanager-from-file-path = 4.1 LoggerManager::from_file(path)
example-config-file-loggermanager-initialized = ✓ LoggerManager initialized
example-config-file-4-2-logging-via-tracing-macros = 4.2 Logging via tracing macros (verifying the logger is active)
example-config-file-info-log-from-the-config-file = INFO log from the config_file example
example-config-file-warn-log-from-the-config-file = WARN log from the config_file example
example-config-file-error-log-from-the-config-file = ERROR log from the config_file example
example-config-file-4-3-from-file-vs-load-comparison = 4.3 from_file vs load comparison
example-config-file-from-file-path-load-the-config = from_file(path) : load the config file from the given path
example-config-file-note-loggermanager-internally = Note: LoggerManager internally creates logs/error.log for system error logs,
example-config-file-and-installs-the-global-tracing = and installs the global tracing subscriber and the log crate adapter.

# config-inspect
example-config-inspect-inklog-config-inspection-example = inklog config inspection example
example-config-inspect-all-config-inspection-examples = All config inspection examples completed.
example-config-inspect-example-1-all-sinks-disabled = Example 1: all sinks disabled → sinks_enabled() returns empty
example-config-inspect-all-sinks-explicitly-disabled = All sinks explicitly disabled:
example-config-inspect-console-sink-enabled = console_sink.enabled   = { $value }
example-config-inspect-file-sink-enabled = file_sink.enabled      = { $value }
example-config-inspect-database-sink-enabled = database_sink.enabled  = { $value }
example-config-inspect-sinks-enabled = sinks_enabled() = { $sinks }
example-config-inspect-returned-vec-length = Returned Vec length = { $value }
example-config-inspect-comparison-console-sink-in = Comparison: console_sink in InklogConfig::default() is enabled by default
example-config-inspect-default-sinks-enabled = default().sinks_enabled() = { $default_sinks }
example-config-inspect-example-2-only-the-console-sink = Example 2: only the console sink enabled
example-config-inspect-config = Config:
example-config-inspect-console-sink-enabled-2 = console_sink.enabled   = { $value } ✓
example-config-inspect-contains-console = contains "console" = { $value }
example-config-inspect-example-3-multiple-sinks = Example 3: multiple sinks enabled (Console + File + Database)
example-config-inspect-file-sink-enabled-path = file_sink.enabled      = { $value } ✓ (path={ $file_path })
example-config-inspect-database-sink-enabled-2 = database_sink.enabled  = { $value } ✓
example-config-inspect-sink-count-expected-3 = Sink count = { $value } (expected 3)
example-config-inspect-verification = Verification:
example-config-inspect-contains-console-2 = contains "console"  = { $has_console } → { $value }
example-config-inspect-contains-file = contains "file"     = { $has_file } → { $value }
example-config-inspect-contains-database = contains "database" = { $has_database } → { $value }
example-config-inspect-example-4-loggermanager-load = Example 4: LoggerManager::load() usage
example-config-inspect-loggermanager-load-flow = LoggerManager::load() flow:
example-config-inspect-1-call-inklogconfig-load-sync = 1. Call InklogConfig::load_sync() to search for the config file
example-config-inspect-search-paths-first-existing = - Search paths (first existing file wins):
example-config-inspect-1-inklog-config-path-explicit = 1. $INKLOG_CONFIG_PATH (explicit env var, highest priority)
example-config-inspect-2-inklog-config-toml-current = 2. ./inklog_config.toml (current directory)
example-config-inspect-2-call-loggermanager-with = 2. Call LoggerManager::with_config() with the loaded config
example-config-inspect-3-initialize-all-enabled-sinks = 3. Initialize all enabled sinks and the HTTP server
example-config-inspect-code-example = Code example:
example-config-inspect-custom-search-paths = Custom search paths:
example-config-inspect-note-load-is-an-async-method = Note: load() is an async method and must be called inside a tokio runtime.
example-config-inspect-example-5-config-inspection = Example 5: config inspection workflow
example-config-inspect-recommended-config-inspection = Recommended config inspection workflow:
example-config-inspect-step-1-load-the-config = Step 1: load the config
example-config-inspect-failed-to-load-config-using-the = ⚠ Failed to load config: { $e }, using the default config
example-config-inspect-config-loaded = ✓ Config loaded
example-config-inspect-step-2-check-enabled-sinks = Step 2: check enabled sinks
example-config-inspect-no-sink-enabled-logs-will-not = ⚠ No sink enabled; logs will not be output
example-config-inspect-enabled-sinks = ✓ Enabled sinks: { $sinks }
example-config-inspect-step-3-verify-config-consistency = Step 3: verify config consistency
example-config-inspect-sink-state-is-consistent = ✓ Sink state is consistent
example-config-inspect-step-4-choose-the = Step 4: choose the initialization method based on the config
example-config-inspect-use-loggermanager-new-default = → Use LoggerManager::new() (default console only)
example-config-inspect-use-loggermanager-with-config = → Use LoggerManager::with_config() (explicit config)
example-config-inspect-full-code-example = Full code example:

# env-overrides
example-env-overrides-inklog-env-var-override-loading = inklog env-var override loading example
example-env-overrides-all-env-var-override-examples = All env-var override examples completed.
example-env-overrides-example-1-supported-environment = Example 1: supported environment variables
example-env-overrides-global-config = Global config:
example-env-overrides-performance-config = Performance config:
example-env-overrides-file-sink-config = File sink config:
example-env-overrides-http-server-config = HTTP server config:
example-env-overrides-database-sink-config = Database sink config:
example-env-overrides-example-2-calling-load-with-env = Example 2: calling load_with_env_overrides() for real
example-env-overrides-calling-inklogconfig-load-with = Calling InklogConfig::load_with_env_overrides()...
example-env-overrides-config-loaded-successfully = ✓ Config loaded successfully
example-env-overrides-loaded-config = Loaded config:
example-env-overrides-global-level = global.level          = "{ $level }"
example-env-overrides-global-format = global.format         = "{ $format }"
example-env-overrides-global-auto-fallback = global.auto_fallback  = { $auto_fallback }
example-env-overrides-performance-channel-capacity = performance.channel_capacity  = { $channel_capacity }
example-env-overrides-performance-worker-threads = performance.worker_threads    = { $worker_threads }
example-env-overrides-performance-channel-strategy = performance.channel_strategy  = { $channel_strategy }
example-env-overrides-file-sink-enabled = file_sink.enabled     = { $value }
example-env-overrides-http-server-enabled = http_server.enabled   = { $value }
example-env-overrides-database-sink-enabled = database_sink.enabled = { $value }
example-env-overrides-failed-to-load-config = ✗ Failed to load config: { $e }
example-env-overrides-this-usually-means-a-config = (This usually means a config file parse error; check the TOML syntax)
example-env-overrides-example-3-env-var-override = Example 3: env-var override effect demo
example-env-overrides-test-id-isolates-concurrent-runs = Test ID: { $test_id } (isolates concurrent runs)
example-env-overrides-baseline-config-no-env-var = Baseline config (no env-var overrides):
example-env-overrides-global-level-2 = global.level = "{ $level }"
example-env-overrides-performance-channel-capacity-2 = performance.channel_capacity = { $channel_capacity }
example-env-overrides-performance-worker-threads-2 = performance.worker_threads = { $worker_threads }
example-env-overrides-setting-environment-variables = Setting environment variables:
example-env-overrides-export = export { $key }={ $value }
example-env-overrides-config-after-overrides = Config after overrides:
example-env-overrides-global-level-expected-debug = global.level = "{ $level }" (expected: "debug")
example-env-overrides-performance-channel-capacity-3 = performance.channel_capacity = { $channel_capacity } (expected: 20000)
example-env-overrides-performance-worker-threads-3 = performance.worker_threads = { $worker_threads } (expected: 4)
example-env-overrides-verification-results = Verification results:
example-env-overrides-global-level-override = global.level override: { $value } → { $value2 }
example-env-overrides-channel-capacity-override = channel_capacity override: { $value } → { $value2 }
example-env-overrides-worker-threads-override = worker_threads override: { $value } → { $value2 }
example-env-overrides-test-environment-variables = (Test environment variables cleaned up)
example-env-overrides-example-4-config-precedence = Example 4: config precedence
example-env-overrides-inklog-config-loading = inklog config loading precedence (low to high):
example-env-overrides-1-code-defaults-default-default = 1. Code defaults (Default::default())
example-env-overrides-2-config-file-inklog-toml = 2. Config file (inklog.toml / INKLOG_CONFIG_PATH)
example-env-overrides-3-env-var-overrides-inklog = 3. Env-var overrides (INKLOG_*)
example-env-overrides-load-with-env-overrides-flow = load_with_env_overrides() flow:
example-env-overrides-1-call-load-sync-to-load = 1. Call load_sync() to load defaults + config file
example-env-overrides-2-call-apply-env-overrides-to = 2. Call apply_env_overrides() to apply INKLOG_* env vars
example-env-overrides-3-return-the-final-config = 3. Return the final config
example-env-overrides-recommendations = Recommendations:
example-env-overrides-ci-cd-use-env-vars-to-switch = - CI/CD: use env vars to switch dev/staging/prod configs
example-env-overrides-code-example = Code example:
example-env-overrides-inklogconfig-load-with-env = InklogConfig::load_with_env_overrides()?

# all-features
example-all-features-inklog-full-feature-demo = === inklog full feature demo ===
example-all-features-all-feature-demos-completed = All feature demos completed.
example-all-features-feature-1-data-masking = --- Feature 1: data masking (DataMasker) ---
example-all-features-built-in-rule-masking = Built-in rule masking:
example-all-features-raw = Raw: { $input }
example-all-features-masked = Masked: { $masked }
example-all-features-email-and-phone-number-masked = ✓ email and phone number masked
example-all-features-custom-rule-masking = Custom rule masking:
example-all-features-raw-2 = Raw: { $order_input }
example-all-features-masked-2 = Masked: { $order_masked }
example-all-features-custom-order-id-rule-works-all = ✓ custom order id rule works (all matches replaced by default)
example-all-features-no-sensitive-data-test = No sensitive data test:
example-all-features-content-unchanged-as-expected = ✓ content unchanged (as expected)
example-all-features-feature-2-log-templates = --- Feature 2: log templates (LogTemplate) ---
example-all-features-default-template-render = Default template render:
example-all-features-output = Output: { $rendered }
example-all-features-render-check-passed = ✓ render check passed
example-all-features-simple-template-render = Simple template render:
example-all-features-output-2 = Output: { $simple_rendered }
example-all-features-exact-match-verified = ✓ exact match verified
example-all-features-multi-format-comparison = Multi-format comparison:
example-all-features-json-style-output-parses-with = ✓ JSON style output parses with serde_json
example-all-features-feature-3-log-level-control = --- Feature 3: log level control ---
example-all-features-log-level-debug = Log level = debug:
example-all-features-trace-not-emitted-below-debug = TRACE - not emitted (below debug)
example-all-features-debug-emitted = DEBUG - emitted
example-all-features-info-emitted = INFO - emitted
example-all-features-trace-filtered-out-debug-and = ✓ trace filtered out; debug and above visible
example-all-features-after-set-level-none-warn-hot = After set_level(None, "warn") hot reload:
example-all-features-info-not-emitted-below-warn = INFO - not emitted (below warn)
example-all-features-warn-emitted = WARN - emitted
example-all-features-error-emitted = ERROR - emitted
example-all-features-info-debug-trace-filtered-out = ✓ info/debug/trace filtered out; warn and above visible
example-all-features-level-restored-to-info-set = ✓ level restored to info (set_level hot reload, logger stays alive)
example-all-features-feature-4-structured-logging = --- Feature 4: structured logging ---
example-all-features-request-log-request-id-method = ✓ request log (request_id, method, path, status, latency_ms)
example-all-features-error-log-error-code-component = ✓ error log (error_code, component, retry_count)
example-all-features-loggermanager-builder-console = LoggerManager::builder().console_json(true) when a whole JSON record is needed
example-all-features-feature-5-path-validation = --- Feature 5: path validation (PathValidator) ---
example-all-features-all-path-validation-results-are = ✓ all path validation results are as expected
example-all-features-feature-6-object-pool-objectpool = --- Feature 6: object pool (ObjectPool) ---
example-all-features-object-pool-get-put-verified = ✓ object pool get/put verified
example-all-features-key1-key2-missing = key1 = { $v1 }, key2 = { $v2 }, missing = { $v3 }
example-all-features-feature-7-multi-module-combined = --- Feature 7: multi-module combined workflow ---
example-all-features-1-create-a-datamasker = 1. Create a DataMasker
example-all-features-2-create-a-logtemplate = 2. Create a LogTemplate
example-all-features-3-raw-log = 3. Raw log: { $raw_message }
example-all-features-4-masked = 4. Masked:   { $safe_message }
example-all-features-5-template-render = 5. Template render: { $rendered }
example-all-features-masking-template-logging = ✓ masking → template → logging workflow verified end to end
example-all-features-best-practices = --- Best practices ---
example-all-features-1-feature-combination-principles = 1. Feature combination principles:
example-all-features-call-datamasker-before-writing = - Call DataMasker before writing a log (mask → record)
example-all-features-logtemplate-unifies-the-log = - LogTemplate unifies the log format; use one template globally
example-all-features-pathvalidator-validates-file = - PathValidator validates file paths from user input
example-all-features-2-masking-rule-design = 2. Masking rule design:
example-all-features-prefer-built-in-rules-email = - Prefer built-in rules (email, phone, password, and so on)
example-all-features-give-custom-rules-meaningful = - Give custom rules meaningful names for maintainability
example-all-features-review-masking-rules-regularly = - Review masking rules regularly to cover new sensitive fields
example-all-features-3-log-level-strategy = 3. Log level strategy:
example-all-features-trace-development-only-disabled = - trace: development only, disabled in production
example-all-features-debug-development-test-enable = - debug: development/test; enable temporarily in production when needed
example-all-features-info-production-default-records = - info: production default, records key business events
example-all-features-warn-recoverable-exceptions = - warn: recoverable exceptions; needs attention but not immediate action
example-all-features-error-functionality-failure = - error: functionality failure that needs human intervention
example-all-features-4-structured-field-conventions = 4. Structured field conventions:
example-all-features-use-snake-case-names-user-id = - Use snake_case names (user_id, request_id)
example-all-features-pass-numeric-values-directly = - Pass numeric values directly (status = 200, not a string)
example-all-features-always-include-component-or = - Always include component or target to identify the source
example-all-features-error-logs-must-include-error = - Error logs must include error_code

# basic
example-basic-inklog-basic-usage-examples = === inklog basic usage examples ===
example-basic-all-basic-examples-completed = All basic examples completed.
example-basic-example-1-default-initialization = --- Example 1: default initialization ---
example-basic-loggermanager-new-initialized = ✓ LoggerManager::new() initialized (logger is created in main)
example-basic-log-system-started = log system started
example-basic-logging-works = ✓ logging works
example-basic-example-2-log-levels = --- Example 2: log levels ---
example-basic-5-standard-log-levels = 5 standard log levels:
example-basic-trace-most-detailed-diagnostics = TRACE - most detailed diagnostics; development only
example-basic-debug-debugging-information-to = DEBUG - debugging information to help locate problems
example-basic-info-routine-runtime = INFO - routine runtime information marking important events
example-basic-warn-potential-issue-that-does = WARN - potential issue that does not affect normal operation
example-basic-error-functionality-failure = ERROR - functionality failure that needs attention
example-basic-level-filtering-rules-default = Level filtering rules (default info):
example-basic-trace-only-tracing-trace-visible = trace → only tracing::trace! visible
example-basic-debug-debug-and-above-visible = debug → debug and above visible
example-basic-info-info-warn-error-visible = info  → info/warn/error visible (default)
example-basic-warn-warn-error-visible = warn  → warn/error visible
example-basic-error-only-error-visible = error → only error visible
example-basic-example-3-structured-logging = --- Example 3: structured logging ---
example-basic-user-operation-logged-user-id = ✓ user operation logged (user_id, action, ip)
example-basic-request-logged-request-id = ✓ request logged (request_id, method, path, status, latency_ms)
example-basic-error-logged-error-code = ✓ error logged (error_code, component, retry_count)
example-basic-example-4-health-status-check = --- Example 4: health status check ---
example-basic-note-health-status-must-be = Note: health status must be checked on a LoggerManager instance.
example-basic-this-example-shows-the-api = This example shows the API usage (it needs to hold a logger reference):
example-basic-example-5-custom-configuration = --- Example 5: custom configuration initialization ---
example-basic-approach-1-modify-the-default = Approach 1: modify the default config
example-basic-approach-2-builder-pattern = Approach 2: Builder pattern
example-basic-example-6-graceful-shutdown = --- Example 6: graceful shutdown ---
example-basic-application-started = application started
example-basic-processing-business-logic = processing business logic...
example-basic-loggermanager-shutdown-completed = ✓ LoggerManager::shutdown() completed
example-basic-all-logs-left-in-the-channel = - All logs left in the channel have been written to sinks
example-basic-all-sinks-are-closed = - All sinks are closed
example-basic-worker-threads-stopped = - Worker threads stopped
example-basic-best-practices = --- Best practices ---
example-basic-1-where-to-initialize = 1. Where to initialize:
example-basic-initialize-loggermanager-as = - Initialize LoggerManager as early as possible in main()
example-basic-bind-the-variable-with-let = - Bind the variable with `let _logger = ...` so Drop does not close it early
example-basic-2-choosing-the-log-level = 2. Choosing the log level:
example-basic-development-debug-or-trace = - Development: debug or trace
example-basic-production-info-default-or-warn = - Production: info (default) or warn
example-basic-troubleshooting-temporarily = - Troubleshooting: temporarily raise to debug, then restore
example-basic-3-structured-fields = 3. Structured fields:
example-basic-use-tracing-structured-fields = - Use tracing structured fields (key = value)
example-basic-avoid-concatenating-json = - Avoid concatenating JSON strings into the message
example-basic-common-fields-request-id-user = - Common fields: request_id, user_id, component, latency_ms
example-basic-4-error-logs = 4. Error logs:
example-basic-error-only-for-failures-that = - error! only for failures that need human intervention
example-basic-warn-for-exceptions-that = - warn!  for exceptions that recover automatically
example-basic-include-error-code-and = - Include error_code and component for easier triage
example-basic-5-shutdown-sequence = 5. Shutdown sequence:
example-basic-call-logger-shutdown-before-the = - Call logger.shutdown() before the application exits
example-basic-call-it-from-signal-handling = - Call it from signal handling (SIGTERM / Ctrl+C)
example-basic-shutdown-waits-until-all-logs = - shutdown() waits until all logs in the channel are written

# builder
example-builder-inklog-builder-pattern = === inklog Builder pattern configuration examples ===
example-builder-all-configuration-examples-have = All configuration examples have been shown.
example-builder-example-1-basic-builder-usage = Example 1: basic Builder usage
example-builder-code-example =
    Code example:
    { $code }
example-builder-configuration-notes = Configuration notes:
example-builder-log-level-debug = ✓ Log level: DEBUG
example-builder-console-sink-enabled = ✓ Console sink: enabled
example-builder-file-sink-not-enabled = ✗ File sink: not enabled
example-builder-example-2-multiple-sinks = Example 2: multiple sinks
example-builder-log-level-info = ✓ Log level: INFO
example-builder-file-sink-logs-app-log = ✓ File Sink: logs/app.log
example-builder-example-3-advanced-options = Example 3: advanced options
example-builder-log-level-trace-most-detailed = ✓ Log level: TRACE (most detailed)
example-builder-console-output-json-lines = ✓ Console output: JSON lines (console_json(true))
example-builder-json-output-requires-console = JSON output requires console_json(true) (no coloring in JSON mode)
example-builder-example-4-performance-options = Example 4: performance options
example-builder-channel-capacity-50-000-default = ✓ Channel capacity: 50,000 (default 10,000)
example-builder-worker-threads-4-default-3 = ✓ Worker threads: 4 (default 3)
example-builder-use-case-high-throughput = ✓ Use case: high-throughput production
example-builder-example-5-console-sink-options = Example 5: console sink options
example-builder-colored-output-enabled = ✓ Colored output: enabled
example-builder-stderr-output-error-and-warn = ✓ stderr output: ERROR and WARN levels
example-builder-stdout-output-all-other-levels = ✓ stdout output: all other levels
example-builder-example-6-file-sink-options = Example 6: file sink options
example-builder-file-path-logs-app-log = ✓ File path: logs/app.log
example-builder-single-file-size-100mb = ✓ Single file size: 100MB
example-builder-compression-zstd-requires-the = ✓ Compression: Zstd (requires the inklog `compression`/`zstd` or `gzip` feature)
example-builder-rotation-policy-daily-rotation = ✓ Rotation policy: daily rotation
example-builder-keep-files-7 = ✓ Keep files: 7
example-builder-only-warns-and-leaves-rotated = only warns and leaves rotated files uncompressed; it does not error
example-builder-example-7-builder-pattern = Example 7: Builder pattern flexibility
example-builder-supports-conditional = ✓ Supports conditional construction
example-builder-supports-environment-variable = ✓ Supports environment variable checks
example-builder-builder-instances-can-be-reused = ✓ Builder instances can be reused
example-builder-fluent-chained-calls = ✓ Fluent chained calls

# error-handling
example-error-handling-inklog-error-handling-examples = inklog error handling examples
example-error-handling-all-error-handling-examples = ✓ all error handling examples completed
example-error-handling-1-all-inklogerror-variants = 1. All InklogError variants
example-error-handling-display-output = { $value } Display output
example-error-handling-error-variants-in-total = ✓ { $value } error variants in total
example-error-handling-2-display-trait-format = 2. Display trait format
example-error-handling-error = Error: { $err }
example-error-handling-3-safe-message-sensitive-data = 3. safe_message() sensitive data redaction
example-error-handling-safe-message-redacts-sensitive = safe_message() redacts sensitive data inside error messages:
example-error-handling-aws-key-redaction = AWS key redaction:
example-error-handling-raw = Raw: { $err }
example-error-handling-safe = Safe: { $value }
example-error-handling-database-connection-string = Database connection string redaction:
example-error-handling-user-path-redaction = User path redaction:
example-error-handling-email-redaction = Email redaction:
example-error-handling-non-sensitive-information-is = Non-sensitive information is left unchanged:
example-error-handling-safe-message-redacts-sensitive-2 = ✓ safe_message() redacts sensitive data correctly
example-error-handling-4-localized-message = 4. localized_message() internationalized messages
example-error-handling-localized-message-returns-the = localized_message() returns the translated error prefix for the current locale:
example-error-handling-localized-message = { $value } localized_message()
example-error-handling-every-error-variant-supports-an = ✓ every error variant supports an internationalized message
example-error-handling-5-convenience-constructors = 5. Convenience constructors
example-error-handling-inklogerror-database-error = InklogError::database_error("connection pool exhausted"):
example-error-handling-display = Display: { $err }
example-error-handling-message = message: { $message }
example-error-handling-source = source:  { $value }
example-error-handling-inklogerror-database-error-with = InklogError::database_error_with_source(...):
example-error-handling-source-has-an-error-chain = source:  { $value } (has an error chain)
example-error-handling-inklogerror-encryption-error = InklogError::encryption_error("invalid key length: 16"):
example-error-handling-inklogerror-encryption-error-2 = InklogError::encryption_error_with_source(...):
example-error-handling-6-automatic-from-trait = 6. Automatic From trait conversion
example-error-handling-io-error-inklogerror = io::Error → InklogError:
example-error-handling-serde-json-error-inklogerror = serde_json::Error → InklogError:
example-error-handling-toml-de-error-inklogerror = toml::de::Error → InklogError:
example-error-handling-standard-error-types-convert-to = ✓ standard error types convert to InklogError automatically
example-error-handling-7-the-inklogresult-t-type-alias = 7. The InklogResult<T> type alias
example-error-handling-err-is-err = Err(...) → is_err = { $value }
example-error-handling-unexpected-success = unexpected success
example-error-handling-propagate-io-error-with-the = Propagate io::Error with the ? operator:
example-error-handling-inklogresult-works-with-the = ✓ InklogResult works with the ? operator
example-error-handling-8-practical-error-handling = 8. Practical error handling patterns
example-error-handling-pattern-1-match-branch-handling = Pattern 1: match branch handling
example-error-handling-operation-succeeded = operation succeeded
example-error-handling-config-error-recoverable = config error (recoverable): { $msg }
example-error-handling-retry-with-the-default = → retry with the default configuration
example-error-handling-io-error = IO error: { $e }
example-error-handling-check-file-permissions-and-disk = → check file permissions and disk space
example-error-handling-other-error = other error: { $e }
example-error-handling-report-and-exit = → report and exit
example-error-handling-pattern-2-safe-log-output = Pattern 2: safe log output
example-error-handling-log-output = Log output: { $value }
example-error-handling-user-message = User message: { $value }
example-error-handling-pattern-3-error-chain-tracing = Pattern 3: error chain tracing
example-error-handling-top-level-error = Top-level error: { $err }
example-error-handling-root-cause = Root cause: { $src }

# i18n
example-i18n-inklog-internationalization = inklog internationalization (i18n) formatting examples
example-i18n-all-i18n-formatting-examples = All i18n formatting examples shown.
example-i18n-example-1-create-multi-locale = Example 1: create multi-locale formatters
example-i18n-create-the-en-us-formatter = Create the en-US formatter:
example-i18n-logi18nformatter-new-en-us = ✓ LogI18nFormatter::new("en-US") created
example-i18n-create-the-zh-cn-formatter = Create the zh-CN formatter:
example-i18n-logi18nformatter-new-zh-cn = ✓ LogI18nFormatter::new("zh-CN") created
example-i18n-supported-bcp-47-language-tags = Supported BCP-47 language tags:
example-i18n-en-us-english-united-states = en-US: English (United States)
example-i18n-zh-cn-chinese-simplified = zh-CN: Chinese (Simplified)
example-i18n-ja-jp-japanese-japan = ja-JP: Japanese (Japan)
example-i18n-de-de-german-germany = de-DE: German (Germany)
example-i18n-field-comparison-example-a-b = Field comparison example: "a" < "b"
example-i18n-field-comparison-example = Field comparison example: { $other }
example-i18n-field-comparison-failed = Field comparison failed: { $e }
example-i18n-example-2-number-formatting = Example 2: number formatting
example-i18n-notes = Notes:
example-i18n-en-us-comma-thousands-separator = - en-US: comma thousands separator, dot decimal point
example-i18n-zh-cn-the-separator-may-differ = - zh-CN: the separator may differ (depends on ICU data)
example-i18n-use-case-format-performance = - Use case: format performance metrics and counters in logs
example-i18n-example-3-event-count-plurals = Example 3: event count plurals
example-i18n-use-case-generate-locale-aware = Use case: generate locale-aware log messages
example-i18n-en-one-event-processed-other = en: "One event processed" / "Other events processed"
example-i18n-zh-other-chinese-has-no-plural = zh: "Other 事件已处理" (Chinese has no plural inflection)
example-i18n-example-4-timestamp-formatting = Example 4: timestamp formatting
example-i18n-use-case-localized-display-of = Use case: localized display of log timestamps
example-i18n-use-the-icu4x-datetimeformatter = - Use the ICU4X DateTimeFormatter medium format
example-i18n-adapts-automatically-to-the = - Adapts automatically to the date conventions of the target language
example-i18n-example-5-log-level = Example 5: log level normalization
example-i18n-log-level-normalization-unified = Log level normalization (unified upper case):
example-i18n-use-case-unify-the-displayed = Use case: unify the displayed log level format
example-i18n-example-6-locale-sensitive = Example 6: locale-sensitive field name comparison
example-i18n-use-case-order-log-fields = Use case: order log fields alphabetically
example-i18n-use-the-icu4x-collator-for = - Use the ICU4X Collator for locale-sensitive ordering
example-i18n-supports-the-ordering-rules-of = - Supports the ordering rules of different languages
example-i18n-example-7-error-handling = Example 7: error handling
example-i18n-invalid-locale-tag = Invalid locale tag:
example-i18n-unexpected-success = unexpected success
example-i18n-expected-error = ✓ expected error: { $e }
example-i18n-non-finite-number-formatting = Non-finite number formatting:
example-i18n-nan-error = ✓ NaN error: { $e }
example-i18n-infinity-error = ✓ Infinity error: { $e }
example-i18n-error-types = Error types:
example-i18n-i18nerror-invalidlocale-the = I18nError::InvalidLocale  - the locale tag failed to parse
example-i18n-i18nerror-invalidnumber-a-non = I18nError::InvalidNumber  - a non-finite number
example-i18n-i18nerror-dateerror-an-invalid = I18nError::DateError      - an invalid date component
example-i18n-i18nerror-formaterror-icu4x = I18nError::FormatError    - ICU4X formatting failed

# production
example-production-inklog-production-configuration = === inklog production configuration examples ===
example-production-all-production-examples = All production examples completed.
example-production-example-1-per-environment = --- Example 1: per-environment configuration comparison ---
example-production-example-2-builder-pattern-for = --- Example 2: Builder pattern for production ---
example-production-production-configuration = ✓ production configuration initialized
example-production-log-level-info = Log level:   info
example-production-console-on-colored = Console:    on (colored)
example-production-file-logs-production-log-100mb = File:       logs/production.log (100MB, daily, keep 30)
example-production-compression-zstd = Compression: Zstd
example-production-channel-10000 = Channel:    10000
example-production-workers-4 = Workers:    4
example-production-production-log-system-started = production log system started
example-production-this-debug-log-is-not-emitted = this DEBUG log is not emitted (level=info)
example-production-level-filter-verified-debug-is = ✓ level filter verified: DEBUG is not visible (as expected)
example-production-3-business-logs-recorded = ✓ 3 business logs recorded
example-production-alert-log-recorded = ✓ alert log recorded
example-production-error-log-recorded = ✓ error log recorded
example-production-shut-down-gracefully = ✓ shut down gracefully
example-production-example-3-toml-configuration = --- Example 3: TOML configuration file format ---
example-production-inklog-toml-recommended = inklog.toml (recommended production configuration):
example-production-how-to-load = How to load:
example-production-example-4-health-check-after = --- Example 4: health check after initialization ---
example-production-health-check = Health check:
example-production-overall-status = overall_status:     { $overall_status }
example-production-uptime-seconds-s = uptime_seconds:     { $uptime_seconds }s
example-production-channel-usage = channel_usage:      { $value }%
example-production-encryption-valid = encryption_valid:   { $encryption_key_valid }
example-production-health-status-retrieved = ✓ health status retrieved
example-production-health-check-log = health check log
example-production-overall-status-after-writing-a = overall_status after writing a log: { $overall_status }
example-production-channel-info = Channel info:
example-production-capacity = capacity: { $capacity }
example-production-queued = queued:   { $queued }
example-production-channel-verified = ✓ channel verified
example-production-example-5-graceful-shutdown = --- Example 5: graceful shutdown sequence ---
example-production-5-logs-written-into-the-channel = ✓ 5 logs written into the channel
example-production-channel-queue-before-shutdown = channel queue before shutdown: { $queued_before } entries
example-production-shutdown-completed = ✓ shutdown() completed
example-production-all-logs-persisted-to-sinks = - All logs persisted to sinks
example-production-worker-threads-stopped = - Worker threads stopped
example-production-resources-released = - Resources released
example-production-example-6-error-handling = --- Example 6: error handling ---
example-production-scenario-1-load-a-nonexistent = Scenario 1: load a nonexistent configuration file
example-production-unexpected-success = unexpected success
example-production-expected-error = ✓ expected error: { $e }
example-production-scenario-2-an-invalid-log-level = Scenario 2: an invalid log level
example-production-handling-use-a-valid-level = Handling: use a valid level (trace/debug/info/warn/error)
example-production-error-handling-advice = Error handling advice:
example-production-always-handle-loggermanager = - Always handle LoggerManager initialization errors with ? or match
example-production-give-a-meaningful-error-message = - Give a meaningful error message when the config file fails to load
example-production-consider-a-config-fallback = - Consider a config fallback chain: env vars → config file → defaults
example-production-production-best-practices = --- Production best practices ---
example-production-1-configuration-management = 1. Configuration management:
example-production-keep-configuration-in-a-toml = - Keep configuration in a TOML file so it is version controlled
example-production-inject-secrets-such-as-the = - Inject secrets (such as the database password) through environment variables
example-production-use-a-different-configuration = - Use a different configuration file per environment
example-production-2-file-rotation = 2. File rotation:
example-production-production-must-enable-log = - Production must enable log rotation so the disk does not fill up
example-production-recommended-daily-rotation-30 = - Recommended: daily rotation + 30 days of retention
example-production-enable-zstd-compression-to-save = - Enable Zstd compression to save storage
example-production-3-performance-tuning = 3. Performance tuning:
example-production-channel-capacity-size-it-for = - channel_capacity: size it for peak QPS (QPS × 10 is a good start)
example-production-worker-threads-match-the-cpu = - worker_threads: match the CPU core count (2-8)
example-production-watch-channel-len-capacity-and = - Watch channel_len / capacity and alert above 80%
example-production-4-monitoring-integration = 4. Monitoring integration:
example-production-enable-the-http-server-to = - Enable the HTTP server to expose /health and /metrics
example-production-have-prometheus-scrape-the = - Have Prometheus scrape the /metrics endpoint
example-production-configure-alert-rules-on = - Configure alert rules on logging system health
example-production-5-security-hardening = 5. Security hardening:
example-production-enable-tls-on-the-production = - Enable TLS on the production HTTP server
example-production-configure-an-ip-whitelist-to = - Configure an IP whitelist to restrict sources
example-production-enable-bearer-token = - Enable Bearer Token authentication
example-production-use-logsanitizer-to-redact = - Use LogSanitizer to redact sensitive information

# template
example-template-inklog-log-template-examples = === inklog log template examples ===
example-template-all-template-examples-completed = All template examples completed.
example-template-example-1-default-template = --- Example 1: default template ---
example-template-output = Output: { $rendered }
example-template-render-check-passed = ✓ render check passed
example-template-example-2-custom-templates = --- Example 2: custom templates ---
example-template-output-2 = Output: { $simple_out }
example-template-exact-match = ✓ exact match
example-template-output-3 = Output: { $standard_out }
example-template-contains-key-fields = ✓ contains key fields
example-template-output-4 = Output: { $detailed_out }
example-template-file-and-line-are-correct = ✓ file and line are correct
example-template-example-3-full-placeholder-list = --- Example 3: full placeholder list ---
example-template-rendered-output = { $value } { $value2 } rendered output
example-template-example-4-structured-field = --- Example 4: structured field rendering ---
example-template-field-rendering-check-passed = ✓ field rendering check passed
example-template-fields-only = Fields only: { $fields_only }
example-template-fields-only-rendering-check = ✓ fields-only rendering check passed
example-template-example-5-multi-format = --- Example 5: multi-format comparison ---
example-template-rendered-output-2 = { $value } rendered output
example-template-example-6-edge-cases = --- Example 6: edge cases ---
example-template-empty-message = Empty message:   '{ $empty_out }'
example-template-empty-message-handled-correctly = ✓ empty message handled correctly
example-template-special-characters = Special characters: '{ $special_out }'
example-template-special-characters-preserved = ✓ special characters preserved intact
example-template-unicode = Unicode:  '{ $unicode_out }'
example-template-unicode-handled-correctly = ✓ Unicode handled correctly
example-template-long-message-chars-chars = Long message: { $value } chars → { $value2 } chars
example-template-long-message-handled-correctly = ✓ long message handled correctly
example-template-example-7-render-output = --- Example 7: render output verification ---
example-template-error-payment-failed-exact-match = ✓ [ERROR] payment failed — exact match
example-template-containment-verified = ✓ { $out } — containment verified
example-template-all-5-levels-render-consistently = ✓ all 5 levels render consistently
example-template-best-practices = --- Best practices ---
example-template-1-template-selection-guide = 1. Template selection guide:
example-template-2-performance-notes = 2. Performance notes:
example-template-create-one-template-instance = - Create one template instance and reuse it globally to avoid re-parsing
example-template-3-placeholder-notes = 3. Placeholder notes:
example-template-4-integration-with-tracing = 4. Integration with tracing:
example-template-logtemplate-is-used-mainly-to = - LogTemplate is used mainly to render LogRecord values
example-template-the-tracing-output-format-is = - The tracing output format is controlled by tracing-subscriber
example-template-they-work-together-tracing = - They work together: tracing collects logs → LogTemplate formats output

# database
example-database-example-1-sqlite-in-memory = Example 1: SQLite in-memory database connection
example-database-database-config = Database config:
example-database-driver = driver: { $driver }
example-database-url = URL: { $url }
example-database-connection-pool = connection pool: { $pool_size }
example-database-batch-size = batch size: { $batch_size }
example-database-flush-interval-ms = flush interval: { $flush_interval_ms }ms
example-database-connecting-to-sqlite-in-memory = connecting to SQLite in-memory database
example-database-connection-pool-created = ✓ connection pool created
example-database-failed-to-create-connection-pool = ✗ failed to create connection pool: { $e }
example-database-session-acquired = ✓ session acquired
example-database-failed-to-acquire-session = ✗ failed to acquire session: { $e }
example-database-logs-table-created = ✓ logs table created
example-database-failed-to-create-logs-table-may = ⚠ failed to create logs table (may already exist): { $e }
example-database-write-test-log = write test log
example-database-write-failed = ✗ write failed: { $e }
example-database-query-verification = query verification
example-database-query-succeeded-table-has-data = ✓ query succeeded, table has data
example-database-query-failed = ✗ query failed: { $e }
example-database-in-memory-database-connection = ✓ In-memory database connection example completed
example-database-note-sqlite-in-memory-data-is = Note: SQLite in-memory data is lost when the connection closes
example-database-each-databasesink-instance = Each DatabaseSink instance creates its own connection
example-database-example-2-batch-write-demo = Example 2: batch write demo
example-database-batch-configuration = Batch configuration:
example-database-batch-write-log = batch write log
example-database-writing-log-records = writing { $value } log records...
example-database-write-round = --- write round { $round } ---
example-database-write-failed-2 = ✗ [{ $value }] write failed: { $e }
example-database-round-done-succeeded = Round { $round } done: { $success_count } / { $value } succeeded
example-database-total-log-records-in-the = ✓ total log records in the database
example-database-count-failed = ✗ count failed: { $e }
example-database-batch-write-demo-completed = ✓ Batch write demo completed
example-database-example-3-query-demo = Example 3: query demo
example-database-prepare-test-data = prepare test data
example-database-writing-log-records-at = writing log records at different levels...
example-database-insert-failed = ⚠ INSERT failed: { $e }
example-database-wrote-test-records = ✓ wrote { $value } test records
example-database-query-1-count-by-level = Query 1: count by level
example-database-sql-select-level-count-as-count = SQL: SELECT level, COUNT(*) as count FROM logs GROUP BY level
example-database-expected-result = Expected result:
example-database-level-count = │ Level │ Count │
example-database-trace-2 = │ TRACE │   2   │
example-database-debug-2 = │ DEBUG │   2   │
example-database-info-3 = │ INFO  │   3   │
example-database-warn-2 = │ WARN  │   2   │
example-database-error-2 = │ ERROR │   2   │
example-database-total-11-records = Total: 11 records
example-database-query-2-count-by-target-module = Query 2: count by target module
example-database-sql-select-target-count-as = SQL: SELECT target, COUNT(*) as count FROM logs GROUP BY target
example-database-target-count = │ Target                 │ Count │
example-database-database-example-api-3 = │ database_example::api  │   3   │
example-database-database-example-query-4 = │ database_example::query│   4   │
example-database-database-example-auth-1 = │ database_example::auth │   1   │
example-database-database-example-cache-1 = │ database_example::cache│   1   │
example-database-database-example-database-2 = │ database_example::database│  2   │
example-database-query-3-filter-by-level-error = Query 3: filter by level (ERROR/WARN)
example-database-sql-select-from-logs-where = SQL: SELECT * FROM logs WHERE level IN ('ERROR', 'WARN')
example-database-warn-database-example-auth-auth = [WARN]  database_example::auth     - auth token expiring soon
example-database-warn-database-example-cache = [WARN]  database_example::cache    - cache hit rate low: 45%
example-database-error-database-example-database = [ERROR] database_example::database  - database connection timeout
example-database-error-database-example-database-2 = [ERROR] database_example::database  - query execution failed
example-database-query-4-search-log-content = Query 4: search log content
example-database-sql-select-from-logs-where-2 = SQL: SELECT * FROM logs WHERE message LIKE '%timeout%' OR message LIKE '%failed%'
example-database-query-5-fetch-recent-logs = Query 5: fetch recent logs
example-database-sql-select-from-logs-order-by = SQL: SELECT * FROM logs ORDER BY id DESC LIMIT 5
example-database-expected-result-ids-descending = Expected result (IDs descending):
example-database-11-error-database-example = 11. [ERROR] database_example::database - query execution failed
example-database-10-error-database-example = 10. [ERROR] database_example::database - database connection timeout
example-database-9-warn-database-example-cache = 9. [WARN]  database_example::cache   - cache hit rate low: 45%
example-database-8-warn-database-example-auth = 8. [WARN]  database_example::auth    - auth token expiring soon
example-database-7-info-database-example-api = 7. [INFO]  database_example::api    - response sent successfully
example-database-query-6-time-range-query = Query 6: time-range query
example-database-sql-select-from-logs-where-3 = SQL: SELECT * FROM logs WHERE timestamp > '2024-01-01'
example-database-expected-result-all-logs-all = Expected result: all logs (all were written recently)
example-database-query-7-combined-multi = Query 7: combined multi-condition query
example-database-sql-select-level-target-message = SQL: SELECT level, target, message FROM logs
example-database-where-level-in-warn-error-and = WHERE level IN ('WARN', 'ERROR') AND target LIKE '%database%'
example-database-query-demo-completed = ✓ Query demo completed
example-database-tip-use-sql-clauses-such-as = Tip: use SQL clauses such as WHERE, GROUP BY, ORDER BY
example-database-enabling-complex-log-queries = enabling complex log queries and analysis
example-database-inklog-database-sink-example = === inklog Database Sink example ===
example-database-this-example-uses-an-sqlite-in = This example uses an SQLite in-memory database (:memory:)
example-database-no-file-cleanup-needed-data = No file cleanup needed; data lives only in memory
example-database-all-examples-completed = All examples completed!
example-database-key-takeaways = Key takeaways:
example-database-1-databasesink-supports-sqlite = 1. DatabaseSink supports SQLite, PostgreSQL, and MySQL
example-database-2-in-memory-databases-suit = 2. In-memory databases suit testing and demos; data is lost on shutdown
example-database-4-supports-queries-by-level = 4. Supports queries by level, time, target, and more
example-database-5-automatically-falls-back-to = 5. Automatically falls back to file output on failure
example-database-sql-query-tips = SQL query tips:
example-database-by-level-where-level-error = - by level: WHERE level = 'ERROR'
example-database-by-module-where-target-like-api = - by module: WHERE target LIKE '%api%'
example-database-by-time-where-timestamp-2024-01 = - by time: WHERE timestamp > '2024-01-01'
example-database-aggregate-stats-group-by-level = - aggregate stats: GROUP BY level
example-database-error-enable-a-database-feature = error: enable a database feature (sqlite/postgres/mysql) to run this example
example-database-run-with-one-of-the-following = Run with one of the following commands:
example-database-cargo-run-bin-database-features = cargo run --bin database --features sqlite
example-database-or-enable-the-sqlite-feature-in = or enable the sqlite feature in examples/Cargo.toml:

# database-pg-mysql
example-database-pg-mysql-inklog-postgresql-mysql = inklog PostgreSQL/MySQL database driver example
example-database-pg-mysql-all-postgresql-mysql-driver = All PostgreSQL/MySQL driver examples completed.
example-database-pg-mysql-example-1-the-three = Example 1: the three DatabaseDriver variants
example-database-pg-mysql-enum-variants = Enum variants:
example-database-pg-mysql-enterprise-first-choice-the = { $postgres } - enterprise first choice, the most complete feature set
example-database-pg-mysql-a-balance-of-performance-and = { $mysql } - a balance of performance and simplicity
example-database-pg-mysql-embedded-zero-configuration = { $sqlite } - embedded, zero configuration
example-database-pg-mysql-default-driver = Default driver:
example-database-pg-mysql-databasedriver-default = DatabaseDriver::default() = { $value }
example-database-pg-mysql-example-2-fromstr-display-traits = Example 2: FromStr / Display traits
example-database-pg-mysql-fromstr-parsing-case-insensitive = FromStr parsing (case-insensitive):
example-database-pg-mysql-parse-ok = { $input }.parse() → Ok({ $driver })
example-database-pg-mysql-parse-err = { $input }.parse() → Err
example-database-pg-mysql-invalid-input = Invalid input:
example-database-pg-mysql-parse-ok-2 = { $input }.parse() → Ok({ $d })
example-database-pg-mysql-parse-err-2 = { $input }.parse() → Err("{ $e }")
example-database-pg-mysql-display-output = Display output:
example-database-pg-mysql-example-3-postgresql-connection = Example 3: PostgreSQL connection config
example-database-pg-mysql-postgresql-databasesinkconfig = PostgreSQL DatabaseSinkConfig:
example-database-pg-mysql-driver = driver            = { $driver }
example-database-pg-mysql-url = url               = "{ $url }"
example-database-pg-mysql-table-name = table_name        = "{ $table_name }"
example-database-pg-mysql-batch-size = batch_size        = { $batch_size }
example-database-pg-mysql-flush-interval-ms = flush_interval_ms = { $flush_interval_ms }
example-database-pg-mysql-pool-size = pool_size         = { $pool_size }
example-database-pg-mysql-connection-url-format = Connection URL format:
example-database-pg-mysql-postgres-user-password-host = postgres://USER:PASSWORD@HOST:PORT/DATABASE
example-database-pg-mysql-example = Example:
example-database-pg-mysql-postgres-inklog-secret-db = postgres://inklog:secret@db.example.com:5432/logs
example-database-pg-mysql-postgres-postgres-localhost = postgres://postgres@localhost/inklog
example-database-pg-mysql-postgres-user-pass-10-0-0-1 = postgres://user:pass@10.0.0.1:5432/prod_logs?sslmode=require
example-database-pg-mysql-corresponding-toml-config = Corresponding TOML config:
example-database-pg-mysql-postgresql-strengths = PostgreSQL strengths:
example-database-pg-mysql-richest-features-partitioned = - Richest features: partitioned tables (PARTITION BY RANGE)
example-database-pg-mysql-great-performance-high = - Great performance: high-concurrency writes, MVCC
example-database-pg-mysql-extensibility-jsonb-full-text = - Extensibility: JSONB, full-text search, geospatial data
example-database-pg-mysql-production-pick-the-first = - Production pick: the first choice for enterprise log storage
example-database-pg-mysql-table-partitioning-high-log = Table partitioning (high log volume):
example-database-pg-mysql-example-4-mysql-connection = Example 4: MySQL connection config
example-database-pg-mysql-mysql-databasesinkconfig = MySQL DatabaseSinkConfig:
example-database-pg-mysql-mysql-user-password-host-port = mysql://USER:PASSWORD@HOST:PORT/DATABASE
example-database-pg-mysql-mysql-root-localhost-inklog = mysql://root@localhost/inklog
example-database-pg-mysql-mysql-inklog-secret-db-example = mysql://inklog:secret@db.example.com:3306/logs
example-database-pg-mysql-mysql-user-pass-10-0-0-1-3306 = mysql://user:pass@10.0.0.1:3306/prod_logs?ssl-ca=/path/to/ca.pem
example-database-pg-mysql-mysql-strengths = MySQL strengths:
example-database-pg-mysql-easy-deployment-works-with = - Easy deployment: works with default settings
example-database-pg-mysql-mature-ecosystem-native-fit = - Mature ecosystem: native fit with LAMP/LEMP stacks
example-database-pg-mysql-balanced-performance-even-read = - Balanced performance: even read/write throughput
example-database-pg-mysql-partitioning-partition-by-range = - Partitioning: PARTITION BY RANGE
example-database-pg-mysql-example-5-three-driver = Example 5: three-driver comparison
example-database-pg-mysql-feature-comparison = Feature comparison:
example-database-pg-mysql-selection-advice = Selection advice:
example-database-pg-mysql-dev-test-sqlite-zero-config = - Dev/test: SQLite (zero-config, fast startup)
example-database-pg-mysql-small-medium-production-mysql = - Small/medium production: MySQL (easy to deploy, balanced performance)
example-database-pg-mysql-large-enterprise-postgresql = - Large/enterprise: PostgreSQL (richest features, strongest performance)
example-database-pg-mysql-example-6-env-var-overrides = Example 6: env-var overrides
example-database-pg-mysql-database-driver-env-vars = Database driver env vars:
example-database-pg-mysql-loading-via-load-with-env = Loading via load_with_env_overrides():
example-database-pg-mysql-config-database-sink-driver = config.database_sink.driver,
example-database-pg-mysql-docker-deployment-example = Docker deployment example:
example-database-pg-mysql-my-app = my-app
example-database-pg-mysql-kubernetes-configmap-example = Kubernetes ConfigMap example:
example-database-pg-mysql-env = env:
example-database-pg-mysql-name-inklog-database-sink-driver = - name: INKLOG_DATABASE_SINK_DRIVER
example-database-pg-mysql-valuefrom = valueFrom:
example-database-pg-mysql-configmapkeyref = configMapKeyRef:
example-database-pg-mysql-name-inklog-config = name: inklog-config
example-database-pg-mysql-key-db-driver = key: db-driver

# di-example
example-di-example-di-dependency-injection-pattern = === DI (Dependency Injection) pattern example ===
example-di-example-1-using-inklogcontainer-real = 1. Using InklogContainer (real adapters):
example-di-example-creating-container = - Creating container...
example-di-example-cache-instance-created-arc-dyn = - Cache instance: created (Arc<dyn Cache>)
example-di-example-config-instance-created-arc-dyn = - Config instance: created (Arc<dyn Config>)
example-di-example-2-injecting-dependencies-via = 2. Injecting dependencies via LoggerBuilder (real adapters):
example-di-example-logger-created = - Logger created!
example-di-example-3-using-with-dependencies-real = 3. Using with_dependencies (real adapters):
example-di-example-4-container-shared-dependency = 4. Container-shared dependency instances (real adapters):
example-di-example-cache1-set-shared-key-shared = - cache1 set shared_key = shared_value
example-di-example-cache2-read-shared-key = - cache2 read shared_key = { $value }
example-di-example-both-instances-share-the-same = - Both instances share the same underlying data: { $value }
example-di-example-5-creating-multiple-loggers = 5. Creating multiple loggers from the container:
example-di-example-logger-1-created = - Logger 1 created
example-di-example-logger-2-created = - Logger 2 created
example-di-example-both-loggers-share-the-same = - Both loggers share the same cache and config instances
example-di-example-mock-type-examples-for-testing = === Mock type examples (for testing) ===
example-di-example-6-testing-with-mockcache = 6. Testing with MockCache:
example-di-example-mockcache-set-and-read = - MockCache set and read: { $value }
example-di-example-test-key-exists = - test_key exists: { $exists }
example-di-example-deleted-test-key = - Deleted test_key: { $deleted }
example-di-example-7-testing-with-mockconfig = 7. Testing with MockConfig:
example-di-example-read-global-level = - Read global.level: { $level }
example-di-example-read-http-server-port = - Read http_server.port: { $port }
example-di-example-read-file-sink-enabled = - Read file_sink.enabled: { $enabled }
example-di-example-global-level-after-modification = - global.level after modification: { $value }
example-di-example-8-testing-with = 8. Testing with MockDatabaseAdapter:
example-di-example-initial-health-status = - Initial health status: { $healthy }
example-di-example-health-status-after-fault = - Health status after fault injection: { $value }
example-di-example-health-status-after-recovery = - Health status after recovery: { $value }
example-di-example-stored-record-count = - Stored record count: { $value }
example-di-example-9-creating-a-logger-with-mock = 9. Creating a logger with mock types:
example-di-example-logger-created-with-mock = - Logger created with mock dependencies!
example-di-example-10-using-mockcache-with-delay = 10. Using MockCache with delay (simulating network latency):
example-di-example-set-operation-took = - Set operation took: { $elapsed }
example-di-example-verified-delay-took-effect-ms = - Verified delay took effect: { $value }ms >= 10ms
example-di-example-examples-completed = === Examples completed ===

# channel-strategy
example-channel-strategy-inklog-adaptive-channel = inklog adaptive channel strategy example
example-channel-strategy-all-channelstrategy-examples = All ChannelStrategy examples completed.
example-channel-strategy-example-1-channelstrategy = Example 1: ChannelStrategy variants
example-channel-strategy-enum-variants = Enum variants:
example-channel-strategy-static-capacity-predictable = { $fixed } - static capacity, predictable memory use
example-channel-strategy-dynamic-sizing-better-burst = { $adaptive } - dynamic sizing, better burst handling
example-channel-strategy-default-strategy = Default strategy:
example-channel-strategy-channelstrategy-default = ChannelStrategy::default() = { $value }
example-channel-strategy-behavior-comparison = Behavior comparison:
example-channel-strategy-example-2-fromstr-display-traits = Example 2: FromStr / Display traits
example-channel-strategy-fromstr-parsing = FromStr parsing:
example-channel-strategy-parse-ok = { $input }.parse() → Ok({ $strategy })
example-channel-strategy-parse-err = { $input }.parse() → Err("{ $e }")
example-channel-strategy-invalid-input = Invalid input:
example-channel-strategy-parse-ok-2 = { $input }.parse() → Ok({ $s })
example-channel-strategy-display-output = Display output:
example-channel-strategy-example-3-adaptive-threshold = Example 3: adaptive threshold parameters
example-channel-strategy-default-performanceconfig = Default PerformanceConfig:
example-channel-strategy-channel-capacity = channel_capacity        = { $channel_capacity }
example-channel-strategy-worker-threads = worker_threads          = { $worker_threads }
example-channel-strategy-channel-strategy = channel_strategy        = { $channel_strategy }
example-channel-strategy-expand-threshold-percent = expand_threshold_percent= { $expand_threshold_percent }
example-channel-strategy-shrink-threshold-percent = shrink_threshold_percent= { $shrink_threshold_percent }
example-channel-strategy-shrink-wait-seconds = shrink_wait_seconds     = { $shrink_wait_seconds }
example-channel-strategy-min-capacity = min_capacity            = { $min_capacity }
example-channel-strategy-max-capacity = max_capacity            = { $max_capacity }
example-channel-strategy-threshold-parameter-semantics = Threshold parameter semantics (only effective for the Adaptive strategy):
example-channel-strategy-expand-threshold-percent-expand = expand_threshold_percent: expand when capacity usage reaches this percent
example-channel-strategy-default-80-expand-at-8000-10000 = default 80% → expand at 8000/10000
example-channel-strategy-shrink-threshold-percent-2 = shrink_threshold_percent: consider shrinking when usage drops to this percent
example-channel-strategy-default-20-shrink-at-2000-10000 = default 20% → shrink at 2000/10000
example-channel-strategy-default-30-seconds = default 30 seconds
example-channel-strategy-min-capacity-max-capacity-lower = min_capacity / max_capacity: lower/upper bounds for shrink/expand
example-channel-strategy-custom-adaptive-config = Custom adaptive config (aggressive expansion, conservative shrinking):
example-channel-strategy-expand-threshold-percent-2 = expand_threshold_percent= { $expand_threshold_percent } (expands earlier)
example-channel-strategy-shrink-threshold-percent-3 = shrink_threshold_percent= { $shrink_threshold_percent } (shrinks later)
example-channel-strategy-shrink-wait-seconds-longer = shrink_wait_seconds     = { $shrink_wait_seconds } (longer debounce)
example-channel-strategy-example-4-performance-config = Example 4: performance config recommendations
example-channel-strategy-channel-capacity-workers = channel_capacity={ $channel_capacity }, workers={ $worker_threads }, strategy={ $channel_strategy }
example-channel-strategy-expand-shrink-wait-s-range = expand@{ $expand_threshold_percent }%, shrink@{ $shrink_threshold_percent }%, wait={ $shrink_wait_seconds }s, range=[{ $min_capacity },{ $max_capacity }]
example-channel-strategy-example-5-formatted-config = Example 5: formatted config display
example-channel-strategy-serialized-to-toml = Serialized to TOML:
example-channel-strategy-env-var-overrides = Env-var overrides:

# circuit-breaker
example-circuit-breaker-inklog-circuit-breaker-example = === inklog circuit breaker example ===
example-circuit-breaker-all-circuit-breaker-examples = ✓ All circuit breaker examples completed
example-circuit-breaker-1-breaker-creation-and-initial = 1. Breaker creation and initial state
example-circuit-breaker-1-1-circuitbreaker-new-failure = 1.1 CircuitBreaker::new(failure_threshold, timeout, success_threshold)
example-circuit-breaker-failure-threshold = failure_threshold = { $value }
example-circuit-breaker-success-threshold = success_threshold = { $value }
example-circuit-breaker-timeout = timeout           = { $value }
example-circuit-breaker-1-2-initial-state-is-closed = 1.2 Initial state is Closed; execution allowed
example-circuit-breaker-state = state()       = { $value }
example-circuit-breaker-failure-count = failure_count = { $value }
example-circuit-breaker-can-execute = can_execute() = { $value }
example-circuit-breaker-2-closed-open-failure-count = 2. Closed → Open: failure count trips the breaker
example-circuit-breaker-2-1-config-failure-threshold-3 = 2.1 Config: failure_threshold = 3
example-circuit-breaker-initial-state = initial state = { $value }
example-circuit-breaker-2-2-failure-1-below-threshold = 2.2 Failure #1 (below threshold)
example-circuit-breaker-state-2 = state         = { $value }
example-circuit-breaker-2-3-failure-2-below-threshold = 2.3 Failure #2 (below threshold)
example-circuit-breaker-2-4-failure-3-threshold-reached = 2.4 Failure #3 (threshold reached → Open)
example-circuit-breaker-2-5-fast-fail-in-open-state-no = 2.5 Fast fail in Open state (no real operation runs)
example-circuit-breaker-in-the-open-state-can-execute = In the Open state, can_execute() returns false and requests are fast-rejected
example-circuit-breaker-avoids-repeatedly-sending = avoids repeatedly sending requests to a failed sink
example-circuit-breaker-3-open-halfopen-automatically = 3. Open → HalfOpen: automatically enters half-open after timeout
example-circuit-breaker-3-1-config-timeout-100ms = 3.1 Config: timeout = 100ms
example-circuit-breaker-state-after-tripping = state = { $value } after tripping
example-circuit-breaker-can-execute-timeout-not-elapsed = can_execute()    = { $value } (timeout not elapsed)
example-circuit-breaker-3-2-waiting-150ms-for-the = 3.2 Waiting 150ms for the timeout
example-circuit-breaker-wait-elapsed-calling-can = Wait elapsed; calling can_execute() again triggers the transition
example-circuit-breaker-3-3-can-execute-triggers-open = 3.3 can_execute() triggers Open → HalfOpen
example-circuit-breaker-can-execute-2 = can_execute() = { $allowed }
example-circuit-breaker-3-4-halfopen-allows-a-limited = 3.4 HalfOpen allows a limited number of requests
example-circuit-breaker-4-halfopen-closed-recovers = 4. HalfOpen → Closed: recovers after accumulated successes
example-circuit-breaker-4-1-config-success-threshold-3 = 4.1 Config: success_threshold = 3
example-circuit-breaker-now-in-halfopen-success = Now in HalfOpen, success_threshold = 3
example-circuit-breaker-4-2-success-1-below-threshold = 4.2 Success #1 (below threshold, still HalfOpen)
example-circuit-breaker-state-3 = state = { $value }
example-circuit-breaker-4-3-success-2-below-threshold = 4.3 Success #2 (below threshold, still HalfOpen)
example-circuit-breaker-4-4-success-3-threshold-reached = 4.4 Success #3 (threshold reached → Closed)
example-circuit-breaker-5-halfopen-open-a-failure-in = 5. HalfOpen → Open: a failure in half-open immediately reopens
example-circuit-breaker-5-1-entering-halfopen = 5.1 Entering HalfOpen
example-circuit-breaker-5-2-one-success-recorded-in = 5.2 One success recorded in half-open (still HalfOpen)
example-circuit-breaker-5-3-failure-in-half-open = 5.3 Failure in half-open → immediately reopens
example-circuit-breaker-6-config-driven-construction = 6. Config-driven construction with with_config + CircuitBreakerConfig
example-circuit-breaker-6-1-customizing = 6.1 Customizing CircuitBreakerConfig
example-circuit-breaker-failure-threshold-2 = failure_threshold = { $failure_threshold }
example-circuit-breaker-success-threshold-2 = success_threshold = { $success_threshold }
example-circuit-breaker-timeout-2 = timeout           = { $timeout }
example-circuit-breaker-6-2-circuitbreaker-with-config = 6.2 CircuitBreaker::with_config(config)
example-circuit-breaker-config-failure-threshold = config().failure_threshold = { $value }
example-circuit-breaker-config-success-threshold = config().success_threshold = { $value }
example-circuit-breaker-config-timeout = config().timeout           = { $value }
example-circuit-breaker-6-3-circuitbreakerconfig-default = 6.3 CircuitBreakerConfig::default()
example-circuit-breaker-default-failure-threshold = default failure_threshold = { $failure_threshold }
example-circuit-breaker-default-success-threshold = default success_threshold = { $success_threshold }
example-circuit-breaker-default-timeout = default timeout           = { $timeout }
example-circuit-breaker-6-4-success-in-closed-state = 6.4 Success in Closed state resets the failure count
example-circuit-breaker-failure-count-after-2-failures = failure_count = { $value } after 2 failures
example-circuit-breaker-failure-count-after-record = failure_count = { $value } after record_success()
example-circuit-breaker-7-manual-reset-with-reset = 7. Manual reset with reset()
example-circuit-breaker-7-1-tripping-the-breaker-into = 7.1 Tripping the breaker into Open
example-circuit-breaker-7-2-reset-returns-to-the = 7.2 reset() returns to the initial state
example-circuit-breaker-8-realistic-simulation-guarding = 8. Realistic simulation: guarding operations with can_execute()
example-circuit-breaker-8-1-simulated-sink-writes = 8.1 Simulated sink writes (failure threshold = 3, timeout = 200ms)
example-circuit-breaker-8-2-phase-1-3-consecutive = 8.2 Phase 1: 3 consecutive failures → breaker trips
example-circuit-breaker-write-failed = write #{ $i } failed: { $e }
example-circuit-breaker-request-rejected-by-the-breaker = request #{ $i } rejected by the breaker
example-circuit-breaker-state-failure-count = state = { $value }, failure_count = { $value2 }
example-circuit-breaker-8-3-phase-2-fast-fail-in-open = 8.3 Phase 2: fast fail in Open (write_op not called)
example-circuit-breaker-request-allowed-to-execute = request #{ $i } allowed to execute
example-circuit-breaker-request-fast-rejected-by-the = request #{ $i } fast-rejected by the breaker (protecting the failed sink)
example-circuit-breaker-8-4-phase-3-waiting-250ms-past = 8.4 Phase 3: waiting 250ms past the timeout → HalfOpen
example-circuit-breaker-probe-request-allowed-after = probe request allowed after timeout, state = { $value }
example-circuit-breaker-8-5-phase-4-2-accumulated = 8.5 Phase 4: 2 accumulated successes in HalfOpen → back to Closed
example-circuit-breaker-probe-succeeded-state = probe #{ $i } succeeded, state = { $value }
example-circuit-breaker-probe-failed-state = probe #{ $i } failed, state = { $value }
example-circuit-breaker-8-6-full-state-machine = 8.6 Full state machine transition summary
example-circuit-breaker-closed-3-failures-open-timeout = Closed ──3 failures──► Open ──timeout──► HalfOpen ──2 successes──► Closed
example-circuit-breaker-sink-recovered-breaker-back-to = ✓ Sink recovered; breaker back to normal mode

# fallback
example-fallback-degrading = [{ $value }] degrading: { $active_sink } -> { $new_sink }
example-fallback-example-1-multi-sink-setup = Example 1: multi-sink setup (console + file)
example-fallback-log-file-path = Log file path: { $log_path }
example-fallback-console-sink-configured = Console sink: configured
example-fallback-file-sink-configured = File sink: configured
example-fallback-writing-logs-to-multiple-sinks = writing logs to multiple sinks
example-fallback-console-file = { $level_marker } -> Console + File: { $message }
example-fallback-verifying-the-file-sink = verifying the file sink
example-fallback-file-content-bytes =
    File content ({ $value } bytes):
    { $content }
example-fallback-multi-sink-setup-completed = ✓ Multi-sink setup completed
example-fallback-console-sink-real-time-output = - Console sink: real-time output to the console
example-fallback-file-sink-persist-to-a-file = - File sink: persist to a file
example-fallback-both-work-simultaneously = - Both work simultaneously without interfering
example-fallback-example-2-failure-simulation = Example 2: failure simulation and fallback switch
example-fallback-fallback-log-file = Fallback log file: { $backup_path }
example-fallback-initial-state-primary-sink = Initial state: primary sink = console
example-fallback-fallback-file-sink-ready = Fallback file sink: ready
example-fallback-writing-logs-triggering-failure = writing logs (triggering failure)
example-fallback-primary-sink-write-succeeded = [{ $value }] [primary sink] write succeeded #{ $success_count }
example-fallback-primary-sink-failure = *** Primary sink failure: { $e } ***
example-fallback-fallback-sink-took-over-writing = [{ $value }] [fallback sink] took over writing
example-fallback-degradation-event-statistics = degradation event statistics
example-fallback-degradation-count = degradation count: { $fallback_count }
example-fallback-last-degradation-time = last degradation time: { $value }
example-fallback-active-sinks = active sinks: { $active_sink }
example-fallback-verifying-the-fallback-file = verifying the fallback file
example-fallback-fallback-file-content-bytes = Fallback file content ({ $value } bytes):
example-fallback-failure-simulation-and-fallback = ✓ Failure simulation and fallback switch completed
example-fallback-primary-sink-fails-after-write = - Primary sink fails after write #{ $value }
example-fallback-fallback-sink-took-over-the = - Fallback sink took over the next { $value } logs
example-fallback-example-3-full-fallback = Example 3: full fallback strategy demo
example-fallback-primary-log-file = Primary log file: { $primary_path }
example-fallback-fallback-log-file-2 = Fallback log file: { $fallback_path }
example-fallback-sink-health-status-types = Sink health status types
example-fallback-sinkstatus-enum = SinkStatus enum:
example-fallback-healthy-sink-fully-healthy = - Healthy: sink fully healthy
example-fallback-degraded-sink-degraded-may-need = - Degraded: sink degraded, may need fallback
example-fallback-unhealthy-sink-unavailable = - Unhealthy: sink unavailable, fallback required
example-fallback-sinkhealth-struct = SinkHealth struct:
example-fallback-is-healthy-bool-whether-healthy = - is_healthy: bool - whether healthy
example-fallback-status-sinkstatus-detailed = - status: SinkStatus - detailed status
example-fallback-last-error-option-string-error = - last_error: Option<String> - error info
example-fallback-simulated-health-check-result = simulated health check result
example-fallback-advice-degrade-to-the-fallback = -> Advice: degrade to the fallback sink
example-fallback-degradation-flow-simulation = degradation flow simulation
example-fallback-primary-sink-failed = [{ $value }] *** Primary sink failed ***
example-fallback-degrading-file-primary-file = Degrading: File(primary) -> File(fallback)
example-fallback-file-content-after-degradation = file content after degradation
example-fallback-primary-file-first-logs = --- Primary file (first { $value } logs) ---
example-fallback-fallback-file-logs-after = --- Fallback file (logs after takeover) ---
example-fallback-fallback-strategy-summary = fallback strategy summary
example-fallback-three-tier-fallback = Three-tier fallback:
example-fallback-1-database-file-when-the = 1. Database -> File (when the database is unavailable)
example-fallback-trigger-connection-timeout-sql = - Trigger: connection timeout, SQL errors
example-fallback-degradation-action-write-to-a = - Degradation action: write to a local file
example-fallback-2-file-console-when-the-file-is = 2. File -> Console (when the file is unavailable)
example-fallback-trigger-disk-full-permission = - Trigger: disk full, permission errors
example-fallback-degradation-action-output-to = - Degradation action: output to the console
example-fallback-3-console-final-fallback = 3. Console (final fallback)
example-fallback-trigger-all-sinks-unavailable = - Trigger: all sinks unavailable
example-fallback-degradation-action-log-to = - Degradation action: log to syslog + alert
example-fallback-degradation-event-log = Degradation event log:
example-fallback-all-degradation-events-are = - All degradation events are logged
example-fallback-includes-degradation-reason = - Includes degradation reason, time, and recovery state
example-fallback-degradation-state-queryable-via = - Degradation state queryable via the HTTP endpoint
example-fallback-full-fallback-strategy-demo = ✓ Full fallback strategy demo completed
example-fallback-cleaned-up-temp-files = Cleaned up { $deleted_count } temp files
example-fallback-inklog-sink-fallback-mechanism = === inklog sink fallback mechanism example ===
example-fallback-all-examples-completed = === All examples completed ===

# log-adapter
example-log-adapter-inklog-log-crate-adapter-example = === inklog log crate adapter example ===
example-log-adapter-all-log-adapter-examples = ✓ All log adapter examples completed
example-log-adapter-1-install-loglogger-and-use-log = 1. Install LogLogger and use log macros
example-log-adapter-loglogger-installed-max-level = ✓ LogLogger installed (max_level = Info)
example-log-adapter-1-1-using-log-info-warn-error = 1.1 Using log::info! / warn! / error! macros
example-log-adapter-1-2-records-received-from-the = 1.2 Records received from the console channel
example-log-adapter-1-3-records-received-from-the = 1.3 Records received from the async channel
example-log-adapter-verified-each-channel-received = ✓ Verified: each channel received 3 records (info/warn/error)
example-log-adapter-2-levelfilter-filtering-behavior = 2. LevelFilter filtering behavior
example-log-adapter-loglogger-new-adapter = LogLogger::new(adapter, LevelFilter::Info) sets max_level = Info
example-log-adapter-log-info-warn-error-pass-through = → log::info! / warn! / error! pass through
example-log-adapter-log-debug-trace-filtered-out = → log::debug! / trace! filtered out
example-log-adapter-logadapter-enabled-impl = LogAdapter::enabled() impl: metadata.level() <= log::max_level()
example-log-adapter-level-enum-order-error-warn = Level enum order: Error < Warn < Info < Debug < Trace
example-log-adapter-level-target-message = [{ $name }] level={ $level }, target={ $target }, message={ $message }
example-log-adapter-channel-received-no-records = ({ $name } channel received no records)
example-log-adapter-channel-received-no-records-the = ({ $name } channel received no records; the global logger is not installed on this adapter)
example-log-adapter-received-records-in-total = Received { $count } { $name } records in total

# log-level
example-log-level-inklog-loglevel-type-example = inklog LogLevel type example
example-log-level-all-loglevel-examples-completed = All LogLevel examples completed.
example-log-level-example-1-the-loglevel-from-str = Example 1: the LogLevel::from_str static method
example-log-level-case-insensitive-parsing = Case-insensitive parsing:
example-log-level-none = { $input } → None
example-log-level-invalid-input-returns-none = Invalid input returns None:
example-log-level-example-2-the-fromstr-trait = Example 2: the FromStr trait (returns Result)
example-log-level-using-parse-loglevel = Using .parse::<LogLevel>():
example-log-level-parse-ok = { $input }.parse() → Ok({ $level })
example-log-level-error-handling = Error handling:
example-log-level-parse-err = { $input }.parse() → Err("{ $e }")
example-log-level-example-3-ordering-ord = Example 3: ordering (Ord / PartialOrd)
example-log-level-levels-low-to-high-higher-value = Levels low to high (higher value = more severe):
example-log-level-comparison-operators = Comparison operators:
example-log-level-error-debug = Error > Debug  → { $value }
example-log-level-info-warn = Info  <= Warn  → { $value }
example-log-level-trace-fatal = Trace < Fatal  → { $value }
example-log-level-error-error = Error == Error → { $value }
example-log-level-practical-use-level-filtering = Practical use: level filtering (record only >= WARN)
example-log-level-example-4-the-display-trait = Example 4: the Display trait
example-log-level-consistency-check-against-as-str = Consistency check against as_str():
example-log-level-display-as-str-match = { $level }: Display="{ $display }", as_str="{ $as_str }", match={ $consistent }
example-log-level-example-5-helper-methods-as-str = Example 5: helper methods as_str / as_short_str
example-log-level-short-format-use-case-compact = Short format use case: compact console output
example-log-level-inf-2026-01-01-service-started = [INF] 2026-01-01 service started
example-log-level-wrn-2026-01-01-missing-config = [WRN] 2026-01-01 missing config entry, using defaults
example-log-level-err-2026-01-01-database = [ERR] 2026-01-01 database connection failed
example-log-level-example-6-error-handling = Example 6: error handling (LogLevelParseError)
example-log-level-input = input: { $value }
example-log-level-error-type = error type: { $e }
example-log-level-error-message = error message: { $msg }
example-log-level-input-included = input included: { $contains_input }

# metrics
example-metrics-inklog-health-monitoring-and = === inklog health monitoring and metrics example ===
example-metrics-all-health-monitoring-and = ✓ All health monitoring and metrics examples completed
example-metrics-1-metrics-creation-and-basic = 1. Metrics creation and basic counters
example-metrics-1-1-creating-an-instance-with = 1.1 Creating an instance with Metrics::new()
example-metrics-initial-logs-written = initial logs_written = { $value }
example-metrics-initial-logs-dropped = initial logs_dropped = { $value }
example-metrics-initial-sink-errors = initial sink_errors  = { $value }
example-metrics-initial-channel-blocked = initial channel_blocked = { $value }
example-metrics-1-2-inc-logs-written = 1.2 inc_logs_written() accumulates log writes
example-metrics-write-cumulative = write #{ $value }, cumulative = { $value2 }
example-metrics-1-3-inc-logs-dropped-inc-sink = 1.3 inc_logs_dropped() / inc_sink_error() / inc_channel_blocked()
example-metrics-logs-dropped = logs_dropped   = { $value }
example-metrics-sink-errors = sink_errors    = { $value }
example-metrics-channel-blocked = channel_blocked = { $value }
example-metrics-1-4-db-batch-metrics-set-db = 1.4 DB batch metrics set_db_batch_size / add_db_batch_records_total
example-metrics-db-batch-size = db_batch_size           = { $value }
example-metrics-db-batch-records-total = db_batch_records_total  = { $value }
example-metrics-1-5-lock-contention-counter = 1.5 Lock contention counter (active_workers is internal; read-only externally)
example-metrics-active-workers-default-0-set-by = active_workers   = { $value } (default 0, set by internal workers)
example-metrics-lock-contention = lock_contention  = { $value }
example-metrics-1-6-uptime-runtime-duration = 1.6 uptime() runtime duration
example-metrics-uptime = uptime = { $uptime }
example-metrics-2-latency-histogram-and = 2. Latency histogram and percentiles
example-metrics-2-1-record-latency-records = 2.1 record_latency() records several latencies
example-metrics-recorded-latency-samples = recorded { $value } latency samples
example-metrics-2-2-latency-aggregates-avg-p50 = 2.2 Latency aggregates (avg / p50 / p95 / p99)
example-metrics-avg-latency-us = avg_latency_us = { $avg_latency_us }
example-metrics-p50-latency-us = p50_latency_us = { $p50_latency_us }
example-metrics-p95-latency-us = p95_latency_us = { $p95_latency_us }
example-metrics-p99-latency-us = p99_latency_us = { $p99_latency_us }
example-metrics-latency-distribution = latency_distribution = { $latency_distribution }
example-metrics-3-gaugef64-floating-point-metric = 3. GaugeF64 floating-point metric
example-metrics-3-1-gaugef64-new-set-get = 3.1 GaugeF64::new() / set() / get()
example-metrics-initial-value = initial value = { $value }
example-metrics-after-set-87-5 = after set(87.5) = { $value }
example-metrics-after-set-99-9 = after set(99.9) = { $value }
example-metrics-3-2-the-pool-hit-rate-metric = 3.2 The pool_hit_rate metric exposed by Metrics
example-metrics-pool-hit-rate = pool_hit_rate = { $value }%
example-metrics-pool-hit-rate-after-reset = pool_hit_rate after reset = { $value }%
example-metrics-3-3-sinkhealth-healthy = 3.3 SinkHealth::healthy() / unhealthy() constructors
example-metrics-healthy-status = healthy.status = { $status }
example-metrics-unhealthy-status = unhealthy.status = { $status }
example-metrics-4-sinkstatus-transitions = 4. SinkStatus transitions
example-metrics-4-1-default-state-notstarted = 4.1 Default state NotStarted
example-metrics-status = status = { $status }
example-metrics-4-2-notstarted-healthy = 4.2 NotStarted → Healthy
example-metrics-4-5-unhealthy-healthy-recovery = 4.5 Unhealthy → Healthy (recovery)
example-metrics-4-6-state-machine-summary = 4.6 State machine summary
example-metrics-notstarted-is-operational = NotStarted.is_operational()  = { $value }
example-metrics-healthy-is-operational = Healthy.is_operational()     = { $value }
example-metrics-degraded-is-operational = Degraded.is_operational()    = { $value }
example-metrics-unhealthy-is-operational = Unhealthy.is_operational()   = { $value }
example-metrics-5-metrics-sink-health-tracking = 5. Metrics sink health tracking
example-metrics-5-1-sink-started-marks-a-sink = 5.1 sink_started() marks a sink as started
example-metrics-failures = { $name } -> { $status }, failures = { $consecutive_failures }
example-metrics-5-2-sink-degraded-marks = 5.2 sink_degraded() marks degradation
example-metrics-file-status = file.status = { $status }
example-metrics-file-last-error = file.last_error = { $last_error }
example-metrics-5-3-update-sink-health = 5.3 update_sink_health() accumulates the failure count
example-metrics-database-status = database.status = { $status }
example-metrics-database-consecutive-failures = database.consecutive_failures = { $consecutive_failures }
example-metrics-5-4-update-sink-health-resets = 5.4 update_sink_health() resets the count on recovery
example-metrics-5-5-update-sink-health-uses-a = 5.5 update_sink_health() uses a default message when no error info exists
example-metrics-file-status-error = file.status.error = { $error }
example-metrics-6-healthstatus-overall-snapshot = 6. HealthStatus overall snapshot
example-metrics-6-1-empty-sinks-notstarted = 6.1 Empty sinks → NotStarted
example-metrics-overall-status = overall_status = { $overall_status }
example-metrics-6-2-all-healthy = 6.2 All Healthy
example-metrics-channel-usage = channel_usage  = { $value }%
example-metrics-uptime-seconds = uptime_seconds = { $uptime_seconds }
example-metrics-encryption-key-valid = encryption_key_valid = { $encryption_key_valid }
example-metrics-6-3-degraded-present-no = 6.3 Degraded present (no Unhealthy)
example-metrics-6-4-unhealthy-present = 6.4 Unhealthy present
example-metrics-error = error = { $error }
example-metrics-6-5-healthstatus-fields-at-a = 6.5 HealthStatus fields at a glance
example-metrics-sinks-count = sinks count = { $value }
example-metrics-metrics-logs-written = metrics.logs_written     = { $logs_written }
example-metrics-metrics-sink-errors = metrics.sink_errors      = { $sink_errors }
example-metrics-metrics-active-workers = metrics.active_workers   = { $active_workers }
example-metrics-metrics-pool-hit-rate = metrics.pool_hit_rate    = { $pool_hit_rate }
example-metrics-pool-stats = pool_stats = { $pool_stats }
example-metrics-7-sinkhealthmonitor-smart = 7. SinkHealthMonitor smart fallback and recovery
example-metrics-7-1-sinkhealthmonitor-default = 7.1 SinkHealthMonitor::default() initial state
example-metrics-database-initial-state = database initial state = { $value }
example-metrics-7-2-failures-below-threshold = 7.2 Failures below threshold → Retry
example-metrics-action = action = { $action }
example-metrics-requires-action = requires_action = { $value }
example-metrics-sink-name = sink_name = { $value }
example-metrics-7-3-threshold-reached-fallback = 7.3 Threshold reached → Fallback to file
example-metrics-database-state = database state = { $state }
example-metrics-target = target = { $target }
example-metrics-reason = reason = { $reason }
example-metrics-7-4-healthy-again = 7.4 Healthy again → AttemptRecovery (exponential backoff)
example-metrics-delay-ms = delay_ms = { $delay_ms }
example-metrics-7-5-healthy-again-during = 7.5 Healthy again during recovery → Wait
example-metrics-7-6-confirm-recovery-confirms = 7.6 confirm_recovery() confirms recovery
example-metrics-database-state-2 = database state = { $value }
example-metrics-7-7-filesink-disk-full-fallback = 7.7 FileSink disk full → fallback to console
example-metrics-file-state = file state = { $state }
example-metrics-7-8-encryption-error-fallback = 7.8 Encryption error → fallback to plaintext
example-metrics-7-9-custom-fallbackconfig = 7.9 Custom FallbackConfig + FallbackStats
example-metrics-stats = stats = { $stats }
example-metrics-7-10-reset-resets-the-monitor = 7.10 reset() resets the monitor
example-metrics-is-any-in-fallback-after-reset = is_any_in_fallback after reset = { $value }
example-metrics-7-11-get-fallback-events-event = 7.11 get_fallback_events() event history
example-metrics-event-count = event count = { $value }
example-metrics-first-event-sink = first event sink = { $value }
example-metrics-first-event-from = first event from = { $value }
example-metrics-first-event-to = first event to   = { $value }
example-metrics-8-prometheus-format-export = 8. Prometheus format export
example-metrics-8-1-simulated-metric-collection = 8.1 Simulated metric collection
example-metrics-8-2-export-prometheus-output = 8.2 export_prometheus() output (key lines excerpted)
example-metrics-8-3-healthstatus-serialization = 8.3 HealthStatus serialization preview (serde_json)
example-metrics-truncated = ... (truncated)
example-metrics-prometheus-export-has = ✓ Prometheus export has { $value } characters, { $value2 } lines

# object-pool
example-object-pool-inklog-object-pool-example = inklog object pool example
example-object-pool-all-object-pool-examples = All object pool examples completed.
example-object-pool-example-1-basic-objectpool = Example 1: basic ObjectPool operations
example-object-pool-1-1-new-defaults-capacity-1024 = 1.1 new() defaults (capacity 1024)
example-object-pool-default-pool-built = default pool built
example-object-pool-1-2-custom-config-via-with = 1.2 Custom config via with_config(ObjectPoolConfig)
example-object-pool-custom-config-pool-built = custom config pool built
example-object-pool-1-3-put-get-access = 1.3 put() / get() access
example-object-pool-greeting = greeting = { $greeting }
example-object-pool-name = name     = { $name }
example-object-pool-version = version  = { $version }
example-object-pool-missing = missing  = { $missing }
example-object-pool-all-access-assertions-passed = ✓ All access assertions passed
example-object-pool-1-4-overwriting = 1.4 Overwriting
example-object-pool-overwriting-greeting-hello-world = overwriting greeting: hello → world
example-object-pool-overwrite-verified = ✓ Overwrite verified
example-object-pool-1-5-explicit-error-propagation = 1.5 Explicit error propagation (Result return)
example-object-pool-put-get-both-return-ok-errors = put/get both return Ok; errors propagate explicitly
example-object-pool-example-2-custom = Example 2: custom ObjectPoolConfig (with TTL)
example-object-pool-pool-with-ttl-60s-built-and = pool with TTL=60s built and accessed successfully
example-object-pool-small-pool-4-filled-with-4 = small pool (4) filled with 4 objects
example-object-pool-small-capacity-pool-fully = ✓ Small-capacity pool fully verified
example-object-pool-example-3-get-log-record-put = Example 3: get_log_record / put_log_record global functions
example-object-pool-3-1-get-log-record-fetches-a = 3.1 get_log_record() fetches a LogRecord
example-object-pool-logrecord-from-the-global-pool = LogRecord from the global pool: level = { $level }
example-object-pool-3-2-get-log-record-again = 3.2 get_log_record() again verifies reset
example-object-pool-fetched-again-level = fetched again: level = { $level }
example-object-pool-10-get-put-rounds-completed = 10 get/put rounds completed without panic
example-object-pool-global-logrecord-pool-verified = ✓ Global LogRecord pool verified
example-object-pool-example-4-get-string-buffer-put = Example 4: get_string_buffer / put_string_buffer global functions
example-object-pool-4-1-get-string-buffer-fetches-a = 4.1 get_string_buffer() fetches a String
example-object-pool-string-from-the-global-pool-len = String from the global pool: len = { $value }
example-object-pool-filled-inklog-string-buffer-and = filled 'inklog string buffer' and returned it to the global pool
example-object-pool-4-2-get-string-buffer-again = 4.2 get_string_buffer() again
example-object-pool-fetched-again-len = fetched again: len = { $value }
example-object-pool-5-get-put-rounds-completed = 5 get/put rounds completed
example-object-pool-global-string-pool-verified = ✓ Global String pool verified
example-object-pool-example-5-edge-cases = Example 5: edge cases
example-object-pool-empty-key-access-works = ✓ Empty-key access works
example-object-pool-empty-value-access-works = ✓ Empty-value access works
example-object-pool-repeated-put-keeps-the-last = ✓ Repeated put keeps the last value
example-object-pool-all-100-key-value-pairs = ✓ All 100 key-value pairs accessed correctly
example-object-pool-example-6-error-handling-checks = Example 6: error handling checks
example-object-pool-put-succeeded-ok = ✓ put succeeded: Ok(())
example-object-pool-put-failed = ✗ put failed: { $e }
example-object-pool-get-succeeded-ok-some = ✓ get succeeded: Ok(Some({ $v }))
example-object-pool-get-returned-ok-none-key-absent = get returned: Ok(None) — key absent
example-object-pool-get-failed = ✗ get failed: { $e }
example-object-pool-unexpected-key-exists = unexpected: key exists
example-object-pool-get-of-a-missing-key-ok-none-as = ✓ get of a missing key: Ok(None) — as expected
example-object-pool-operator-error-propagation-works = ✓ ? operator error propagation works
example-object-pool-best-practices = Best practices
example-object-pool-1-when-to-use-an-object-pool = 1. When to use an object pool:
example-object-pool-when-creating-destroying-the = - When creating/destroying the same type at high frequency (e.g. log pipelines)
example-object-pool-reduces-gc-pressure-and = - Reduces GC pressure and allocation overhead
example-object-pool-typical-uses-logrecord-reuse = - Typical uses: LogRecord reuse, String buffer reuse
example-object-pool-2-capacity-config = 2. Capacity config:
example-object-pool-max-capacity-size-by = - max_capacity: size by concurrency (threads × per-thread cache recommended)
example-object-pool-too-small-objects-are-created = - Too small: objects are created constantly, defeating pooling
example-object-pool-too-large-wastes-memory-at-most = - Too large: wastes memory; at most 2x the real need is advised
example-object-pool-3-ttl-config = 3. TTL config:
example-object-pool-ttl-secs-maximum-lifetime-of-an = - ttl_secs: maximum lifetime of an object in the pool
example-object-pool-suits-stateful-objects-that = - Suits stateful objects that need periodic cleanup
example-object-pool-none-never-expires-suits = - None: never expires (suits stateless objects like String)
example-object-pool-4-global-functions-vs-direct-api = 4. Global functions vs direct API:
example-object-pool-get-log-record-put-log-record = - get_log_record/put_log_record: thread-local pool, zero lock contention
example-object-pool-objectpool-new-shared-pool = - ObjectPool::new(): shared pool, shared across threads
example-object-pool-prefer-the-global-functions-in = - Prefer the global functions in single-threaded scenarios
example-object-pool-use-an-objectpool-instance-for = - Use an ObjectPool instance for multi-threaded sharing
example-object-pool-5-usage-patterns = 5. Usage patterns:
example-object-pool-always-pair-get-put-to-avoid = - Always pair get/put to avoid leaking objects
example-object-pool-objects-are-reset-on-put-do-not = - Objects are reset on put; do not keep referencing old values
example-object-pool-do-not-keep-using-a-reference = - Do not keep using a reference returned by get after put

# output-format
example-output-format-inklog-output-format-example = inklog output format example
example-output-format-all-output-format-examples = ✓ All output format examples completed
example-output-format-example-1-outputformat-enum = Example 1: OutputFormat enum variants
example-output-format-outputformat-text = OutputFormat::Text    = { $text }
example-output-format-outputformat-json = OutputFormat::Json    = { $json }
example-output-format-outputformat-default = OutputFormat::default() = { $default }
example-output-format-default-format-human-readable = Default format: { $default } (human-readable text)
example-output-format-equivalence-check = Equivalence check:
example-output-format-text-text = Text == Text → { $is_text_equal }
example-output-format-text-json = Text == Json → { $value }
example-output-format-example-2-fromstr-string-parsing = Example 2: FromStr string parsing
example-output-format-string-outputformat-parsing = String → OutputFormat parsing:
example-output-format-error = { $s } → error: { $e }
example-output-format-error-invalid-value = { $value } → error (invalid value): { $value2 }
example-output-format-example-3-display-trait-output = Example 3: Display trait output
example-output-format-outputformat-string = OutputFormat → string:
example-output-format-outputformat-text-2 = OutputFormat::Text → "{ $value }"
example-output-format-outputformat-json-2 = OutputFormat::Json → "{ $value }"
example-output-format-example-4-text-rendering-default = Example 4: Text rendering (default)
example-output-format-logtemplate-default-rendering = LogTemplate::default() rendering:
example-output-format-characteristics = Characteristics:
example-output-format-human-readable-suits-consoles = - Human-readable; suits consoles and log files
example-output-format-contains-timestamp-level-target = - Contains timestamp, level, target, message
example-output-format-structured-fields-appended-as = - Structured fields appended as JSON at the end
example-output-format-example-5-json-rendering-ndjson = Example 5: Json rendering (NDJSON)
example-output-format-ndjson-output = NDJSON output:
example-output-format-machine-parseable-suits-log = - Machine-parseable; suits log collection systems
example-output-format-one-complete-json-object-per = - One complete JSON object per line
example-output-format-directly-collectable-by-fluentd = - Directly collectable by Fluentd/Logstash/Promtail
example-output-format-supports-nested-structured = - Supports nested structured fields
example-output-format-json-output-parses-correctly = ✓ JSON output parses correctly with standard parsers
example-output-format-example-6-outputformat-in-sink = Example 6: OutputFormat in sink configs
example-output-format-console-sink-config-json-format = Console sink config (JSON format):
example-output-format-output-format = output_format = { $output_format }
example-output-format-file-sink-config-text-format = File sink config (Text format):
example-output-format-toml-config-example = TOML config example:
example-output-format-7-best-practices = 7. Best practices
example-output-format-outputformat-selection-guide = OutputFormat selection guide:
example-output-format-1-text-format-use-cases = 1. Text format use cases:
example-output-format-development-console-output = - Development console output
example-output-format-log-files-read-directly-by = - Log files read directly by humans
example-output-format-debugging-and-troubleshooting = - Debugging and troubleshooting
example-output-format-2-json-format-use-cases = 2. Json format use cases:
example-output-format-production-structured-log = - Production structured log collection
example-output-format-log-analytics-platforms-elk = - Log analytics platforms (ELK/Loki/Datadog)
example-output-format-suits-field-level-querying-and = - Suits field-level querying and filtering
example-output-format-3-mixing-formats = 3. Mixing formats:
example-output-format-console-text-easy-for = - Console → Text (easy for developers to read)
example-output-format-file-json-easy-to-collect-and = - File → Json (easy to collect and analyze)
example-output-format-database-structured-fields = - Database → structured fields stored directly
example-output-format-4-performance-considerations = 4. Performance considerations:
example-output-format-text-format-renders-faster = - Text format renders faster (simple string concatenation)
example-output-format-json-format-needs-serialization = - Json format needs serialization (~2-3x overhead)
example-output-format-for-high-throughput-use-text = - For high throughput, use Text for files and convert to Json at the collector
example-output-format-from-str-parsed-check = { $s } → { $fmt } ✓

# performance
example-performance-inklog-performance-test-example = === inklog performance test example ===
example-performance-all-performance-tests-completed = ✓ All performance tests completed
example-performance-1-console-sink-throughput-test = 1. Console sink throughput test
example-performance-1-1-throughput-test-5-000-logs = 1.1 Throughput test (5,000 logs)
example-performance-total-logs = Total logs:   { $count }
example-performance-total-time = Total time:   { $value }
example-performance-throughput-logs-s = Throughput:   { $throughput } logs/s
example-performance-avg-latency = Avg latency:  { $value }
example-performance-2-file-sink-throughput-test = 2. File sink throughput test
example-performance-2-1-throughput-test-2-000-logs = 2.1 Throughput test (2,000 logs)
example-performance-note-file-sink-throughput = Note: file sink throughput depends on channel capacity and worker thread count
example-performance-current-throughput-reflects = Current throughput reflects async write overhead
example-performance-warning-failed-to-delete-temp = Warning: failed to delete temp file: { $log_path } ({ $e })
example-performance-cleaned-up-temp-file = ✓ Cleaned up temp file: { $log_path }
example-performance-3-latency-analysis-p50-p95-p99 = 3. Latency analysis (P50/P95/P99)
example-performance-3-1-single-log-latency = 3.1 Single-log latency measurement (500 samples)
example-performance-total-samples = Total samples:  { $count }
example-performance-p50-latency = P50 latency:  { $value }
example-performance-p95-latency = P95 latency:  { $value }
example-performance-p99-latency = P99 latency:  { $value }
example-performance-actual-numbers-depend-on-system = Actual numbers depend on system load and channel state
example-performance-4-backpressure-test = 4. Backpressure test
example-performance-channel-capacity = Channel capacity: { $capacity }
example-performance-enqueue-time = Enqueue time:     { $value }
example-performance-flush-time = Flush time:       { $value }
example-performance-total-time-2 = Total time:       { $value }
example-performance-throughput-logs-s-2 = Throughput:       { $throughput } logs/s
example-performance-note-throughput-includes-total = Note: throughput includes total log processing and flush time
example-performance-a-small-channel-may-cause = ⚠️  A small channel may cause backpressure
example-performance-capacity-is-sufficient-no = ✓  Capacity is sufficient; no obvious backpressure
example-performance-5-concurrency-test = 5. Concurrency test
example-performance-5-1-multi-task-concurrent = 5.1 Multi-task concurrent writes (4 tasks × 100 logs)
example-performance-concurrent-tasks = Concurrent tasks: { $task_count }
example-performance-logs-per-task = Logs per task:  { $logs_per_task }
example-performance-total-logs-2 = Total logs:     { $total_logs }
example-performance-total-time-3 = Total time:     { $value }
example-performance-throughput-logs-s-3 = Throughput:     { $throughput } logs/s

# rate-limiter
example-rate-limiter-inklog-rate-limiter-example = inklog rate limiter example
example-rate-limiter-all-rate-limiter-examples = ✓ All rate limiter examples completed
example-rate-limiter-example-1-creating-the-rate = Example 1: creating the rate limiter
example-rate-limiter-ratelimiter-new-1000-1000 = RateLimiter::new(1000) — 1000 tokens per second
example-rate-limiter-initial-state-bucket-full = Initial state: bucket full (tokens = max_tokens = 1000)
example-rate-limiter-refill-rate-1000-tokens-sec = Refill rate: 1000 tokens/sec
example-rate-limiter-try-acquire-true-bucket-full = ✓ try_acquire() → true (bucket full, allowed)
example-rate-limiter-2-rate-limiting-behavior = 2. Rate limiting behavior
example-rate-limiter-ratelimiter-new-5-5-tokens-per = RateLimiter::new(5) — 5 tokens per second
example-rate-limiter-rapid-consecutive-try-acquire = Rapid consecutive try_acquire():
example-rate-limiter-result-allowed-rejected = Result: { $allowed } allowed, { $rejected } rejected
example-rate-limiter-token-bucket-rate-limits = ✓ Token bucket rate-limits correctly
example-rate-limiter-example-3-dropped-count = Example 3: dropped_count() statistics
example-rate-limiter-ratelimiter-new-2-2-tokens-per = RateLimiter::new(2) — 2 tokens per second
example-rate-limiter-after-consuming-2-tokens = After consuming 2 tokens: dropped_count = { $value }
example-rate-limiter-after-3-rejections-dropped-count = After 3 rejections: dropped_count = { $value }
example-rate-limiter-dropped-count-correctly-counts = ✓ dropped_count correctly counts rate-limited logs
example-rate-limiter-example-4-automatic-token-refill = Example 4: automatic token refill
example-rate-limiter-ratelimiter-new-100-100-tokens = RateLimiter::new(100) — 100 tokens per second
example-rate-limiter-after-draining-100-tokens = After draining 100 tokens:
example-rate-limiter-try-acquire-false-bucket-empty = try_acquire() → false (bucket empty)
example-rate-limiter-after-waiting-50ms-5-tokens = After waiting 50ms (~5 tokens refilled):
example-rate-limiter-acquired-again-allowed-expected = Acquired again: { $refilled } allowed (expected ~5)
example-rate-limiter-tokens-refill-automatically-at = ✓ Tokens refill automatically at refill_rate
example-rate-limiter-example-5-log-rate-limiting = Example 5: log rate-limiting patterns
example-rate-limiter-scenario-cap-log-output-at-10 = Scenario: cap log output at 10 logs/second
example-rate-limiter-simulating-a-burst-of-logs-20 = Simulating a burst of logs (20):
example-rate-limiter-info-log-written = [INFO] log #{ $i } — written
example-rate-limiter-statistics = Statistics:
example-rate-limiter-written = Written: { $written }
example-rate-limiter-dropped = Dropped: { $dropped }
example-rate-limiter-total-dropped = Total dropped: { $value }
example-rate-limiter-rate-limiting-works-protects = ✓ Rate limiting works: protects downstream systems under bursts
example-rate-limiter-example-6-zero-rate-limiter = Example 6: zero-rate limiter (rate=0)
example-rate-limiter-ratelimiter-new-0-allows-no = RateLimiter::new(0) — allows no logs at all
example-rate-limiter-try-acquire-always-false = try_acquire() → always false
example-rate-limiter-dropped-count = dropped_count = { $value }
example-rate-limiter-rate-0-is-equivalent-to-full = ✓ rate=0 is equivalent to full silence
example-rate-limiter-7-best-practices = 7. Best practices
example-rate-limiter-ratelimiter-usage-advice = RateLimiter usage advice:
example-rate-limiter-1-rate-selection = 1. Rate selection:
example-rate-limiter-development-1000-5000-tokens = - Development: 1000-5000 tokens/sec (loose)
example-rate-limiter-production-100-1000-tokens-sec = - Production: 100-1000 tokens/sec (based on downstream capacity)
example-rate-limiter-failure-scenarios-10-50-tokens = - Failure scenarios: 10-50 tokens/sec (prevents log storms)
example-rate-limiter-2-integrating-with-loggermanager = 2. Integrating with LoggerManager:
example-rate-limiter-3-monitoring-dropped-count = 3. Monitoring dropped_count:
example-rate-limiter-report-dropped-count-to-metrics = - Report dropped_count to Metrics periodically
example-rate-limiter-continuously-growing-dropped = - continuously growing dropped_count means the limit is too strict
example-rate-limiter-dropped-count-0-means-the-limit = - dropped_count = 0 means the limit is loose or there is no pressure
example-rate-limiter-4-performance-characteristics = 4. Performance characteristics:
example-rate-limiter-try-acquire-is-o-1 = - try_acquire() is O(1)
example-rate-limiter-uses-parking-lot-mutex-20ns = - Uses parking_lot::Mutex (~20ns lock overhead)
example-rate-limiter-suits-high-frequency-logging = - Suits high-frequency logging

# runtime-ops
example-runtime-ops-inklog-loggermanager-runtime = inklog LoggerManager runtime operations API example
example-runtime-ops-all-runtime-operations-api = All runtime operations API examples completed.
example-runtime-ops-example-1-get-health-status = Example 1: get_health_status() health state
example-runtime-ops-api-signature = API signature:
example-runtime-ops-healthstatus-fields = HealthStatus fields:
example-runtime-ops-overall-status-sinkstatus = overall_status:      SinkStatus (Healthy/Degraded/Unhealthy/NotStarted)
example-runtime-ops-sinks-hashmap-string-sinkhealth = sinks:               HashMap<String, SinkHealth>
example-runtime-ops-channel-usage-f64-0-0-1-0 = channel_usage:       f64 (0.0 ~ 1.0)
example-runtime-ops-uptime-seconds-u64 = uptime_seconds:      u64
example-runtime-ops-metrics-metricssnapshot = metrics:             MetricsSnapshot
example-runtime-ops-pool-stats-option-poolstats = pool_stats:          Option<PoolStats>
example-runtime-ops-encryption-key-valid-bool = encryption_key_valid: bool
example-runtime-ops-sinkstatus-enum-values = SinkStatus enum values:
example-runtime-ops-healthy-sink-running-normally = Healthy             - sink running normally
example-runtime-ops-notstarted-sink-not-started-yet = NotStarted          - sink not started yet
example-runtime-ops-use-cases = Use cases:
example-runtime-ops-poll-health-status-periodically = - Poll health status periodically (e.g. every 30 seconds)
example-runtime-ops-integrate-into-the-health-http = - Integrate into the /health HTTP endpoint
example-runtime-ops-trigger-alert-notifications = - Trigger alert notifications
example-runtime-ops-code-example = Code example:
example-runtime-ops-example-2-channel-capacity-and = Example 2: channel capacity and queue length
example-runtime-ops-config-defaults = Config defaults:
example-runtime-ops-performance-channel-capacity = performance.channel_capacity = { $channel_capacity } (default)
example-runtime-ops-performance-worker-threads = performance.worker_threads   = { $worker_threads } (default)
example-runtime-ops-monitor-channel-usage-to-warn = - Monitor channel usage to warn about backpressure
example-runtime-ops-channel-len-effective-channel = - channel_len / effective_channel_capacity > 0.8 → alert
example-runtime-ops-adjust-channel-capacity = - Adjust channel_capacity dynamically (requires rebuilding LoggerManager)
example-runtime-ops-example-3-sink-recovery = Example 3: sink recovery operations
example-runtime-ops-recover-sink-usage = recover_sink() usage:
example-runtime-ops-manually-recover-a-sink-by-name = - Manually recover a sink by name
example-runtime-ops-sink-names-come-from-the-config = - Sink names come from the config (e.g. "console", "file", "database")
example-runtime-ops-returns-err-inklogerror-when = - Returns Err(InklogError) when recovery fails
example-runtime-ops-trigger-recovery-for-unhealthy = trigger_recovery_for_unhealthy_sinks() usage:
example-runtime-ops-automatically-scan-all = - Automatically scan all unhealthy sinks
example-runtime-ops-try-to-recover-every-unhealthy = - Try to recover every unhealthy sink
example-runtime-ops-returns-the-list-of = - Returns the list of successfully recovered sink names
example-runtime-ops-recovery-strategies = Recovery strategies:
example-runtime-ops-console-sink-usually-needs-no = - Console sink: usually needs no recovery (writes straight to stdout)
example-runtime-ops-file-sink-reopen-the-file-handle = - File sink: reopen the file handle
example-runtime-ops-database-sink-rebuild-the = - Database sink: rebuild the connection pool
example-runtime-ops-example-4-graceful-shutdown = Example 4: graceful shutdown with shutdown()
example-runtime-ops-shutdown-behavior = shutdown() behavior:
example-runtime-ops-1-stop-accepting-new-log-records = 1. Stop accepting new log records
example-runtime-ops-2-wait-for-all-logs-in-the = 2. Wait for all logs in the channel to be written to sinks
example-runtime-ops-3-close-all-sinks-file-handles = 3. Close all sinks (file handles, database connections, etc.)
example-runtime-ops-4-stop-the-http-server-if = 4. Stop the HTTP server (if enabled)
example-runtime-ops-5-stop-all-worker-threads = 5. Stop all worker threads
example-runtime-ops-caveats = Caveats:
example-runtime-ops-shutdown-is-synchronous-it = - shutdown() is synchronous; it blocks until all logs are written
example-runtime-ops-shutdown-can-take-a-while-if = - shutdown() can take a while if the channel holds many pending logs
example-runtime-ops-call-shutdown-from-the-app-s = - Call shutdown() from the app's exit signal handler
example-runtime-ops-full-lifecycle-example = Full lifecycle example:

# http
example-http-1-start-the-http-server = 1. Start the HTTP server
example-http-http-server-configuration = HTTP server configuration
example-http-listen-address = Listen address: { $addr }
example-http-endpoints = Endpoints:
example-http-get-health-health-status-as-json = - GET /health  → health status as JSON
example-http-get-metrics-metrics-in = - GET /metrics → metrics in Prometheus format
example-http-http-server-started-on-port = ✓ HTTP server started on port { $port }
example-http-failed-to-handle-connection = failed to handle connection { $addr }: { $e }
example-http-connection-handler-task-aborted = connection { $addr } handler task aborted: { $join_err }
example-http-failed-to-accept-a-connection = failed to accept a connection: { $e }
example-http-shutdown-signal-received = ✓ shutdown signal received, closing the server
example-http-request-read-timed-out-s = request read timed out ({ $value }s), closing the connection
example-http-2-health-check-endpoint-demo = 2. Health check endpoint demo
example-http-send-a-get-health-request = Send a GET /health request
example-http-request-url = Request URL: { $url }
example-http-response-status = Response status: { $status_code }
example-http-content-type = Content-Type: { $content_type }
example-http-response-body-json = Response body (JSON):
example-http-health-status = ✓ Health status: { $overall_status }
example-http-sink = ✓ Sink '{ $name }': { $value }
example-http-health-endpoint-test-passed = ✓ /health endpoint test passed
example-http-3-metrics-endpoint-demo = 3. Metrics endpoint demo
example-http-send-a-get-metrics-request = Send a GET /metrics request
example-http-prometheus-format-metrics-output = Prometheus format metrics output
example-http-key-metrics-summary = Key metrics summary
example-http-metrics-in-total = ... ({ $value } metrics in total)
example-http-metrics-endpoint-test-passed = ✓ /metrics endpoint test passed
example-http-4-error-handling-endpoint-demo = 4. Error handling endpoint demo
example-http-request-a-nonexistent-endpoint = Request a nonexistent endpoint
example-http-response-status-expected-404 = Response status: { $status_code } (expected: 404)
example-http-response-body =
    Response body:
    { $value }
example-http-404-error-handling-test-passed = ✓ 404 error handling test passed
example-http-inklog-http-health-check-and = === inklog HTTP health check and metrics endpoint examples ===
example-http-5-integration-guide = 5. Integration guide
example-http-configure-scraping-in-prometheus = Configure scraping in Prometheus:
example-http-prometheus-yml = # prometheus.yml
example-http-scrape-configs = scrape_configs:
example-http-job-name-inklog = - job_name: 'inklog'
example-http-static-configs = static_configs:
example-http-targets-localhost = - targets: ['localhost:{ $port }']
example-http-then-run-prometheus-config-file = Then run: prometheus --config.file=prometheus.yml
example-http-test-with-curl = Test with curl:
example-http-health-check = # health check
example-http-curl-http-localhost-health-jq = curl http://localhost:{ $port }/health | jq
example-http-prometheus-metrics = # Prometheus metrics
example-http-curl-http-localhost-metrics = curl http://localhost:{ $port }/metrics
example-http-press-ctrl-c-to-exit = Press Ctrl+C to exit...

# http-auth
example-http-auth-inklog-http-auth-and-ip = inklog HTTP auth and IP whitelist examples
example-http-auth-all-http-auth-examples-shown = All HTTP auth examples shown.
example-http-auth-example-1-httpauthconfig-bearer = Example 1: HttpAuthConfig bearer token auth
example-http-auth-default-httpauthconfig = Default HttpAuthConfig:
example-http-auth-enabled-disabled-by-default = enabled   = { $enabled } (disabled by default)
example-http-auth-token-env-environment-variable = token_env = "{ $token_env }" (environment variable name)
example-http-auth-enable-auth = Enable auth:
example-http-auth-enabled = enabled   = { $enabled }
example-http-auth-token-env = token_env = "{ $token_env }"
example-http-auth-how-it-works = How it works:
example-http-auth-2-the-server-reads-the-expected = 2. The server reads the expected token from environment variable { $token_env }
example-http-auth-3-it-compares-the-bearer-token = 3. It compares the bearer token in the Authorization header
example-http-auth-4-a-match-returns-data-a = 4. A match returns data, a mismatch returns 401 Unauthorized
example-http-auth-custom-token-environment = Custom token environment variable:
example-http-auth-token-env-custom = token_env = "{ $token_env }" (custom)
example-http-auth-example-2-ip-whitelist-ip = Example 2: IP whitelist (ip_whitelist)
example-http-auth-supported-formats = Supported formats:
example-http-auth-1-exact-ip-192-168-1-100 = 1. Exact IP:    "192.168.1.100"
example-http-auth-2-cidr-range-10-0-0-0-8 = 2. CIDR range:  "10.0.0.0/8"
example-http-auth-config-example-1-internal = Config example 1: internal network only
example-http-auth-ip-whitelist = ip_whitelist = { $ip_whitelist }
example-http-auth-inklogconfig-validate-passed = ✓ InklogConfig::validate() passed
example-http-auth-the-invalid-192-168-was-rejected = ✓ the invalid "192.168.*.*" was rejected: { $err }
example-http-auth-config-example-2-specific = Config example 2: specific monitoring hosts only
example-http-auth-config-example-3-whitelist = Config example 3: whitelist disabled (all IPs allowed)
example-http-auth-ip-whitelist-none-allow-all = ip_whitelist = { $ip_whitelist } (None = allow all)
example-http-auth-example-3-httperrormode-error = Example 3: HttpErrorMode error handling modes
example-http-auth-a-comparison-of-the-two-modes = A comparison of the two modes (they apply when the HTTP server fails to start):
example-http-auth-missing-or-invalid-auth-401 = missing or invalid auth → 401 Unauthorized
example-http-auth-ip-not-in-the-whitelist-403 = IP not in the whitelist → 403 Forbidden
example-http-auth-default-mode = Default mode:
example-http-auth-httperrormode-default = HttpErrorMode::default() = { $value }
example-http-auth-recommended-usage = Recommended usage:
example-http-auth-dev-test-warn-a-busy-port-or = Dev/test:    Warn  (a busy port or TLS error only warns)
example-http-auth-example-4-full-production-grade = Example 4: full production-grade security config
example-http-auth-production-grade = Production-grade HttpServerConfig:
example-http-auth-enabled-2 = enabled      = { $enabled }
example-http-auth-host-port = host:port    = { $host }:{ $port }
example-http-auth-metrics-path = metrics_path = "{ $metrics_path }"
example-http-auth-health-path = health_path  = "{ $health_path }"
example-http-auth-error-mode = error_mode   = { $error_mode }
example-http-auth-auth-enabled = auth.enabled = { $value }
example-http-auth-auth-token-env = auth.token_env = "{ $value }"
example-http-auth-the-corresponding-toml = The corresponding TOML configuration:
example-http-auth-environment-variables-to-set = Environment variables to set before starting:
example-http-auth-example-5-environment-variable = Example 5: environment variable overrides
example-http-auth-environment-variables-supported = Environment variables supported by the HTTP server:
example-http-auth-load-with-load-with-env = Load with load_with_env_overrides():

# tls-config
example-tls-config-inklog-tls-configuration = inklog TLS configuration examples
example-tls-config-all-tls-configuration-examples = All TLS configuration examples shown.
example-tls-config-example-1-tlsconfig-basics = Example 1: TlsConfig basics
example-tls-config-tlsconfig-fields = TlsConfig fields:
example-tls-config-cert-path-path-to-the-pem = cert_path: path to the PEM encoded certificate file
example-tls-config-key-path-path-to-the-pem = key_path:  path to the PEM encoded private key file
example-tls-config-basic-tlsconfig = Basic TlsConfig:
example-tls-config-cert-path = cert_path = "{ $cert_path }"
example-tls-config-key-path = key_path  = "{ $key_path }"
example-tls-config-certificate-format-requirements = Certificate format requirements:
example-tls-config-certificate-x-509-pem-format = - Certificate: X.509 PEM format (.crt / .pem)
example-tls-config-private-key-pkcs-8-pem-format = - Private key: PKCS#8 PEM format (.key / .pem)
example-tls-config-self-signed-certificates-work = - Self-signed certificates work (development/test)
example-tls-config-in-production-use-a-certificate = - In production use a certificate from Let's Encrypt or a commercial CA
example-tls-config-example-2-an-http-server-with = Example 2: an HTTP server with TLS enabled
example-tls-config-default-configuration-no-tls = Default configuration (no TLS):
example-tls-config-tls-none-tls-disabled = tls = { $tls } (None = TLS disabled)
example-tls-config-http-server-configuration-with = HTTP server configuration with TLS:
example-tls-config-enabled = enabled      = { $enabled }
example-tls-config-host-port = host:port    = { $host }:{ $port }
example-tls-config-tls-cert = tls.cert     = "{ $value }"
example-tls-config-tls-key = tls.key      = "{ $value }"
example-tls-config-how-access-changes = How access changes:
example-tls-config-no-tls-http-localhost-9090 = No TLS: http://localhost:9090/health
example-tls-config-with-tls-https-localhost-9443 = With TLS: https://localhost:9443/health
example-tls-config-curl-k-https-localhost-9443 = curl -k https://localhost:9443/metrics
example-tls-config-example-3-full-production-grade = Example 3: full production-grade TLS config
example-tls-config-production-grade = Production-grade HttpServerConfig (TLS + auth + whitelist):
example-tls-config-error-mode = error_mode   = { $error_mode }
example-tls-config-auth-enabled = auth.enabled = { $value }
example-tls-config-ip-whitelist = ip_whitelist = { $ip_whitelist }
example-tls-config-security-layers = Security layers:
example-tls-config-1-tls-transport-encryption = 1. TLS transport encryption → stops man-in-the-middle attacks
example-tls-config-2-bearer-token-auth-stops = 2. Bearer token auth → stops unauthorized access
example-tls-config-3-ip-whitelist-restricts-access = 3. IP whitelist → restricts access sources
example-tls-config-4-strict-error-mode-fail-closed = 4. Strict error mode → fail closed
example-tls-config-example-4-toml-configuration = Example 4: TOML configuration file format
example-tls-config-the-tls-configuration-inside = The TLS configuration inside inklog.toml:
example-tls-config-load-the-configuration = Load the configuration:
example-tls-config-example-5-tls-best-practices = Example 5: TLS best practices
example-tls-config-certificate-management = Certificate management:
example-tls-config-use-a-let-s-encrypt-or = - Use a Let's Encrypt or commercial CA certificate in production
example-tls-config-a-self-signed-certificate-is = - A self-signed certificate is fine in development:
example-tls-config-openssl-req-x509-newkey-rsa = openssl req -x509 -newkey rsa:4096 -days 365 \
example-tls-config-keyout-inklog-key-out-inklog = -keyout inklog.key -out inklog.crt -nodes \
example-tls-config-subj-cn-localhost = -subj '/CN=localhost'
example-tls-config-rotate-certificates-regularly = - Rotate certificates regularly (90 days is a good target)
example-tls-config-private-key-protection = Private key protection:
example-tls-config-set-the-private-key-file-mode = - Set the private key file mode to 600 (owner only)
example-tls-config-use-chmod-600-etc-ssl-private = - Use chmod 600 /etc/ssl/private/inklog.key
example-tls-config-consider-managing-keys-with = - Consider managing keys with Kubernetes Secrets or Vault
example-tls-config-port-selection = Port selection:
example-tls-config-standard-https-port-443-needs = - Standard HTTPS port: 443 (needs root)
example-tls-config-custom-https-port-8443-9443 = - Custom HTTPS port: 8443 / 9443 (recommended)
example-tls-config-do-not-share-it-with-the-http = - Do not share it with the HTTP port (9090 vs 9443)
example-tls-config-monitoring-and-alerting = Monitoring and alerting:
example-tls-config-alert-on-certificate-expiry-30 = - Alert on certificate expiry (30 days ahead is recommended)
example-tls-config-watch-the-tls-handshake-failure = - Watch the TLS handshake failure rate
example-tls-config-use-the-health-endpoint-to = - Use the /health endpoint to verify availability

# encryption
example-encryption-temporary-file-paths = Temporary file paths:
example-encryption-plaintext = Plaintext: { $plain_path }
example-encryption-encrypted = Encrypted: { $encrypted_path }
example-encryption-environment-variable-cleared = ✓ environment variable cleared: { $key_env }
example-encryption-key-management-best-practices = Key management best practices
example-encryption-in-production-follow-these-best = In production, follow these best practices:
example-encryption-1-key-storage = 1. Key storage:
example-encryption-use-a-managed-key-service-such = - Use a managed key service (such as AWS KMS or HashiCorp Vault)
example-encryption-never-hard-code-keys-in-source = - Never hard-code keys in source code
example-encryption-pass-keys-through-environment = - Pass keys through environment variables or key files
example-encryption-2-key-rotation = 2. Key rotation:
example-encryption-rotate-encryption-keys-regularly = - Rotate encryption keys regularly
example-encryption-keep-old-keys-to-decrypt = - Keep old keys to decrypt historical logs
example-encryption-use-key-versioning = - Use key versioning
example-encryption-3-access-control = 3. Access control:
example-encryption-restrict-who-can-access-keys = - Restrict who can access keys
example-encryption-log-key-usage = - Log key usage
example-encryption-apply-least-privilege = - Apply least privilege
example-encryption-4-secure-transport = 4. Secure transport:
example-encryption-transport-keys-over-tls = - Transport keys over TLS
example-encryption-never-print-keys-in-logs = - Never print keys in logs
example-encryption-use-a-secure-ci-cd-pipeline = - Use a secure CI/CD pipeline
example-encryption-encryption-performance = Encryption performance
example-encryption-aes-256-gcm-performance = AES-256-GCM performance advantages:
example-encryption-1-hardware-acceleration = 1. Hardware acceleration:
example-encryption-modern-cpus-support-the-aes-ni = - Modern CPUs support the AES-NI instruction set
example-encryption-hardware-acceleration-reaches = - Hardware acceleration reaches 10+ GB/s
example-encryption-log-write-performance-is-barely = - Log write performance is barely affected
example-encryption-2-aead-authentication = 2. AEAD authentication:
example-encryption-provides-encryption-and = - Provides encryption and integrity together
example-encryption-detects-tampering-automatically = - Detects tampering automatically
example-encryption-protects-against-ciphertext = - Protects against ciphertext attacks
example-encryption-3-parallelism = 3. Parallelism:
example-encryption-gcm-mode-supports-parallel = - GCM mode supports parallel encryption
example-encryption-uses-multiple-cores-for = - Uses multiple cores for throughput
example-encryption-suits-bulk-log-processing = - Suits bulk log processing
example-encryption-supported-key-formats = Supported key formats
example-encryption-inklog-supports-three-key = inklog supports three key formats:
example-encryption-1-base64-encoded-recommended = 1. Base64 encoded (recommended):
example-encryption-the-base64-encoding-of-a-32 = - The Base64 encoding of a 32-byte key
example-encryption-example = - Example: MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE=
example-encryption-safe-standard-and-easy-to = - Safe, standard, and easy to transport
example-encryption-2-raw-bytes = 2. Raw bytes:
example-encryption-uses-the-32-raw-key-bytes = - Uses the 32 raw key bytes directly
example-encryption-example-2 = - Example: abcdefghijklmnopqrstuvwxyz123456
example-encryption-fits-simple-test-scenarios = - Fits simple test scenarios
example-encryption-3-password-derivation = 3. Password derivation:
example-encryption-derives-a-key-from-a-password = - Derives a key from a password with PBKDF2
example-encryption-example-my-secret-password = - Example: my-secret-password
example-encryption-fits-user-supplied-passwords = - Fits user-supplied passwords
example-encryption-all-examples-completed = ✓ all examples completed

# log-sanitizer
example-log-sanitizer-inklog-log-sanitizer-examples = === inklog log sanitizer examples ===
example-log-sanitizer-all-log-sanitizer-examples = ✓ all log sanitizer examples completed
example-log-sanitizer-1-default-sensitive-data-masking = 1. Default sensitive data masking
example-log-sanitizer-1-1-email-masking = 1.1 Email masking
example-log-sanitizer-raw = Raw: { $input }
example-log-sanitizer-sanitized = Sanitized: { $output }
example-log-sanitizer-1-2-password-masking = 1.2 Password masking
example-log-sanitizer-1-3-api-key-masking = 1.3 API key masking
example-log-sanitizer-1-4-bank-card-number-masking = 1.4 Bank card number masking
example-log-sanitizer-1-5-bearer-token-masking = 1.5 Bearer token masking
example-log-sanitizer-2-custom-sensitive-patterns = 2. Custom sensitive patterns (phone, national ID)
example-log-sanitizer-2-1-phone-number-masking = 2.1 Phone number masking
example-log-sanitizer-2-2-national-id-masking = 2.2 National ID masking
example-log-sanitizer-3-escapemode-strict-escaping = 3. EscapeMode::Strict escaping
example-log-sanitizer-3-1-control-and-special = 3.1 Control and special character escaping
example-log-sanitizer-3-2-printable-characters-are = 3.2 Printable characters are unchanged
example-log-sanitizer-4-escapemode-jsonsafe-json = 4. EscapeMode::JsonSafe JSON escaping
example-log-sanitizer-5-escapemode-minimal-escaping = 5. EscapeMode::Minimal escaping (default)
example-log-sanitizer-6-length-limiting-max-length = 6. Length limiting (max_length)
example-log-sanitizer-raw-bytes = Raw ({ $value } bytes): { $input }
example-log-sanitizer-sanitized-bytes = Sanitized ({ $value } bytes): { $output }
example-log-sanitizer-7-newline-escaping-default = 7. Newline escaping (default custom_replacements)
example-log-sanitizer-8-custom-string-replacement-add = 8. Custom string replacement (add_replacement)

# masking
example-masking-inklog-data-masking-examples = === inklog data masking examples ===
example-masking-1-basic-masking = 1. Basic masking
example-masking-1-1-email-masking = 1.1 Email masking
example-masking-raw-email = Raw email: { $email }
example-masking-masked = Masked:    { $masked_email }
example-masking-1-2-phone-number-masking = 1.2 Phone number masking
example-masking-raw-phone = Raw phone: { $phone }
example-masking-masked-2 = Masked:    { $masked_phone }
example-masking-1-3-national-id-masking = 1.3 National ID masking
example-masking-raw-id = Raw ID: { $id_card }
example-masking-masked-3 = Masked:     { $masked_id }
example-masking-1-4-bank-card-masking = 1.4 Bank card masking
example-masking-raw-bank-card = Raw bank card: { $bank_card }
example-masking-masked-4 = Masked:     { $masked_card }
example-masking-2-mixed-sensitive-data = 2. Mixed sensitive data
example-masking-raw-text = Raw text: { $mixed_text }
example-masking-masked-5 = Masked:   { $masked_text }
example-masking-3-json-data-masking = 3. JSON data masking
example-masking-raw-json = Raw JSON:
example-masking-masked-json = Masked JSON:
example-masking-4-nested-json-masking = 4. Nested JSON masking
example-masking-raw-nested-json = Raw nested JSON:
example-masking-masked-nested-json = Masked nested JSON:
example-masking-5-sensitive-field-detection = 5. Sensitive field detection
example-masking-5-1-sensitive-field-examples = 5.1 Sensitive field examples
example-masking-these-fields-are-detected-as = These fields are detected as sensitive:
example-masking-5-2-non-sensitive-field-examples = 5.2 Non-sensitive field examples
example-masking-these-fields-are-not-sensitive = These fields are not sensitive:
example-masking-6-hashmap-data-masking = 6. HashMap data masking
example-masking-raw-hashmap = Raw HashMap:
example-masking-masked-hashmap = Masked HashMap:
example-masking-7-advanced-sensitive-data = 7. Advanced sensitive data
example-masking-7-1-jwt-token-masking = 7.1 JWT token masking
example-masking-raw-jwt = Raw JWT: { $value }...
example-masking-masked-6 = Masked:   { $masked_jwt }
example-masking-7-2-aws-access-key-masking = 7.2 AWS access key masking
example-masking-raw-aws-key = Raw AWS key: { $aws_key }
example-masking-masked-7 = Masked:       { $masked_aws }
example-masking-7-4-credit-card-masking-luhn = 7.4 Credit card masking (Luhn)
example-masking-raw-visa = Raw Visa:  { $visa }
example-masking-masked-8 = Masked:     { $masked_visa }
example-masking-raw-amex = Raw Amex:  { $amex }
example-masking-masked-9 = Masked:     { $masked_amex }
example-masking-7-5-international-phone-masking = 7.5 International phone masking
example-masking-raw-international-phone = Raw international phone: { $intl_phone }
example-masking-masked-10 = Masked:       { $masked_intl }
example-masking-7-6-ip-mac-address-masking = 7.6 IP/MAC address masking
example-masking-ipv4 = IPv4: { $ipv4 } → { $value }
example-masking-ipv6 = IPv6: { $ipv6 } → { $value }
example-masking-mac = MAC:  { $mac } → { $value }
example-masking-7-7-third-party-token-masking = 7.7 Third-party token masking
example-masking-github = GitHub: { $github_token } → { $value }
example-masking-slack = Slack:  { $slack_token } → { $value }
example-masking-stripe = Stripe: { $stripe_key } → { $value }
example-masking-8-real-world-scenarios = 8. Real-world scenarios
example-masking-8-1-masking-log-messages = 8.1 Masking log messages
example-masking-raw-log =
    Raw log:
    { $log_message }
example-masking-masked-log =
    Masked log:
    { $masked_log }
example-masking-8-2-masking-api-responses = 8.2 Masking API responses
example-masking-raw-api-response = Raw API response:
example-masking-masked-api-response = Masked API response:
example-masking-9-custom-rules = 9. Custom rules
example-masking-9-1-maskrulebuilder = 9.1 MaskRuleBuilder
example-masking-custom-rule-priority = Custom rule: { $value } (priority={ $value2 })
example-masking-9-2-datamaskerbuilder = 9.2 DataMaskerBuilder
example-masking-raw = Raw: { $emp_text }
example-masking-masked-11 = Masked: { $value }
example-masking-9-3-maskruleregistry = 9.3 MaskRuleRegistry
example-masking-built-in-rule-count = Built-in rule count: { $value }
example-masking-after-registering-the-custom = After registering the custom rule: { $value }
example-masking-active-rule-count = Active rule count: { $value }
example-masking-9-4-maskruleregistry-load-from = 9.4 MaskRuleRegistry::load_from_toml
example-masking-loaded-custom-rules-from-toml = Loaded { $value } custom rules from TOML
example-masking-rule-priority-enabled = Rule: { $value } (priority={ $value2 }, enabled={ $value3 })
example-masking-raw-2 = Raw: { $order_text }
example-masking-10-performance-notes = 10. Performance notes
example-masking-datamasker-performance-notes = DataMasker performance notes:
example-masking-1-uses-pre-compiled-lazylock = 1. Uses pre-compiled LazyLock regular expressions
example-masking-2-handles-hashmap-and-json-in = 2. Handles HashMap and JSON in bulk
example-masking-3-recurses-into-nested = 3. Recurses into nested structures
example-masking-4-field-name-detection-is-case = 4. Field name detection is case-insensitive
example-masking-5-rules-run-in-priority-order = 5. Rules run in priority order
example-masking-6-rules-can-be-managed = 6. Rules can be managed dynamically through the registry
example-masking-all-examples-completed = ✓ all examples completed
example-masking-press-ctrl-c-to-exit = Press Ctrl+C to exit...

# path-validator
example-path-validator-inklog-path-validator-examples = === inklog path validator examples ===
example-path-validator-all-path-validator-examples = ✓ all path validator examples completed
example-path-validator-1-path-traversal-detection = 1. Path traversal detection (".." components)
example-path-validator-1-1-etc-passwd = 1.1 "../etc/passwd"
example-path-validator-valid-error = valid = { $valid }, error = { $error }
example-path-validator-1-2-foo-bar = 1.2 "foo/../../bar"
example-path-validator-1-3-the-safe-relative-path-logs = 1.3 The safe relative path "logs/app.log"
example-path-validator-valid = valid = { $valid }
example-path-validator-2-dangerous-component-detection = 2. Dangerous component detection (deny_components)
example-path-validator-2-1-etc-passwd = 2.1 "/etc/passwd"
example-path-validator-2-2-ssh-id-rsa = 2.2 "~/.ssh/id_rsa"
example-path-validator-2-3-env = 2.3 "./.env"
example-path-validator-2-4-project-git-config = 2.4 "project/.git/config"
example-path-validator-2-5-etc-passwd-passes-with-an = 2.5 "etc/passwd" passes with an empty deny_components
example-path-validator-3-absolute-path-restriction = 3. Absolute path restriction (allow_absolute)
example-path-validator-3-1-validating-absolute-path = 3.1 Validating "/absolute/path" after disabling absolute paths
example-path-validator-3-2-the-relative-path-relative = 3.2 The relative path "relative/path" still works
example-path-validator-4-symlink-detection-allow = 4. Symlink detection (allow_symlinks)
example-path-validator-4-1-a-nonexistent-path-no = 4.1 A nonexistent path (no symlink, validation passes)
example-path-validator-4-2-the-allow-symlinks-true = 4.2 The allow_symlinks = true configuration
example-path-validator-valid-allow-symlinks-true-does = valid = { $valid } (allow_symlinks=true does not reject)
example-path-validator-5-base-dir-constraint = 5. base_dir constraint
example-path-validator-5-1-a-path-inside-base-dir = 5.1 A path inside base_dir (passes)
example-path-validator-5-2-a-path-outside-base-dir = 5.2 A path outside base_dir (rejected)
example-path-validator-temporary-directory-cleaned = Temporary directory cleaned
example-path-validator-6-the-sanitize-method = 6. The sanitize method
example-path-validator-6-1-foo-bar-remove-parentdir = 6.1 "foo/../bar" → remove ParentDir
example-path-validator-sanitized = sanitized = { $value }
example-path-validator-6-2-foo-bar-remove-curdir = 6.2 "foo/./bar" → remove CurDir
example-path-validator-6-3-foo-bar-baz-multi-level = 6.3 "foo/../bar/../baz" → multi-level removal
example-path-validator-6-3-1-etc-passwd-pure-traversal = 6.3.1 "../../etc/passwd" → pure traversal is rejected
example-path-validator-6-4-the-validate-and-sanitize = 6.4 The validate_and_sanitize() combination
example-path-validator-valid-sanitized = valid = { $valid }, sanitized = { $sanitized_path }
example-path-validator-7-validationresult-constructors = 7. ValidationResult constructors
example-path-validator-7-1-validationresult-valid = 7.1 ValidationResult::valid()
example-path-validator-valid-error-sanitized = valid = { $valid }, error = { $error }, sanitized = { $sanitized_path }
example-path-validator-7-2-validationresult-invalid-msg = 7.2 ValidationResult::invalid(msg)
example-path-validator-7-3-validationresult-sanitized = 7.3 ValidationResult::sanitized(path)

# archive-format
example-archive-format-inklog-archive-format-examples = inklog archive format examples
example-archive-format-all-archive-format-examples = ✓ all archive format examples completed
example-archive-format-1-archiveformat-enum-variants = 1. ArchiveFormat enum variants
example-archive-format-archiveformat-json = ArchiveFormat::Json    = { $json }
example-archive-format-archiveformat-parquet = ArchiveFormat::Parquet = { $parquet }
example-archive-format-archiveformat-csv = ArchiveFormat::Csv     = { $csv }
example-archive-format-archiveformat-default = ArchiveFormat::default() = { $default }
example-archive-format-default-format-json-lines = Default format: { $default } (JSON Lines)
example-archive-format-2-fromstr-string-parsing = 2. FromStr string parsing
example-archive-format-string-archiveformat-parsing = String → ArchiveFormat parsing:
example-archive-format-error = { $s } → error: { $e }
example-archive-format-error-invalid-value = { $value } → error (invalid value): { $value2 }
example-archive-format-3-display-trait-output = 3. Display trait output
example-archive-format-archiveformat-string = ArchiveFormat → string:
example-archive-format-archiveformat-json-2 = ArchiveFormat::Json    → "{ $value }"
example-archive-format-archiveformat-parquet-2 = ArchiveFormat::Parquet → "{ $value }"
example-archive-format-archiveformat-csv-2 = ArchiveFormat::Csv     → "{ $value }"
example-archive-format-4-archive-configuration-in = 4. Archive configuration in DatabaseSinkConfig
example-archive-format-json-archive-default = JSON archive (default):
example-archive-format-archive-format = archive_format = { $archive_format }
example-archive-format-output-jsonl-files-one-json = Output: .jsonl files (one JSON object per line)
example-archive-format-parquet-archive = Parquet archive:
example-archive-format-output-parquet-files-columnar = Output: .parquet files (columnar storage, high compression)
example-archive-format-csv-archive = CSV archive:
example-archive-format-output-csv-files-comma = Output: .csv files (comma separated, Excel compatible)
example-archive-format-5-archive-format-comparison = 5. Archive format comparison
example-archive-format-6-toml-configuration-file-format = 6. TOML configuration file format
example-archive-format-parquet-archive-with-the = Parquet archive (with the Parquet settings):
example-archive-format-7-how-to-choose-an-archive = 7. How to choose an archive format
example-archive-format-when-to-choose-json = When to choose Json:
example-archive-format-you-need-human-readable-archive = - You need human-readable archive files
example-archive-format-you-query-them-with-grep-jq = - You query them with grep/jq
example-archive-format-a-log-shipper-consumes-them = - A log shipper consumes them (Fluentd/Logstash)
example-archive-format-fewer-than-1m-log-records-per = - Fewer than 1M log records per day
example-archive-format-when-to-choose-parquet = When to choose Parquet:
example-archive-format-a-big-data-analytics-platform = - A big-data analytics platform (Spark/DuckDB/BigQuery)
example-archive-format-you-need-a-high-ratio-to-save = - You need a high ratio to save storage cost
example-archive-format-you-run-columnar-queries = - You run columnar queries (aggregate/filter by field)
example-archive-format-more-than-1m-log-records-per-day = - More than 1M log records per day
example-archive-format-compliance-retention-long-term = - Compliance retention (long-term cold storage)
example-archive-format-when-to-choose-csv = When to choose Csv:
example-archive-format-you-import-it-into-excel-or-a = - You import it into Excel or a BI tool
example-archive-format-simple-reports-and-data-sharing = - Simple reports and data sharing
example-archive-format-integrating-with-legacy-systems = - Integrating with legacy systems (CSV parsers are widely supported)
example-archive-format-performance-characteristics = Performance characteristics:
example-archive-format-json-fast-writes-large-files = - Json: fast writes, large files, slow queries
example-archive-format-parquet-slow-writes-cpu = - Parquet: slow writes (CPU intensive), small files, fast queries
example-archive-format-csv-fast-writes-large-files-no = - Csv: fast writes, large files, no nested structures

# compression
example-compression-inklog-zstd-compression-examples = === inklog Zstd compression examples ===
example-compression-all-compression-examples = ✓ all compression examples completed
example-compression-1-in-memory-zstd-compress = 1. In-memory Zstd compress/decompress
example-compression-1-1-zstdcompression-default = 1.1 ZstdCompression::default() (level=3)
example-compression-algorithm-name = Algorithm name: { $value }
example-compression-extension = Extension  : { $value }
example-compression-1-2-compress-a-string = 1.2 compress() a string
example-compression-original-size-bytes = Original size : { $value } bytes
example-compression-compressed-size-bytes = Compressed size : { $value } bytes
example-compression-1-3-decompress-back-to-the = 1.3 decompress() back to the original
example-compression-decompressed-size-bytes = Decompressed size : { $value } bytes
example-compression-decompressed-data-matches-the = ✓ decompressed data matches the original
example-compression-1-4-the-compress-data = 1.4 the compress_data() convenience function

# console
example-console-example-1-basic-console-output = Example 1: basic console output (no color)
example-console-basic-console-output-complete = ✓ basic console output complete (no color)
example-console-example-2-colored-console-output = Example 2: colored console output
example-console-colored-log-output = Colored log output
example-console-colored-console-output-complete = ✓ colored console output complete
example-console-example-3-stderr-routing = Example 3: stderr routing
example-console-configuration-error-and-warn = Configuration: ERROR and WARN → stderr
example-console-info-debug-trace-stdout = INFO, DEBUG, TRACE → stdout
example-console-stderr-routing-complete-check = ✓ stderr routing complete (check the output streams)
example-console-inklog-console-sink-examples = === inklog console sink examples ===
example-console-all-examples-complete = All examples complete!

# file
example-file-example-1-basic-file-writing = Example 1: basic file writing
example-file-log-file-path = Log file path: { $log_path }
example-file-write-logs = Write logs
example-file-file-content = File content
example-file-basic-file-writing-complete = ✓ basic file writing complete
example-file-example-2-file-rotation = Example 2: file rotation
example-file-write-logs-trigger-rotation = Write logs (trigger rotation)
example-file-rotated-file-list = Rotated file list
example-file-directory = Directory: { $value }
example-file-file-list = File list:
example-file-bytes = { $file_name } ({ $size } bytes){ $marker }
example-file-rotation-count = Rotation count: { $rotation_count }
example-file-file-rotation-demo-complete = ✓ file rotation demo complete
example-file-example-3-zstd-compression = Example 3: Zstd compression
example-file-write-logs-trigger-rotation-and = Write logs (trigger rotation and compression)
example-file-wait-for-compression-to-finish = Wait for compression to finish
example-file-compressed-file-list = Compressed file list
example-file-compressed-file-count = Compressed file count: { $compression_count }
example-file-zstd-compression-demo-complete = ✓ Zstd compression demo complete
example-file-clean-up-temporary-files = Clean up temporary files
example-file-deleting = Deleting: { $file_name }
example-file-no-files-needed-cleaning = No files needed cleaning
example-file-deleted-files-in-total = Deleted { $deleted_count } files in total
example-file-inklog-file-sink-examples = === inklog file sink examples ===
example-file-all-examples-complete = All examples complete!

# parquet-archive
example-parquet-archive-inklog-parquet-archive-examples = inklog Parquet archive examples
example-parquet-archive-all-parquet-archive-examples = All Parquet archive examples shown.
example-parquet-archive-example-1-parquetconfig-fields = Example 1: ParquetConfig fields
example-parquet-archive-default-parquetconfig = Default ParquetConfig:
example-parquet-archive-compression-level-zstandard-0-10 = compression_level  = { $compression_level } (Zstandard 0-10)
example-parquet-archive-encoding = encoding           = "{ $encoding }"
example-parquet-archive-max-row-group-size-rows-per-row = max_row_group_size = { $max_row_group_size } (rows per row group)
example-parquet-archive-max-page-size-bytes-per-page = max_page_size      = { $max_page_size } (bytes per page)
example-parquet-archive-field-semantics = Field semantics:
example-parquet-archive-compression-level-the-zstandard = compression_level: the Zstandard compression level
example-parquet-archive-0-none-3-balanced-recommended = 0 = none, 3 = balanced (recommended), 10 = maximum (slower)
example-parquet-archive-max-row-group-size-the-maximum = max_row_group_size: the maximum rows per row group
example-parquet-archive-a-smaller-value-helps-selective = a smaller value helps selective queries
example-parquet-archive-a-larger-value-gives-a-better = a larger value gives a better ratio
example-parquet-archive-max-page-size-the-maximum-bytes = max_page_size: the maximum bytes per page; controls read memory use
example-parquet-archive-custom-configuration-high = Custom configuration (high compression):
example-parquet-archive-compression-level-maximum = compression_level  = { $compression_level } (maximum compression)
example-parquet-archive-max-row-group-size-larger-row = max_row_group_size = { $max_row_group_size } (larger row groups)
example-parquet-archive-max-page-size-2mb = max_page_size      = { $max_page_size } (2MB)
example-parquet-archive-custom-configuration-low = Custom configuration (low-latency queries):
example-parquet-archive-compression-level-fast = compression_level  = { $compression_level } (fast)
example-parquet-archive-max-row-group-size-smaller-row = max_row_group_size = { $max_row_group_size } (smaller row groups)
example-parquet-archive-max-page-size-512kb = max_page_size      = { $max_page_size } (512KB)
example-parquet-archive-example-2-calling-convert-logs = Example 2: calling convert_logs_to_parquet() for real
example-parquet-archive-create-test-logrecord-values = Create test LogRecord values...
example-parquet-archive-created-logrecord-values = ✓ created { $value } LogRecord values
example-parquet-archive-call-convert-logs-to-parquet = Call convert_logs_to_parquet()...
example-parquet-archive-parquet-conversion-succeeded = ✓ Parquet conversion succeeded
example-parquet-archive-input-logrecord-values = Input: { $value } LogRecord values
example-parquet-archive-output-bytes-of-parquet-data = Output: { $value } bytes of Parquet data
example-parquet-archive-ratio-bytes-record = Ratio: { $value } bytes/record
example-parquet-archive-parquet-magic-expected-80-65-82 = Parquet magic: { $magic } (expected: [80, 65, 82, 49] = "PAR1")
example-parquet-archive-conversion-failed = ✗ conversion failed: { $e }
example-parquet-archive-empty-record-test = Empty record test:
example-parquet-archive-empty-record-conversion = ✓ empty record conversion succeeded: { $value } bytes
example-parquet-archive-empty-record-conversion-failed = ✗ empty record conversion failed: { $e }
example-parquet-archive-example-3-databasesinkconfig = Example 3: DatabaseSinkConfig Parquet integration
example-parquet-archive-a-databasesinkconfig-with = A DatabaseSinkConfig with Parquet archiving enabled:
example-parquet-archive-enabled = enabled          = { $enabled }
example-parquet-archive-driver = driver           = { $driver }
example-parquet-archive-url = url              = "{ $url }"
example-parquet-archive-table-name = table_name       = "{ $table_name }"
example-parquet-archive-batch-size = batch_size       = { $batch_size }
example-parquet-archive-flush-interval-ms = flush_interval_ms= { $flush_interval_ms }
example-parquet-archive-archive-format-enables-the = archive_format   = "{ $archive_format }" ← enables the Parquet archive
example-parquet-archive-parquet-config-compression-level = parquet_config.compression_level  = { $compression_level }
example-parquet-archive-parquet-config-max-row-group = parquet_config.max_row_group_size = { $max_row_group_size }
example-parquet-archive-the-corresponding-toml = The corresponding TOML configuration:
example-parquet-archive-example-4-the-parquet-archive = Example 4: the Parquet archive workflow
example-parquet-archive-the-parquet-archive-workflow = The Parquet archive workflow:
example-parquet-archive-1-logs-are-written-into-the = 1. Logs are written into the database
example-parquet-archive-logrecord-databasesink-insert = → LogRecord → DatabaseSink → INSERT INTO logs
example-parquet-archive-2-periodically-export-to = 2. Periodically export to Parquet (in the background)
example-parquet-archive-flush-interval-ms-triggers = flush_interval_ms triggers → convert_logs_to_parquet()
example-parquet-archive-produce-the-parquet-byte-stream = → produce the Parquet byte stream
example-parquet-archive-3-parquet-file-storage = 3. Parquet file storage
example-parquet-archive-write-the-local-file-tmp-inklog = → write the local file /tmp/inklog_archive_20260101.parquet
example-parquet-archive-or-upload-to-object-cold = → or upload to object/cold storage (integration is up to you)
example-parquet-archive-4-analytical-queries-external = 4. Analytical queries (external tools)
example-parquet-archive-query-parquet-with-duckdb = # query Parquet with DuckDB
example-parquet-archive-duckdb-c-select-level-count = duckdb -c "SELECT level, COUNT(*) FROM 'archive.parquet' GROUP BY level"
example-parquet-archive-use-apache-arrow-from-python = # use Apache Arrow from Python
example-parquet-archive-import-pyarrow-parquet-as-pq = import pyarrow.parquet as pq
example-parquet-archive-table-pq-read-table-archive = table = pq.read_table('archive.parquet')
example-parquet-archive-use-cases = Use cases:
example-parquet-archive-analytics-load-into-snowflake = - Analytics: load into Snowflake/BigQuery/Redshift
example-parquet-archive-archive-object-storage-cold = - Archive: object storage cold tier (integration is up to you)
example-parquet-archive-compliance-efficient-storage = - Compliance: efficient storage that meets regulatory retention
example-parquet-archive-performance-characteristics = Performance characteristics:
example-parquet-archive-asynchronous-background-export = - Asynchronous background export that does not affect the main log path
example-parquet-archive-columnar-storage-with-a-high = - Columnar storage with a high ratio (typically 5-10x)
example-parquet-archive-predicate-pushdown-keeps = - Predicate pushdown keeps queries efficient

# partition-strategy
example-partition-strategy-inklog-database-partition = inklog database partition strategy examples
example-partition-strategy-all-partition-strategy-examples = All partition strategy examples shown.
example-partition-strategy-example-1-partitionstrategy = Example 1: PartitionStrategy enum values
example-partition-strategy-partitionstrategy-supports-two = PartitionStrategy supports two partition strategies:
example-partition-strategy-monthly-default = Monthly (default): { $monthly }
example-partition-strategy-yearly = Yearly:         { $yearly }
example-partition-strategy-default = Default:        { $default }
example-partition-strategy-string-parsing = String parsing:
example-partition-strategy-monthly = "monthly" → { $from_str_monthly }
example-partition-strategy-yearly-2 = "yearly"  → { $from_str_yearly }
example-partition-strategy-month-alias = "month"   → { $from_str_month } (alias)
example-partition-strategy-year-alias = "year"    → { $from_str_year } (alias)
example-partition-strategy-invalid-value-parsing = Invalid value parsing:
example-partition-strategy-daily-returns-an-error = "daily" → { $value } (returns an error)
example-partition-strategy-example-2-partition = Example 2: partition configuration in DatabaseSinkConfig
example-partition-strategy-monthly-partition-configuration = Monthly partition configuration:
example-partition-strategy-driver = driver    = { $driver }
example-partition-strategy-partition = partition = { $partition }
example-partition-strategy-table-name-pattern-logs-2026-01 = Table name pattern = logs_2026_01, logs_2026_02, ...
example-partition-strategy-yearly-partition-configuration = Yearly partition configuration:
example-partition-strategy-table-name-pattern-logs-2026 = Table name pattern = logs_2026, logs_2027, ...
example-partition-strategy-example-3-partition-strategy = Example 3: partition strategy comparison
example-partition-strategy-default-strategy = Default strategy: { $value }
example-partition-strategy-example-4-toml-configuration = Example 4: TOML configuration file format
example-partition-strategy-monthly-partition-default = Monthly partition (default):
example-partition-strategy-yearly-partition = Yearly partition:
example-partition-strategy-note-when-the-partition-field = Note: when the partition field is omitted it defaults to "monthly".
example-partition-strategy-example-5-how-to-choose-a = Example 5: how to choose a partition strategy
example-partition-strategy-when-to-choose-monthly = When to choose Monthly:
example-partition-strategy-more-than-100k-log-records-per = - More than 100k log records per day
example-partition-strategy-you-archive-or-purge-history-by = - You archive or purge history by month
example-partition-strategy-queries-are-usually-limited-to = - Queries are usually limited to specific months
example-partition-strategy-you-use-postgresql-partitioned = - You use PostgreSQL partitioned tables or MySQL partitions
example-partition-strategy-when-to-choose-yearly = When to choose Yearly:
example-partition-strategy-fewer-than-10k-log-records-per = - Fewer than 10k log records per day
example-partition-strategy-retention-is-measured-in-years = - Retention is measured in years
example-partition-strategy-management-simplicity-comes = - Management simplicity comes first
example-partition-strategy-you-do-not-purge-history = - You do not purge history frequently
example-partition-strategy-partitioned-table-management = Partitioned table management advice:
example-partition-strategy-drop-expired-partitions = - Drop expired partitions periodically (DROP PARTITION)
example-partition-strategy-watch-per-partition-sizes-to = - Watch per-partition sizes to avoid data skew
example-partition-strategy-index-the-fields-that-queries = - Index the fields that queries hit most
example-partition-strategy-archive-cold-data-as-parquet = - Archive cold data as Parquet (see the parquet_archive example)

# ring-buffered-file
example-ring-buffered-file-inklog-channelbufferedfilesink = === inklog ChannelBufferedFileSink examples ===
example-ring-buffered-file-all-channelbufferedfilesink = ✓ all ChannelBufferedFileSink examples completed
example-ring-buffered-file-1-channelbufferedconfig-defaults = 1. ChannelBufferedConfig defaults
example-ring-buffered-file-1-1-default-default = 1.1 Default::default()
example-ring-buffered-file-channel-capacity = channel_capacity    = { $channel_capacity }
example-ring-buffered-file-flush-batch-size = flush_batch_size    = { $flush_batch_size }
example-ring-buffered-file-flush-interval-ms = flush_interval_ms   = { $flush_interval_ms }
example-ring-buffered-file-backpressure-strategy = backpressure_strategy = { $backpressure_strategy }
example-ring-buffered-file-1-2-backpressurestrategy-enum = 1.2 BackpressureStrategy enum values
example-ring-buffered-file-block = Block       = { $value }
example-ring-buffered-file-dropoldest = DropOldest  = { $value }
example-ring-buffered-file-dropnewest = DropNewest  = { $value }
example-ring-buffered-file-1-3-custom-configuration-small = 1.3 custom configuration (small capacity + DropNewest)
example-ring-buffered-file-2-block-backpressure-strategy = 2. Block backpressure strategy (block when the channel is full)
example-ring-buffered-file-2-1-construct-a = 2.1 construct a ChannelBufferedFileSink
example-ring-buffered-file-sink-created-channel-capacity = sink created, channel_capacity = { $value }
example-ring-buffered-file-2-2-write-20-log-records = 2.2 write 20 log records
example-ring-buffered-file-2-3-flush-shutdown-to-ensure = 2.3 flush + shutdown to ensure data reaches disk
example-ring-buffered-file-file-size-bytes = File size: { $value } bytes
example-ring-buffered-file-all-20-log-records-written-the = ✓ all 20 log records written (the Block strategy loses nothing)
example-ring-buffered-file-metrics-after-shutdown-dropped = metrics after shutdown: dropped_count = { $dropped_count }
example-ring-buffered-file-it-counts-incomplete-batches = it counts incomplete batches (recv_count < batch_size) on the IO thread,
example-ring-buffered-file-3-dropnewest-backpressure = 3. DropNewest backpressure strategy (drop the newest when full)
example-ring-buffered-file-3-1-small-channel-dropnewest = 3.1 small channel + DropNewest
example-ring-buffered-file-3-2-write-64-records-quickly = 3.2 write 64 records quickly (over capacity)
example-ring-buffered-file-metrics-dropped-count = metrics.dropped_count = { $dropped_count }
example-ring-buffered-file-dropnewest-drops-new-records = ✓ DropNewest drops new records when the channel is full, dropped_count > 0
example-ring-buffered-file-4-dropoldest-backpressure = 4. DropOldest backpressure strategy (evict the oldest when full)
example-ring-buffered-file-4-1-small-channel-dropoldest = 4.1 small channel + DropOldest
example-ring-buffered-file-4-2-write-64-records-quickly = 4.2 write 64 records quickly (over capacity)
example-ring-buffered-file-dropoldest-evicts-old-records = ✓ DropOldest evicts old records when the channel is full, dropped_count > 0
example-ring-buffered-file-5-channelbufferedmetrics = 5. ChannelBufferedMetrics runtime statistics
example-ring-buffered-file-5-1-construct-the-sink-channel = 5.1 construct the sink (channel_capacity=16, flush_interval=10ms)
example-ring-buffered-file-5-2-write-10-log-records = 5.2 write 10 log records
example-ring-buffered-file-5-3-wait-for-the-io-thread-and = 5.3 wait for the IO thread and read metrics
example-ring-buffered-file-channel-capacity-2 = channel_capacity = { $channel_capacity }
example-ring-buffered-file-channel-len = channel_len      = { $channel_len }
example-ring-buffered-file-bytes-written = bytes_written    = { $bytes_written }
example-ring-buffered-file-flush-count = flush_count      = { $flush_count }
example-ring-buffered-file-dropped-count = dropped_count    = { $dropped_count }
example-ring-buffered-file-the-file-contains-all-10 = ✓ the file contains all 10 records and the metrics are correct

# rotation
example-rotation-inklog-log-rotation-examples = === inklog log rotation examples ===
example-rotation-all-rotation-examples-completed = ✓ all rotation examples completed
example-rotation-1-size-based-rotation = 1. Size-based rotation (SizeBasedRotation)
example-rotation-1-1-from-size-string-100mb = 1.1 from_size_string("100MB") construction
example-rotation-max-size-bytes-mb = max_size = { $value } bytes ({ $value2 } MB)
example-rotation-1-2-file-size-within-the-limit = 1.2 file size within the limit → no rotation
example-rotation-current-size-mb-limit-100-mb = current_size = { $value } MB, limit = 100 MB
example-rotation-should-rotate = should_rotate = { $should_rotate }
example-rotation-1-3-file-size-over-the-limit = 1.3 file size over the limit → rotation triggers
example-rotation-reason = reason        = { $reason }
example-rotation-2-time-based-rotation = 2. Time-based rotation (TimeBasedRotation)
example-rotation-2-1-the-four-built-in-intervals = 2.1 the four built-in intervals
example-rotation-interval-secs-sec-hours = { $interval } interval_secs = { $value } sec ({ $value2 } hours)
example-rotation-2-2-an-unknown-interval-returns = 2.2 an unknown interval returns an error
example-rotation-error-message = Error message: { $err }
example-rotation-2-3-interval-not-reached-no = 2.3 interval not reached → no rotation (daily, 1 hour elapsed)
example-rotation-elapsed-sec-interval-sec = elapsed { $value } sec, interval { $value2 } sec
example-rotation-2-4-interval-reached-rotation = 2.4 interval reached → rotation triggers (daily, 24 hours elapsed)
example-rotation-3-rotated-file-naming-rules = 3. Rotated file naming rules
example-rotation-3-1-generate-next-path-default = 3.1 generate_next_path default format
example-rotation-base-path = base path : { $value }
example-rotation-next-path = next path : { $value }
example-rotation-3-2-different-sequence-values = 3.2 different sequence values produce different file names
example-rotation-seq = seq={ $i } → { $p }
example-rotation-3-3-a-path-without-an-extension = 3.3 a path without an extension falls back to .log
example-rotation-4-composite-rotation-strategy = 4. Composite rotation strategy (CompositeRotation)
example-rotation-4-1-composite-size-time-neither = 4.1 composite size + time, neither condition met
example-rotation-strategy-name = strategy name = { $value }
example-rotation-size-100b-1kb-elapsed-1h-24h = size=100B(<1KB), elapsed=1h(<24h) → should_rotate={ $should_rotate }
example-rotation-4-2-the-size-condition-is-met = 4.2 the size condition is met → rotation triggers
example-rotation-size-2048b-1kb-elapsed-1h-24h = size=2048B(>1KB), elapsed=1h(<24h) → should_rotate={ $should_rotate }
example-rotation-4-3-the-time-condition-is-met = 4.3 the time condition is met → rotation triggers
example-rotation-size-100b-1kb-elapsed-24h-24h = size=100B(<1KB), elapsed=24h(>=24h) → should_rotate={ $should_rotate }
example-rotation-5-filesinkconfig-rotation-fields = 5. FileSinkConfig rotation fields
example-rotation-5-1-default-configuration = 5.1 default configuration
example-rotation-max-size = max_size      = { $max_size }
example-rotation-rotation-time = rotation_time = { $rotation_time }
example-rotation-keep-files = keep_files    = { $keep_files }
example-rotation-compress = compress      = { $compress }
example-rotation-5-2-custom-rotation = 5.2 custom rotation configuration (small file + hourly)
example-rotation-path = path          = { $value }
example-rotation-compression-level = compression_level = { $compression_level }
