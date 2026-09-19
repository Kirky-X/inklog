# Sink 健康监控（SinkHealthMonitor）运行时消息

# 降级与恢复追踪事件
metrics-sink_recovering = Sink { $sink } 正在从 { $target } 恢复
metrics-sink_recovery_max_retries = Sink { $sink } 超过最大恢复尝试次数（{ $max_retries }）
metrics-recovery_attempt_delay = 尝试恢复，延迟 { $delay_ms }ms
metrics-sink_failed_fallback_disabled = Sink { $sink } 故障但自动降级已禁用
metrics-sink_fallback_triggered = Sink { $sink } 降级到 { $target }，原因: { $error }
metrics-sink_failure_warning = Sink { $sink } 连续第 { $count } 次故障
metrics-encryption_error_fallback = 加密密钥错误，降级为明文写入
metrics-sink_recovery_confirmed = Sink { $sink } 恢复成功，已切回正常模式

# 降级原因字符串（存入 FallbackState/FallbackEvent 记录）
metrics-fallback_reason_database = Database 故障: { $error }
metrics-fallback_reason_disk_full = 磁盘空间不足: { $error }
metrics-fallback_reason_file = FileSink 故障: { $error }
metrics-fallback_reason_unknown = 未知故障: { $error }
metrics-fallback_reason_encryption = 加密密钥错误: { $error }
metrics-fallback_reason_plaintext = 明文写入（加密错误）: { $error }
metrics-recovery_succeeded = 恢复成功
metrics-unknown_error = 未知错误
