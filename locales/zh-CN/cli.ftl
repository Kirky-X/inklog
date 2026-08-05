# CLI 验证消息

cli-validate-validating = 正在验证配置文件: { $path }
cli-validate-valid = ✓ 配置文件有效
cli-validate-global-level = ✓ 全局日志级别: { $level }
cli-validate-global-format = ✓ 全局格式: { $len } 字符
cli-validate-console-enabled = ✓ 控制台 Sink 已启用: { $enabled }
cli-validate-console-stderr = ✓ 控制台 stderr 级别: { $count } 个
cli-validate-file-enabled = ✓ 文件 Sink 已启用: { $enabled }
cli-validate-file-path = ✓ 文件路径: { $path }
cli-validate-file-max-size = ✓ 最大大小: { $size }
cli-validate-file-encrypt-key = ✓ 加密密钥环境变量: { $env }
cli-validate-perf-capacity = ✓ 通道容量: { $n }
cli-validate-perf-threads = ✓ 工作线程: { $n }
cli-validate-db-enabled = ✓ 数据库 Sink 已启用: { $enabled }
cli-validate-db-driver = ✓ 数据库驱动: { $driver }
cli-validate-db-url = ✓ 数据库 URL: { $len } 字节
cli-validate-http-port = ✓ HTTP 端口: { $n }

# CLI 验证错误消息

cli-err-config-not-exist = 配置文件不存在: { $path }
cli-err-read-config = 读取配置文件失败: { $path }
cli-err-parse-toml = 解析 TOML 内容失败
cli-err-invalid-log-level = 无效的日志级别 '{ $level }'。有效级别: { $valid }
cli-err-empty-format = 全局格式不能为空
cli-err-console-enabled = console_sink.enabled 必须是布尔值
cli-err-console-colored = console_sink.colored 必须是布尔值
cli-err-console-stderr = console_sink.stderr_levels 必须是字符串数组
cli-err-file-enabled = file_sink.enabled 必须是布尔值
cli-err-file-path-empty = file_sink.path 不能为空
cli-err-file-max-size = file_sink.max_size 格式无效: { $size }。请使用 '100MB'、'1GB' 格式
cli-err-file-keep-files = file_sink.keep_files 必须 >= 1
cli-err-file-retention = file_sink.retention_days 必须 >= 1
cli-err-file-compress = file_sink.compress 必须是布尔值
cli-err-file-encrypt = file_sink.encrypt 必须是布尔值
cli-err-file-encrypt-no-key = file_sink.encrypt 为 true 但未设置 encryption_key_env
cli-err-file-key-env-type = encryption_key_env 必须是字符串
cli-err-file-key-env-empty = file_sink.encrypt 为 true 但 encryption_key_env 为空
cli-err-perf-capacity = performance.channel_capacity 必须 >= 1
cli-err-perf-threads = performance.worker_threads 必须 >= 1
cli-err-db-enabled = db_config.enabled 必须是布尔值
cli-err-db-driver-type = db_config.driver 必须是字符串，实际为 { $driver }
cli-err-db-driver-invalid = 无效的数据库驱动 '{ $driver }'。有效驱动: { $valid }
cli-err-db-url-empty = db_config.url 不能为空
cli-err-db-url-invalid = 无效的数据库 URL。必须以下列前缀之一开头: { $prefixes }
cli-err-db-pool-size = db_config.pool_size 必须在 1 到 100 之间
cli-err-db-batch-size = db_config.batch_size 必须 >= 1
cli-err-db-table-empty = db_config.table_name 不能为空
cli-err-db-table-chars = db_config.table_name 只能包含字母数字和下划线
cli-err-http-enabled = http_server.enabled 必须是布尔值
cli-err-http-port = http_server.port 必须在 1 到 65535 之间
cli-err-http-host = http_server.host 不能为空

# CLI 验证警告

cli-warn-unknown-section = ⚠ 未知的配置节: [{ $section }]
cli-warn-dual-sink = ⚠ 同时启用了文件和数据库 Sink - 日志将写入两者

# CLI 前置检查

cli-prereq-checking = 正在检查前置条件...
cli-prereq-rust = Rust 版本:
cli-prereq-cargo = Cargo 版本:
cli-prereq-not-found = 未找到
cli-prereq-optional = 可选依赖:
cli-prereq-openssl-ok = ✓ OpenSSL 可用
cli-prereq-openssl-miss = ⚠ 未找到 OpenSSL（加密功能需要）
cli-prereq-zstd-ok = ✓ zstd 可用
cli-prereq-zstd-miss = ⚠ 未找到 zstd（压缩支持需要）
cli-prereq-config-check = 配置检查:
cli-prereq-sys-ok = ✓ 系统配置存在: { $path }
cli-prereq-sys-miss = ⚠ 系统配置未找到: { $path }
cli-prereq-local-ok = ✓ 本地配置存在: { $path }
cli-prereq-local-miss = ⚠ 本地配置未找到: { $path }
cli-prereq-example-ok = ✓ 配置示例存在: { $path }
cli-prereq-done = 前置检查完成。
cli-prereq-missing = 缺少关键前置条件: { $deps }。请在继续之前安装它们。

