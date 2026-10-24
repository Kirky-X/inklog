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

# Channel-buffered 文件 Sink 维护（清理线程与轮转）
sink-channel_buffered_cleanup_failed = ChannelBufferedFileSink 清理失败: { $err }
sink-channel_buffered_cleanup_panic = 清理线程异常: { $msg }
sink-rotate_rename_copy_both_failed = 轮转时重命名与复制均失败，目标文件: { $path }

# Fallback 冲刷维护
sink-fallback_flush_ticker_spawn_failed = 未能启动 fallback 定时冲刷线程；ERROR/FATAL 降级缓冲仅在退出时排空

# File sink worker 与操作失败消息
sink-file_open_failed = 打开日志文件失败：{ $err }
sink-file-rename_failed = 重命名日志文件失败：{ $err }
sink-cleanup_failed = 清理失败：{ $err }
sink-cleanup_dir_read_failed = 清理时读取目录 '{ $dir }' 失败：{ $err }
sink-size_cleanup_remove_failed = 大小清理阶段删除 { $path } 失败：{ $err }
sink-expiry_cleanup_remove_failed = 过期清理阶段删除 { $path } 失败：{ $err }
sink-record_lost = 日志记录丢失：{ $reason }
sink-batch_write_failed = 批量写入失败：{ $err }
sink-batch_write_requeued = 批量写入失败：{ $unsent } 条未发送记录已重新入队等待重试
sink-rotate_copy_then_remove_failed = 轮转时复制后删除原文件失败：{ $err }
sink-encrypt_read_input_failed = 读取待加密文件失败：{ $err }
sink-encryption_failed = 加密失败：{ $err }
sink-encrypted_file_create_failed = 创建加密文件失败：{ $err }
sink-plaintext_residue_remove_failed = 加密后删除明文源文件 { $path } 失败：{ $err }
sink-original_after_compression_remove_failed = 压缩后删除原文件 { $path } 失败：{ $err }
sink-original_after_encryption_remove_failed = 加密后删除原文件 { $path } 失败：{ $err }
sink-no_compression_backend = 请求了压缩，但未启用任何压缩后端特性（请启用 "gzip" 或 "compression"）；文件保持未压缩
sink-low_disk_space_check = 磁盘空间不足——写入前检查
sink-rotation_failed = 轮转失败：{ $err }
sink-rotation_timer_shutdown_timeout = 告警：轮转定时器关闭超时（超过 { $timeout_ms }ms）
sink-cleanup_timer_shutdown_timeout = 告警：清理定时器关闭超时（超过 { $timeout_ms }ms）

# ChannelBufferedFileSink 写入/刷新失败消息
sink-channel_write_failed = ChannelBufferedFileSink：写入失败：{ $err }
sink-channel_flush_failed = ChannelBufferedFileSink：刷新失败：{ $err }
sink-channel_drain_write_failed = ChannelBufferedFileSink：drain 期间写入失败：{ $err }
sink-channel_final_flush_failed = ChannelBufferedFileSink：最终刷新失败：{ $err }
sink-channel_periodic_flush_failed = ChannelBufferedFileSink：周期刷新失败：{ $err }
sink-channel_reopen_after_rotation_failed = ChannelBufferedFileSink：轮转后重开活动文件失败：{ $err }

# Database sink 运维消息
sink-database_buffer_overflow = database sink 缓冲区溢出，已丢弃最旧的缓冲记录
sink-database_line_num_clamp = line_num 超出 i32 范围，已收敛
db-batch_insert_warn_failed = 数据库批量插入失败

# 装配不变量与 worker 健康面（panic/健康）
sink-db_sender = db sender 必然在位
sink-db_sender_present_when_db_enabled = db_enabled 时 db sender 必然在位
sink-worker_handles_lock_poisoned = worker 句柄锁已中毒：{ $err }
manager-level_state_mutex_poisoned = level_state 互斥锁已中毒
sink-high_error_count = 严重：Sink '{ $name }' 连续错误数过高（{ $failures }）

