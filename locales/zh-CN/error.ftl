# 错误类型前缀

error-config_error = 配置错误
error-io_error = I/O 错误
error-serialization_error = 序列化错误
error-database_error = 数据库错误
error-cache_error = 缓存错误
error-encryption_error = 加密错误
error-secret_scan_limit = Secret 扫描超限
error-secret_scan_oversized_input = Secret 扫描输入超长
error-shutdown_error = 关闭错误
error-channel_error = 通道错误
error-compression_error = 压缩错误
error-runtime_error = 运行时错误
error-http_server_error = HTTP 服务器错误
error-unknown_error = 未知错误
query-decompression_limit_exceeded = 解压输出超过 1 GiB 安全上限（疑似解压炸弹），查询已中止
masking-limit-exceeded = 输入超过 1 MiB 上限，已跳过脱敏扫描并整体遮蔽（不扫描即不可证安全）
file-retention-compliance-hint = encrypt 已开启且 retention_days 低于 180：等保 2.0 要求日志留存不少于 6 个月，建议调大 retention_days 与 keep_files
error-redaction_pattern_compile_failed = 敏感脱敏正则编译失败；该规则已停用，对应消息将不进行脱敏
log_record-non_finite_field = inklog：字段 '{ $field }' 非有限值（{ $sentinel }），已按字符串哨兵存储

# 查询 / 解密 / 解压错误明细（生产代码路径）

query-encrypted-file-needs-key-env = 加密日志文件“{ $path }”需要密钥环境变量（使用 --key-env，约定名 INKLOG_ENCRYPTION_KEY）
query-decrypted-not-utf8 = 解密后的日志文件“{ $path }”不是有效 UTF-8：{ $err }
query-not-utf8 = 日志文件“{ $path }”不是有效 UTF-8：{ $err }
query-decompressed-not-utf8 = 解压后的日志文件“{ $path }”不是有效 UTF-8：{ $err }
query-compression-feature-missing = “{ $ext }”日志文件需要启用“{ $feature }”特性
query-decode-failed = “{ $path }”的 { $codec } 解码失败：{ $err }
query-encrypted-file-too-small = 加密文件“{ $path }”太小，无法容纳文件头
query-truncated-v1-header = v1 文件头被截断
query-truncated-legacy-header = legacy 文件头被截断
query-decryption-failed-wrong-key = 解密失败（密钥错误？）
query-unsupported-encryption-version = “{ $path }”使用不支持的加密版本 { $version }
