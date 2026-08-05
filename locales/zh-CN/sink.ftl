# Worker 和 sink 内部追踪消息

# File sink worker 消息
sink-file_recovery_received = File sink: 收到恢复命令
sink-file_recovered = File sink: 恢复成功
sink-file_recovery_failed = File sink: 恢复失败
sink-file_auto_recovery = File sink: 因连续失败触发自动恢复
sink-file_auto_recovery_ok = File sink: 自动恢复成功

# Database sink worker 消息
sink-db_recovery_received = Database sink: 收到恢复命令
sink-db_recovered = Database sink: 恢复成功
sink-db_recovery_failed = Database sink: 恢复失败
sink-db_auto_recovery = Database sink: 因连续失败触发自动恢复
sink-db_auto_recovery_ok = Database sink: 自动恢复成功

# 健康检查消息
sink-health_unhealthy = 健康检查: Sink '{ $name }' 不健康。最后错误: { $error }
sink-health_attempting_recovery = 健康检查: 正在尝试恢复 sink '{ $name }'
sink-health_send_failed = 健康检查: 发送恢复命令失败 '{ $name }': { $err }

# File sink 操作消息
sink-file_reject_path = 拒绝不安全的日志路径 { $path }: { $reason }
sink-file_mkdir_failed = 创建日志目录失败 { $dir }: { $err }
sink-file_cleanup_panic = cleanup_timer 线程异常: { $msg }
sink-file_rotation_panic = rotation_timer 线程异常: { $msg }

# Database 适配器消息
db-pool_create_failed = 创建连接池失败: { $err }
db-session_failed = 获取会话失败: { $err }
db-batch_insert_failed = 批量插入失败: { $err }
db-table_empty = 表名不能为空
db-table_invalid_start = 无效的表名 '{ $name }': 必须以字母或下划线开头
db-table_invalid_char = 无效的表名 '{ $name }': 包含禁止字符 '{ $char }'
db-ensure_table_failed = 确保表存在失败: { $err }

# Cache 适配器消息
cache-get_failed = 获取缓存键 '{ $key }' 失败: { $err }
cache-set_failed = 设置缓存键 '{ $key }' 失败: { $err }
cache-delete_failed = 删除缓存键 '{ $key }' 失败: { $err }
cache-exists_failed = 缓存键 '{ $key }' 存在性检查失败: { $err }
cache-check_failed = 检查缓存键 '{ $key }' 是否存在失败: { $err }
cache-build_failed = 构建 oxcache 失败: { $err }
cache-capacity_zero = OxCacheAdapterBuilder: capacity 必须 > 0

# 配置验证警告
warn-db_batch_size_zero = database_sink.batch_size 为 0，重置为默认值 100
warn-db_flush_interval_zero = database_sink.flush_interval_ms 为 0，重置为默认值 500
warn-db_compression_level_clamp = parquet_config.compression_level 超出范围 1-22，已调整为 3
warn-fallback_retries_zero = fallback_max_retries 为 0，重置为 1
warn-rate_limit_zero = rate_limit = 0 无效，重置为 None（无限制）
warn-threshold_reset = shrink_threshold >= expand_threshold，已重置为默认值
warn-delay_clamp = fallback_initial_delay_ms > fallback_max_delay_ms，已调整
warn-weak_password = 加密密码较弱（< 16 字符）。建议使用更长的密码或随机的 32 字节密钥
warn-db_health_check_failed = 数据库健康检查失败: { $err }
warn-fallback_write_failed = 降级 sink 写入失败: { $err }
info-db_shutdown_complete = 数据库 sink 关闭完成
warn-cache_ttl_zero = OxCacheAdapterBuilder: TTL 为零，使用默认 TTL