# 掩码 / secret 扫描引擎告警与正则编译不变量
masking-duplicate_rule_name = TOML 配置中存在重复的掩码规则名；保留首个定义并跳过后者
masking-cjk_name_shape_invariant = 非法中文姓名形态正则
masking-email_invariant = 非法邮箱正则
masking-phone_invariant = 非法手机号正则
masking-id_card_invariant = 非法身份证号正则
masking-bank_card_invariant = 非法银行卡号正则
masking-api_key_invariant = 非法 API key 正则
masking-aws_key_invariant = 非法 AWS 密钥正则
masking-jwt_invariant = 非法 JWT 正则
masking-generic_secret_invariant = 非法通用密钥正则
masking-international_phone_invariant = 非法国际电话正则
masking-credit_card_invariant = 非法银行卡正则
masking-ipv4_invariant = 非法 IPv4 正则
masking-ipv6_invariant = 非法 IPv6 正则
masking-mac_address_invariant = 非法 MAC 地址正则
masking-passport_invariant = 非法护照号正则
masking-ssn_invariant = 非法 SSN 正则
masking-db_connection_invariant = 非法数据库连接串正则
masking-github_token_invariant = 非法 GitHub 令牌正则
masking-slack_token_invariant = 非法 Slack 令牌正则
masking-stripe_key_invariant = 非法 Stripe 密钥正则
masking-google_api_key_invariant = 非法 Google API key 正则
masking-private_key_invariant = 非法私钥 PEM 块正则
secret-token_regex_invariant = token 正则必然可编译
secret-builtin_pattern_invariant = 内置 secret 模式必然可编译
secret-extended_pattern_invariant = 扩展 secret 模式必然可编译
template-unclosed_placeholder = 模板含未闭合的占位符；将按字面文本渲染

# confers 审计 writer 不变量（panic 面）与模块构建观察者
audit-serialize-invariant = AuditEvent 序列化不应失败：变体载荷均为可序列化基本类型
audit-writer-rx-lock-not-poisoned = writer 接收端锁不会中毒（临界区无 panic 点）
audit-writer-rx-present-at-startup = writer 首次启动时接收端必然在位
kit-observer-module_build_failed = inklog 构建观察者：模块构建失败

# 运行期日志消息：sink worker 生命周期与文件 sink 操作
log-sink_factory_failed_on_startup = { $sink } sink 工厂启动失败；进入降级重试模式（指数退避：1 秒起倍增至 30 秒，无限重试）；重试期间到达的记录转发至 error sink 并计为 failed
log-sink_factory_retry_failed = { $sink } sink 工厂重试失败；worker 保持存活并以指数退避继续重试
log-sink_factory_recovered = { $sink } sink 工厂重试成功；worker 恢复正常写入
log-sink_factory_registered = 正在注册 sink 工厂：{ $type }
log-sink_recovery_sent = 健康检查：已向 sink '{ $name }' 发送恢复命令
log-record_lost_stderr = 丢失日志记录（sink: { $path }，target: { $target }，level: { $level }）：{ $reason }
log-file_rotating = 正在轮转日志文件：{ $path }
log-file_rotated_to = 日志已轮转至：{ $path }
log-file_opened = 已打开日志文件：{ $path }（大小：{ $size } 字节）
log-file_encrypted = 已加密日志文件：{ $path }
log-encryption_raw_key_warning = 32 字节输入被直接用作原始加密密钥；建议使用 Base64 编码的随机密钥或长度不为 32 的密码
log-kit-module_ready = InklogModule：on_ready — 数据库依赖已验证
log-kit-module_shutdown = InklogModule：on_shutdown — 正在释放数据库适配器
log-kit_observer_module_build_started = inklog 构建观察者：模块构建开始
log-kit_observer_module_built = inklog 构建观察者：模块构建成功
log-db_batch_insert_succeeded = 数据库批量插入成功
log-db_batch_insert_succeeded_parameterized = 数据库批量插入成功（参数化）
log-custom_sink_write_failed = 自定义 sink '{ $name }' 写入失败（第 { $attempt }/{ $max_attempts } 次尝试）：{ $err }
sink-parquet_feature_not_enabled = 未启用 parquet 特性：请以 `parquet` 特性重新构建 inklog 以将日志导出为 Parquet
