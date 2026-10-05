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
db-duckdb_ddl_not_create_table = DuckDB DDL 通道仅允许 CREATE TABLE 建表模板，收到: { $sql }

# Cache 适配器消息
cache-get_failed = 获取缓存键 '{ $key }' 失败: { $err }
cache-set_failed = 设置缓存键 '{ $key }' 失败: { $err }
cache-delete_failed = 删除缓存键 '{ $key }' 失败: { $err }
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

# Subscriber 消息
subscriber-drop-fallback-pending = LoggerSubscriber 已丢弃，尚有 { $count } 条回退记录未刷写

# 文件 sink 批量写入消息
sink-batch_flush_failed = 批量刷写失败: { $err }
sink-fsync_failed = 批量写入后 fsync 失败: { $err }
sink-rotate_postprocess_failed = 轮转日志后处理失败: { $err }
sink-rotate_postprocess_panicked = 轮转后处理线程异常: { $msg }
sink-audit_manifest_write_failed = 写入审计链 manifest { $path } 失败: { $err }
sink-plaintext_residue = 存在明文残留：加密后删除失败

# fallback journal 消息
journal-replay-skipped-corrupt = fallback journal 重放跳过了损坏行
journal-replay-no-durable-sink = fallback journal 重放跳过：未配置持久化 sink
journal-push-spawn-failed = journal 推送线程启动失败: { $err }
journal-push-empty-addr = 地址为空
journal-undecryptable-preserved = fallback journal: 存在无法解密的段；文件已保留
journal-plaintext-append-refused = fallback journal: 明文实例拒绝向加密 journal 追加（配置回滚）
journal-encrypted-spill-failed = fallback journal: 加密落盘失败
journal-aesgcm-encrypt-failed = fallback journal: AES-GCM 加密失败
journal-unsupported-format = fallback journal: 不支持的加密格式，未重放任何记录
journal-push-failed-kept = fallback journal 推送失败；记录已保留在磁盘

# 审计链消息
audit-chain-key-missing = 已启用 audit_chain_enabled 但未设置 INKLOG_AUDIT_KEY；审计链已禁用

# Secret 扫描 / 脱敏内部追踪消息
secret-entropy-threshold-not-finite = EntropyScanner: threshold_bits 必须是有限值；NaN/无穷大会静默禁用熵检测
secret-scan-oversized-rejected = 超大输入被 fail-closed Secret 扫描出口拒绝
secret-scan-limit-withheld = Secret 扫描超限；输出已扣留（fail-closed）
secret-pattern-scan-match-error = Secret 模式扫描期间正则匹配出错；跳过本次匹配
masking-detect-match-error = detect 期间正则匹配出错；跳过本次匹配
masking-from-rules-duplicate-names = from_rules: 规则名重复会使 detect 归因产生歧义

# 配置验证警告
warn-http_feature_not_compiled = http_server.enabled = true 但未编译 'http' feature；监控服务器不会启动
