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
