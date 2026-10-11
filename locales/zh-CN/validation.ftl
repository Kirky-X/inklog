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
