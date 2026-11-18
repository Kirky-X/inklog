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
cli-generate-schema = 已生成配置 JSON Schema: { $path }
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
cli-decrypt-err-read-dir = 读取输入目录失败: { $path }
cli-decrypt-err-no-filename = 路径没有文件名: { $path }
cli-decrypt-err-glob = 无效的 glob 模式: { $err }
cli-decrypt-err-input-utf8 = 输入路径不是有效的 UTF-8: { $path }
cli-decrypt-err-input-not-found = 输入路径不存在: { $path }
cli-decrypt-err-output-symlink = 输出路径是符号链接: { $path }
cli-decrypt-err-input-dir-symlink = 输入目录是符号链接: { $path }
cli-decrypt-err-subdir-symlink = 子目录条目是符号链接（已跳过）: { $path }

cli-decrypt-err-truncated-v2 = v2 头部被截断: 期望 40 字节（含 16 字节 PBKDF2 盐），实际 { $read_count }

# CLI generate 路径验证

cli-generate-err-path-traversal = 输出路径包含遍历模式: { $path }

# CLI 帮助文本（clap about/help，动态生成）

cli-about = inklog - 企业级 Rust 日志基础设施 CLI
cli-help-json = 输出机器可读的 JSON 结果
cli-err-prefix = 错误

cli-decrypt-about = 解密已加密的日志文件
cli-decrypt-help-input = 输入的加密文件或目录
cli-decrypt-help-output = 输出文件或目录
cli-decrypt-help-key-env = 存放解密密钥的环境变量名
cli-decrypt-help-recursive = 递归解密目录
cli-decrypt-help-batch = 启用多文件批量解密模式

cli-generate-about = 生成 inklog 配置文件
cli-generate-help-output = 输出目录或文件路径
cli-generate-help-config-type = 配置类型: minimal、full、database、file
cli-generate-help-env-example = 生成环境变量示例文件
cli-generate-help-schema = 生成 inklog 配置 JSON Schema 文件（--output 为文件路径，默认 config_schema.json）；与其他生成选项互斥，启用时优先执行

cli-validate-about = 校验 inklog 配置文件（默认: inklog_config.toml）
cli-validate-help-config = 配置文件路径
cli-validate-help-prerequisites = 检查系统前置条件而非配置文件

cli-query-about = 按时间范围/级别/关键字搜索本地日志文件（自动解包加密/压缩归档）
cli-query-help-path = 要搜索的日志文件或目录（递归；可多次指定）
cli-query-help-since = 仅返回该时间之后的记录（RFC3339，如 2026-09-11T00:00:00Z）
cli-query-help-until = 仅返回该时间之前的记录（RFC3339）
cli-query-help-level = 最低级别: trace/debug/info/warn/error/fatal
cli-query-help-grep = 对消息与 target 应用子串过滤
cli-query-help-limit = 最大返回记录数（0 = 不限制）
cli-query-help-key-env = 存放 .enc 文件解密密钥的环境变量名

cli-query-err-invalid-rfc3339 = --{ $flag } '{ $raw }' 无效: 期望 RFC3339 时间戳 ({ $err })
cli-query-err-no-path = query 至少需要一个 --path（文件或目录）

cli-verify-chain-about = 校验归档审计链（HMAC-SHA256）manifest 是否被篡改
cli-verify-chain-help-manifest = 审计链 manifest（<stem>.chain.jsonl）
cli-verify-chain-help-key-env = 存放审计链密钥的环境变量名
cli-verify-chain-help-anchor = 锚文件（缺省探测 manifest 同目录 <stem>.anchor.jsonl，存在即校验）
cli-verify-chain-help-anchor-key-env = 持有 HMAC 锚密钥的环境变量名（传变量名，非密钥值）
cli-verify-chain-help-anchor-pubkey = Ed25519 公钥（hex），用于非对称锚校验（audit-anchor feature）
cli-verify-chain-err-manifest-unreadable = 无法读取审计链 manifest
cli-verify-chain-err-line-invalid = manifest 行格式非法
cli-verify-chain-err-key-missing = 审计链密钥环境变量未设置
cli-verify-chain-ok = 链完整
cli-verify-chain-tampered = 链已被篡改（检测到删除/重排/截尾/内容修改）
cli-verify-chain-empty = 链 manifest 为空（0 个条目）：无法证明完整性（归档被删除或截断）
cli-verify-chain-seal-unreadable = 无法读取链封条（{ $path }）：{ $err }——无法证明完整性
cli-verify-chain-ok-detail = { $chain }: { $path }（{ $count } 个条目）
cli-verify-chain-rotated-during-verify = 校验期间归档链被重写（manifest 与封条重读后配对一致）——以下为重读后的结果
cli-verify-chain-generation-mismatch = 链封条与 manifest 不匹配（封条覆盖 { $seal_count } 个条目，manifest 实际 { $count } 个）：两者不同步（轮转中断或条目被删除）——无法证明完整性
cli-verify-chain-anchor-ok = 锚校验通过（{ $count }）——独立密钥锚定确认链头
cli-verify-chain-anchor-fail = 锚校验失败：{ $detail }——完整性无法证明
cli-verify-chain-anchor-no-anchor = 链未锚定（manifest 同目录无 <stem>.anchor.jsonl）——锚保护未启用
cli-verify-chain-failover-markers = 链内含 { $count } 个故障转移标记（begin/end/replay）——已作为普通链条目逐条校验

