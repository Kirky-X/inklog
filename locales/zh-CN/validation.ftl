# 路径验证消息

validation-path_traversal = 检测到路径遍历
validation-dangerous_component = 危险的路径组件: { $component }
validation-no_absolute = 不允许绝对路径
validation-no_symlinks = 不允许符号链接
validation-outside_base = 路径在基础目录之外

# 内置 sanitizer 正则编译不变量（panic 面）
sanitize-ansi_sgr_invariant = 内置 ANSI SGR 正则必然合法
sanitize-card_number_invariant = 内置银行卡号正则必然合法
sanitize-email_invariant = 内置邮箱正则必然合法
sanitize-ssn_invariant = 内置 SSN 正则必然合法
sanitize-password_invariant = 内置密码赋值正则必然合法
sanitize-token_invariant = 内置令牌赋值正则必然合法
sanitize-api_key_invariant = 内置 api_key 赋值正则必然合法
sanitize-bearer_token_invariant = 内置 Bearer 令牌正则必然合法
sanitize-basic_auth_invariant = 内置 Basic 认证正则必然合法

# LoggerBuilder 校验错误（累积进 validation_errors）

builder-invalid-level = 无效的日志级别“{ $level }”。有效级别：{ $levels }
builder-driver-conflict-explicit = 数据库驱动“{ $driver }”（显式设置）与从 URL“{ $url }”推断的驱动“{ $inferred }”冲突。以推断的驱动为准。
builder-driver-conflict = 数据库驱动“{ $driver }”与从 URL“{ $url }”推断的驱动“{ $inferred }”冲突。
builder-http-port-invalid = HTTP 端口必须在 1 到 65535 之间
builder-http-error-mode-unknown = 未知的 HTTP 错误模式“{ $mode }”。有效模式：warn、strict。使用默认值“strict”。
