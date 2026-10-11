# 配置加载和验证消息

config-load_failed = 加载配置失败: { $err }
config-read_failed = 读取配置文件 '{ $path }' 失败: { $err }
config-parse_failed = 解析配置文件 '{ $path }' 失败: { $err }
config-exceeds_max_size = 配置文件 '{ $path }' 超过最大大小（{ $size } 字节 > { $max } 字节）

# 验证错误
config-channel_capacity_zero = channel_capacity 不能为 0
config-worker_threads_zero = worker_threads 不能为 0
config-max_capacity_lt_min = performance.max_capacity ({ $max }) 必须 >= min_capacity ({ $min })
config-invalid_log_level = 无效的日志级别 '{ $level }'。有效级别: { $valid }
config-file_path_empty = 启用文件 sink 时 file_sink.path 不能为空
config-file_batch_size_zero = file_sink.batch_size 必须 > 0
config-http_port_zero = 启用时 http_server.port 不能为 0

# 环境变量覆盖警告
config-env_invalid_level = 无效的 INKLOG_GLOBAL_LEVEL '{ $val }'，保留当前值 '{ $current }'
config-env_unsafe_path = INKLOG_FILE_SINK_PATH '{ $path }' 包含不安全字符，已忽略
config-env_invalid_size = INKLOG_FILE_SINK_MAX_SIZE '{ $val }' 不是有效的大小格式（如 '100MB'），已忽略
config-env_unknown_error_mode = 未知的 INKLOG_HTTP_SERVER_ERROR_MODE '{ $val }'，保留当前值
config-env_unsafe_http_path = HTTP 路径 '{ $path }'（环境变量: { $env_var }）不是有效的 URL 路径（必须以 '/' 开头且不含遍历模式），已忽略
config-invalid_stderr_level = 无效的 stderr_levels 条目 '{ $level }'。有效级别: { $valid }
config-env_invalid_format = INKLOG_GLOBAL_FORMAT 为空，已忽略
config-env_invalid_db_url = INKLOG_DATABASE_SINK_URL 包含不安全字符，已忽略
config-env_blank_value_ignored = 环境变量 '{ $var }' 为空，已忽略
config-env_invalid_static_field = INKLOG_GLOBAL_STATIC_FIELDS 段 '{ $segment }' 不是有效的 key=value（完整原始值: '{ $raw }'），已忽略
config-env_control_char_ignored = 环境变量 '{ $var }' 含控制字符，已忽略

# 数据库配置解析错误
config-unknown_db_driver = 未知的数据库驱动 '{ $driver }'。有效驱动: { $valid }
config-unknown_partition_strategy = 未知的分区策略: '{ $strategy }'。有效: monthly, yearly
config-unknown_archive_format = 未知的归档格式: '{ $format }'。有效: json, parquet, csv

# 性能配置解析错误
config-unknown_channel_strategy = 未知的通道策略: '{ $strategy }'。有效: fixed, adaptive

# 轮转配置解析错误
config-unknown_rotation_interval = 未知的轮转间隔: '{ $interval }'。有效: hourly, daily, weekly, monthly
config-invalid_size_number = 无效的大小数字: '{ $num }'

# Sink 注册表错误
config-unknown_sink_type = 未知的 sink 类型: '{ $type }'

# 脱敏规则错误
config-mask_rule_requires_pattern = 脱敏规则 '{ $name }' 缺少 pattern
config-invalid_regex_in_rule = 规则 '{ $name }' 中的正则无效: { $err }
config-rule_already_registered = 脱敏规则 '{ $name }' 已注册
config-failed_parse_toml = TOML 解析失败: { $err }
config-toml_missing_masking_rules = TOML 缺少 '[[masking_rules]]' 数组
config-masking_missing_name = masking_rules 条目缺少 'name' 字段
config-masking_missing_pattern = masking_rules 条目 '{ $name }' 缺少 'pattern' 字段

# HTTP 配置错误
config-http_auth_token_empty = HTTP 认证已启用但 token 环境变量 '{ $env }' 为空
config-http_auth_token_not_set = HTTP 认证已启用但 token 环境变量 '{ $env }' 未设置
config-invalid_http_address = 无效的 HTTP 服务器地址: '{ $addr }': { $err }

