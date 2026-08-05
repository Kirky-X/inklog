# CLI validate messages

cli-validate-validating = Validating configuration file: { $path }
cli-validate-valid = ✓ Configuration file is valid
cli-validate-global-level = ✓ Global level: { $level }
cli-validate-global-format = ✓ Global format: { $len } chars
cli-validate-console-enabled = ✓ Console sink enabled: { $enabled }
cli-validate-console-stderr = ✓ Console stderr_levels: { $count } levels
cli-validate-file-enabled = ✓ File sink enabled: { $enabled }
cli-validate-file-path = ✓ File path: { $path }
cli-validate-file-max-size = ✓ Max size: { $size }
cli-validate-file-encrypt-key = ✓ Encryption key env: { $env }
cli-validate-perf-capacity = ✓ Channel capacity: { $n }
cli-validate-perf-threads = ✓ Worker threads: { $n }
cli-validate-db-enabled = ✓ Database sink enabled: { $enabled }
cli-validate-db-driver = ✓ Database driver: { $driver }
cli-validate-db-url = ✓ Database URL: { $len } bytes
cli-validate-http-port = ✓ HTTP port: { $n }

# CLI validate error messages

cli-err-config-not-exist = Config file does not exist: { $path }
cli-err-read-config = Failed to read config file: { $path }
cli-err-parse-toml = Failed to parse TOML content
cli-err-invalid-log-level = Invalid log level '{ $level }'. Valid levels: { $valid }
cli-err-empty-format = Global format cannot be empty
cli-err-console-enabled = console_sink.enabled must be a boolean
cli-err-console-colored = console_sink.colored must be a boolean
cli-err-console-stderr = console_sink.stderr_levels must be an array of strings
cli-err-file-enabled = file_sink.enabled must be a boolean
cli-err-file-path-empty = file_sink.path cannot be empty
cli-err-file-max-size = Invalid file_sink.max_size format: { $size }. Use format like '100MB', '1GB'
cli-err-file-keep-files = file_sink.keep_files must be >= 1
cli-err-file-retention = file_sink.retention_days must be >= 1
cli-err-file-compress = file_sink.compress must be a boolean
cli-err-file-encrypt = file_sink.encrypt must be a boolean
cli-err-file-encrypt-no-key = file_sink.encrypt is true but encryption_key_env is not set
cli-err-file-key-env-type = encryption_key_env must be a string
cli-err-file-key-env-empty = file_sink.encrypt is true but encryption_key_env is empty
cli-err-perf-capacity = performance.channel_capacity must be >= 1
cli-err-perf-threads = performance.worker_threads must be >= 1
cli-err-db-enabled = db_config.enabled must be a boolean
cli-err-db-driver-type = db_config.driver must be a string, got { $driver }
cli-err-db-driver-invalid = Invalid database driver '{ $driver }'. Valid drivers: { $valid }
cli-err-db-url-empty = db_config.url cannot be empty
cli-err-db-url-invalid = Invalid database URL. Must start with one of: { $prefixes }
cli-err-db-pool-size = db_config.pool_size must be between 1 and 100
cli-err-db-batch-size = db_config.batch_size must be >= 1
cli-err-db-table-empty = db_config.table_name cannot be empty
cli-err-db-table-chars = db_config.table_name must contain only alphanumeric characters and underscores
cli-err-http-enabled = http_server.enabled must be a boolean
cli-err-http-port = http_server.port must be between 1 and 65535
cli-err-http-host = http_server.host cannot be empty

# CLI validate warnings

cli-warn-unknown-section = ⚠ Unknown configuration section: [{ $section }]
cli-warn-dual-sink = ⚠ Both file and database sinks enabled - logs will be written to both

# CLI prerequisites

cli-prereq-checking = Checking prerequisites...
cli-prereq-rust = Rust version:
cli-prereq-cargo = Cargo version:
cli-prereq-not-found = not found
cli-prereq-optional = Optional dependencies:
cli-prereq-openssl-ok = ✓ OpenSSL available
cli-prereq-openssl-miss = ⚠ OpenSSL not found (needed for encryption)
cli-prereq-zstd-ok = ✓ zstd available
cli-prereq-zstd-miss = ⚠ zstd not found (for compression support)
cli-prereq-config-check = Configuration check:
cli-prereq-sys-ok = ✓ System config exist: { $path }
cli-prereq-sys-miss = ⚠ System config not found: { $path }
cli-prereq-local-ok = ✓ Local config exists: { $path }
cli-prereq-local-miss = ⚠ Local config not found: { $path }
cli-prereq-example-ok = ✓ Config example exists: { $path }
cli-prereq-done = Prerequisites check complete.
cli-prereq-missing = Missing critical prerequisites: { $deps }. Please install them before continuing.