# CLI generate schema 错误

cli-generate-err-schema-serialize = 序列化配置 JSON Schema 失败

# 示例：confers 审计桥演示

example-confers-audit-title = confers 审计桥（confers-audit feature）
example-confers-audit-write-events = 写入审计事件（三类：密钥访问/解密/配置加载）
example-confers-audit-result = 桥接结果：accepted={ $accepted } dropped={ $dropped } write_failures={ $failures }
example-confers-audit-chain-dir = 本地审计链文件目录：{ $path }

# doctor 子命令（安全姿态评估）

cli-doctor-about = 评估当前构建、配置与环境的安全姿态
cli-doctor-help-security = 执行安全姿态评估
cli-doctor-help-config = 配置文件路径
cli-doctor-err-no-target = doctor 需要指定评估目标，如 --security
cli-doctor-check-ok = ✓ { $capability }: { $detail }
cli-doctor-check-warn = ! { $capability }: { $detail }
cli-doctor-check-critical = ✗ { $capability }: { $detail }
cli-doctor-check-unavailable = - { $capability }: { $detail }

# 姿态检查项能力名与证据/修复文案

posture-pii-masking = PII 掩码
posture-pii-masking-detail-no-sinks = 无启用的 sink——没有持久化输出
posture-pii-masking-detail-all = 全部 { $count } 个活跃 sink 均已启用（{ $names }）
posture-pii-masking-detail-partial = { $count } 个活跃 sink 中仅部分启用（未掩码: { $unmasked }）
posture-pii-masking-detail-none = 全部 { $count } 个活跃 sink 均未启用（{ $names }）
posture-pii-masking-remedy = 为每个启用的 sink 设置 masking_enabled = true（或 pii_masking_enabled）

posture-file-encryption = 文件加密
posture-file-encryption-detail-no-sink = 无启用的文件 sink——不适用
posture-file-encryption-detail-on = 已启用且密钥来源可解析（env { $env }）
posture-file-encryption-detail-off = 未启用——落盘日志为明文
posture-file-encryption-detail-key-missing = 已启用但密钥来源缺失或无效（env { $env }）
posture-file-encryption-remedy = 设置加密密钥环境变量（{ $env }）或 encryption_key_file

posture-audit-chain = 审计链
posture-audit-chain-detail-no-sink = 无启用的文件 sink——不适用
posture-audit-chain-detail-ok = 已启用且 { $env } 已设置
posture-audit-chain-detail-key-missing = audit_chain_enabled = true 但 { $env } 未设置——运行时链将被静默禁用
posture-audit-chain-detail-off = 未启用——无法检测归档篡改
posture-audit-chain-remedy = 在启动 inklog 前设置 { $env }（HMAC 密钥）

posture-secret-scan = Secret 扫描
posture-secret-scan-detail-on = 已编译进（secret-scan feature）
posture-secret-scan-detail-off = 未编译进（secret-scan feature 关闭）
posture-secret-scan-remedy = 以 --features secret-scan 重新构建以掩码裸 API 密钥

posture-net-sink-tls = 网络出口 TLS
posture-net-sink-tls-detail-on = 已编译进且证书校验生效
posture-net-sink-tls-detail-insecure = insecure-tls feature 已启用——证书校验被禁用
posture-net-sink-tls-detail-off = 未编译进（net-sink feature 关闭）
posture-net-sink-tls-remedy = 去掉 --features insecure-tls 重新构建

posture-journal-crypto = Fallback journal 加密
posture-journal-crypto-detail-on = 已启用（key env { $env }）
posture-journal-crypto-detail-off = fallback journal 未启用
posture-journal-crypto-detail-plaintext = 已启用但未加密——溢出记录为明文（key env 应为 { $env }）
posture-journal-crypto-remedy = 设置 global.fallback_journal_encrypt = true 并提供密钥

posture-db-backend = 数据库后端
posture-db-backend-detail-on = 已编译进: { $names }
posture-db-backend-detail-off = 未编译进（无数据库后端 feature）

posture-kms = KMS 密钥提供者
posture-kms-detail-on = 已编译进（kms feature）
posture-kms-detail-off = 未编译进（kms feature 关闭）
posture-kms-remedy = 以 --features kms 重新构建以支持外部密钥提供者
cli-doctor-summary-ok = 姿态健康：无 Critical 发现（{ $critical } 项 Critical，{ $warn } 项 Warn）
cli-doctor-summary-unsound = 姿态不健康：{ $critical } 项 Critical，{ $warn } 项 Warn——见上方修复提示

# CLI MCP 子命令（feature `mcp`）
cli-mcp-about = 启动 MCP stdio 服务器（Agent 工具：query、audit verify、stats）
cli-mcp-help-config = 持有 MCP 策略的配置文件路径（[mcp] 段）