# 加密配置错误
config-encryption_key_not_set = 加密密钥环境变量 '{ $env }' 未设置
config-encryption_base64_wrong_length = Base64 加密密钥必须恰好为 32 字节（256 位），实际为 { $got } 字节
config-encryption_key_wrong_length = 加密密钥必须恰好为 32 字节（256 位），实际为 { $got } 字节
config-encryption_password_too_short = 加密密码必须至少 12 个字符，实际为 { $got }

# 管理器/构建器错误
config-failed_read_config = 读取配置文件失败: { $err }
config-failed_parse_config = 解析配置文件失败: { $err }
config-failed_load_config = 加载配置失败: { $err }
config-failed_send_recovery = 发送恢复命令失败: { $err }
config-builder_validation_failed = 构建器验证失败，共 { $count } 个错误
config-ring_buffer_dropped = 记录被环形缓冲区丢弃（已丢弃计数: { $count }）
config-unsafe_path_rejected = 不安全的日志路径被拒绝: { $reason }
config-require_dbnexus = 需要 DbNexusModule: { $err }
config-db_not_available = InklogModule on_ready: DbNexusModule 不可用: { $err }
config-http_startup_failed = HTTP 服务器启动失败（继续运行）: { $err }
config-shutdown_signal_lost = 关闭信号丢失: worker 通道已断开
config-http_serialize_failed = 序列化健康状态失败: { $err }
config-https_server_error = HTTPS 服务器错误: { $err }
config-http_bind_failed = 绑定 HTTP 服务器到 { $addr } 失败: { $err }
config-http_server_error = HTTP 服务器错误: { $err }
config-json_serialize_failed = 序列化日志字段为 JSON 失败: { $err }
config-compression_remove_failed = 压缩后删除原始文件失败: { $err }
config-open_input_failed = 打开输入文件失败: { $path }
config-read_header_failed = 读取文件头失败
config-read_ciphertext_failed = 读取密文失败
config-write_decrypted_failed = 写入解密数据失败
config-create_output_failed = 创建输出文件失败: { $path }
config-get_key_failed = 从环境变量获取加密密钥失败: { $env }
config-encryption_error = 加密错误: { $err }
config-decryption_failed = 解密失败: { $err }
config-invalid_header = 无效的文件头: 不是加密的 inklog 文件
config-unsupported_version = 不支持的文件版本: { $version }
config-unsupported_algorithm = 不支持的加密算法: { $algo }
config-create_config_failed = 创建配置文件失败: { $path }
config-write_config_failed = 写入配置内容失败
config-create_env_failed = 创建环境变量示例文件失败: { $path }
config-write_env_failed = 写入环境变量示例内容失败
config-unknown_log_level = 未知的日志级别: { $level }
config-invalid_encryption_key = 无效的密钥: { $err }
config-key_env_empty = 加密密钥环境变量 '{ $env }' 已设置但为空；请移除该变量以回退密钥文件
config-key_source_missing = 加密已启用但未找到密钥：请设置环境变量 '{ $env }' 或密钥文件 '{ $file }'
config-key_file_read = 读取密钥文件 '{ $file }' 失败: { $err }
config-key_file_permission = 密钥文件 '{ $file }' 权限必须为 0600（当前 { $mode }）；拒绝使用组/其他用户可读的密钥
config-journal_header_conflict = journal 文件 '{ $file }' 为其他格式；请先重放或删除后再启用加密 journal
config-journal_push_invalid_addr = journal 推送地址 '{ $addr }' 无效：应为 host:port

# 身份字段校验错误
config-global_identity_blank = global.{ $name } 已设置但为空白
config-global_static_fields_empty_key = global.static_fields 包含空键
config-control_char_rejected = { $field } 含控制字符（U+{ $code }）

# 加密密钥来源校验
config-encrypt_no_key_source = encrypt 已启用但既未设置 encryption_key_env 也未设置 encryption_key_file