# CLI generate messages

cli-generate-config = Generated config file: { $path }
cli-generate-env = Generated env example file: { $path }
cli-generate-unknown-type = Unknown config type: { $type }. Use: minimal, full, database, file

# CLI decrypt messages

cli-decrypt-progress = Decrypting: { $input } -> { $output }
cli-decrypt-fail = Failed to decrypt { $path }: { $err }
cli-decrypt-path-fail = Path validation failed for { $path }: { $err }
cli-decrypt-done = Decrypted: { $input } -> { $output }
cli-decrypt-dir-done = Decrypted all files in { $input } to { $output }
cli-decrypt-partial = Decryption completed with { $count } failure(s). Check output above for details.
cli-decrypt-batch-result = Batch decryption completed: { $ok } succeeded, { $fail } failed.
cli-decrypt-skip-symlink = Skipping symlink input path: { $path }

# CLI decrypt error messages

cli-decrypt-err-path-char = Invalid path character detected in: { $path }
cli-decrypt-err-traversal = Path traversal pattern detected in: { $path }
cli-decrypt-err-symlink = Symbolic links are not allowed: { $path }
cli-decrypt-err-canonical = Cannot canonicalize file path: { $err }
cli-decrypt-err-base = Cannot canonicalize base directory: { $err }
cli-decrypt-err-traversal-detail = Path traversal attempt detected: { $path } is outside base directory { $base }
cli-decrypt-err-output-name = Invalid output file name: { $path }
cli-decrypt-err-no-parent = Output path has no parent directory
cli-decrypt-err-canonical-parent = Cannot canonicalize output parent directory '{ $path }': { $err }
cli-decrypt-err-canonical-base = Cannot canonicalize base directory '{ $path }': { $err }
cli-decrypt-err-traversal-output = Path traversal attempt detected: output { $path } is outside base directory { $base }
cli-decrypt-err-glob-absolute = Absolute paths are not allowed in glob patterns
cli-decrypt-err-glob-traversal = Path traversal is not allowed in glob patterns
cli-decrypt-err-glob-char = Invalid character in glob pattern
cli-decrypt-err-glob-abs = Absolute paths are not allowed
cli-decrypt-err-glob-parent = Parent directory references are not allowed
cli-decrypt-err-glob-prefix = Path prefixes are not allowed
cli-decrypt-err-glob-root = Root directory references are not allowed
cli-decrypt-err-header = Invalid file header: not an encrypted inklog file
cli-decrypt-err-version = Unsupported file version: { $version }
cli-decrypt-err-algo = Unsupported encryption algorithm: { $algo }
cli-decrypt-err-read-header = Failed to read file header
cli-decrypt-err-read-cipher = Failed to read ciphertext
cli-decrypt-err-decrypt = Decryption failed: { $err }
cli-decrypt-err-open = Failed to open input file: { $path }
cli-decrypt-err-create = Failed to create output file: { $path }
cli-decrypt-err-write = Failed to write decrypted data
cli-decrypt-err-small = File too small to be a valid encrypted file
cli-decrypt-err-small-v1 = File too small for V1 format
cli-decrypt-err-key = Failed to get encryption key from env var: { $env }
cli-decrypt-err-input-dir = Input directory does not exist: { $path }
cli-decrypt-err-create-dir = Failed to create output directory: { $path }
cli-decrypt-err-output-dir = Invalid output directory: { $err }
cli-decrypt-err-read-dir = Failed to read input directory: { $path }
cli-decrypt-err-no-filename = path has no file name: { $path }
cli-decrypt-err-glob = Invalid glob pattern: { $err }
cli-decrypt-err-input-utf8 = Input path is not valid UTF-8: { $path }
cli-decrypt-err-output-symlink = Output path is a symbolic link: { $path }
cli-decrypt-err-input-dir-symlink = Input directory is a symbolic link: { $path }
cli-decrypt-err-subdir-symlink = Subdirectory entry is a symbolic link (skipping): { $path }

# CLI generate path validation

cli-generate-err-path-traversal = Output path contains traversal pattern: { $path }
cli-generate-err-path-absolute = Output path must be relative: { $path }
