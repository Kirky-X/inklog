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
config-http_lock_poisoned = HTTP 服务器句柄锁中毒: { $err }
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