# 限流 / 采样配置校验错误
config-rate_limit_rules_key_empty = rate_limit.rules 的键不能为空
config-rate_limit_rules_rate_invalid = rate_limit.rules["{ $prefix }"] 的速率必须 >= 1，实际为 0
config-sampling_per_level_invalid = sampling.per_level["{ $level }"] 不是有效级别。有效级别: { $valid }
config-sampling_per_level_rate_invalid = sampling.per_level["{ $level }"] 的速率必须 >= 1，实际为 0
config-sampling_prefix_empty = sampling.per_target_prefix 的键不能为空
config-sampling_sample_every_n_invalid = sampling.per_target_prefix["{ $prefix }"].sample_every_n 必须 >= 1，实际为 0
config-sampling_keep_level_invalid = sampling.per_target_prefix["{ $prefix }"].keep_level "{ $level }" 不是有效级别。有效级别: { $valid }

# HTTP 监控服务消息
config-http_invalid_token = 无效令牌
config-http_missing_auth_header = 缺少或非法的 Authorization 头
config-http_ip_not_whitelisted = IP 不在白名单内
config-http_serialization_failed = 序列化失败
config-http_ip_whitelist_entry_invalid = ip_whitelist 条目不是合法 IP 或 CIDR；它永不匹配（失败关闭）

# 数据库连接池限幅与运行时调级告警
config-db_pool_size_capped = 数据库 pool_size 已收敛到 min(configured, num_cpus, 4)
config-injected_db_pool_above_limit = 注入的 Database 配置的 pool_size 超过 effective_db_worker_limit；请在适配器层收敛连接池
config-set_level_no_live_filter = set_level：没有可替换的已安装过滤器（detached build？）；指令集已更新

# 配置修复与适配器告警
config-stderr_levels_entry_removed = stderr_levels 条目非法，已移除
config-min_max_capacity_swapped = min_capacity > max_capacity，已交换
config-adapter_fallback = InklogConfigAdapter::new 失败，回退到默认配置：{ $err }
config-reload_failed_keep_previous = 配置热更新失败；保留既有配置

# Schema 序列化不变量
config-json_schema_serialize_invariant = JsonSchema 序列化不会失败
config-json_path_has_segment_invariant = 点分路径至少包含一个段
config-json_intermediate_node_object_invariant = 配置树中间节点必然为对象
config-json_leaf_parent_object_invariant = 配置树叶父节点必然为对象
config-serialize_invariant = InklogConfig 序列化不会失败（纯 derive 数据）
config-env_patch_types_invariant = env 补丁保持字段类型不变（逐条规则校验）

# 压缩后端失败消息
config-compression-create_failed = 创建压缩文件失败：{ $err }
config-compression-finish_failed = 完成压缩失败：{ $err }
config-compression-open_failed = 打开待压缩文件失败：{ $err }
config-compression-zstd_encoder_failed = 创建 zstd 编码器失败：{ $err }

# 运行期日志消息：HTTP 服务生命周期与管理器事件
log-https_server_started = HTTPS 服务已启动于 { $addr }（auth: { $auth }，ip_whitelist: { $whitelist }）
log-http_server_started = HTTP 服务已启动于 { $addr }（auth: { $auth }，ip_whitelist: { $whitelist }）
log-http_server_stopped = HTTP 服务已停止
log-http_monitoring_configured = HTTP 监控服务已配置于 { $addr }
log-http_shutdown_signal_sent = 已发送 HTTP 服务关闭信号
log-manager_initialized = 日志管理器已初始化
log-global_subscriber_already_set = 全局 subscriber 已被设置；跳过 inklog registry
log-log_logger_already_set = log crate logger 已被设置；跳过 inklog LogLogger
log-level_changed = 运行期日志级别已变更
log-target_level_changed = 运行期目标日志级别已变更
log-config_hot_reloaded = 配置已热更新
log-config_validation_failed = 配置校验失败：{ $err }
log-console_secret_scan_feature_missing = console_sink.secret_scan_enabled=true 需要 `secret-scan` 特性；当前构建忽略该开关
log-database_secret_scan_feature_missing = database_sink.secret_scan_enabled=true 需要 `secret-scan` 特性；当前构建忽略该开关
log-console_stderr_levels_entries_removed = 已移除非法的 stderr_levels 条目