# CLI generate 消息

cli-generate-config = 已生成配置文件: { $path }
cli-generate-env = 已生成环境变量示例文件: { $path }
cli-generate-unknown-type = 未知的配置类型: { $type }。可用: minimal, full, database, file

# CLI decrypt 消息

cli-decrypt-progress = 正在解密: { $input } -> { $output }
cli-decrypt-fail = 解密失败 { $path }: { $err }
cli-decrypt-path-fail = 路径验证失败 { $path }: { $err }
cli-decrypt-done = 已解密: { $input } -> { $output }
cli-decrypt-dir-done = 已解密所有文件: { $input } -> { $output }
cli-decrypt-partial = 解密完成，{ $count } 个失败。详情请查看上方输出。
cli-decrypt-batch-result = 批量解密完成: { $ok } 成功, { $fail } 失败
cli-decrypt-skip-symlink = 跳过符号链接输入路径: { $path }

# CLI decrypt 错误消息

cli-decrypt-err-path-char = 检测到无效路径字符: { $path }
cli-decrypt-err-traversal = 检测到路径遍历模式: { $path }
cli-decrypt-err-symlink = 不允许符号链接: { $path }
cli-decrypt-err-canonical = 无法规范化文件路径: { $err }
cli-decrypt-err-base = 无法规范化基础目录: { $err }
cli-decrypt-err-traversal-detail = 检测到路径遍历尝试: { $path } 在基础目录 { $base } 之外
cli-decrypt-err-output-name = 无效的输出文件名: { $path }
cli-decrypt-err-no-parent = 输出路径没有父目录
cli-decrypt-err-canonical-parent = 无法规范化输出父目录 '{ $path }': { $err }
cli-decrypt-err-canonical-base = 无法规范化基础目录 '{ $path }': { $err }
cli-decrypt-err-traversal-output = 检测到路径遍历尝试: 输出 { $path } 在基础目录 { $base } 之外
cli-decrypt-err-glob-absolute = glob 模式中不允许绝对路径
cli-decrypt-err-glob-traversal = glob 模式中不允许路径遍历
cli-decrypt-err-glob-char = glob 模式中包含无效字符
cli-decrypt-err-glob-abs = 不允许绝对路径
cli-decrypt-err-glob-parent = 不允许父目录引用
cli-decrypt-err-glob-prefix = 不允许路径前缀
cli-decrypt-err-glob-root = 不允许根目录引用
cli-decrypt-err-header = 无效的文件头: 不是 inklog 加密文件
cli-decrypt-err-version = 不支持的文件版本: { $version }
cli-decrypt-err-algo = 不支持的加密算法: { $algo }
cli-decrypt-err-read-header = 读取文件头失败
cli-decrypt-err-read-cipher = 读取密文失败
cli-decrypt-err-decrypt = 解密失败: { $err }
cli-decrypt-err-open = 打开输入文件失败: { $path }
cli-decrypt-err-create = 创建输出文件失败: { $path }
cli-decrypt-err-write = 写入解密数据失败
cli-decrypt-err-small = 文件太小，不是有效的加密文件
cli-decrypt-err-small-v1 = 文件太小，不是有效的 V1 格式
cli-decrypt-err-key = 从环境变量获取加密密钥失败: { $env }
cli-decrypt-err-input-dir = 输入目录不存在: { $path }
cli-decrypt-err-create-dir = 创建输出目录失败: { $path }
cli-decrypt-err-output-dir = 无效的输出目录: { $err }
cli-decrypt-err-read-dir = 读取输入目录失败: { $path }
cli-decrypt-err-no-filename = 路径没有文件名: { $path }
cli-decrypt-err-glob = 无效的 glob 模式: { $err }
cli-decrypt-err-input-utf8 = 输入路径不是有效的 UTF-8: { $path }
cli-decrypt-err-output-symlink = 输出路径是符号链接: { $path }
cli-decrypt-err-input-dir-symlink = 输入目录是符号链接: { $path }
cli-decrypt-err-subdir-symlink = 子目录条目是符号链接（已跳过）: { $path }

# CLI generate 路径验证

cli-generate-err-path-traversal = 输出路径包含遍历模式: { $path }
cli-generate-err-path-absolute = 输出路径必须是相对路径: { $path }
