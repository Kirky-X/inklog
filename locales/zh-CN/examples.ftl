# 示例程序输出（每条打印消息一个键；运行时见 src/i18n）。
# 参数以 { $name } 占位，经 inklog::i18n::tr_args 填充。

# config-file
example-config-file-inklog-config-file-loading = === inklog 配置文件加载示例 ===
example-config-file-all-config-file-loading = ✓ 配置文件加载示例全部演示完毕
example-config-file-1-toml-config-file-format = 1. TOML 配置文件格式
example-config-file-the-config-file-contains-the = --- 配置文件包含以下配置段 ---
example-config-file-global-global-log-level-format = - [global]            全局日志级别、格式、掩码、降级
example-config-file-console-sink-console-output = - [console_sink]      控制台输出（颜色、stderr 路由）
example-config-file-file-sink-file-output-path = - [file_sink]         文件输出（路径、轮转、压缩、加密）
example-config-file-database-sink-database-output = - [database_sink]     数据库输出（驱动、连接、批处理）
example-config-file-performance-performance-tuning = - [performance]       性能调优（通道容量、worker 线程数）
example-config-file-http-server-http-health-check = - [http_server]       HTTP 健康检查与 Prometheus 指标端点
example-config-file-2-inklogconfig-sync-parsing = 2. InklogConfig 同步解析（from_str）
example-config-file-2-1-parsing-config-with-toml = 2.1 使用 toml::from_str 解析配置
example-config-file-global-level = global.level           = { $level }
example-config-file-global-format = global.format          = { $format }
example-config-file-global-masking-enabled = global.masking_enabled = { $masking_enabled }
example-config-file-console-sink-enabled = console_sink.enabled   = { $value }
example-config-file-console-sink-colored = console_sink.colored   = { $value }
example-config-file-2-2-file-sink-fields = 2.2 file_sink 字段
example-config-file-file-sink-enabled = file_sink.enabled          = { $enabled }
example-config-file-file-sink-path = file_sink.path             = { $value }
example-config-file-file-sink-max-size = file_sink.max_size         = { $max_size }
example-config-file-file-sink-rotation-time = file_sink.rotation_time    = { $rotation_time }
example-config-file-file-sink-compress = file_sink.compress         = { $compress }
example-config-file-file-sink-compression-level = file_sink.compression_level = { $compression_level }
example-config-file-2-3-database-sink-fields = 2.3 database_sink 字段
example-config-file-database-sink-enabled = database_sink.enabled    = { $enabled }
example-config-file-database-sink-driver = database_sink.driver     = { $driver }
example-config-file-database-sink-url = database_sink.url        = { $url }
example-config-file-2-4-performance-fields = 2.4 performance 字段
example-config-file-channel-capacity = channel_capacity = { $channel_capacity }
example-config-file-worker-threads = worker_threads   = { $worker_threads }
example-config-file-2-5-http-server-fields = 2.5 http_server 字段
example-config-file-http-server-enabled = http_server.enabled = { $enabled }
example-config-file-http-server-host = http_server.host    = { $host }
example-config-file-http-server-port = http_server.port    = { $port }
example-config-file-2-6-validate-check = 2.6 validate() 校验
example-config-file-config-validation-passed = ✓ 配置校验通过
example-config-file-3-from-search-paths-auto-search = 3. from_search_paths 自动搜索（load 的配置来源）
example-config-file-3-1-setting-the-inklog-config = 3.1 设置 INKLOG_CONFIG_PATH 环境变量
example-config-file-inklog-config-path = INKLOG_CONFIG_PATH = { $value }
example-config-file-3-2-reading-and-parsing-with = 3.2 用 from_search_paths 读取并解析
example-config-file-loaded-from-inklog-config-path = ✓ 已从 INKLOG_CONFIG_PATH 加载：level = { $level }
example-config-file-inklog-config-path-env-var = INKLOG_CONFIG_PATH 环境变量已移除
example-config-file-note-loggermanager-load-calls = 注意：LoggerManager::load() 调用 InklogConfig::load_sync()，
example-config-file-searching-for-the-config-file = 按以下顺序搜索配置文件（首个存在的文件胜出）：
example-config-file-1-inklog-config-path = 1. $INKLOG_CONFIG_PATH
example-config-file-2-inklog-config-toml-current = 2. ./inklog_config.toml（当前目录）
example-config-file-3-config-inklog-config-toml = 3. ~/.config/inklog/config.toml
example-config-file-4-etc-inklog-config-toml = 4. /etc/inklog/config.toml
example-config-file-if-none-exist-default-default = 若均不存在，则使用 Default::default()。
example-config-file-4-initializing-the-logger-with = 4. 使用 LoggerManager::from_file 初始化 logger
example-config-file-temporary-config-file = 临时配置文件：{ $value }
example-config-file-4-1-loggermanager-from-file-path = 4.1 LoggerManager::from_file(path)
example-config-file-loggermanager-initialized = ✓ LoggerManager 已初始化
example-config-file-4-2-logging-via-tracing-macros = 4.2 通过 tracing 宏记录日志（验证 logger 处于活动状态）
example-config-file-info-log-from-the-config-file = 来自 config_file 示例的 INFO 日志
example-config-file-warn-log-from-the-config-file = 来自 config_file 示例的 WARN 日志
example-config-file-error-log-from-the-config-file = 来自 config_file 示例的 ERROR 日志
example-config-file-4-3-from-file-vs-load-comparison = 4.3 from_file 与 load 对比
example-config-file-from-file-path-load-the-config = from_file(path)：从给定路径加载配置文件
example-config-file-note-loggermanager-internally = 注意：LoggerManager 内部会创建 logs/error.log 用于系统错误日志，
example-config-file-and-installs-the-global-tracing = 并安装全局 tracing subscriber 与 log crate 适配器。

# config-inspect
example-config-inspect-inklog-config-inspection-example = inklog 配置检查示例
example-config-inspect-all-config-inspection-examples = 配置检查示例全部演示完毕。
example-config-inspect-example-1-all-sinks-disabled = 示例 1：全部 sink 禁用 → sinks_enabled() 返回空
example-config-inspect-all-sinks-explicitly-disabled = 显式禁用全部 sink：
example-config-inspect-console-sink-enabled = console_sink.enabled   = { $value }
example-config-inspect-file-sink-enabled = file_sink.enabled      = { $value }
example-config-inspect-database-sink-enabled = database_sink.enabled  = { $value }
example-config-inspect-sinks-enabled = sinks_enabled() = { $sinks }
example-config-inspect-returned-vec-length = 返回的 Vec 长度 = { $value }
example-config-inspect-comparison-console-sink-in = 对比：InklogConfig::default() 中的 console_sink 默认启用
example-config-inspect-default-sinks-enabled = default().sinks_enabled() = { $default_sinks }
example-config-inspect-example-2-only-the-console-sink = 示例 2：仅启用 console sink
example-config-inspect-config = 配置：
example-config-inspect-console-sink-enabled-2 = console_sink.enabled   = { $value } ✓
example-config-inspect-file-sink-enabled-path = file_sink.enabled      = { $value } ✓ (path={ $file_path })
example-config-inspect-database-sink-enabled-2 = database_sink.enabled  = { $value } ✓
example-config-inspect-contains-console = 包含 "console" = { $value }
example-config-inspect-example-3-multiple-sinks = 示例 3：启用多个 sink（Console + File + Database）
example-config-inspect-sink-count-expected-3 = Sink 数量 = { $value }（期望 3）
example-config-inspect-verification = 校验：
example-config-inspect-contains-console-2 = 包含 "console"  = { $has_console } → { $value }
example-config-inspect-contains-file = 包含 "file"     = { $has_file } → { $value }
example-config-inspect-contains-database = 包含 "database" = { $has_database } → { $value }
example-config-inspect-example-4-loggermanager-load = 示例 4：LoggerManager::load() 用法
example-config-inspect-loggermanager-load-flow = LoggerManager::load() 流程：
example-config-inspect-1-call-inklogconfig-load-sync = 1. 调用 InklogConfig::load_sync() 搜索配置文件
example-config-inspect-search-paths-first-existing = - 搜索路径（首个存在的文件胜出）：
example-config-inspect-1-inklog-config-path-explicit = 1. $INKLOG_CONFIG_PATH（显式环境变量，最高优先级）
example-config-inspect-2-inklog-config-toml-current = 2. ./inklog_config.toml（当前目录）
example-config-inspect-2-call-loggermanager-with = 2. 用加载的配置调用 LoggerManager::with_config()
example-config-inspect-3-initialize-all-enabled-sinks = 3. 初始化所有启用的 sink 与 HTTP 服务
example-config-inspect-code-example = 代码示例：
example-config-inspect-custom-search-paths = 自定义搜索路径：
example-config-inspect-note-load-is-an-async-method = 注意：load() 是异步方法，必须在 tokio runtime 中调用。
example-config-inspect-example-5-config-inspection = 示例 5：配置检查工作流
example-config-inspect-recommended-config-inspection = 推荐的配置检查工作流：
example-config-inspect-step-1-load-the-config = 步骤 1：加载配置
example-config-inspect-failed-to-load-config-using-the = ⚠ 配置加载失败：{ $e }，改用默认配置
example-config-inspect-config-loaded = ✓ 配置已加载
example-config-inspect-step-2-check-enabled-sinks = 步骤 2：检查启用的 sink
example-config-inspect-no-sink-enabled-logs-will-not = ⚠ 未启用任何 sink；日志将不会输出
example-config-inspect-enabled-sinks = ✓ 已启用的 sink：{ $sinks }
example-config-inspect-step-3-verify-config-consistency = 步骤 3：校验配置一致性
example-config-inspect-sink-state-is-consistent = ✓ Sink 状态一致
example-config-inspect-step-4-choose-the = 步骤 4：根据配置选择初始化方式
example-config-inspect-use-loggermanager-new-default = → 使用 LoggerManager::new()（仅默认 console）
example-config-inspect-use-loggermanager-with-config = → 使用 LoggerManager::with_config()（显式配置）
example-config-inspect-full-code-example = 完整代码示例：

# env-overrides
example-env-overrides-inklog-env-var-override-loading = inklog 环境变量覆盖加载示例
example-env-overrides-all-env-var-override-examples = 环境变量覆盖示例全部演示完毕。
example-env-overrides-example-1-supported-environment = 示例 1：支持的环境变量
example-env-overrides-global-config = 全局配置：
example-env-overrides-performance-config = 性能配置：
example-env-overrides-file-sink-config = File sink 配置：
example-env-overrides-http-server-config = HTTP 服务配置：
example-env-overrides-database-sink-config = Database sink 配置：
example-env-overrides-example-2-calling-load-with-env = 示例 2：实际调用 load_with_env_overrides()
example-env-overrides-calling-inklogconfig-load-with = 正在调用 InklogConfig::load_with_env_overrides()……
example-env-overrides-config-loaded-successfully = ✓ 配置加载成功
example-env-overrides-loaded-config = 已加载的配置：
example-env-overrides-global-level = global.level          = "{ $level }"
example-env-overrides-global-format = global.format         = "{ $format }"
example-env-overrides-global-auto-fallback = global.auto_fallback  = { $auto_fallback }
example-env-overrides-performance-channel-capacity = performance.channel_capacity  = { $channel_capacity }
example-env-overrides-performance-worker-threads = performance.worker_threads    = { $worker_threads }
example-env-overrides-performance-channel-strategy = performance.channel_strategy  = { $channel_strategy }
example-env-overrides-file-sink-enabled = file_sink.enabled     = { $value }
example-env-overrides-http-server-enabled = http_server.enabled   = { $value }
example-env-overrides-database-sink-enabled = database_sink.enabled = { $value }
example-env-overrides-failed-to-load-config = ✗ 配置加载失败：{ $e }
example-env-overrides-this-usually-means-a-config = （通常意味着配置文件解析错误；请检查 TOML 语法）
example-env-overrides-example-3-env-var-override = 示例 3：环境变量覆盖效果演示
example-env-overrides-test-id-isolates-concurrent-runs = 测试 ID：{ $test_id }（隔离并发运行）
example-env-overrides-baseline-config-no-env-var = 基线配置（无环境变量覆盖）：
example-env-overrides-global-level-2 = global.level = "{ $level }"
example-env-overrides-performance-channel-capacity-2 = performance.channel_capacity = { $channel_capacity }
example-env-overrides-performance-worker-threads-2 = performance.worker_threads = { $worker_threads }
example-env-overrides-setting-environment-variables = 设置环境变量：
example-env-overrides-export = export { $key }={ $value }
example-env-overrides-config-after-overrides = 覆盖后的配置：
example-env-overrides-global-level-expected-debug = global.level = "{ $level }"（期望："debug"）
example-env-overrides-performance-channel-capacity-3 = performance.channel_capacity = { $channel_capacity }（期望：20000）
example-env-overrides-performance-worker-threads-3 = performance.worker_threads = { $worker_threads }（期望：4）
example-env-overrides-verification-results = 校验结果：
example-env-overrides-global-level-override = global.level 覆盖：{ $value } → { $value2 }
example-env-overrides-channel-capacity-override = channel_capacity 覆盖：{ $value } → { $value2 }
example-env-overrides-worker-threads-override = worker_threads 覆盖：{ $value } → { $value2 }
example-env-overrides-test-environment-variables = （测试环境变量已清理）
example-env-overrides-example-4-config-precedence = 示例 4：配置优先级
example-env-overrides-inklog-config-loading = inklog 配置加载优先级（由低到高）：
example-env-overrides-1-code-defaults-default-default = 1. 代码默认值（Default::default()）
example-env-overrides-2-config-file-inklog-toml = 2. 配置文件（inklog.toml / INKLOG_CONFIG_PATH）
example-env-overrides-3-env-var-overrides-inklog = 3. 环境变量覆盖（INKLOG_*）
example-env-overrides-load-with-env-overrides-flow = load_with_env_overrides() 流程：
example-env-overrides-1-call-load-sync-to-load = 1. 调用 load_sync() 加载默认值 + 配置文件
example-env-overrides-2-call-apply-env-overrides-to = 2. 调用 apply_env_overrides() 应用 INKLOG_* 环境变量
example-env-overrides-3-return-the-final-config = 3. 返回最终配置
example-env-overrides-recommendations = 建议：
example-env-overrides-ci-cd-use-env-vars-to-switch = - CI/CD：用环境变量切换 dev/staging/prod 配置
example-env-overrides-code-example = 代码示例：
example-env-overrides-inklogconfig-load-with-env = InklogConfig::load_with_env_overrides()?

# all-features
example-all-features-inklog-full-feature-demo = === inklog 全功能演示 ===
example-all-features-all-feature-demos-completed = 全部功能演示完毕。
example-all-features-feature-1-data-masking = --- 功能 1：数据掩码（DataMasker）---
example-all-features-built-in-rule-masking = 内置规则掩码：
example-all-features-raw = 原始：{ $input }
example-all-features-masked = 掩码后：{ $masked }
example-all-features-email-and-phone-number-masked = ✓ 邮箱与手机号已掩码
example-all-features-custom-rule-masking = 自定义规则掩码：
example-all-features-raw-2 = 原始：{ $order_input }
example-all-features-masked-2 = 掩码后：{ $order_masked }
example-all-features-custom-order-id-rule-works-all = ✓ 自定义订单号规则生效（默认替换全部匹配）
example-all-features-no-sensitive-data-test = 无敏感数据测试：
example-all-features-content-unchanged-as-expected = ✓ 内容保持不变（符合预期）
example-all-features-feature-2-log-templates = --- 功能 2：日志模板（LogTemplate）---
example-all-features-default-template-render = 默认模板渲染：
example-all-features-output = 输出：{ $rendered }
example-all-features-render-check-passed = ✓ 渲染检查通过
example-all-features-simple-template-render = 简单模板渲染：
example-all-features-output-2 = 输出：{ $simple_rendered }
example-all-features-exact-match-verified = ✓ 精确匹配校验通过
example-all-features-multi-format-comparison = 多格式对比：
example-all-features-json-style-output-parses-with = ✓ JSON 风格输出可被 serde_json 解析
example-all-features-feature-3-log-level-control = --- 功能 3：日志级别控制 ---
example-all-features-log-level-debug = 日志级别 = debug：
example-all-features-trace-not-emitted-below-debug = TRACE - 未输出（低于 debug）
example-all-features-debug-emitted = DEBUG - 已输出
example-all-features-info-emitted = INFO - 已输出
example-all-features-trace-filtered-out-debug-and = ✓ trace 被过滤；debug 及以上可见
example-all-features-after-set-level-none-warn-hot = set_level(None, "warn") 热更新后：
example-all-features-info-not-emitted-below-warn = INFO - 未输出（低于 warn）
example-all-features-warn-emitted = WARN - 已输出
example-all-features-error-emitted = ERROR - 已输出
example-all-features-info-debug-trace-filtered-out = ✓ info/debug/trace 被过滤；warn 及以上可见
example-all-features-level-restored-to-info-set = ✓ 级别已恢复为 info（set_level 热更新，logger 保持存活）
example-all-features-feature-4-structured-logging = --- 功能 4：结构化日志 ---
example-all-features-request-log-request-id-method = ✓ 请求日志（request_id、method、path、status、latency_ms）
example-all-features-error-log-error-code-component = ✓ 错误日志（error_code、component、retry_count）
example-all-features-loggermanager-builder-console = 需要整条 JSON 记录时使用 LoggerManager::builder().console_json(true)
example-all-features-feature-5-path-validation = --- 功能 5：路径校验（PathValidator）---
example-all-features-all-path-validation-results-are = ✓ 全部路径校验结果符合预期
example-all-features-feature-6-object-pool-objectpool = --- 功能 6：对象池（ObjectPool）---
example-all-features-object-pool-get-put-verified = ✓ 对象池 get/put 校验通过
example-all-features-key1-key2-missing = key1 = { $v1 }，key2 = { $v2 }，missing = { $v3 }
example-all-features-feature-7-multi-module-combined = --- 功能 7：多模块组合工作流 ---
example-all-features-1-create-a-datamasker = 1. 创建 DataMasker
example-all-features-2-create-a-logtemplate = 2. 创建 LogTemplate
example-all-features-3-raw-log = 3. 原始日志：{ $raw_message }
example-all-features-4-masked = 4. 掩码后：  { $safe_message }
example-all-features-5-template-render = 5. 模板渲染：{ $rendered }
example-all-features-masking-template-logging = ✓ 掩码 → 模板 → 日志 全链路校验通过
example-all-features-best-practices = --- 最佳实践 ---
example-all-features-1-feature-combination-principles = 1. 功能组合原则：
example-all-features-call-datamasker-before-writing = - 写日志前先调用 DataMasker（掩码 → 记录）
example-all-features-logtemplate-unifies-the-log = - LogTemplate 统一日志格式；全局共用一个模板
example-all-features-pathvalidator-validates-file = - PathValidator 校验来自用户输入的文件路径
example-all-features-2-masking-rule-design = 2. 掩码规则设计：
example-all-features-prefer-built-in-rules-email = - 优先使用内置规则（邮箱、手机号、密码等）
example-all-features-give-custom-rules-meaningful = - 自定义规则使用可读名称，便于维护
example-all-features-review-masking-rules-regularly = - 定期评审掩码规则，覆盖新增敏感字段
example-all-features-3-log-level-strategy = 3. 日志级别策略：
example-all-features-trace-development-only-disabled = - trace：仅开发环境，生产禁用
example-all-features-debug-development-test-enable = - debug：开发/测试环境；生产按需临时开启
example-all-features-info-production-default-records = - info：生产默认，记录关键业务事件
example-all-features-warn-recoverable-exceptions = - warn：可恢复异常；需要关注但无需立即处理
example-all-features-error-functionality-failure = - error：功能失败，需要人工介入
example-all-features-4-structured-field-conventions = 4. 结构化字段约定：
example-all-features-use-snake-case-names-user-id = - 使用 snake_case 命名（user_id、request_id）
example-all-features-pass-numeric-values-directly = - 数值直接传（status = 200，不要转字符串）
example-all-features-always-include-component-or = - 始终带上 component 或 target 以标识来源
example-all-features-error-logs-must-include-error = - 错误日志必须包含 error_code

# basic
example-basic-inklog-basic-usage-examples = === inklog 基础用法示例 ===
example-basic-all-basic-examples-completed = 基础示例演示完毕。
example-basic-example-1-default-initialization = --- 示例 1：默认初始化 ---
example-basic-loggermanager-new-initialized = ✓ LoggerManager::new() 初始化成功（logger 在 main 中创建）
example-basic-log-system-started = 日志系统已启动
example-basic-logging-works = ✓ 日志记录正常
example-basic-example-2-log-levels = --- 示例 2：日志级别 ---
example-basic-5-standard-log-levels = 5 个标准日志级别：
example-basic-trace-most-detailed-diagnostics = TRACE - 最详细的诊断信息，仅开发环境启用
example-basic-debug-debugging-information-to = DEBUG - 调试信息，帮助定位问题
example-basic-info-routine-runtime = INFO - 常规运行信息，标记重要事件
example-basic-warn-potential-issue-that-does = WARN - 潜在问题，不影响正常运行
example-basic-error-functionality-failure = ERROR - 功能失败，需要关注
example-basic-level-filtering-rules-default = 级别过滤规则（默认 info）：
example-basic-trace-only-tracing-trace-visible = trace → 仅 tracing::trace! 可见
example-basic-debug-debug-and-above-visible = debug → debug 及以上可见
example-basic-info-info-warn-error-visible = info  → info/warn/error 可见（默认）
example-basic-warn-warn-error-visible = warn  → warn/error 可见
example-basic-error-only-error-visible = error → 仅 error 可见
example-basic-example-3-structured-logging = --- 示例 3：结构化日志 ---
example-basic-user-operation-logged-user-id = ✓ 用户操作已记录（user_id、action、ip）
example-basic-request-logged-request-id = ✓ 请求已记录（request_id、method、path、status、latency_ms）
example-basic-error-logged-error-code = ✓ 错误已记录（error_code、component、retry_count）
example-basic-example-4-health-status-check = --- 示例 4：健康状态检查 ---
example-basic-note-health-status-must-be = 注意：健康状态必须在 LoggerManager 实例上检查。
example-basic-this-example-shows-the-api = 本示例展示 API 用法（需要持有 logger 引用）：
example-basic-example-5-custom-configuration = --- 示例 5：自定义配置初始化 ---
example-basic-approach-1-modify-the-default = 方式 1：修改默认配置
example-basic-approach-2-builder-pattern = 方式 2：Builder 模式
example-basic-example-6-graceful-shutdown = --- 示例 6：优雅关闭 ---
example-basic-application-started = 应用已启动
example-basic-processing-business-logic = 正在处理业务逻辑……
example-basic-loggermanager-shutdown-completed = ✓ LoggerManager::shutdown() 完成
example-basic-all-logs-left-in-the-channel = - 通道中剩余的日志已全部写入 sink
example-basic-all-sinks-are-closed = - 所有 sink 已关闭
example-basic-worker-threads-stopped = - worker 线程已停止
example-basic-best-practices = --- 最佳实践 ---
example-basic-1-where-to-initialize = 1. 在哪里初始化：
example-basic-initialize-loggermanager-as = - 尽量在 main() 中尽早初始化 LoggerManager
example-basic-bind-the-variable-with-let = - 用 `let _logger = ...` 绑定变量，避免 Drop 提前关闭
example-basic-2-choosing-the-log-level = 2. 选择日志级别：
example-basic-development-debug-or-trace = - 开发环境：debug 或 trace
example-basic-production-info-default-or-warn = - 生产环境：info（默认）或 warn
example-basic-troubleshooting-temporarily = - 排障：临时提升到 debug，事后恢复
example-basic-3-structured-fields = 3. 结构化字段：
example-basic-use-tracing-structured-fields = - 使用 tracing 结构化字段（key = value）
example-basic-avoid-concatenating-json = - 避免把 JSON 字符串拼接进消息
example-basic-common-fields-request-id-user = - 常用字段：request_id、user_id、component、latency_ms
example-basic-4-error-logs = 4. 错误日志：
example-basic-error-only-for-failures-that = - error! 仅用于需要人工介入的故障
example-basic-warn-for-exceptions-that = - warn!  用于可自动恢复的异常
example-basic-include-error-code-and = - 带上 error_code 与 component，便于分诊
example-basic-5-shutdown-sequence = 5. 关闭流程：
example-basic-call-logger-shutdown-before-the = - 应用退出前调用 logger.shutdown()
example-basic-call-it-from-signal-handling = - 从信号处理中调用（SIGTERM / Ctrl+C）
example-basic-shutdown-waits-until-all-logs = - shutdown() 会等待通道内日志全部写入

# builder
example-builder-inklog-builder-pattern = === inklog Builder 模式配置示例 ===
example-builder-all-configuration-examples-have = 配置示例已全部展示。
example-builder-example-1-basic-builder-usage = 示例 1：Builder 基础用法
example-builder-code-example =
    代码示例：
    { $code }
example-builder-configuration-notes = 配置说明：
example-builder-log-level-debug = ✓ 日志级别：DEBUG
example-builder-console-sink-enabled = ✓ Console sink：已启用
example-builder-file-sink-not-enabled = ✗ File sink：未启用
example-builder-example-2-multiple-sinks = 示例 2：多个 sink
example-builder-log-level-info = ✓ 日志级别：INFO
example-builder-file-sink-logs-app-log = ✓ File Sink：logs/app.log
example-builder-example-3-advanced-options = 示例 3：高级选项
example-builder-log-level-trace-most-detailed = ✓ 日志级别：TRACE（最详细）
example-builder-console-output-json-lines = ✓ 控制台输出：JSON 行（console_json(true)）
example-builder-json-output-requires-console = JSON 输出需要 console_json(true)（JSON 模式下不着色）
example-builder-example-4-performance-options = 示例 4：性能选项
example-builder-channel-capacity-50-000-default = ✓ 通道容量：50,000（默认 10,000）
example-builder-worker-threads-4-default-3 = ✓ worker 线程数：4（默认 3）
example-builder-use-case-high-throughput = ✓ 适用场景：高吞吐生产环境
example-builder-example-5-console-sink-options = 示例 5：console sink 选项
example-builder-colored-output-enabled = ✓ 彩色输出：已启用
example-builder-stderr-output-error-and-warn = ✓ stderr 输出：ERROR 与 WARN 级别
example-builder-stdout-output-all-other-levels = ✓ stdout 输出：其余级别
example-builder-example-6-file-sink-options = 示例 6：file sink 选项
example-builder-file-path-logs-app-log = ✓ 文件路径：logs/app.log
example-builder-single-file-size-100mb = ✓ 单文件大小：100MB
example-builder-compression-zstd-requires-the = ✓ 压缩：Zstd（需要 inklog 的 `compression`/`zstd` 或 `gzip` feature）
example-builder-rotation-policy-daily-rotation = ✓ 轮转策略：按天轮转
example-builder-keep-files-7 = ✓ 保留文件数：7
example-builder-only-warns-and-leaves-rotated = 仅告警并保留未压缩的轮转文件，不会报错
example-builder-example-7-builder-pattern = 示例 7：Builder 模式的灵活性
example-builder-supports-conditional = ✓ 支持条件化构建
example-builder-supports-environment-variable = ✓ 支持环境变量判断
example-builder-builder-instances-can-be-reused = ✓ Builder 实例可复用
example-builder-fluent-chained-calls = ✓ 流畅的链式调用

# error-handling
example-error-handling-inklog-error-handling-examples = inklog 错误处理示例
example-error-handling-all-error-handling-examples = ✓ 错误处理示例全部演示完毕
example-error-handling-1-all-inklogerror-variants = 1. 全部 InklogError 变体
example-error-handling-display-output = { $value } 的 Display 输出
example-error-handling-error-variants-in-total = ✓ 共 { $value } 个错误变体
example-error-handling-2-display-trait-format = 2. Display trait 格式
example-error-handling-error = 错误：{ $err }
example-error-handling-3-safe-message-sensitive-data = 3. safe_message() 敏感数据脱敏
example-error-handling-safe-message-redacts-sensitive = safe_message() 会脱敏错误消息中的敏感数据：
example-error-handling-aws-key-redaction = AWS 密钥脱敏：
example-error-handling-raw = 原始：{ $err }
example-error-handling-safe = 脱敏后：{ $value }
example-error-handling-database-connection-string = 数据库连接串脱敏：
example-error-handling-user-path-redaction = 用户路径脱敏：
example-error-handling-email-redaction = 邮箱脱敏：
example-error-handling-non-sensitive-information-is = 非敏感信息保持原样：
example-error-handling-safe-message-redacts-sensitive-2 = ✓ safe_message() 正确脱敏敏感数据
example-error-handling-4-localized-message = 4. localized_message() 国际化消息
example-error-handling-localized-message-returns-the = localized_message() 返回当前 locale 对应的错误前缀译文：
example-error-handling-localized-message = { $value } 的 localized_message()
example-error-handling-every-error-variant-supports-an = ✓ 每个错误变体都支持国际化消息
example-error-handling-5-convenience-constructors = 5. 便捷构造函数
example-error-handling-inklogerror-database-error = InklogError::database_error("connection pool exhausted")：
example-error-handling-display = Display：{ $err }
example-error-handling-message = message：{ $message }
example-error-handling-source = source： { $value }
example-error-handling-inklogerror-database-error-with = InklogError::database_error_with_source(...)：
example-error-handling-source-has-an-error-chain = source： { $value }（含错误链）
example-error-handling-inklogerror-encryption-error = InklogError::encryption_error("invalid key length: 16")：
example-error-handling-inklogerror-encryption-error-2 = InklogError::encryption_error_with_source(...)：
example-error-handling-6-automatic-from-trait = 6. From trait 自动转换
example-error-handling-io-error-inklogerror = io::Error → InklogError：
example-error-handling-serde-json-error-inklogerror = serde_json::Error → InklogError：
example-error-handling-toml-de-error-inklogerror = toml::de::Error → InklogError：
example-error-handling-standard-error-types-convert-to = ✓ 标准错误类型可自动转换为 InklogError
example-error-handling-7-the-inklogresult-t-type-alias = 7. InklogResult<T> 类型别名
example-error-handling-err-is-err = Err(...) → is_err = { $value }
example-error-handling-unexpected-success = 意外成功
example-error-handling-propagate-io-error-with-the = 用 ? 运算符向上传播 io::Error：
example-error-handling-inklogresult-works-with-the = ✓ InklogResult 可与 ? 运算符配合
example-error-handling-8-practical-error-handling = 8. 实用错误处理模式
example-error-handling-pattern-1-match-branch-handling = 模式 1：match 分支处理
example-error-handling-operation-succeeded = 操作成功
example-error-handling-config-error-recoverable = 配置错误（可恢复）：{ $msg }
example-error-handling-retry-with-the-default = → 使用默认配置重试
example-error-handling-io-error = IO 错误：{ $e }
example-error-handling-check-file-permissions-and-disk = → 检查文件权限与磁盘空间
example-error-handling-other-error = 其他错误：{ $e }
example-error-handling-report-and-exit = → 上报后退出
example-error-handling-pattern-2-safe-log-output = 模式 2：安全日志输出
example-error-handling-log-output = 日志输出：{ $value }
example-error-handling-user-message = 用户消息：{ $value }
example-error-handling-pattern-3-error-chain-tracing = 模式 3：错误链追踪
example-error-handling-top-level-error = 顶层错误：{ $err }
example-error-handling-root-cause = 根因：{ $src }

# i18n
example-i18n-inklog-internationalization = inklog 国际化（i18n）格式化示例
example-i18n-all-i18n-formatting-examples = i18n 格式化示例已全部展示。
example-i18n-example-1-create-multi-locale = 示例 1：创建多 locale formatter
example-i18n-create-the-en-us-formatter = 创建 en-US formatter：
example-i18n-logi18nformatter-new-en-us = ✓ 已创建 LogI18nFormatter::new("en-US")
example-i18n-create-the-zh-cn-formatter = 创建 zh-CN formatter：
example-i18n-logi18nformatter-new-zh-cn = ✓ 已创建 LogI18nFormatter::new("zh-CN")
example-i18n-supported-bcp-47-language-tags = 支持的 BCP-47 语言标签：
example-i18n-en-us-english-united-states = en-US：英语（美国）
example-i18n-zh-cn-chinese-simplified = zh-CN：简体中文
example-i18n-ja-jp-japanese-japan = ja-JP：日语（日本）
example-i18n-de-de-german-germany = de-DE：德语（德国）
example-i18n-field-comparison-example-a-b = 字段比较示例："a" < "b"
example-i18n-field-comparison-example = 字段比较示例：{ $other }
example-i18n-field-comparison-failed = 字段比较失败：{ $e }
example-i18n-example-2-number-formatting = 示例 2：数字格式化
example-i18n-notes = 说明：
example-i18n-en-us-comma-thousands-separator = - en-US：千位分隔符为逗号，小数点为点号
example-i18n-zh-cn-the-separator-may-differ = - zh-CN：分隔符可能不同（取决于 ICU 数据）
example-i18n-use-case-format-performance = - 适用场景：格式化日志中的性能指标与计数器
example-i18n-example-3-event-count-plurals = 示例 3：事件计数的复数形式
example-i18n-use-case-generate-locale-aware = 适用场景：生成随 locale 变化的日志消息
example-i18n-en-one-event-processed-other = en："One event processed" / "Other events processed"
example-i18n-zh-other-chinese-has-no-plural = zh："Other 事件已处理"（中文无复数形态变化）
example-i18n-example-4-timestamp-formatting = 示例 4：时间戳格式化
example-i18n-use-case-localized-display-of = 适用场景：按 locale 显示日志时间戳
example-i18n-use-the-icu4x-datetimeformatter = - 使用 ICU4X DateTimeFormatter 的 medium 格式
example-i18n-adapts-automatically-to-the = - 自动适配目标语言的日期习惯
example-i18n-example-5-log-level = 示例 5：日志级别归一化
example-i18n-log-level-normalization-unified = 日志级别归一化（统一大写）：
example-i18n-use-case-unify-the-displayed = 适用场景：统一日志级别的显示格式
example-i18n-example-6-locale-sensitive = 示例 6：locale 敏感的字段名比较
example-i18n-use-case-order-log-fields = 适用场景：按字母顺序排列日志字段
example-i18n-use-the-icu4x-collator-for = - 使用 ICU4X Collator 进行 locale 敏感排序
example-i18n-supports-the-ordering-rules-of = - 支持不同语言的排序规则
example-i18n-example-7-error-handling = 示例 7：错误处理
example-i18n-invalid-locale-tag = 非法 locale 标签：
example-i18n-unexpected-success = 意外成功
example-i18n-expected-error = ✓ 如期报错：{ $e }
example-i18n-non-finite-number-formatting = 非有限数格式化：
example-i18n-nan-error = ✓ NaN 报错：{ $e }
example-i18n-infinity-error = ✓ Infinity 报错：{ $e }
example-i18n-error-types = 错误类型：
example-i18n-i18nerror-invalidlocale-the = I18nError::InvalidLocale  - locale 标签解析失败
example-i18n-i18nerror-invalidnumber-a-non = I18nError::InvalidNumber  - 非有限数
example-i18n-i18nerror-dateerror-an-invalid = I18nError::DateError      - 非法日期分量
example-i18n-i18nerror-formaterror-icu4x = I18nError::FormatError    - ICU4X 格式化失败

# production
example-production-inklog-production-configuration = === inklog 生产配置示例 ===
example-production-all-production-examples = 生产示例全部演示完毕。
example-production-example-1-per-environment = --- 示例 1：分环境配置对比 ---
example-production-example-2-builder-pattern-for = --- 示例 2：生产环境 Builder 模式 ---
example-production-production-configuration = ✓ 生产配置已初始化
example-production-log-level-info = 日志级别：  info
example-production-console-on-colored = 控制台：   开（彩色）
example-production-file-logs-production-log-100mb = 文件：     logs/production.log（100MB，按天，保留 30 个）
example-production-compression-zstd = 压缩：     Zstd
example-production-channel-10000 = 通道容量：  10000
example-production-workers-4 = worker 数： 4
example-production-production-log-system-started = 生产日志系统已启动
example-production-this-debug-log-is-not-emitted = 本 DEBUG 日志不会输出（level=info）
example-production-level-filter-verified-debug-is = ✓ 级别过滤校验通过：DEBUG 不可见（符合预期）
example-production-3-business-logs-recorded = ✓ 已记录 3 条业务日志
example-production-alert-log-recorded = ✓ 告警日志已记录
example-production-error-log-recorded = ✓ 错误日志已记录
example-production-shut-down-gracefully = ✓ 已优雅关闭
example-production-example-3-toml-configuration = --- 示例 3：TOML 配置文件格式 ---
example-production-inklog-toml-recommended = inklog.toml（推荐的生产配置）：
example-production-how-to-load = 加载方式：
example-production-example-4-health-check-after = --- 示例 4：初始化后的健康检查 ---
example-production-health-check = 健康检查：
example-production-overall-status = overall_status:     { $overall_status }
example-production-uptime-seconds-s = uptime_seconds:     { $uptime_seconds }s
example-production-channel-usage = channel_usage:      { $value }%
example-production-encryption-valid = encryption_valid:   { $encryption_key_valid }
example-production-health-status-retrieved = ✓ 健康状态已获取
example-production-health-check-log = 健康检查日志
example-production-overall-status-after-writing-a = 写入一条日志后的 overall_status：{ $overall_status }
example-production-channel-info = 通道信息：
example-production-capacity = capacity: { $capacity }
example-production-queued = queued:   { $queued }
example-production-channel-verified = ✓ 通道校验通过
example-production-example-5-graceful-shutdown = --- 示例 5：优雅关闭流程 ---
example-production-5-logs-written-into-the-channel = ✓ 已向通道写入 5 条日志
example-production-channel-queue-before-shutdown = 关闭前通道队列：{ $queued_before } 条
example-production-shutdown-completed = ✓ shutdown() 完成
example-production-all-logs-persisted-to-sinks = - 所有日志已落盘到 sink
example-production-worker-threads-stopped = - worker 线程已停止
example-production-resources-released = - 资源已释放
example-production-example-6-error-handling = --- 示例 6：错误处理 ---
example-production-scenario-1-load-a-nonexistent = 场景 1：加载不存在的配置文件
example-production-unexpected-success = 意外成功
example-production-expected-error = ✓ 如期报错：{ $e }
example-production-scenario-2-an-invalid-log-level = 场景 2：非法的日志级别
example-production-handling-use-a-valid-level = 处理方式：使用合法级别（trace/debug/info/warn/error）
example-production-error-handling-advice = 错误处理建议：
example-production-always-handle-loggermanager = - 用 ? 或 match 处理 LoggerManager 初始化错误，不要忽略
example-production-give-a-meaningful-error-message = - 配置文件加载失败时给出明确的错误信息
example-production-consider-a-config-fallback = - 考虑配置回退链：环境变量 → 配置文件 → 默认值
example-production-production-best-practices = --- 生产环境最佳实践 ---
example-production-1-configuration-management = 1. 配置管理：
example-production-keep-configuration-in-a-toml = - 配置放 TOML 文件，纳入版本控制
example-production-inject-secrets-such-as-the = - 密钥（如数据库密码）通过环境变量注入
example-production-use-a-different-configuration = - 每个环境使用独立配置文件
example-production-2-file-rotation = 2. 文件轮转：
example-production-production-must-enable-log = - 生产必须开启日志轮转，避免磁盘写满
example-production-recommended-daily-rotation-30 = - 推荐：按天轮转 + 保留 30 天
example-production-enable-zstd-compression-to-save = - 开启 Zstd 压缩以节省存储
example-production-3-performance-tuning = 3. 性能调优：
example-production-channel-capacity-size-it-for = - channel_capacity：按峰值 QPS 设定（QPS × 10 起步）
example-production-worker-threads-match-the-cpu = - worker_threads：与 CPU 核数匹配（2-8）
example-production-watch-channel-len-capacity-and = - 监控 channel_len / capacity，超过 80% 告警
example-production-4-monitoring-integration = 4. 监控接入：
example-production-enable-the-http-server-to = - 启用 HTTP 服务暴露 /health 与 /metrics
example-production-have-prometheus-scrape-the = - 用 Prometheus 抓取 /metrics 端点
example-production-configure-alert-rules-on = - 针对日志系统健康状态配置告警规则
example-production-5-security-hardening = 5. 安全加固：
example-production-enable-tls-on-the-production = - 生产 HTTP 服务启用 TLS
example-production-configure-an-ip-whitelist-to = - 配置 IP 白名单限制来源
example-production-enable-bearer-token = - 启用 Bearer Token 认证
example-production-use-logsanitizer-to-redact = - 使用 LogSanitizer 脱敏敏感信息

# template
example-template-inklog-log-template-examples = === inklog 日志模板示例 ===
example-template-all-template-examples-completed = 模板示例全部演示完毕。
example-template-example-1-default-template = --- 示例 1：默认模板 ---
example-template-output = 输出：{ $rendered }
example-template-render-check-passed = ✓ 渲染检查通过
example-template-example-2-custom-templates = --- 示例 2：自定义模板 ---
example-template-output-2 = 输出：{ $simple_out }
example-template-exact-match = ✓ 精确匹配
example-template-output-3 = 输出：{ $standard_out }
example-template-contains-key-fields = ✓ 包含关键字段
example-template-output-4 = 输出：{ $detailed_out }
example-template-file-and-line-are-correct = ✓ 文件与行号正确
example-template-example-3-full-placeholder-list = --- 示例 3：完整占位符列表 ---
example-template-rendered-output = { $value } { $value2 } 的渲染结果
example-template-example-4-structured-field = --- 示例 4：结构化字段渲染 ---
example-template-field-rendering-check-passed = ✓ 字段渲染检查通过
example-template-fields-only = 仅字段：{ $fields_only }
example-template-fields-only-rendering-check = ✓ 仅字段渲染检查通过
example-template-example-5-multi-format = --- 示例 5：多格式对比 ---
example-template-rendered-output-2 = { $value } 的渲染结果
example-template-example-6-edge-cases = --- 示例 6：边界情况 ---
example-template-empty-message = 空消息：  '{ $empty_out }'
example-template-empty-message-handled-correctly = ✓ 空消息处理正确
example-template-special-characters = 特殊字符：'{ $special_out }'
example-template-special-characters-preserved = ✓ 特殊字符原样保留
example-template-unicode = Unicode： '{ $unicode_out }'
example-template-unicode-handled-correctly = ✓ Unicode 处理正确
example-template-long-message-chars-chars = 长消息：{ $value } 字符 → { $value2 } 字符
example-template-long-message-handled-correctly = ✓ 长消息处理正确
example-template-example-7-render-output = --- 示例 7：渲染输出校验 ---
example-template-error-payment-failed-exact-match = ✓ [ERROR] payment failed — 精确匹配
example-template-containment-verified = ✓ { $out } — 包含关系校验通过
example-template-all-5-levels-render-consistently = ✓ 5 个级别渲染一致
example-template-best-practices = --- 最佳实践 ---
example-template-1-template-selection-guide = 1. 模板选型：
example-template-2-performance-notes = 2. 性能说明：
example-template-create-one-template-instance = - 全局只创建一个模板实例并复用，避免重复解析
example-template-3-placeholder-notes = 3. 占位符说明：
example-template-4-integration-with-tracing = 4. 与 tracing 的集成：
example-template-logtemplate-is-used-mainly-to = - LogTemplate 主要用于渲染 LogRecord
example-template-the-tracing-output-format-is = - tracing 的输出格式由 tracing-subscriber 控制
example-template-they-work-together-tracing = - 二者协同：tracing 采集日志 → LogTemplate 格式化输出

# database
example-database-example-1-sqlite-in-memory = 示例 1：SQLite 内存数据库连接
example-database-database-config = 数据库配置：
example-database-driver = 驱动：{ $driver }
example-database-url = URL：{ $url }
example-database-connection-pool = 连接池：{ $pool_size }
example-database-batch-size = 批大小：{ $batch_size }
example-database-flush-interval-ms = 冲刷间隔：{ $flush_interval_ms }ms
example-database-connecting-to-sqlite-in-memory = 正在连接 SQLite 内存数据库
example-database-connection-pool-created = ✓ 连接池已创建
example-database-failed-to-create-connection-pool = ✗ 创建连接池失败：{ $e }
example-database-session-acquired = ✓ 会话已获取
example-database-failed-to-acquire-session = ✗ 获取会话失败：{ $e }
example-database-logs-table-created = ✓ logs 表已创建
example-database-failed-to-create-logs-table-may = ⚠ 创建 logs 表失败（可能已存在）：{ $e }
example-database-write-test-log = 写入测试日志
example-database-write-failed = ✗ 写入失败：{ $e }
example-database-query-verification = 查询验证
example-database-query-succeeded-table-has-data = ✓ 查询成功，表中已有数据
example-database-query-failed = ✗ 查询失败：{ $e }
example-database-in-memory-database-connection = ✓ 内存数据库连接示例完成
example-database-note-sqlite-in-memory-data-is = 注意：连接关闭后 SQLite 内存数据即丢失
example-database-each-databasesink-instance = 每个 DatabaseSink 实例各自创建连接
example-database-example-2-batch-write-demo = 示例 2：批量写入演示
example-database-batch-configuration = 批量配置：
example-database-batch-write-log = 批量写入日志
example-database-writing-log-records = 正在写入 { $value } 条日志记录……
example-database-write-round = --- 第 { $round } 轮写入 ---
example-database-write-failed-2 = ✗ [{ $value }] 写入失败：{ $e }
example-database-round-done-succeeded = 第 { $round } 轮完成：成功 { $success_count } / { $value } 条
example-database-total-log-records-in-the = ✓ 数据库中的日志记录总数
example-database-count-failed = ✗ 统计失败：{ $e }
example-database-batch-write-demo-completed = ✓ 批量写入演示完成
example-database-example-3-query-demo = 示例 3：查询演示
example-database-prepare-test-data = 准备测试数据
example-database-writing-log-records-at = 正在写入不同级别的日志记录……
example-database-insert-failed = ⚠ INSERT 失败：{ $e }
example-database-wrote-test-records = ✓ 已写入 { $value } 条测试记录
example-database-query-1-count-by-level = 查询 1：按级别统计
example-database-sql-select-level-count-as-count = SQL: SELECT level, COUNT(*) as count FROM logs GROUP BY level
example-database-expected-result = 预期结果：
example-database-level-count = │ Level │ Count │
example-database-trace-2 = │ TRACE │   2   │
example-database-debug-2 = │ DEBUG │   2   │
example-database-info-3 = │ INFO  │   3   │
example-database-warn-2 = │ WARN  │   2   │
example-database-error-2 = │ ERROR │   2   │
example-database-total-11-records = 合计：11 条记录
example-database-query-2-count-by-target-module = 查询 2：按 target 模块统计
example-database-sql-select-target-count-as = SQL: SELECT target, COUNT(*) as count FROM logs GROUP BY target
example-database-target-count = │ Target                 │ Count │
example-database-database-example-api-3 = │ database_example::api  │   3   │
example-database-database-example-query-4 = │ database_example::query│   4   │
example-database-database-example-auth-1 = │ database_example::auth │   1   │
example-database-database-example-cache-1 = │ database_example::cache│   1   │
example-database-database-example-database-2 = │ database_example::database│  2   │
example-database-query-3-filter-by-level-error = 查询 3：按级别过滤（ERROR/WARN）
example-database-sql-select-from-logs-where = SQL: SELECT * FROM logs WHERE level IN ('ERROR', 'WARN')
example-database-warn-database-example-auth-auth = [WARN]  database_example::auth     - 认证令牌即将过期
example-database-warn-database-example-cache = [WARN]  database_example::cache    - 缓存命中率偏低：45%
example-database-error-database-example-database = [ERROR] database_example::database  - 数据库连接超时
example-database-error-database-example-database-2 = [ERROR] database_example::database  - 查询执行失败
example-database-query-4-search-log-content = 查询 4：搜索日志内容
example-database-sql-select-from-logs-where-2 = SQL: SELECT * FROM logs WHERE message LIKE '%timeout%' OR message LIKE '%failed%'
example-database-query-5-fetch-recent-logs = 查询 5：获取最近日志
example-database-sql-select-from-logs-order-by = SQL: SELECT * FROM logs ORDER BY id DESC LIMIT 5
example-database-expected-result-ids-descending = 预期结果（ID 降序）：
example-database-11-error-database-example = 11. [ERROR] database_example::database - 查询执行失败
example-database-10-error-database-example = 10. [ERROR] database_example::database - 数据库连接超时
example-database-9-warn-database-example-cache = 9. [WARN]  database_example::cache   - 缓存命中率偏低：45%
example-database-8-warn-database-example-auth = 8. [WARN]  database_example::auth    - 认证令牌即将过期
example-database-7-info-database-example-api = 7. [INFO]  database_example::api    - 响应发送成功
example-database-query-6-time-range-query = 查询 6：时间范围查询
example-database-sql-select-from-logs-where-3 = SQL: SELECT * FROM logs WHERE timestamp > '2024-01-01'
example-database-expected-result-all-logs-all = 预期结果：全部日志（均为最近写入）
example-database-query-7-combined-multi = 查询 7：组合多条件查询
example-database-sql-select-level-target-message = SQL: SELECT level, target, message FROM logs
example-database-where-level-in-warn-error-and = WHERE level IN ('WARN', 'ERROR') AND target LIKE '%database%'
example-database-query-demo-completed = ✓ 查询演示完成
example-database-tip-use-sql-clauses-such-as = 提示：可使用 WHERE、GROUP BY、ORDER BY 等 SQL 子句
example-database-enabling-complex-log-queries = 以支持复杂的日志查询与分析
example-database-inklog-database-sink-example = === inklog Database Sink 示例 ===
example-database-this-example-uses-an-sqlite-in = 本示例使用 SQLite 内存数据库（:memory:）
example-database-no-file-cleanup-needed-data = 无需清理文件；数据仅存在于内存中
example-database-all-examples-completed = 全部示例演示完毕！
example-database-key-takeaways = 要点回顾：
example-database-1-databasesink-supports-sqlite = 1. DatabaseSink 支持 SQLite、PostgreSQL 与 MySQL
example-database-2-in-memory-databases-suit = 2. 内存数据库适合测试与演示；关闭后数据即丢失
example-database-4-supports-queries-by-level = 4. 支持按级别、时间、target 等维度查询
example-database-5-automatically-falls-back-to = 5. 失败时自动回退到文件输出
example-database-sql-query-tips = SQL 查询提示：
example-database-by-level-where-level-error = - 按级别：WHERE level = 'ERROR'
example-database-by-module-where-target-like-api = - 按模块：WHERE target LIKE '%api%'
example-database-by-time-where-timestamp-2024-01 = - 按时间：WHERE timestamp > '2024-01-01'
example-database-aggregate-stats-group-by-level = - 聚合统计：GROUP BY level
example-database-error-enable-a-database-feature = 错误：需启用数据库 feature（sqlite/postgres/mysql）才能运行本示例
example-database-run-with-one-of-the-following = 使用以下任一命令运行：
example-database-cargo-run-bin-database-features = cargo run --bin database --features sqlite
example-database-or-enable-the-sqlite-feature-in = 或在 examples/Cargo.toml 中启用 sqlite feature：

# database-pg-mysql
example-database-pg-mysql-inklog-postgresql-mysql = inklog PostgreSQL/MySQL 数据库驱动示例
example-database-pg-mysql-all-postgresql-mysql-driver = PostgreSQL/MySQL 驱动示例全部演示完毕。
example-database-pg-mysql-example-1-the-three = 示例 1：三种 DatabaseDriver 变体
example-database-pg-mysql-enum-variants = 枚举变体：
example-database-pg-mysql-enterprise-first-choice-the = { $postgres } - 企业首选，功能最完整
example-database-pg-mysql-a-balance-of-performance-and = { $mysql } - 性能与简洁性的平衡
example-database-pg-mysql-embedded-zero-configuration = { $sqlite } - 嵌入式，零配置
example-database-pg-mysql-default-driver = 默认驱动：
example-database-pg-mysql-databasedriver-default = DatabaseDriver::default() = { $value }
example-database-pg-mysql-example-2-fromstr-display-traits = 示例 2：FromStr / Display trait
example-database-pg-mysql-fromstr-parsing-case-insensitive = FromStr 解析（大小写不敏感）：
example-database-pg-mysql-parse-ok = { $input }.parse() → Ok({ $driver })
example-database-pg-mysql-parse-err = { $input }.parse() → Err
example-database-pg-mysql-invalid-input = 非法输入：
example-database-pg-mysql-parse-ok-2 = { $input }.parse() → Ok({ $d })
example-database-pg-mysql-parse-err-2 = { $input }.parse() → Err("{ $e }")
example-database-pg-mysql-display-output = Display 输出：
example-database-pg-mysql-example-3-postgresql-connection = 示例 3：PostgreSQL 连接配置
example-database-pg-mysql-postgresql-databasesinkconfig = PostgreSQL DatabaseSinkConfig：
example-database-pg-mysql-driver = driver            = { $driver }
example-database-pg-mysql-url = url               = "{ $url }"
example-database-pg-mysql-table-name = table_name        = "{ $table_name }"
example-database-pg-mysql-batch-size = batch_size        = { $batch_size }
example-database-pg-mysql-flush-interval-ms = flush_interval_ms = { $flush_interval_ms }
example-database-pg-mysql-pool-size = pool_size         = { $pool_size }
example-database-pg-mysql-connection-url-format = 连接 URL 格式：
example-database-pg-mysql-postgres-user-password-host = postgres://USER:PASSWORD@HOST:PORT/DATABASE
example-database-pg-mysql-example = 示例：
example-database-pg-mysql-postgres-inklog-secret-db = postgres://inklog:secret@db.example.com:5432/logs
example-database-pg-mysql-postgres-postgres-localhost = postgres://postgres@localhost/inklog
example-database-pg-mysql-postgres-user-pass-10-0-0-1 = postgres://user:pass@10.0.0.1:5432/prod_logs?sslmode=require
example-database-pg-mysql-corresponding-toml-config = 对应的 TOML 配置：
example-database-pg-mysql-postgresql-strengths = PostgreSQL 优势：
example-database-pg-mysql-richest-features-partitioned = - 功能最丰富：分区表（PARTITION BY RANGE）
example-database-pg-mysql-great-performance-high = - 性能出色：高并发写入、MVCC
example-database-pg-mysql-extensibility-jsonb-full-text = - 扩展性强：JSONB、全文检索、地理数据
example-database-pg-mysql-production-pick-the-first = - 生产首选：企业级日志存储的第一选择
example-database-pg-mysql-table-partitioning-high-log = 表分区（大日志量）：
example-database-pg-mysql-example-4-mysql-connection = 示例 4：MySQL 连接配置
example-database-pg-mysql-mysql-databasesinkconfig = MySQL DatabaseSinkConfig：
example-database-pg-mysql-mysql-user-password-host-port = mysql://USER:PASSWORD@HOST:PORT/DATABASE
example-database-pg-mysql-mysql-root-localhost-inklog = mysql://root@localhost/inklog
example-database-pg-mysql-mysql-inklog-secret-db-example = mysql://inklog:secret@db.example.com:3306/logs
example-database-pg-mysql-mysql-user-pass-10-0-0-1-3306 = mysql://user:pass@10.0.0.1:3306/prod_logs?ssl-ca=/path/to/ca.pem
example-database-pg-mysql-mysql-strengths = MySQL 优势：
example-database-pg-mysql-easy-deployment-works-with = - 部署简单：默认配置即可用
example-database-pg-mysql-mature-ecosystem-native-fit = - 生态成熟：与 LAMP/LEMP 技术栈天然契合
example-database-pg-mysql-balanced-performance-even-read = - 性能均衡：读写吞吐稳定
example-database-pg-mysql-partitioning-partition-by-range = - 分区：PARTITION BY RANGE
example-database-pg-mysql-example-5-three-driver = 示例 5：三种驱动对比
example-database-pg-mysql-feature-comparison = 功能对比：
example-database-pg-mysql-selection-advice = 选型建议：
example-database-pg-mysql-dev-test-sqlite-zero-config = - 开发/测试：SQLite（零配置，启动快）
example-database-pg-mysql-small-medium-production-mysql = - 中小生产：MySQL（部署简单，性能均衡）
example-database-pg-mysql-large-enterprise-postgresql = - 大型/企业：PostgreSQL（功能最全，性能最强）
example-database-pg-mysql-example-6-env-var-overrides = 示例 6：环境变量覆盖
example-database-pg-mysql-database-driver-env-vars = 数据库驱动环境变量：
example-database-pg-mysql-loading-via-load-with-env = 通过 load_with_env_overrides() 加载：
example-database-pg-mysql-config-database-sink-driver = config.database_sink.driver,
example-database-pg-mysql-docker-deployment-example = Docker 部署示例：
example-database-pg-mysql-my-app = my-app
example-database-pg-mysql-kubernetes-configmap-example = Kubernetes ConfigMap 示例：
example-database-pg-mysql-env = env:
example-database-pg-mysql-name-inklog-database-sink-driver = - name: INKLOG_DATABASE_SINK_DRIVER
example-database-pg-mysql-valuefrom = valueFrom:
example-database-pg-mysql-configmapkeyref = configMapKeyRef:
example-database-pg-mysql-name-inklog-config = name: inklog-config
example-database-pg-mysql-key-db-driver = key: db-driver

# di-example
example-di-example-di-dependency-injection-pattern = === DI（依赖注入）模式示例 ===
example-di-example-1-using-inklogcontainer-real = 1. 使用 InklogContainer（真实适配器）：
example-di-example-creating-container = - 正在创建容器……
example-di-example-cache-instance-created-arc-dyn = - Cache 实例：已创建（Arc<dyn Cache>）
example-di-example-config-instance-created-arc-dyn = - Config 实例：已创建（Arc<dyn Config>）
example-di-example-2-injecting-dependencies-via = 2. 通过 LoggerBuilder 注入依赖（真实适配器）：
example-di-example-logger-created = - Logger 已创建！
example-di-example-3-using-with-dependencies-real = 3. 使用 with_dependencies（真实适配器）：
example-di-example-4-container-shared-dependency = 4. 容器内共享的依赖实例（真实适配器）：
example-di-example-cache1-set-shared-key-shared = - cache1 写入 shared_key = shared_value
example-di-example-cache2-read-shared-key = - cache2 读取 shared_key = { $value }
example-di-example-both-instances-share-the-same = - 两个实例共享同一份底层数据：{ $value }
example-di-example-5-creating-multiple-loggers = 5. 从容器创建多个 logger：
example-di-example-logger-1-created = - Logger 1 已创建
example-di-example-logger-2-created = - Logger 2 已创建
example-di-example-both-loggers-share-the-same = - 两个 logger 共享同一批 cache 与 config 实例
example-di-example-mock-type-examples-for-testing = === Mock 类型示例（用于测试）===
example-di-example-6-testing-with-mockcache = 6. 用 MockCache 测试：
example-di-example-mockcache-set-and-read = - MockCache 写入与读取：{ $value }
example-di-example-test-key-exists = - test_key 是否存在：{ $exists }
example-di-example-deleted-test-key = - 已删除 test_key：{ $deleted }
example-di-example-7-testing-with-mockconfig = 7. 用 MockConfig 测试：
example-di-example-read-global-level = - 读取 global.level：{ $level }
example-di-example-read-http-server-port = - 读取 http_server.port：{ $port }
example-di-example-read-file-sink-enabled = - 读取 file_sink.enabled：{ $enabled }
example-di-example-global-level-after-modification = - 修改后的 global.level：{ $value }
example-di-example-8-testing-with = 8. 用 MockDatabaseAdapter 测试：
example-di-example-initial-health-status = - 初始健康状态：{ $healthy }
example-di-example-health-status-after-fault = - 注入故障后的健康状态：{ $value }
example-di-example-health-status-after-recovery = - 恢复后的健康状态：{ $value }
example-di-example-stored-record-count = - 已存储记录数：{ $value }
example-di-example-9-creating-a-logger-with-mock = 9. 用 mock 类型创建 logger：
example-di-example-logger-created-with-mock = - 使用 mock 依赖的 Logger 已创建！
example-di-example-10-using-mockcache-with-delay = 10. 使用带延迟的 MockCache（模拟网络延迟）：
example-di-example-set-operation-took = - set 操作耗时：{ $elapsed }
example-di-example-verified-delay-took-effect-ms = - 已验证延迟生效：{ $value }ms >= 10ms
example-di-example-examples-completed = === 示例演示完毕 ===

# channel-strategy
example-channel-strategy-inklog-adaptive-channel = inklog 自适应通道策略示例
example-channel-strategy-all-channelstrategy-examples = ChannelStrategy 示例全部演示完毕。
example-channel-strategy-example-1-channelstrategy = 示例 1：ChannelStrategy 变体
example-channel-strategy-enum-variants = 枚举变体：
example-channel-strategy-static-capacity-predictable = { $fixed } - 固定容量，内存占用可预测
example-channel-strategy-dynamic-sizing-better-burst = { $adaptive } - 动态调整容量，突发流量处理更好
example-channel-strategy-default-strategy = 默认策略：
example-channel-strategy-channelstrategy-default = ChannelStrategy::default() = { $value }
example-channel-strategy-behavior-comparison = 行为对比：
example-channel-strategy-example-2-fromstr-display-traits = 示例 2：FromStr / Display trait
example-channel-strategy-fromstr-parsing = FromStr 解析：
example-channel-strategy-parse-ok = { $input }.parse() → Ok({ $strategy })
example-channel-strategy-parse-err = { $input }.parse() → Err("{ $e }")
example-channel-strategy-invalid-input = 非法输入：
example-channel-strategy-parse-ok-2 = { $input }.parse() → Ok({ $s })
example-channel-strategy-display-output = Display 输出：
example-channel-strategy-example-3-adaptive-threshold = 示例 3：自适应阈值参数
example-channel-strategy-default-performanceconfig = 默认 PerformanceConfig：
example-channel-strategy-channel-capacity = channel_capacity        = { $channel_capacity }
example-channel-strategy-worker-threads = worker_threads          = { $worker_threads }
example-channel-strategy-channel-strategy = channel_strategy        = { $channel_strategy }
example-channel-strategy-expand-threshold-percent = expand_threshold_percent= { $expand_threshold_percent }
example-channel-strategy-shrink-threshold-percent = shrink_threshold_percent= { $shrink_threshold_percent }
example-channel-strategy-shrink-wait-seconds = shrink_wait_seconds     = { $shrink_wait_seconds }
example-channel-strategy-min-capacity = min_capacity            = { $min_capacity }
example-channel-strategy-max-capacity = max_capacity            = { $max_capacity }
example-channel-strategy-threshold-parameter-semantics = 阈值参数语义（仅 Adaptive 策略生效）：
example-channel-strategy-expand-threshold-percent-expand = expand_threshold_percent：容量使用率达到该百分比时扩容
example-channel-strategy-default-80-expand-at-8000-10000 = 默认 80% → 在 8000/10000 时扩容
example-channel-strategy-shrink-threshold-percent-2 = shrink_threshold_percent：使用率降到该百分比时考虑缩容
example-channel-strategy-default-20-shrink-at-2000-10000 = 默认 20% → 在 2000/10000 时缩容
example-channel-strategy-default-30-seconds = 默认 30 秒
example-channel-strategy-min-capacity-max-capacity-lower = min_capacity / max_capacity：缩容/扩容的下界与上界
example-channel-strategy-custom-adaptive-config = 自定义自适应配置（激进扩容，保守缩容）：
example-channel-strategy-expand-threshold-percent-2 = expand_threshold_percent= { $expand_threshold_percent }（更早扩容）
example-channel-strategy-shrink-threshold-percent-3 = shrink_threshold_percent= { $shrink_threshold_percent }（更晚缩容）
example-channel-strategy-shrink-wait-seconds-longer = shrink_wait_seconds     = { $shrink_wait_seconds }（更长的去抖时间）
example-channel-strategy-example-4-performance-config = 示例 4：性能配置建议
example-channel-strategy-channel-capacity-workers = channel_capacity={ $channel_capacity }, workers={ $worker_threads }, strategy={ $channel_strategy }
example-channel-strategy-expand-shrink-wait-s-range = expand@{ $expand_threshold_percent }%, shrink@{ $shrink_threshold_percent }%, wait={ $shrink_wait_seconds }s, range=[{ $min_capacity },{ $max_capacity }]
example-channel-strategy-example-5-formatted-config = 示例 5：配置格式化展示
example-channel-strategy-serialized-to-toml = 序列化为 TOML：
example-channel-strategy-env-var-overrides = 环境变量覆盖：

# circuit-breaker
example-circuit-breaker-inklog-circuit-breaker-example = === inklog 熔断器示例 ===
example-circuit-breaker-all-circuit-breaker-examples = ✓ 熔断器示例全部演示完毕
example-circuit-breaker-1-breaker-creation-and-initial = 1. 熔断器创建与初始状态
example-circuit-breaker-1-1-circuitbreaker-new-failure = 1.1 CircuitBreaker::new(failure_threshold, timeout, success_threshold)
example-circuit-breaker-failure-threshold = failure_threshold = { $value }
example-circuit-breaker-success-threshold = success_threshold = { $value }
example-circuit-breaker-timeout = timeout           = { $value }
example-circuit-breaker-1-2-initial-state-is-closed = 1.2 初始状态为 Closed；允许执行
example-circuit-breaker-state = state()       = { $value }
example-circuit-breaker-failure-count = failure_count = { $value }
example-circuit-breaker-can-execute = can_execute() = { $value }
example-circuit-breaker-2-closed-open-failure-count = 2. Closed → Open：失败计数达到阈值触发熔断
example-circuit-breaker-2-1-config-failure-threshold-3 = 2.1 配置：failure_threshold = 3
example-circuit-breaker-initial-state = 初始状态 = { $value }
example-circuit-breaker-2-2-failure-1-below-threshold = 2.2 第 1 次失败（未达阈值）
example-circuit-breaker-state-2 = state         = { $value }
example-circuit-breaker-2-3-failure-2-below-threshold = 2.3 第 2 次失败（未达阈值）
example-circuit-breaker-2-4-failure-3-threshold-reached = 2.4 第 3 次失败（达到阈值 → Open）
example-circuit-breaker-2-5-fast-fail-in-open-state-no = 2.5 Open 状态快速失败（不执行真实操作）
example-circuit-breaker-in-the-open-state-can-execute = Open 状态下 can_execute() 返回 false，请求被快速拒绝
example-circuit-breaker-avoids-repeatedly-sending = 避免持续向已失败的 sink 发送请求
example-circuit-breaker-3-open-halfopen-automatically = 3. Open → HalfOpen：超时后自动进入半开
example-circuit-breaker-3-1-config-timeout-100ms = 3.1 配置：timeout = 100ms
example-circuit-breaker-state-after-tripping = 触发熔断后 state = { $value }
example-circuit-breaker-can-execute-timeout-not-elapsed = can_execute()    = { $value }（超时未到）
example-circuit-breaker-3-2-waiting-150ms-for-the = 3.2 等待 150ms 让超时到期
example-circuit-breaker-wait-elapsed-calling-can = 等待结束；再次调用 can_execute() 触发状态迁移
example-circuit-breaker-3-3-can-execute-triggers-open = 3.3 can_execute() 触发 Open → HalfOpen
example-circuit-breaker-can-execute-2 = can_execute() = { $allowed }
example-circuit-breaker-3-4-halfopen-allows-a-limited = 3.4 HalfOpen 允许有限数量的请求
example-circuit-breaker-4-halfopen-closed-recovers = 4. HalfOpen → Closed：累计成功次数后恢复
example-circuit-breaker-4-1-config-success-threshold-3 = 4.1 配置：success_threshold = 3
example-circuit-breaker-now-in-halfopen-success = 当前为 HalfOpen，success_threshold = 3
example-circuit-breaker-4-2-success-1-below-threshold = 4.2 第 1 次成功（未达阈值，仍为 HalfOpen）
example-circuit-breaker-state-3 = state = { $value }
example-circuit-breaker-4-3-success-2-below-threshold = 4.3 第 2 次成功（未达阈值，仍为 HalfOpen）
example-circuit-breaker-4-4-success-3-threshold-reached = 4.4 第 3 次成功（达到阈值 → Closed）
example-circuit-breaker-5-halfopen-open-a-failure-in = 5. HalfOpen → Open：半开状态下失败立即重新熔断
example-circuit-breaker-5-1-entering-halfopen = 5.1 进入 HalfOpen
example-circuit-breaker-5-2-one-success-recorded-in = 5.2 半开状态下记录 1 次成功（仍为 HalfOpen）
example-circuit-breaker-5-3-failure-in-half-open = 5.3 半开状态下失败 → 立即重新熔断
example-circuit-breaker-6-config-driven-construction = 6. 用 with_config + CircuitBreakerConfig 按配置构建
example-circuit-breaker-6-1-customizing = 6.1 自定义 CircuitBreakerConfig
example-circuit-breaker-failure-threshold-2 = failure_threshold = { $failure_threshold }
example-circuit-breaker-success-threshold-2 = success_threshold = { $success_threshold }
example-circuit-breaker-timeout-2 = timeout           = { $timeout }
example-circuit-breaker-6-2-circuitbreaker-with-config = 6.2 CircuitBreaker::with_config(config)
example-circuit-breaker-config-failure-threshold = config().failure_threshold = { $value }
example-circuit-breaker-config-success-threshold = config().success_threshold = { $value }
example-circuit-breaker-config-timeout = config().timeout           = { $value }
example-circuit-breaker-6-3-circuitbreakerconfig-default = 6.3 CircuitBreakerConfig::default()
example-circuit-breaker-default-failure-threshold = 默认 failure_threshold = { $failure_threshold }
example-circuit-breaker-default-success-threshold = 默认 success_threshold = { $success_threshold }
example-circuit-breaker-default-timeout = 默认 timeout           = { $timeout }
example-circuit-breaker-6-4-success-in-closed-state = 6.4 Closed 状态下成功会重置失败计数
example-circuit-breaker-failure-count-after-2-failures = 2 次失败后 failure_count = { $value }
example-circuit-breaker-failure-count-after-record = record_success() 后 failure_count = { $value }
example-circuit-breaker-7-manual-reset-with-reset = 7. 用 reset() 手动复位
example-circuit-breaker-7-1-tripping-the-breaker-into = 7.1 触发熔断进入 Open
example-circuit-breaker-7-2-reset-returns-to-the = 7.2 reset() 回到初始状态
example-circuit-breaker-8-realistic-simulation-guarding = 8. 真实场景模拟：用 can_execute() 保护操作
example-circuit-breaker-8-1-simulated-sink-writes = 8.1 模拟 sink 写入（failure threshold = 3，timeout = 200ms）
example-circuit-breaker-8-2-phase-1-3-consecutive = 8.2 阶段 1：连续 3 次失败 → 熔断触发
example-circuit-breaker-write-failed = 第 { $i } 次写入失败：{ $e }
example-circuit-breaker-request-rejected-by-the-breaker = 第 { $i } 个请求被熔断器拒绝
example-circuit-breaker-state-failure-count = state = { $value }, failure_count = { $value2 }
example-circuit-breaker-8-3-phase-2-fast-fail-in-open = 8.3 阶段 2：Open 状态快速失败（不调用 write_op）
example-circuit-breaker-request-allowed-to-execute = 第 { $i } 个请求获准执行
example-circuit-breaker-request-fast-rejected-by-the = 第 { $i } 个请求被熔断器快速拒绝（保护已失败的 sink）
example-circuit-breaker-8-4-phase-3-waiting-250ms-past = 8.4 阶段 3：等待 250ms 超过超时 → HalfOpen
example-circuit-breaker-probe-request-allowed-after = 超时后探针请求获准执行，state = { $value }
example-circuit-breaker-8-5-phase-4-2-accumulated = 8.5 阶段 4：HalfOpen 累计 2 次成功 → 回到 Closed
example-circuit-breaker-probe-succeeded-state = 第 { $i } 次探针成功，state = { $value }
example-circuit-breaker-probe-failed-state = 第 { $i } 次探针失败，state = { $value }
example-circuit-breaker-8-6-full-state-machine = 8.6 状态机完整迁移汇总
example-circuit-breaker-closed-3-failures-open-timeout = Closed ──3 次失败──► Open ──超时──► HalfOpen ──2 次成功──► Closed
example-circuit-breaker-sink-recovered-breaker-back-to = ✓ Sink 已恢复；熔断器回到正常模式

# fallback
example-fallback-degrading = [{ $value }] 降级：{ $active_sink } -> { $new_sink }
example-fallback-example-1-multi-sink-setup = 示例 1：多 sink 组合（console + file）
example-fallback-log-file-path = 日志文件路径：{ $log_path }
example-fallback-console-sink-configured = Console sink：已配置
example-fallback-file-sink-configured = File sink：已配置
example-fallback-writing-logs-to-multiple-sinks = 向多个 sink 写入日志
example-fallback-console-file = { $level_marker } -> Console + File：{ $message }
example-fallback-verifying-the-file-sink = 校验文件 sink
example-fallback-file-content-bytes =
    文件内容（{ $value } 字节）：
    { $content }
example-fallback-multi-sink-setup-completed = ✓ 多 sink 组合演示完成
example-fallback-console-sink-real-time-output = - Console sink：实时输出到控制台
example-fallback-file-sink-persist-to-a-file = - File sink：持久化到文件
example-fallback-both-work-simultaneously = - 两者同时工作，互不干扰
example-fallback-example-2-failure-simulation = 示例 2：故障模拟与回退切换
example-fallback-fallback-log-file = 回退日志文件：{ $backup_path }
example-fallback-initial-state-primary-sink = 初始状态：主 sink = console
example-fallback-fallback-file-sink-ready = 回退文件 sink：就绪
example-fallback-writing-logs-triggering-failure = 写入日志（触发故障）
example-fallback-primary-sink-write-succeeded = [{ $value }] [主 sink] 第 { $success_count } 次写入成功
example-fallback-primary-sink-failure = *** 主 sink 故障：{ $e } ***
example-fallback-fallback-sink-took-over-writing = [{ $value }] [回退 sink] 接管写入
example-fallback-degradation-event-statistics = 降级事件统计
example-fallback-degradation-count = 降级次数：{ $fallback_count }
example-fallback-last-degradation-time = 最近降级时间：{ $value }
example-fallback-active-sinks = 活动 sink：{ $active_sink }
example-fallback-verifying-the-fallback-file = 校验回退文件
example-fallback-fallback-file-content-bytes = 回退文件内容（{ $value } 字节）：
example-fallback-failure-simulation-and-fallback = ✓ 故障模拟与回退切换演示完成
example-fallback-primary-sink-fails-after-write = - 主 sink 在第 { $value } 次写入后故障
example-fallback-fallback-sink-took-over-the = - 回退 sink 接管后续 { $value } 条日志
example-fallback-example-3-full-fallback = 示例 3：完整回退策略演示
example-fallback-primary-log-file = 主日志文件：{ $primary_path }
example-fallback-fallback-log-file-2 = 回退日志文件：{ $fallback_path }
example-fallback-sink-health-status-types = Sink 健康状态类型
example-fallback-sinkstatus-enum = SinkStatus 枚举：
example-fallback-healthy-sink-fully-healthy = - Healthy：sink 完全健康
example-fallback-degraded-sink-degraded-may-need = - Degraded：sink 已降级，可能需要回退
example-fallback-unhealthy-sink-unavailable = - Unhealthy：sink 不可用，必须回退
example-fallback-sinkhealth-struct = SinkHealth 结构体：
example-fallback-is-healthy-bool-whether-healthy = - is_healthy: bool - 是否健康
example-fallback-status-sinkstatus-detailed = - status: SinkStatus - 详细状态
example-fallback-last-error-option-string-error = - last_error: Option<String> - 错误信息
example-fallback-simulated-health-check-result = 模拟健康检查结果
example-fallback-advice-degrade-to-the-fallback = -> 建议：降级到回退 sink
example-fallback-degradation-flow-simulation = 降级流程模拟
example-fallback-primary-sink-failed = [{ $value }] *** 主 sink 故障 ***
example-fallback-degrading-file-primary-file = 降级：File(主) -> File(回退)
example-fallback-file-content-after-degradation = 降级后的文件内容
example-fallback-primary-file-first-logs = --- 主文件（前 { $value } 条日志）---
example-fallback-fallback-file-logs-after = --- 回退文件（接管后的日志）---
example-fallback-fallback-strategy-summary = 回退策略汇总
example-fallback-three-tier-fallback = 三级回退：
example-fallback-1-database-file-when-the = 1. Database -> File（数据库不可用时）
example-fallback-trigger-connection-timeout-sql = - 触发条件：连接超时、SQL 错误
example-fallback-degradation-action-write-to-a = - 降级动作：写入本地文件
example-fallback-2-file-console-when-the-file-is = 2. File -> Console（文件不可用时）
example-fallback-trigger-disk-full-permission = - 触发条件：磁盘写满、权限错误
example-fallback-degradation-action-output-to = - 降级动作：输出到控制台
example-fallback-3-console-final-fallback = 3. Console（最终兜底）
example-fallback-trigger-all-sinks-unavailable = - 触发条件：全部 sink 不可用
example-fallback-degradation-action-log-to = - 降级动作：写 syslog + 告警
example-fallback-degradation-event-log = 降级事件日志：
example-fallback-all-degradation-events-are = - 所有降级事件都会被记录
example-fallback-includes-degradation-reason = - 包含降级原因、时间与恢复状态
example-fallback-degradation-state-queryable-via = - 降级状态可通过 HTTP 端点查询
example-fallback-full-fallback-strategy-demo = ✓ 完整回退策略演示完成
example-fallback-cleaned-up-temp-files = 已清理 { $deleted_count } 个临时文件
example-fallback-inklog-sink-fallback-mechanism = === inklog sink 回退机制示例 ===
example-fallback-all-examples-completed = === 全部示例演示完毕 ===

# log-adapter
example-log-adapter-inklog-log-crate-adapter-example = === inklog log crate 适配器示例 ===
example-log-adapter-all-log-adapter-examples = ✓ 日志适配器示例全部演示完毕
example-log-adapter-1-install-loglogger-and-use-log = 1. 安装 LogLogger 并使用 log 宏
example-log-adapter-loglogger-installed-max-level = ✓ LogLogger 已安装（max_level = Info）
example-log-adapter-1-1-using-log-info-warn-error = 1.1 使用 log::info! / warn! / error! 宏
example-log-adapter-1-2-records-received-from-the = 1.2 从 console 通道收到的记录
example-log-adapter-1-3-records-received-from-the = 1.3 从 async 通道收到的记录
example-log-adapter-verified-each-channel-received = ✓ 已验证：每个通道各收到 3 条记录（info/warn/error）
example-log-adapter-2-levelfilter-filtering-behavior = 2. LevelFilter 过滤行为
example-log-adapter-loglogger-new-adapter = LogLogger::new(adapter, LevelFilter::Info) 将 max_level 设为 Info
example-log-adapter-log-info-warn-error-pass-through = → log::info! / warn! / error! 通过
example-log-adapter-log-debug-trace-filtered-out = → log::debug! / trace! 被过滤
example-log-adapter-logadapter-enabled-impl = LogAdapter::enabled() 实现：metadata.level() <= log::max_level()
example-log-adapter-level-enum-order-error-warn = Level 枚举顺序：Error < Warn < Info < Debug < Trace
example-log-adapter-level-target-message = [{ $name }] level={ $level }, target={ $target }, message={ $message }
example-log-adapter-channel-received-no-records = （{ $name } 通道未收到记录）
example-log-adapter-channel-received-no-records-the = （{ $name } 通道未收到记录；该适配器上未安装全局 logger）
example-log-adapter-received-records-in-total = 共收到 { $count } 条 { $name } 记录

# log-level
example-log-level-inklog-loglevel-type-example = inklog LogLevel 类型示例
example-log-level-all-loglevel-examples-completed = LogLevel 示例全部演示完毕。
example-log-level-example-1-the-loglevel-from-str = 示例 1：LogLevel::from_str 静态方法
example-log-level-case-insensitive-parsing = 大小写不敏感解析：
example-log-level-none = { $input } → None
example-log-level-invalid-input-returns-none = 非法输入返回 None：
example-log-level-example-2-the-fromstr-trait = 示例 2：FromStr trait（返回 Result）
example-log-level-using-parse-loglevel = 使用 .parse::<LogLevel>()：
example-log-level-parse-ok = { $input }.parse() → Ok({ $level })
example-log-level-error-handling = 错误处理：
example-log-level-parse-err = { $input }.parse() → Err("{ $e }")
example-log-level-example-3-ordering-ord = 示例 3：排序（Ord / PartialOrd）
example-log-level-levels-low-to-high-higher-value = 级别由低到高（数值越大越严重）：
example-log-level-comparison-operators = 比较运算符：
example-log-level-error-debug = Error > Debug  → { $value }
example-log-level-info-warn = Info  <= Warn  → { $value }
example-log-level-trace-fatal = Trace < Fatal  → { $value }
example-log-level-error-error = Error == Error → { $value }
example-log-level-practical-use-level-filtering = 实际用法：级别过滤（仅记录 >= WARN）
example-log-level-example-4-the-display-trait = 示例 4：Display trait
example-log-level-consistency-check-against-as-str = 与 as_str() 的一致性检查：
example-log-level-display-as-str-match = { $level }：Display="{ $display }"，as_str="{ $as_str }"，match={ $consistent }
example-log-level-example-5-helper-methods-as-str = 示例 5：辅助方法 as_str / as_short_str
example-log-level-short-format-use-case-compact = 短格式用法：紧凑的控制台输出
example-log-level-inf-2026-01-01-service-started = [INF] 2026-01-01 服务已启动
example-log-level-wrn-2026-01-01-missing-config = [WRN] 2026-01-01 缺少配置项，使用默认值
example-log-level-err-2026-01-01-database = [ERR] 2026-01-01 数据库连接失败
example-log-level-example-6-error-handling = 示例 6：错误处理（LogLevelParseError）
example-log-level-input = 输入：{ $value }
example-log-level-error-type = 错误类型：{ $e }
example-log-level-error-message = 错误消息：{ $msg }
example-log-level-input-included = 包含输入：{ $contains_input }

# metrics
example-metrics-inklog-health-monitoring-and = === inklog 健康监控与指标示例 ===
example-metrics-all-health-monitoring-and = ✓ 健康监控与指标示例全部演示完毕
example-metrics-1-metrics-creation-and-basic = 1. Metrics 创建与基础计数器
example-metrics-1-1-creating-an-instance-with = 1.1 用 Metrics::new() 创建实例
example-metrics-initial-logs-written = 初始 logs_written = { $value }
example-metrics-initial-logs-dropped = 初始 logs_dropped = { $value }
example-metrics-initial-sink-errors = 初始 sink_errors  = { $value }
example-metrics-initial-channel-blocked = 初始 channel_blocked = { $value }
example-metrics-1-2-inc-logs-written = 1.2 inc_logs_written() 累计日志写入
example-metrics-write-cumulative = 第 { $value } 次写入，累计 = { $value2 }
example-metrics-1-3-inc-logs-dropped-inc-sink = 1.3 inc_logs_dropped() / inc_sink_error() / inc_channel_blocked()
example-metrics-logs-dropped = logs_dropped   = { $value }
example-metrics-sink-errors = sink_errors    = { $value }
example-metrics-channel-blocked = channel_blocked = { $value }
example-metrics-1-4-db-batch-metrics-set-db = 1.4 DB 批量指标 set_db_batch_size / add_db_batch_records_total
example-metrics-db-batch-size = db_batch_size           = { $value }
example-metrics-db-batch-records-total = db_batch_records_total  = { $value }
example-metrics-1-5-lock-contention-counter = 1.5 锁竞争计数（active_workers 为内部字段；外部只读）
example-metrics-active-workers-default-0-set-by = active_workers   = { $value }（默认 0，由内部 worker 设置）
example-metrics-lock-contention = lock_contention  = { $value }
example-metrics-1-6-uptime-runtime-duration = 1.6 uptime() 运行时长
example-metrics-uptime = uptime = { $uptime }
example-metrics-2-latency-histogram-and = 2. 延迟直方图与分位数
example-metrics-2-1-record-latency-records = 2.1 record_latency() 记录若干延迟
example-metrics-recorded-latency-samples = 已记录 { $value } 个延迟样本
example-metrics-2-2-latency-aggregates-avg-p50 = 2.2 延迟聚合（avg / p50 / p95 / p99）
example-metrics-avg-latency-us = avg_latency_us = { $avg_latency_us }
example-metrics-p50-latency-us = p50_latency_us = { $p50_latency_us }
example-metrics-p95-latency-us = p95_latency_us = { $p95_latency_us }
example-metrics-p99-latency-us = p99_latency_us = { $p99_latency_us }
example-metrics-latency-distribution = latency_distribution = { $latency_distribution }
example-metrics-3-gaugef64-floating-point-metric = 3. GaugeF64 浮点指标
example-metrics-3-1-gaugef64-new-set-get = 3.1 GaugeF64::new() / set() / get()
example-metrics-initial-value = 初始值 = { $value }
example-metrics-after-set-87-5 = set(87.5) 之后 = { $value }
example-metrics-after-set-99-9 = set(99.9) 之后 = { $value }
example-metrics-3-2-the-pool-hit-rate-metric = 3.2 Metrics 暴露的 pool_hit_rate 指标
example-metrics-pool-hit-rate = pool_hit_rate = { $value }%
example-metrics-pool-hit-rate-after-reset = reset 后 pool_hit_rate = { $value }%
example-metrics-3-3-sinkhealth-healthy = 3.3 SinkHealth::healthy() / unhealthy() 构造函数
example-metrics-healthy-status = healthy.status = { $status }
example-metrics-unhealthy-status = unhealthy.status = { $status }
example-metrics-4-sinkstatus-transitions = 4. SinkStatus 状态迁移
example-metrics-4-1-default-state-notstarted = 4.1 默认状态 NotStarted
example-metrics-status = status = { $status }
example-metrics-4-2-notstarted-healthy = 4.2 NotStarted → Healthy
example-metrics-4-5-unhealthy-healthy-recovery = 4.5 Unhealthy → Healthy（恢复）
example-metrics-4-6-state-machine-summary = 4.6 状态机汇总
example-metrics-notstarted-is-operational = NotStarted.is_operational()  = { $value }
example-metrics-healthy-is-operational = Healthy.is_operational()     = { $value }
example-metrics-degraded-is-operational = Degraded.is_operational()    = { $value }
example-metrics-unhealthy-is-operational = Unhealthy.is_operational()   = { $value }
example-metrics-5-metrics-sink-health-tracking = 5. Metrics 的 sink 健康跟踪
example-metrics-5-1-sink-started-marks-a-sink = 5.1 sink_started() 将 sink 标记为已启动
example-metrics-failures = { $name } -> { $status }，失败次数 = { $consecutive_failures }
example-metrics-5-2-sink-degraded-marks = 5.2 sink_degraded() 标记降级
example-metrics-file-status = file.status = { $status }
example-metrics-file-last-error = file.last_error = { $last_error }
example-metrics-5-3-update-sink-health = 5.3 update_sink_health() 累计失败次数
example-metrics-database-status = database.status = { $status }
example-metrics-database-consecutive-failures = database.consecutive_failures = { $consecutive_failures }
example-metrics-5-4-update-sink-health-resets = 5.4 update_sink_health() 在恢复时重置计数
example-metrics-5-5-update-sink-health-uses-a = 5.5 无错误信息时 update_sink_health() 使用默认消息
example-metrics-file-status-error = file.status.error = { $error }
example-metrics-6-healthstatus-overall-snapshot = 6. HealthStatus 整体快照
example-metrics-6-1-empty-sinks-notstarted = 6.1 sink 为空 → NotStarted
example-metrics-overall-status = overall_status = { $overall_status }
example-metrics-6-2-all-healthy = 6.2 全部健康
example-metrics-channel-usage = channel_usage  = { $value }%
example-metrics-uptime-seconds = uptime_seconds = { $uptime_seconds }
example-metrics-encryption-key-valid = encryption_key_valid = { $encryption_key_valid }
example-metrics-6-3-degraded-present-no = 6.3 存在 Degraded（无 Unhealthy）
example-metrics-6-4-unhealthy-present = 6.4 存在 Unhealthy
example-metrics-error = error = { $error }
example-metrics-6-5-healthstatus-fields-at-a = 6.5 HealthStatus 字段一览
example-metrics-sinks-count = sink 数量 = { $value }
example-metrics-metrics-logs-written = metrics.logs_written     = { $logs_written }
example-metrics-metrics-sink-errors = metrics.sink_errors      = { $sink_errors }
example-metrics-metrics-active-workers = metrics.active_workers   = { $active_workers }
example-metrics-metrics-pool-hit-rate = metrics.pool_hit_rate    = { $pool_hit_rate }
example-metrics-pool-stats = pool_stats = { $pool_stats }
example-metrics-7-sinkhealthmonitor-smart = 7. SinkHealthMonitor 智能回退与恢复
example-metrics-7-1-sinkhealthmonitor-default = 7.1 SinkHealthMonitor::default() 初始状态
example-metrics-database-initial-state = database 初始状态 = { $value }
example-metrics-7-2-failures-below-threshold = 7.2 失败次数未达阈值 → Retry
example-metrics-action = action = { $action }
example-metrics-requires-action = requires_action = { $value }
example-metrics-sink-name = sink_name = { $value }
example-metrics-7-3-threshold-reached-fallback = 7.3 达到阈值 → 回退到 file
example-metrics-database-state = database 状态 = { $state }
example-metrics-target = target = { $target }
example-metrics-reason = reason = { $reason }
example-metrics-7-4-healthy-again = 7.4 恢复健康 → AttemptRecovery（指数退避）
example-metrics-delay-ms = delay_ms = { $delay_ms }
example-metrics-7-5-healthy-again-during = 7.5 恢复期间再次健康 → Wait
example-metrics-7-6-confirm-recovery-confirms = 7.6 confirm_recovery() 确认恢复
example-metrics-database-state-2 = database 状态 = { $value }
example-metrics-7-7-filesink-disk-full-fallback = 7.7 FileSink 磁盘写满 → 回退到 console
example-metrics-file-state = file 状态 = { $state }
example-metrics-7-8-encryption-error-fallback = 7.8 加密错误 → 回退为明文
example-metrics-7-9-custom-fallbackconfig = 7.9 自定义 FallbackConfig + FallbackStats
example-metrics-stats = stats = { $stats }
example-metrics-7-10-reset-resets-the-monitor = 7.10 reset() 重置监控器
example-metrics-is-any-in-fallback-after-reset = reset 后 is_any_in_fallback = { $value }
example-metrics-7-11-get-fallback-events-event = 7.11 get_fallback_events() 事件历史
example-metrics-event-count = 事件数 = { $value }
example-metrics-first-event-sink = 首个事件 sink = { $value }
example-metrics-first-event-from = 首个事件 from = { $value }
example-metrics-first-event-to = 首个事件 to   = { $value }
example-metrics-8-prometheus-format-export = 8. Prometheus 格式导出
example-metrics-8-1-simulated-metric-collection = 8.1 模拟指标采集
example-metrics-8-2-export-prometheus-output = 8.2 export_prometheus() 输出（摘录关键行）
example-metrics-8-3-healthstatus-serialization = 8.3 HealthStatus 序列化预览（serde_json）
example-metrics-truncated = ……（已截断）
example-metrics-prometheus-export-has = ✓ Prometheus 导出共 { $value } 字符、{ $value2 } 行

# object-pool
example-object-pool-inklog-object-pool-example = inklog 对象池示例
example-object-pool-all-object-pool-examples = 对象池示例全部演示完毕。
example-object-pool-example-1-basic-objectpool = 示例 1：ObjectPool 基础操作
example-object-pool-1-1-new-defaults-capacity-1024 = 1.1 new() 默认值（容量 1024）
example-object-pool-default-pool-built = 默认池已构建
example-object-pool-1-2-custom-config-via-with = 1.2 通过 with_config(ObjectPoolConfig) 自定义配置
example-object-pool-custom-config-pool-built = 自定义配置池已构建
example-object-pool-1-3-put-get-access = 1.3 put() / get() 存取
example-object-pool-greeting = greeting = { $greeting }
example-object-pool-name = name     = { $name }
example-object-pool-version = version  = { $version }
example-object-pool-missing = missing  = { $missing }
example-object-pool-all-access-assertions-passed = ✓ 全部存取断言通过
example-object-pool-1-4-overwriting = 1.4 覆盖写入
example-object-pool-overwriting-greeting-hello-world = 覆盖 greeting：hello → world
example-object-pool-overwrite-verified = ✓ 覆盖写入校验通过
example-object-pool-1-5-explicit-error-propagation = 1.5 显式错误传播（返回 Result）
example-object-pool-put-get-both-return-ok-errors = put/get 均返回 Ok；错误显式传播
example-object-pool-example-2-custom = 示例 2：自定义 ObjectPoolConfig（带 TTL）
example-object-pool-pool-with-ttl-60s-built-and = TTL=60s 的池已构建并成功存取
example-object-pool-small-pool-4-filled-with-4 = 小容量池（4）已填满 4 个对象
example-object-pool-small-capacity-pool-fully = ✓ 小容量池校验通过
example-object-pool-example-3-get-log-record-put = 示例 3：get_log_record / put_log_record 全局函数
example-object-pool-3-1-get-log-record-fetches-a = 3.1 get_log_record() 取出一个 LogRecord
example-object-pool-logrecord-from-the-global-pool = 来自全局池的 LogRecord：level = { $level }
example-object-pool-3-2-get-log-record-again = 3.2 再次 get_log_record() 验证已复位
example-object-pool-fetched-again-level = 再次取出：level = { $level }
example-object-pool-10-get-put-rounds-completed = 完成 10 轮 get/put，未发生 panic
example-object-pool-global-logrecord-pool-verified = ✓ 全局 LogRecord 池校验通过
example-object-pool-example-4-get-string-buffer-put = 示例 4：get_string_buffer / put_string_buffer 全局函数
example-object-pool-4-1-get-string-buffer-fetches-a = 4.1 get_string_buffer() 取出一个 String
example-object-pool-string-from-the-global-pool-len = 来自全局池的 String：len = { $value }
example-object-pool-filled-inklog-string-buffer-and = 写入 'inklog string buffer' 后归还全局池
example-object-pool-4-2-get-string-buffer-again = 4.2 再次 get_string_buffer()
example-object-pool-fetched-again-len = 再次取出：len = { $value }
example-object-pool-5-get-put-rounds-completed = 完成 5 轮 get/put
example-object-pool-global-string-pool-verified = ✓ 全局 String 池校验通过
example-object-pool-example-5-edge-cases = 示例 5：边界情况
example-object-pool-empty-key-access-works = ✓ 空键存取正常
example-object-pool-empty-value-access-works = ✓ 空值存取正常
example-object-pool-repeated-put-keeps-the-last = ✓ 重复 put 保留最后一次的值
example-object-pool-all-100-key-value-pairs = ✓ 100 组键值对全部存取正确
example-object-pool-example-6-error-handling-checks = 示例 6：错误处理检查
example-object-pool-put-succeeded-ok = ✓ put 成功：Ok(())
example-object-pool-put-failed = ✗ put 失败：{ $e }
example-object-pool-get-succeeded-ok-some = ✓ get 成功：Ok(Some({ $v }))
example-object-pool-get-returned-ok-none-key-absent = get 返回：Ok(None) — 键不存在
example-object-pool-get-failed = ✗ get 失败：{ $e }
example-object-pool-unexpected-key-exists = 不符合预期：键存在
example-object-pool-get-of-a-missing-key-ok-none-as = ✓ 取不存在的键：Ok(None) — 符合预期
example-object-pool-operator-error-propagation-works = ✓ ? 运算符错误传播正常
example-object-pool-best-practices = 最佳实践
example-object-pool-1-when-to-use-an-object-pool = 1. 何时使用对象池：
example-object-pool-when-creating-destroying-the = - 高频创建/销毁同一类型时（如日志管道）
example-object-pool-reduces-gc-pressure-and = - 降低 GC 压力与分配开销
example-object-pool-typical-uses-logrecord-reuse = - 典型用途：LogRecord 复用、String 缓冲复用
example-object-pool-2-capacity-config = 2. 容量配置：
example-object-pool-max-capacity-size-by = - max_capacity：按并发量设定（建议 线程数 × 单线程缓存）
example-object-pool-too-small-objects-are-created = - 过小：对象不断新建，池化失去意义
example-object-pool-too-large-wastes-memory-at-most = - 过大：浪费内存；建议不超过实际需求的 2 倍
example-object-pool-3-ttl-config = 3. TTL 配置：
example-object-pool-ttl-secs-maximum-lifetime-of-an = - ttl_secs：对象在池中的最长存活时间
example-object-pool-suits-stateful-objects-that = - 适合需要定期清理的有状态对象
example-object-pool-none-never-expires-suits = - None：永不过期（适合 String 等无状态对象）
example-object-pool-4-global-functions-vs-direct-api = 4. 全局函数与直接 API 的取舍：
example-object-pool-get-log-record-put-log-record = - get_log_record/put_log_record：线程本地池，无锁竞争
example-object-pool-objectpool-new-shared-pool = - ObjectPool::new()：共享池，可跨线程共享
example-object-pool-prefer-the-global-functions-in = - 单线程场景优先使用全局函数
example-object-pool-use-an-objectpool-instance-for = - 多线程共享场景使用 ObjectPool 实例
example-object-pool-5-usage-patterns = 5. 使用模式：
example-object-pool-always-pair-get-put-to-avoid = - get/put 必须成对，避免对象泄漏
example-object-pool-objects-are-reset-on-put-do-not = - put 时对象会复位；不要继续引用旧值
example-object-pool-do-not-keep-using-a-reference = - put 之后不要继续使用 get 返回的引用

# output-format
example-output-format-inklog-output-format-example = inklog 输出格式示例
example-output-format-all-output-format-examples = ✓ 输出格式示例全部演示完毕
example-output-format-example-1-outputformat-enum = 示例 1：OutputFormat 枚举变体
example-output-format-outputformat-text = OutputFormat::Text    = { $text }
example-output-format-outputformat-json = OutputFormat::Json    = { $json }
example-output-format-outputformat-default = OutputFormat::default() = { $default }
example-output-format-default-format-human-readable = 默认格式：{ $default }（人类可读文本）
example-output-format-equivalence-check = 等价性检查：
example-output-format-text-text = Text == Text → { $is_text_equal }
example-output-format-text-json = Text == Json → { $value }
example-output-format-example-2-fromstr-string-parsing = 示例 2：FromStr 字符串解析
example-output-format-string-outputformat-parsing = 字符串 → OutputFormat 解析：
example-output-format-error = { $s } → 错误：{ $e }
example-output-format-from-str-parsed-check = { $s } → { $fmt } ✓
example-output-format-error-invalid-value = { $value } → 错误（非法值）：{ $value2 }
example-output-format-example-3-display-trait-output = 示例 3：Display trait 输出
example-output-format-outputformat-string = OutputFormat → 字符串：
example-output-format-outputformat-text-2 = OutputFormat::Text → "{ $value }"
example-output-format-outputformat-json-2 = OutputFormat::Json → "{ $value }"
example-output-format-example-4-text-rendering-default = 示例 4：Text 格式渲染（默认）
example-output-format-logtemplate-default-rendering = LogTemplate::default() 渲染结果：
example-output-format-characteristics = 特点：
example-output-format-human-readable-suits-consoles = - 人类可读；适合控制台与日志文件
example-output-format-contains-timestamp-level-target = - 包含时间戳、级别、target、消息
example-output-format-structured-fields-appended-as = - 结构化字段以 JSON 形式追加在末尾
example-output-format-example-5-json-rendering-ndjson = 示例 5：Json 格式渲染（NDJSON）
example-output-format-ndjson-output = NDJSON 输出：
example-output-format-machine-parseable-suits-log = - 机器可解析；适合日志采集系统
example-output-format-one-complete-json-object-per = - 每行一个完整 JSON 对象
example-output-format-directly-collectable-by-fluentd = - 可被 Fluentd/Logstash/Promtail 直接采集
example-output-format-supports-nested-structured = - 支持嵌套的结构化字段
example-output-format-json-output-parses-correctly = ✓ JSON 输出可被标准解析器正确解析
example-output-format-example-6-outputformat-in-sink = 示例 6：Sink 配置中的 OutputFormat
example-output-format-console-sink-config-json-format = Console sink 配置（JSON 格式）：
example-output-format-output-format = output_format = { $output_format }
example-output-format-file-sink-config-text-format = File sink 配置（Text 格式）：
example-output-format-toml-config-example = TOML 配置示例：
example-output-format-7-best-practices = 7. 最佳实践
example-output-format-outputformat-selection-guide = OutputFormat 选型指南：
example-output-format-1-text-format-use-cases = 1. Text 格式适用场景：
example-output-format-development-console-output = - 开发期控制台输出
example-output-format-log-files-read-directly-by = - 人工直接阅读的日志文件
example-output-format-debugging-and-troubleshooting = - 调试与排障
example-output-format-2-json-format-use-cases = 2. Json 格式适用场景：
example-output-format-production-structured-log = - 生产环境结构化日志采集
example-output-format-log-analytics-platforms-elk = - 日志分析平台（ELK/Loki/Datadog）
example-output-format-suits-field-level-querying-and = - 适合字段级查询与过滤
example-output-format-3-mixing-formats = 3. 混用格式：
example-output-format-console-text-easy-for = - Console → Text（便于开发者阅读）
example-output-format-file-json-easy-to-collect-and = - File → Json（便于采集与分析）
example-output-format-database-structured-fields = - Database → 结构化字段直存
example-output-format-4-performance-considerations = 4. 性能考量：
example-output-format-text-format-renders-faster = - Text 格式渲染更快（简单字符串拼接）
example-output-format-json-format-needs-serialization = - Json 格式需要序列化（约 2-3 倍开销）
example-output-format-for-high-throughput-use-text = - 高吞吐场景：文件用 Text，在采集端再转 Json

# performance
example-performance-inklog-performance-test-example = === inklog 性能测试示例 ===
example-performance-all-performance-tests-completed = ✓ 全部性能测试完成
example-performance-1-console-sink-throughput-test = 1. Console sink 吞吐测试
example-performance-1-1-throughput-test-5-000-logs = 1.1 吞吐测试（5,000 条日志）
example-performance-total-logs = 日志总数：  { $count }
example-performance-total-time = 总耗时：   { $value }
example-performance-throughput-logs-s = 吞吐：     { $throughput } logs/s
example-performance-avg-latency = 平均延迟：  { $value }
example-performance-2-file-sink-throughput-test = 2. File sink 吞吐测试
example-performance-2-1-throughput-test-2-000-logs = 2.1 吞吐测试（2,000 条日志）
example-performance-note-file-sink-throughput = 说明：file sink 吞吐取决于通道容量与 worker 线程数
example-performance-current-throughput-reflects = 当前吞吐反映了异步写入开销
example-performance-warning-failed-to-delete-temp = 警告：删除临时文件失败：{ $log_path }（{ $e }）
example-performance-cleaned-up-temp-file = ✓ 已清理临时文件：{ $log_path }
example-performance-3-latency-analysis-p50-p95-p99 = 3. 延迟分析（P50/P95/P99）
example-performance-3-1-single-log-latency = 3.1 单条日志延迟测量（500 个样本）
example-performance-total-samples = 样本总数：  { $count }
example-performance-p50-latency = P50 延迟：  { $value }
example-performance-p95-latency = P95 延迟：  { $value }
example-performance-p99-latency = P99 延迟：  { $value }
example-performance-actual-numbers-depend-on-system = 实际数值取决于系统负载与通道状态
example-performance-4-backpressure-test = 4. 背压测试
example-performance-channel-capacity = 通道容量：{ $capacity }
example-performance-enqueue-time = 入队耗时：   { $value }
example-performance-flush-time = 冲刷耗时：   { $value }
example-performance-total-time-2 = 总耗时：     { $value }
example-performance-throughput-logs-s-2 = 吞吐：       { $throughput } logs/s
example-performance-note-throughput-includes-total = 说明：吞吐包含日志处理与冲刷的总时间
example-performance-a-small-channel-may-cause = ⚠️  通道过小可能造成背压
example-performance-capacity-is-sufficient-no = ✓  容量充足，无明显背压
example-performance-5-concurrency-test = 5. 并发测试
example-performance-5-1-multi-task-concurrent = 5.1 多任务并发写入（4 个任务 × 100 条日志）
example-performance-concurrent-tasks = 并发任务数：{ $task_count }
example-performance-logs-per-task = 每任务日志数： { $logs_per_task }
example-performance-total-logs-2 = 日志总数：    { $total_logs }
example-performance-total-time-3 = 总耗时：     { $value }
example-performance-throughput-logs-s-3 = 吞吐：     { $throughput } logs/s

# rate-limiter
example-rate-limiter-inklog-rate-limiter-example = inklog 限流器示例
example-rate-limiter-all-rate-limiter-examples = ✓ 限流器示例全部演示完毕
example-rate-limiter-example-1-creating-the-rate = 示例 1：创建限流器
example-rate-limiter-ratelimiter-new-1000-1000 = RateLimiter::new(1000) — 每秒 1000 个令牌
example-rate-limiter-initial-state-bucket-full = 初始状态：令牌桶已满（tokens = max_tokens = 1000）
example-rate-limiter-refill-rate-1000-tokens-sec = 补充速率：1000 令牌/秒
example-rate-limiter-try-acquire-true-bucket-full = ✓ try_acquire() → true（桶已满，放行）
example-rate-limiter-2-rate-limiting-behavior = 2. 限流行为
example-rate-limiter-ratelimiter-new-5-5-tokens-per = RateLimiter::new(5) — 每秒 5 个令牌
example-rate-limiter-rapid-consecutive-try-acquire = 连续快速 try_acquire()：
example-rate-limiter-result-allowed-rejected = 结果：放行 { $allowed } 条，拒绝 { $rejected } 条
example-rate-limiter-token-bucket-rate-limits = ✓ 令牌桶限流正确
example-rate-limiter-example-3-dropped-count = 示例 3：dropped_count() 统计
example-rate-limiter-ratelimiter-new-2-2-tokens-per = RateLimiter::new(2) — 每秒 2 个令牌
example-rate-limiter-after-consuming-2-tokens = 消耗 2 个令牌后：dropped_count = { $value }
example-rate-limiter-after-3-rejections-dropped-count = 3 次拒绝后：dropped_count = { $value }
example-rate-limiter-dropped-count-correctly-counts = ✓ dropped_count 正确统计被限流的日志
example-rate-limiter-example-4-automatic-token-refill = 示例 4：令牌自动补充
example-rate-limiter-ratelimiter-new-100-100-tokens = RateLimiter::new(100) — 每秒 100 个令牌
example-rate-limiter-after-draining-100-tokens = 耗尽 100 个令牌后：
example-rate-limiter-try-acquire-false-bucket-empty = try_acquire() → false（桶已空）
example-rate-limiter-after-waiting-50ms-5-tokens = 等待 50ms 后（约补充 5 个令牌）：
example-rate-limiter-acquired-again-allowed-expected = 再次获取：放行 { $refilled } 条（预期约 5）
example-rate-limiter-tokens-refill-automatically-at = ✓ 令牌按 refill_rate 自动补充
example-rate-limiter-example-5-log-rate-limiting = 示例 5：日志限流模式
example-rate-limiter-scenario-cap-log-output-at-10 = 场景：将日志输出限制在每秒 10 条
example-rate-limiter-simulating-a-burst-of-logs-20 = 模拟突发日志（20 条）：
example-rate-limiter-info-log-written = [INFO] 第 { $i } 条日志 — 已写入
example-rate-limiter-statistics = 统计：
example-rate-limiter-written = 已写入：{ $written }
example-rate-limiter-dropped = 已丢弃：{ $dropped }
example-rate-limiter-total-dropped = 累计丢弃：{ $value }
example-rate-limiter-rate-limiting-works-protects = ✓ 限流生效：突发流量下保护下游系统
example-rate-limiter-example-6-zero-rate-limiter = 示例 6：零速率限流器（rate=0）
example-rate-limiter-ratelimiter-new-0-allows-no = RateLimiter::new(0) — 完全不放行
example-rate-limiter-try-acquire-always-false = try_acquire() → 恒为 false
example-rate-limiter-dropped-count = dropped_count = { $value }
example-rate-limiter-rate-0-is-equivalent-to-full = ✓ rate=0 等价于完全静默
example-rate-limiter-7-best-practices = 7. 最佳实践
example-rate-limiter-ratelimiter-usage-advice = RateLimiter 使用建议：
example-rate-limiter-1-rate-selection = 1. 速率选择：
example-rate-limiter-development-1000-5000-tokens = - 开发环境：1000-5000 令牌/秒（宽松）
example-rate-limiter-production-100-1000-tokens-sec = - 生产环境：100-1000 令牌/秒（视下游容量）
example-rate-limiter-failure-scenarios-10-50-tokens = - 故障场景：10-50 令牌/秒（防止日志风暴）
example-rate-limiter-2-integrating-with-loggermanager = 2. 与 LoggerManager 集成：
example-rate-limiter-3-monitoring-dropped-count = 3. 监控 dropped_count：
example-rate-limiter-report-dropped-count-to-metrics = - 定期把 dropped_count 上报到 Metrics
example-rate-limiter-continuously-growing-dropped = - dropped_count 持续增长说明限流过严
example-rate-limiter-dropped-count-0-means-the-limit = - dropped_count = 0 说明限流宽松或没有压力
example-rate-limiter-4-performance-characteristics = 4. 性能特征：
example-rate-limiter-try-acquire-is-o-1 = - try_acquire() 为 O(1)
example-rate-limiter-uses-parking-lot-mutex-20ns = - 使用 parking_lot::Mutex（加锁开销约 20ns）
example-rate-limiter-suits-high-frequency-logging = - 适合高频日志场景

# runtime-ops
example-runtime-ops-inklog-loggermanager-runtime = inklog LoggerManager 运行期操作 API 示例
example-runtime-ops-all-runtime-operations-api = 运行期操作 API 示例全部演示完毕。
example-runtime-ops-example-1-get-health-status = 示例 1：get_health_status() 健康状态
example-runtime-ops-api-signature = API 签名：
example-runtime-ops-healthstatus-fields = HealthStatus 字段：
example-runtime-ops-overall-status-sinkstatus = overall_status:      SinkStatus（Healthy/Degraded/Unhealthy/NotStarted）
example-runtime-ops-sinks-hashmap-string-sinkhealth = sinks:               HashMap<String, SinkHealth>
example-runtime-ops-channel-usage-f64-0-0-1-0 = channel_usage:       f64 (0.0 ~ 1.0)
example-runtime-ops-uptime-seconds-u64 = uptime_seconds:      u64
example-runtime-ops-metrics-metricssnapshot = metrics:             MetricsSnapshot
example-runtime-ops-pool-stats-option-poolstats = pool_stats:          Option<PoolStats>
example-runtime-ops-encryption-key-valid-bool = encryption_key_valid: bool
example-runtime-ops-sinkstatus-enum-values = SinkStatus 枚举取值：
example-runtime-ops-healthy-sink-running-normally = Healthy             - sink 运行正常
example-runtime-ops-notstarted-sink-not-started-yet = NotStarted          - sink 尚未启动
example-runtime-ops-use-cases = 适用场景：
example-runtime-ops-poll-health-status-periodically = - 定期轮询健康状态（如每 30 秒）
example-runtime-ops-integrate-into-the-health-http = - 接入 /health HTTP 端点
example-runtime-ops-trigger-alert-notifications = - 触发告警通知
example-runtime-ops-code-example = 代码示例：
example-runtime-ops-example-2-channel-capacity-and = 示例 2：通道容量与队列长度
example-runtime-ops-config-defaults = 配置默认值：
example-runtime-ops-performance-channel-capacity = performance.channel_capacity = { $channel_capacity }（默认）
example-runtime-ops-performance-worker-threads = performance.worker_threads   = { $worker_threads }（默认）
example-runtime-ops-monitor-channel-usage-to-warn = - 监控通道使用率，预警背压
example-runtime-ops-channel-len-effective-channel = - channel_len / effective_channel_capacity > 0.8 → 告警
example-runtime-ops-adjust-channel-capacity = - 动态调整 channel_capacity（需重建 LoggerManager）
example-runtime-ops-example-3-sink-recovery = 示例 3：sink 恢复操作
example-runtime-ops-recover-sink-usage = recover_sink() 用法：
example-runtime-ops-manually-recover-a-sink-by-name = - 按名称手动恢复单个 sink
example-runtime-ops-sink-names-come-from-the-config = - sink 名称来自配置（如 "console"、"file"、"database"）
example-runtime-ops-returns-err-inklogerror-when = - 恢复失败时返回 Err(InklogError)
example-runtime-ops-trigger-recovery-for-unhealthy = trigger_recovery_for_unhealthy_sinks() 用法：
example-runtime-ops-automatically-scan-all = - 自动扫描全部不健康的 sink
example-runtime-ops-try-to-recover-every-unhealthy = - 尝试恢复每个不健康的 sink
example-runtime-ops-returns-the-list-of = - 返回恢复成功的 sink 名称列表
example-runtime-ops-recovery-strategies = 恢复策略：
example-runtime-ops-console-sink-usually-needs-no = - Console sink：通常无需恢复（直接写 stdout）
example-runtime-ops-file-sink-reopen-the-file-handle = - File sink：重新打开文件句柄
example-runtime-ops-database-sink-rebuild-the = - Database sink：重建连接池
example-runtime-ops-example-4-graceful-shutdown = 示例 4：用 shutdown() 优雅关闭
example-runtime-ops-shutdown-behavior = shutdown() 行为：
example-runtime-ops-1-stop-accepting-new-log-records = 1. 停止接收新的日志记录
example-runtime-ops-2-wait-for-all-logs-in-the = 2. 等待通道内所有日志写入 sink
example-runtime-ops-3-close-all-sinks-file-handles = 3. 关闭所有 sink（文件句柄、数据库连接等）
example-runtime-ops-4-stop-the-http-server-if = 4. 停止 HTTP 服务（若启用）
example-runtime-ops-5-stop-all-worker-threads = 5. 停止全部 worker 线程
example-runtime-ops-caveats = 注意事项：
example-runtime-ops-shutdown-is-synchronous-it = - shutdown() 是同步的；会阻塞直到所有日志写入完成
example-runtime-ops-shutdown-can-take-a-while-if = - 通道积压较多时 shutdown() 可能耗时较长
example-runtime-ops-call-shutdown-from-the-app-s = - 从应用的退出信号处理中调用 shutdown()
example-runtime-ops-full-lifecycle-example = 完整生命周期示例：

# http
example-http-1-start-the-http-server = 1. 启动 HTTP 服务
example-http-http-server-configuration = HTTP 服务配置
example-http-listen-address = 监听地址：{ $addr }
example-http-endpoints = 端点：
example-http-get-health-health-status-as-json = - GET /health  → JSON 格式的健康状态
example-http-get-metrics-metrics-in = - GET /metrics → Prometheus 格式的指标
example-http-http-server-started-on-port = ✓ HTTP 服务已启动，端口 { $port }
example-http-failed-to-handle-connection = 处理连接 { $addr } 失败：{ $e }
example-http-connection-handler-task-aborted = 连接 { $addr } 的处理任务被中止：{ $join_err }
example-http-failed-to-accept-a-connection = 接受连接失败：{ $e }
example-http-shutdown-signal-received = ✓ 收到关闭信号，正在关闭服务
example-http-request-read-timed-out-s = 请求读取超时（{ $value }s），关闭连接
example-http-2-health-check-endpoint-demo = 2. 健康检查端点演示
example-http-send-a-get-health-request = 发送 GET /health 请求
example-http-request-url = 请求 URL：{ $url }
example-http-response-status = 响应状态：{ $status_code }
example-http-content-type = Content-Type：{ $content_type }
example-http-response-body-json = 响应体（JSON）：
example-http-health-status = ✓ 健康状态：{ $overall_status }
example-http-sink = ✓ Sink '{ $name }'：{ $value }
example-http-health-endpoint-test-passed = ✓ /health 端点测试通过
example-http-3-metrics-endpoint-demo = 3. 指标端点演示
example-http-send-a-get-metrics-request = 发送 GET /metrics 请求
example-http-prometheus-format-metrics-output = Prometheus 格式指标输出
example-http-key-metrics-summary = 关键指标摘要
example-http-metrics-in-total = ……（共 { $value } 项指标）
example-http-metrics-endpoint-test-passed = ✓ /metrics 端点测试通过
example-http-4-error-handling-endpoint-demo = 4. 错误处理端点演示
example-http-request-a-nonexistent-endpoint = 请求不存在的端点
example-http-response-status-expected-404 = 响应状态：{ $status_code }（预期：404）
example-http-response-body =
    响应体：
    { $value }
example-http-404-error-handling-test-passed = ✓ 404 错误处理测试通过
example-http-inklog-http-health-check-and = === inklog HTTP 健康检查与指标端点示例 ===
example-http-5-integration-guide = 5. 集成指南
example-http-configure-scraping-in-prometheus = 在 Prometheus 中配置抓取：
example-http-prometheus-yml = # prometheus.yml
example-http-scrape-configs = scrape_configs:
example-http-job-name-inklog = - job_name: 'inklog'
example-http-static-configs = static_configs:
example-http-targets-localhost = - targets: ['localhost:{ $port }']
example-http-then-run-prometheus-config-file = 然后运行：prometheus --config.file=prometheus.yml
example-http-test-with-curl = 用 curl 测试：
example-http-health-check = # 健康检查
example-http-curl-http-localhost-health-jq = curl http://localhost:{ $port }/health | jq
example-http-prometheus-metrics = # Prometheus 指标
example-http-curl-http-localhost-metrics = curl http://localhost:{ $port }/metrics
example-http-press-ctrl-c-to-exit = 按 Ctrl+C 退出……

# http-auth
example-http-auth-inklog-http-auth-and-ip = inklog HTTP 认证与 IP 白名单示例
example-http-auth-all-http-auth-examples-shown = HTTP 认证示例已全部展示。
example-http-auth-example-1-httpauthconfig-bearer = 示例 1：HttpAuthConfig Bearer 令牌认证
example-http-auth-default-httpauthconfig = 默认 HttpAuthConfig：
example-http-auth-enabled-disabled-by-default = enabled   = { $enabled }（默认关闭）
example-http-auth-token-env-environment-variable = token_env = "{ $token_env }"（环境变量名）
example-http-auth-enable-auth = 启用认证：
example-http-auth-enabled = enabled   = { $enabled }
example-http-auth-token-env = token_env = "{ $token_env }"
example-http-auth-how-it-works = 工作原理：
example-http-auth-2-the-server-reads-the-expected = 2. 服务端从环境变量 { $token_env } 读取期望令牌
example-http-auth-3-it-compares-the-bearer-token = 3. 与 Authorization 头中的 bearer 令牌比对
example-http-auth-4-a-match-returns-data-a = 4. 匹配返回数据，不匹配返回 401 Unauthorized
example-http-auth-custom-token-environment = 自定义令牌环境变量：
example-http-auth-token-env-custom = token_env = "{ $token_env }"（自定义）
example-http-auth-example-2-ip-whitelist-ip = 示例 2：IP 白名单（ip_whitelist）
example-http-auth-supported-formats = 支持格式：
example-http-auth-1-exact-ip-192-168-1-100 = 1. 精确 IP：  "192.168.1.100"
example-http-auth-2-cidr-range-10-0-0-0-8 = 2. CIDR 网段："10.0.0.0/8"
example-http-auth-config-example-1-internal = 配置示例 1：仅内网
example-http-auth-ip-whitelist = ip_whitelist = { $ip_whitelist }
example-http-auth-inklogconfig-validate-passed = ✓ InklogConfig::validate() 通过
example-http-auth-the-invalid-192-168-was-rejected = ✓ 非法值 "192.168.*.*" 已被拒绝：{ $err }
example-http-auth-config-example-2-specific = 配置示例 2：仅指定监控主机
example-http-auth-config-example-3-whitelist = 配置示例 3：白名单关闭（允许全部 IP）
example-http-auth-ip-whitelist-none-allow-all = ip_whitelist = { $ip_whitelist }（None = 允许全部）
example-http-auth-example-3-httperrormode-error = 示例 3：HttpErrorMode 错误处理模式
example-http-auth-a-comparison-of-the-two-modes = 两种模式对比（作用于 HTTP 服务启动失败时）：
example-http-auth-missing-or-invalid-auth-401 = 认证缺失或非法 → 401 Unauthorized
example-http-auth-ip-not-in-the-whitelist-403 = IP 不在白名单 → 403 Forbidden
example-http-auth-default-mode = 默认模式：
example-http-auth-httperrormode-default = HttpErrorMode::default() = { $value }
example-http-auth-recommended-usage = 推荐用法：
example-http-auth-dev-test-warn-a-busy-port-or = 开发/测试：Warn（端口占用或 TLS 错误仅告警）
example-http-auth-example-4-full-production-grade = 示例 4：完整生产级安全配置
example-http-auth-production-grade = 生产级 HttpServerConfig：
example-http-auth-enabled-2 = enabled      = { $enabled }
example-http-auth-host-port = host:port    = { $host }:{ $port }
example-http-auth-metrics-path = metrics_path = "{ $metrics_path }"
example-http-auth-health-path = health_path  = "{ $health_path }"
example-http-auth-error-mode = error_mode   = { $error_mode }
example-http-auth-auth-enabled = auth.enabled = { $value }
example-http-auth-auth-token-env = auth.token_env = "{ $value }"
example-http-auth-the-corresponding-toml = 对应的 TOML 配置：
example-http-auth-environment-variables-to-set = 启动前需设置的环境变量：
example-http-auth-example-5-environment-variable = 示例 5：环境变量覆盖
example-http-auth-environment-variables-supported = HTTP 服务支持的环境变量：
example-http-auth-load-with-load-with-env = 使用 load_with_env_overrides() 加载：

# tls-config
example-tls-config-inklog-tls-configuration = inklog TLS 配置示例
example-tls-config-all-tls-configuration-examples = TLS 配置示例已全部展示。
example-tls-config-example-1-tlsconfig-basics = 示例 1：TlsConfig 基础
example-tls-config-tlsconfig-fields = TlsConfig 字段：
example-tls-config-cert-path-path-to-the-pem = cert_path：PEM 编码证书文件路径
example-tls-config-key-path-path-to-the-pem = key_path： PEM 编码私钥文件路径
example-tls-config-basic-tlsconfig = 基础 TlsConfig：
example-tls-config-cert-path = cert_path = "{ $cert_path }"
example-tls-config-key-path = key_path  = "{ $key_path }"
example-tls-config-certificate-format-requirements = 证书格式要求：
example-tls-config-certificate-x-509-pem-format = - 证书：X.509 PEM 格式（.crt / .pem）
example-tls-config-private-key-pkcs-8-pem-format = - 私钥：PKCS#8 PEM 格式（.key / .pem）
example-tls-config-self-signed-certificates-work = - 自签名证书可用（开发/测试）
example-tls-config-in-production-use-a-certificate = - 生产环境使用 Let's Encrypt 或商业 CA 证书
example-tls-config-example-2-an-http-server-with = 示例 2：启用 TLS 的 HTTP 服务
example-tls-config-default-configuration-no-tls = 默认配置（无 TLS）：
example-tls-config-tls-none-tls-disabled = tls = { $tls }（None = 关闭 TLS）
example-tls-config-http-server-configuration-with = 启用 TLS 的 HTTP 服务配置：
example-tls-config-enabled = enabled      = { $enabled }
example-tls-config-host-port = host:port    = { $host }:{ $port }
example-tls-config-tls-cert = tls.cert     = "{ $value }"
example-tls-config-tls-key = tls.key      = "{ $value }"
example-tls-config-how-access-changes = 访问方式变化：
example-tls-config-no-tls-http-localhost-9090 = 无 TLS：http://localhost:9090/health
example-tls-config-with-tls-https-localhost-9443 = 启用 TLS：https://localhost:9443/health
example-tls-config-curl-k-https-localhost-9443 = curl -k https://localhost:9443/metrics
example-tls-config-example-3-full-production-grade = 示例 3：完整生产级 TLS 配置
example-tls-config-production-grade = 生产级 HttpServerConfig（TLS + 认证 + 白名单）：
example-tls-config-error-mode = error_mode   = { $error_mode }
example-tls-config-auth-enabled = auth.enabled = { $value }
example-tls-config-ip-whitelist = ip_whitelist = { $ip_whitelist }
example-tls-config-security-layers = 安全层次：
example-tls-config-1-tls-transport-encryption = 1. TLS 传输加密 → 阻断中间人攻击
example-tls-config-2-bearer-token-auth-stops = 2. Bearer 令牌认证 → 阻断未授权访问
example-tls-config-3-ip-whitelist-restricts-access = 3. IP 白名单 → 限制访问来源
example-tls-config-4-strict-error-mode-fail-closed = 4. 严格错误模式 → fail-closed
example-tls-config-example-4-toml-configuration = 示例 4：TOML 配置文件格式
example-tls-config-the-tls-configuration-inside = inklog.toml 中的 TLS 配置：
example-tls-config-load-the-configuration = 加载配置：
example-tls-config-example-5-tls-best-practices = 示例 5：TLS 最佳实践
example-tls-config-certificate-management = 证书管理：
example-tls-config-use-a-let-s-encrypt-or = - 生产环境使用 Let's Encrypt 或商业 CA 证书
example-tls-config-a-self-signed-certificate-is = - 开发环境自签名证书即可：
example-tls-config-openssl-req-x509-newkey-rsa = openssl req -x509 -newkey rsa:4096 -days 365 \
example-tls-config-keyout-inklog-key-out-inklog = -keyout inklog.key -out inklog.crt -nodes \
example-tls-config-subj-cn-localhost = -subj '/CN=localhost'
example-tls-config-rotate-certificates-regularly = - 定期轮换证书（建议 90 天）
example-tls-config-private-key-protection = 私钥保护：
example-tls-config-set-the-private-key-file-mode = - 私钥文件权限设为 600（仅所有者）
example-tls-config-use-chmod-600-etc-ssl-private = - 使用 chmod 600 /etc/ssl/private/inklog.key
example-tls-config-consider-managing-keys-with = - 可考虑用 Kubernetes Secrets 或 Vault 管理密钥
example-tls-config-port-selection = 端口选择：
example-tls-config-standard-https-port-443-needs = - 标准 HTTPS 端口：443（需要 root）
example-tls-config-custom-https-port-8443-9443 = - 自定义 HTTPS 端口：8443 / 9443（推荐）
example-tls-config-do-not-share-it-with-the-http = - 与 HTTP 端口分开（9090 与 9443）
example-tls-config-monitoring-and-alerting = 监控与告警：
example-tls-config-alert-on-certificate-expiry-30 = - 证书到期告警（建议提前 30 天）
example-tls-config-watch-the-tls-handshake-failure = - 关注 TLS 握手失败率
example-tls-config-use-the-health-endpoint-to = - 用 /health 端点验证可用性

# encryption
example-encryption-temporary-file-paths = 临时文件路径：
example-encryption-plaintext = 明文：{ $plain_path }
example-encryption-encrypted = 密文：{ $encrypted_path }
example-encryption-environment-variable-cleared = ✓ 环境变量已清除：{ $key_env }
example-encryption-key-management-best-practices = 密钥管理最佳实践
example-encryption-in-production-follow-these-best = 生产环境请遵循以下最佳实践：
example-encryption-1-key-storage = 1. 密钥存储：
example-encryption-use-a-managed-key-service-such = - 使用托管密钥服务（如 AWS KMS 或 HashiCorp Vault）
example-encryption-never-hard-code-keys-in-source = - 切勿在源码中硬编码密钥
example-encryption-pass-keys-through-environment = - 通过环境变量或密钥文件传入
example-encryption-2-key-rotation = 2. 密钥轮换：
example-encryption-rotate-encryption-keys-regularly = - 定期轮换加密密钥
example-encryption-keep-old-keys-to-decrypt = - 保留旧密钥用于解密历史日志
example-encryption-use-key-versioning = - 使用密钥版本化
example-encryption-3-access-control = 3. 访问控制：
example-encryption-restrict-who-can-access-keys = - 限制可访问密钥的人员
example-encryption-log-key-usage = - 记录密钥使用情况
example-encryption-apply-least-privilege = - 遵循最小权限原则
example-encryption-4-secure-transport = 4. 安全传输：
example-encryption-transport-keys-over-tls = - 通过 TLS 传输密钥
example-encryption-never-print-keys-in-logs = - 切勿在日志中打印密钥
example-encryption-use-a-secure-ci-cd-pipeline = - 使用安全的 CI/CD 流水线
example-encryption-encryption-performance = 加密性能
example-encryption-aes-256-gcm-performance = AES-256-GCM 性能优势：
example-encryption-1-hardware-acceleration = 1. 硬件加速：
example-encryption-modern-cpus-support-the-aes-ni = - 现代 CPU 支持 AES-NI 指令集
example-encryption-hardware-acceleration-reaches = - 硬件加速可达 10+ GB/s
example-encryption-log-write-performance-is-barely = - 日志写入性能几乎不受影响
example-encryption-2-aead-authentication = 2. AEAD 认证：
example-encryption-provides-encryption-and = - 同时提供加密与完整性保护
example-encryption-detects-tampering-automatically = - 自动检测篡改
example-encryption-protects-against-ciphertext = - 防御密文攻击
example-encryption-3-parallelism = 3. 并行性：
example-encryption-gcm-mode-supports-parallel = - GCM 模式支持并行加密
example-encryption-uses-multiple-cores-for = - 利用多核提升吞吐
example-encryption-suits-bulk-log-processing = - 适合批量日志处理
example-encryption-supported-key-formats = 支持的密钥格式
example-encryption-inklog-supports-three-key = inklog 支持三种密钥格式：
example-encryption-1-base64-encoded-recommended = 1. Base64 编码（推荐）：
example-encryption-the-base64-encoding-of-a-32 = - 32 字节密钥的 Base64 编码
example-encryption-example = - 示例：MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE=
example-encryption-safe-standard-and-easy-to = - 安全、标准、便于传输
example-encryption-2-raw-bytes = 2. 原始字节：
example-encryption-uses-the-32-raw-key-bytes = - 直接使用 32 字节原始密钥
example-encryption-example-2 = - 示例：abcdefghijklmnopqrstuvwxyz123456
example-encryption-fits-simple-test-scenarios = - 适合简单测试场景
example-encryption-3-password-derivation = 3. 口令派生：
example-encryption-derives-a-key-from-a-password = - 用 PBKDF2 从口令派生密钥
example-encryption-example-my-secret-password = - 示例：my-secret-password
example-encryption-fits-user-supplied-passwords = - 适合用户自设口令
example-encryption-all-examples-completed = ✓ 示例全部演示完毕

# log-sanitizer
example-log-sanitizer-inklog-log-sanitizer-examples = === inklog 日志清洗器示例 ===
example-log-sanitizer-all-log-sanitizer-examples = ✓ 日志清洗器示例全部演示完毕
example-log-sanitizer-1-default-sensitive-data-masking = 1. 默认敏感数据掩码
example-log-sanitizer-1-1-email-masking = 1.1 邮箱掩码
example-log-sanitizer-raw = 原始：{ $input }
example-log-sanitizer-sanitized = 清洗后：{ $output }
example-log-sanitizer-1-2-password-masking = 1.2 密码掩码
example-log-sanitizer-1-3-api-key-masking = 1.3 API key 掩码
example-log-sanitizer-1-4-bank-card-number-masking = 1.4 银行卡号掩码
example-log-sanitizer-1-5-bearer-token-masking = 1.5 Bearer 令牌掩码
example-log-sanitizer-2-custom-sensitive-patterns = 2. 自定义敏感模式（手机号、身份证号）
example-log-sanitizer-2-1-phone-number-masking = 2.1 手机号掩码
example-log-sanitizer-2-2-national-id-masking = 2.2 身份证号掩码
example-log-sanitizer-3-escapemode-strict-escaping = 3. EscapeMode::Strict 转义
example-log-sanitizer-3-1-control-and-special = 3.1 控制字符与特殊字符转义
example-log-sanitizer-3-2-printable-characters-are = 3.2 可打印字符保持不变
example-log-sanitizer-4-escapemode-jsonsafe-json = 4. EscapeMode::JsonSafe JSON 转义
example-log-sanitizer-5-escapemode-minimal-escaping = 5. EscapeMode::Minimal 转义（默认）
example-log-sanitizer-6-length-limiting-max-length = 6. 长度限制（max_length）
example-log-sanitizer-raw-bytes = 原始（{ $value } 字节）：{ $input }
example-log-sanitizer-sanitized-bytes = 清洗后（{ $value } 字节）：{ $output }
example-log-sanitizer-7-newline-escaping-default = 7. 换行转义（默认 custom_replacements）
example-log-sanitizer-8-custom-string-replacement-add = 8. 自定义字符串替换（add_replacement）

# masking
example-masking-inklog-data-masking-examples = === inklog 数据掩码示例 ===
example-masking-1-basic-masking = 1. 基础掩码
example-masking-1-1-email-masking = 1.1 邮箱掩码
example-masking-raw-email = 原始邮箱：{ $email }
example-masking-masked = 掩码后： { $masked_email }
example-masking-1-2-phone-number-masking = 1.2 手机号掩码
example-masking-raw-phone = 原始手机号：{ $phone }
example-masking-masked-2 = 掩码后： { $masked_phone }
example-masking-1-3-national-id-masking = 1.3 身份证号掩码
example-masking-raw-id = 原始身份证号：{ $id_card }
example-masking-masked-3 = 掩码后：    { $masked_id }
example-masking-1-4-bank-card-masking = 1.4 银行卡号掩码
example-masking-raw-bank-card = 原始银行卡号：{ $bank_card }
example-masking-masked-4 = 掩码后：    { $masked_card }
example-masking-2-mixed-sensitive-data = 2. 混合敏感数据
example-masking-raw-text = 原始文本：{ $mixed_text }
example-masking-masked-5 = 掩码后： { $masked_text }
example-masking-3-json-data-masking = 3. JSON 数据掩码
example-masking-raw-json = 原始 JSON：
example-masking-masked-json = 掩码后 JSON：
example-masking-4-nested-json-masking = 4. 嵌套 JSON 掩码
example-masking-raw-nested-json = 原始嵌套 JSON：
example-masking-masked-nested-json = 掩码后嵌套 JSON：
example-masking-5-sensitive-field-detection = 5. 敏感字段识别
example-masking-5-1-sensitive-field-examples = 5.1 敏感字段示例
example-masking-these-fields-are-detected-as = 以下字段会被识别为敏感：
example-masking-5-2-non-sensitive-field-examples = 5.2 非敏感字段示例
example-masking-these-fields-are-not-sensitive = 以下字段不敏感：
example-masking-6-hashmap-data-masking = 6. HashMap 数据掩码
example-masking-raw-hashmap = 原始 HashMap：
example-masking-masked-hashmap = 掩码后 HashMap：
example-masking-7-advanced-sensitive-data = 7. 进阶敏感数据
example-masking-7-1-jwt-token-masking = 7.1 JWT 令牌掩码
example-masking-raw-jwt = 原始 JWT：{ $value }……
example-masking-masked-6 = 掩码后： { $masked_jwt }
example-masking-7-2-aws-access-key-masking = 7.2 AWS 访问密钥掩码
example-masking-raw-aws-key = 原始 AWS 密钥：{ $aws_key }
example-masking-masked-7 = 掩码后：      { $masked_aws }
example-masking-7-4-credit-card-masking-luhn = 7.4 银行卡掩码（Luhn 校验）
example-masking-raw-visa = 原始 Visa 卡号：{ $visa }
example-masking-masked-8 = 掩码后：    { $masked_visa }
example-masking-raw-amex = 原始 Amex 卡号：{ $amex }
example-masking-masked-9 = 掩码后：    { $masked_amex }
example-masking-7-5-international-phone-masking = 7.5 国际电话号码掩码
example-masking-raw-international-phone = 原始国际号码：{ $intl_phone }
example-masking-masked-10 = 掩码后：      { $masked_intl }
example-masking-7-6-ip-mac-address-masking = 7.6 IP/MAC 地址掩码
example-masking-ipv4 = IPv4: { $ipv4 } → { $value }
example-masking-ipv6 = IPv6: { $ipv6 } → { $value }
example-masking-mac = MAC:  { $mac } → { $value }
example-masking-7-7-third-party-token-masking = 7.7 第三方令牌掩码
example-masking-github = GitHub: { $github_token } → { $value }
example-masking-slack = Slack:  { $slack_token } → { $value }
example-masking-stripe = Stripe: { $stripe_key } → { $value }
example-masking-8-real-world-scenarios = 8. 真实场景
example-masking-8-1-masking-log-messages = 8.1 掩码日志消息
example-masking-raw-log =
    原始日志：
    { $log_message }
example-masking-masked-log =
    掩码后日志：
    { $masked_log }
example-masking-8-2-masking-api-responses = 8.2 掩码 API 响应
example-masking-raw-api-response = 原始 API 响应：
example-masking-masked-api-response = 掩码后 API 响应：
example-masking-9-custom-rules = 9. 自定义规则
example-masking-9-1-maskrulebuilder = 9.1 MaskRuleBuilder
example-masking-custom-rule-priority = 自定义规则：{ $value }（priority={ $value2 }）
example-masking-9-2-datamaskerbuilder = 9.2 DataMaskerBuilder
example-masking-raw = 原始：{ $emp_text }
example-masking-masked-11 = 掩码后：{ $value }
example-masking-9-3-maskruleregistry = 9.3 MaskRuleRegistry
example-masking-built-in-rule-count = 内置规则数：{ $value }
example-masking-after-registering-the-custom = 注册自定义规则后：{ $value }
example-masking-active-rule-count = 生效规则数：{ $value }
example-masking-9-4-maskruleregistry-load-from = 9.4 MaskRuleRegistry::load_from_toml
example-masking-loaded-custom-rules-from-toml = 已从 TOML 加载 { $value } 条自定义规则
example-masking-rule-priority-enabled = 规则：{ $value }（priority={ $value2 }，enabled={ $value3 }）
example-masking-raw-2 = 原始：{ $order_text }
example-masking-10-performance-notes = 10. 性能说明
example-masking-datamasker-performance-notes = DataMasker 性能说明：
example-masking-1-uses-pre-compiled-lazylock = 1. 使用预编译的 LazyLock 正则
example-masking-2-handles-hashmap-and-json-in = 2. 批量处理 HashMap 与 JSON
example-masking-3-recurses-into-nested = 3. 递归处理嵌套结构
example-masking-4-field-name-detection-is-case = 4. 字段名识别大小写不敏感
example-masking-5-rules-run-in-priority-order = 5. 规则按优先级顺序执行
example-masking-6-rules-can-be-managed = 6. 可通过 registry 动态管理规则
example-masking-all-examples-completed = ✓ 示例全部演示完毕
example-masking-press-ctrl-c-to-exit = 按 Ctrl+C 退出……

# path-validator
example-path-validator-inklog-path-validator-examples = === inklog 路径校验器示例 ===
example-path-validator-all-path-validator-examples = ✓ 路径校验器示例全部演示完毕
example-path-validator-1-path-traversal-detection = 1. 路径穿越检测（".." 分量）
example-path-validator-1-1-etc-passwd = 1.1 "../etc/passwd"
example-path-validator-valid-error = valid = { $valid }, error = { $error }
example-path-validator-1-2-foo-bar = 1.2 "foo/../../bar"
example-path-validator-1-3-the-safe-relative-path-logs = 1.3 安全相对路径 "logs/app.log"
example-path-validator-valid = valid = { $valid }
example-path-validator-2-dangerous-component-detection = 2. 危险分量检测（deny_components）
example-path-validator-2-1-etc-passwd = 2.1 "/etc/passwd"
example-path-validator-2-2-ssh-id-rsa = 2.2 "~/.ssh/id_rsa"
example-path-validator-2-3-env = 2.3 "./.env"
example-path-validator-2-4-project-git-config = 2.4 "project/.git/config"
example-path-validator-2-5-etc-passwd-passes-with-an = 2.5 deny_components 为空时 "etc/passwd" 可放行
example-path-validator-3-absolute-path-restriction = 3. 绝对路径限制（allow_absolute）
example-path-validator-3-1-validating-absolute-path = 3.1 禁用绝对路径后校验 "/absolute/path"
example-path-validator-3-2-the-relative-path-relative = 3.2 相对路径 "relative/path" 仍然可用
example-path-validator-4-symlink-detection-allow = 4. 符号链接检测（allow_symlinks）
example-path-validator-4-1-a-nonexistent-path-no = 4.1 不存在的路径（无符号链接，校验通过）
example-path-validator-4-2-the-allow-symlinks-true = 4.2 allow_symlinks = true 配置
example-path-validator-valid-allow-symlinks-true-does = valid = { $valid }（allow_symlinks=true 不拒绝）
example-path-validator-5-base-dir-constraint = 5. base_dir 约束
example-path-validator-5-1-a-path-inside-base-dir = 5.1 base_dir 内的路径（通过）
example-path-validator-5-2-a-path-outside-base-dir = 5.2 base_dir 外的路径（拒绝）
example-path-validator-temporary-directory-cleaned = 临时目录已清理
example-path-validator-6-the-sanitize-method = 6. sanitize 方法
example-path-validator-6-1-foo-bar-remove-parentdir = 6.1 "foo/../bar" → 移除 ParentDir
example-path-validator-sanitized = sanitized = { $value }
example-path-validator-6-2-foo-bar-remove-curdir = 6.2 "foo/./bar" → 移除 CurDir
example-path-validator-6-3-foo-bar-baz-multi-level = 6.3 "foo/../bar/../baz" → 多级移除
example-path-validator-6-3-1-etc-passwd-pure-traversal = 6.3.1 "../../etc/passwd" → 纯穿越路径被拒绝
example-path-validator-6-4-the-validate-and-sanitize = 6.4 validate_and_sanitize() 组合用法
example-path-validator-valid-sanitized = valid = { $valid }, sanitized = { $sanitized_path }
example-path-validator-7-validationresult-constructors = 7. ValidationResult 构造函数
example-path-validator-7-1-validationresult-valid = 7.1 ValidationResult::valid()
example-path-validator-valid-error-sanitized = valid = { $valid }, error = { $error }, sanitized = { $sanitized_path }
example-path-validator-7-2-validationresult-invalid-msg = 7.2 ValidationResult::invalid(msg)
example-path-validator-7-3-validationresult-sanitized = 7.3 ValidationResult::sanitized(path)

# archive-format
example-archive-format-inklog-archive-format-examples = inklog 归档格式示例
example-archive-format-all-archive-format-examples = ✓ 归档格式示例全部演示完毕
example-archive-format-1-archiveformat-enum-variants = 1. ArchiveFormat 枚举变体
example-archive-format-archiveformat-json = ArchiveFormat::Json    = { $json }
example-archive-format-archiveformat-parquet = ArchiveFormat::Parquet = { $parquet }
example-archive-format-archiveformat-csv = ArchiveFormat::Csv     = { $csv }
example-archive-format-archiveformat-default = ArchiveFormat::default() = { $default }
example-archive-format-default-format-json-lines = 默认格式：{ $default }（JSON Lines）
example-archive-format-2-fromstr-string-parsing = 2. FromStr 字符串解析
example-archive-format-string-archiveformat-parsing = 字符串 → ArchiveFormat 解析：
example-archive-format-error = { $s } → 错误：{ $e }
example-archive-format-error-invalid-value = { $value } → 错误（非法值）：{ $value2 }
example-archive-format-3-display-trait-output = 3. Display trait 输出
example-archive-format-archiveformat-string = ArchiveFormat → 字符串：
example-archive-format-archiveformat-json-2 = ArchiveFormat::Json    → "{ $value }"
example-archive-format-archiveformat-parquet-2 = ArchiveFormat::Parquet → "{ $value }"
example-archive-format-archiveformat-csv-2 = ArchiveFormat::Csv     → "{ $value }"
example-archive-format-4-archive-configuration-in = 4. DatabaseSinkConfig 中的归档配置
example-archive-format-json-archive-default = JSON 归档（默认）：
example-archive-format-archive-format = archive_format = { $archive_format }
example-archive-format-output-jsonl-files-one-json = 输出：.jsonl 文件（每行一个 JSON 对象）
example-archive-format-parquet-archive = Parquet 归档：
example-archive-format-output-parquet-files-columnar = 输出：.parquet 文件（列式存储，高压缩比）
example-archive-format-csv-archive = CSV 归档：
example-archive-format-output-csv-files-comma = 输出：.csv 文件（逗号分隔，Excel 兼容）
example-archive-format-5-archive-format-comparison = 5. 归档格式对比
example-archive-format-6-toml-configuration-file-format = 6. TOML 配置文件格式
example-archive-format-parquet-archive-with-the = Parquet 归档（含 Parquet 配置）：
example-archive-format-7-how-to-choose-an-archive = 7. 如何选择归档格式
example-archive-format-when-to-choose-json = 何时选择 Json：
example-archive-format-you-need-human-readable-archive = - 需要人类可读的归档文件
example-archive-format-you-query-them-with-grep-jq = - 用 grep/jq 直接查询
example-archive-format-a-log-shipper-consumes-them = - 由日志采集器消费（Fluentd/Logstash）
example-archive-format-fewer-than-1m-log-records-per = - 每天日志记录少于 100 万条
example-archive-format-when-to-choose-parquet = 何时选择 Parquet：
example-archive-format-a-big-data-analytics-platform = - 用于大数据分析平台（Spark/DuckDB/BigQuery）
example-archive-format-you-need-a-high-ratio-to-save = - 需要高压缩比以节省存储成本
example-archive-format-you-run-columnar-queries = - 需要列式查询（按字段聚合/过滤）
example-archive-format-more-than-1m-log-records-per-day = - 每天日志记录超过 100 万条
example-archive-format-compliance-retention-long-term = - 合规留存（长期冷存储）
example-archive-format-when-to-choose-csv = 何时选择 Csv：
example-archive-format-you-import-it-into-excel-or-a = - 需要导入 Excel 或 BI 工具
example-archive-format-simple-reports-and-data-sharing = - 简单报表与数据共享
example-archive-format-integrating-with-legacy-systems = - 与遗留系统集成（CSV 解析器支持广泛）
example-archive-format-performance-characteristics = 性能特征：
example-archive-format-json-fast-writes-large-files = - Json：写入快、文件大、查询慢
example-archive-format-parquet-slow-writes-cpu = - Parquet：写入慢（CPU 密集）、文件小、查询快
example-archive-format-csv-fast-writes-large-files-no = - Csv：写入快、文件大、不支持嵌套结构

# compression
example-compression-inklog-zstd-compression-examples = === inklog Zstd 压缩示例 ===
example-compression-all-compression-examples = ✓ 压缩示例全部演示完毕
example-compression-1-in-memory-zstd-compress = 1. 内存中 Zstd 压缩/解压
example-compression-1-1-zstdcompression-default = 1.1 ZstdCompression::default()（level=3）
example-compression-algorithm-name = 算法名称：{ $value }
example-compression-extension = 扩展名：  { $value }
example-compression-1-2-compress-a-string = 1.2 compress() 压缩字符串
example-compression-original-size-bytes = 原始大小：{ $value } 字节
example-compression-compressed-size-bytes = 压缩后大小：{ $value } 字节
example-compression-1-3-decompress-back-to-the = 1.3 decompress() 还原
example-compression-decompressed-size-bytes = 解压后大小：{ $value } 字节
example-compression-decompressed-data-matches-the = ✓ 解压数据与原始数据一致
example-compression-1-4-the-compress-data = 1.4 compress_data() 便捷函数

# console
example-console-example-1-basic-console-output = 示例 1：基础控制台输出（无颜色）
example-console-basic-console-output-complete = ✓ 基础控制台输出完成（无颜色）
example-console-example-2-colored-console-output = 示例 2：彩色控制台输出
example-console-colored-log-output = 彩色日志输出
example-console-colored-console-output-complete = ✓ 彩色控制台输出完成
example-console-example-3-stderr-routing = 示例 3：stderr 路由
example-console-configuration-error-and-warn = 配置：ERROR 与 WARN → stderr
example-console-info-debug-trace-stdout = INFO、DEBUG、TRACE → stdout
example-console-stderr-routing-complete-check = ✓ stderr 路由完成（请检查输出流）
example-console-inklog-console-sink-examples = === inklog console sink 示例 ===
example-console-all-examples-complete = 全部示例演示完毕！

# file
example-file-example-1-basic-file-writing = 示例 1：基础文件写入
example-file-log-file-path = 日志文件路径：{ $log_path }
example-file-write-logs = 写入日志
example-file-file-content = 文件内容
example-file-basic-file-writing-complete = ✓ 基础文件写入完成
example-file-example-2-file-rotation = 示例 2：文件轮转
example-file-write-logs-trigger-rotation = 写入日志（触发轮转）
example-file-rotated-file-list = 轮转后的文件列表
example-file-directory = 目录：{ $value }
example-file-file-list = 文件列表：
example-file-bytes = { $file_name }（{ $size } 字节）{ $marker }
example-file-rotation-count = 轮转次数：{ $rotation_count }
example-file-file-rotation-demo-complete = ✓ 文件轮转演示完成
example-file-example-3-zstd-compression = 示例 3：Zstd 压缩
example-file-write-logs-trigger-rotation-and = 写入日志（触发轮转与压缩）
example-file-wait-for-compression-to-finish = 等待压缩完成
example-file-compressed-file-list = 压缩后的文件列表
example-file-compressed-file-count = 压缩文件数：{ $compression_count }
example-file-zstd-compression-demo-complete = ✓ Zstd 压缩演示完成
example-file-clean-up-temporary-files = 清理临时文件
example-file-deleting = 正在删除：{ $file_name }
example-file-no-files-needed-cleaning = 没有需要清理的文件
example-file-deleted-files-in-total = 共删除 { $deleted_count } 个文件
example-file-inklog-file-sink-examples = === inklog file sink 示例 ===
example-file-all-examples-complete = 全部示例演示完毕！

# parquet-archive
example-parquet-archive-inklog-parquet-archive-examples = inklog Parquet 归档示例
example-parquet-archive-all-parquet-archive-examples = Parquet 归档示例已全部展示。
example-parquet-archive-example-1-parquetconfig-fields = 示例 1：ParquetConfig 字段
example-parquet-archive-default-parquetconfig = 默认 ParquetConfig：
example-parquet-archive-compression-level-zstandard-0-10 = compression_level  = { $compression_level }（Zstandard 0-10）
example-parquet-archive-encoding = encoding           = "{ $encoding }"
example-parquet-archive-max-row-group-size-rows-per-row = max_row_group_size = { $max_row_group_size }（每个 row group 的行数）
example-parquet-archive-max-page-size-bytes-per-page = max_page_size      = { $max_page_size }（每页字节数）
example-parquet-archive-field-semantics = 字段语义：
example-parquet-archive-compression-level-the-zstandard = compression_level：Zstandard 压缩级别
example-parquet-archive-0-none-3-balanced-recommended = 0 = 不压缩，3 = 均衡（推荐），10 = 最大（更慢）
example-parquet-archive-max-row-group-size-the-maximum = max_row_group_size：每个 row group 的最大行数
example-parquet-archive-a-smaller-value-helps-selective = 值越小越有利于选择性查询
example-parquet-archive-a-larger-value-gives-a-better = 值越大压缩比越好
example-parquet-archive-max-page-size-the-maximum-bytes = max_page_size：每页最大字节数；控制读取内存占用
example-parquet-archive-custom-configuration-high = 自定义配置（高压缩）：
example-parquet-archive-compression-level-maximum = compression_level  = { $compression_level }（最大压缩）
example-parquet-archive-max-row-group-size-larger-row = max_row_group_size = { $max_row_group_size }（更大的 row group）
example-parquet-archive-max-page-size-2mb = max_page_size      = { $max_page_size }（2MB）
example-parquet-archive-custom-configuration-low = 自定义配置（低延迟查询）：
example-parquet-archive-compression-level-fast = compression_level  = { $compression_level }（快）
example-parquet-archive-max-row-group-size-smaller-row = max_row_group_size = { $max_row_group_size }（更小的 row group）
example-parquet-archive-max-page-size-512kb = max_page_size      = { $max_page_size }（512KB）
example-parquet-archive-example-2-calling-convert-logs = 示例 2：实际调用 convert_logs_to_parquet()
example-parquet-archive-create-test-logrecord-values = 构造测试 LogRecord……
example-parquet-archive-created-logrecord-values = ✓ 已构造 { $value } 条 LogRecord
example-parquet-archive-call-convert-logs-to-parquet = 调用 convert_logs_to_parquet()……
example-parquet-archive-parquet-conversion-succeeded = ✓ Parquet 转换成功
example-parquet-archive-input-logrecord-values = 输入：{ $value } 条 LogRecord
example-parquet-archive-output-bytes-of-parquet-data = 输出：{ $value } 字节 Parquet 数据
example-parquet-archive-ratio-bytes-record = 比率：{ $value } 字节/条
example-parquet-archive-parquet-magic-expected-80-65-82 = Parquet 魔数：{ $magic }（期望：[80, 65, 82, 49] = "PAR1"）
example-parquet-archive-conversion-failed = ✗ 转换失败：{ $e }
example-parquet-archive-empty-record-test = 空记录测试：
example-parquet-archive-empty-record-conversion = ✓ 空记录转换成功：{ $value } 字节
example-parquet-archive-empty-record-conversion-failed = ✗ 空记录转换失败：{ $e }
example-parquet-archive-example-3-databasesinkconfig = 示例 3：DatabaseSinkConfig 的 Parquet 集成
example-parquet-archive-a-databasesinkconfig-with = 启用 Parquet 归档的 DatabaseSinkConfig：
example-parquet-archive-enabled = enabled          = { $enabled }
example-parquet-archive-driver = driver           = { $driver }
example-parquet-archive-url = url              = "{ $url }"
example-parquet-archive-table-name = table_name       = "{ $table_name }"
example-parquet-archive-batch-size = batch_size       = { $batch_size }
example-parquet-archive-flush-interval-ms = flush_interval_ms= { $flush_interval_ms }
example-parquet-archive-archive-format-enables-the = archive_format   = "{ $archive_format }" ← 启用 Parquet 归档
example-parquet-archive-parquet-config-compression-level = parquet_config.compression_level  = { $compression_level }
example-parquet-archive-parquet-config-max-row-group = parquet_config.max_row_group_size = { $max_row_group_size }
example-parquet-archive-the-corresponding-toml = 对应的 TOML 配置：
example-parquet-archive-example-4-the-parquet-archive = 示例 4：Parquet 归档工作流
example-parquet-archive-the-parquet-archive-workflow = Parquet 归档工作流：
example-parquet-archive-1-logs-are-written-into-the = 1. 日志写入数据库
example-parquet-archive-logrecord-databasesink-insert = → LogRecord → DatabaseSink → INSERT INTO logs
example-parquet-archive-2-periodically-export-to = 2. 定期导出为 Parquet（后台执行）
example-parquet-archive-flush-interval-ms-triggers = flush_interval_ms 触发 → convert_logs_to_parquet()
example-parquet-archive-produce-the-parquet-byte-stream = → 生成 Parquet 字节流
example-parquet-archive-3-parquet-file-storage = 3. Parquet 文件存储
example-parquet-archive-write-the-local-file-tmp-inklog = → 写入本地文件 /tmp/inklog_archive_20260101.parquet
example-parquet-archive-or-upload-to-object-cold = → 或上传到对象/冷存储（集成方式由使用方决定）
example-parquet-archive-4-analytical-queries-external = 4. 分析查询（外部工具）
example-parquet-archive-query-parquet-with-duckdb = # 用 DuckDB 查询 Parquet
example-parquet-archive-duckdb-c-select-level-count = duckdb -c "SELECT level, COUNT(*) FROM 'archive.parquet' GROUP BY level"
example-parquet-archive-use-apache-arrow-from-python = # 在 Python 中使用 Apache Arrow
example-parquet-archive-import-pyarrow-parquet-as-pq = import pyarrow.parquet as pq
example-parquet-archive-table-pq-read-table-archive = table = pq.read_table('archive.parquet')
example-parquet-archive-use-cases = 适用场景：
example-parquet-archive-analytics-load-into-snowflake = - 分析：导入 Snowflake/BigQuery/Redshift
example-parquet-archive-archive-object-storage-cold = - 归档：对象存储冷层（集成方式由使用方决定）
example-parquet-archive-compliance-efficient-storage = - 合规：满足监管留存的低成本存储
example-parquet-archive-performance-characteristics = 性能特征：
example-parquet-archive-asynchronous-background-export = - 异步后台导出，不影响主日志路径
example-parquet-archive-columnar-storage-with-a-high = - 列式存储，压缩比高（通常 5-10 倍）
example-parquet-archive-predicate-pushdown-keeps = - 谓词下推保证查询高效

# partition-strategy
example-partition-strategy-inklog-database-partition = inklog 数据库分区策略示例
example-partition-strategy-all-partition-strategy-examples = 分区策略示例已全部展示。
example-partition-strategy-example-1-partitionstrategy = 示例 1：PartitionStrategy 枚举取值
example-partition-strategy-partitionstrategy-supports-two = PartitionStrategy 支持两种分区策略：
example-partition-strategy-monthly-default = Monthly（默认）：{ $monthly }
example-partition-strategy-yearly = Yearly：        { $yearly }
example-partition-strategy-default = Default：       { $default }
example-partition-strategy-string-parsing = 字符串解析：
example-partition-strategy-monthly = "monthly" → { $from_str_monthly }
example-partition-strategy-yearly-2 = "yearly"  → { $from_str_yearly }
example-partition-strategy-month-alias = "month"   → { $from_str_month }（别名）
example-partition-strategy-year-alias = "year"    → { $from_str_year }（别名）
example-partition-strategy-invalid-value-parsing = 非法值解析：
example-partition-strategy-daily-returns-an-error = "daily" → { $value }（返回错误）
example-partition-strategy-example-2-partition = 示例 2：DatabaseSinkConfig 中的分区配置
example-partition-strategy-monthly-partition-configuration = 按月分区配置：
example-partition-strategy-driver = driver    = { $driver }
example-partition-strategy-partition = partition = { $partition }
example-partition-strategy-table-name-pattern-logs-2026-01 = 表名模式 = logs_2026_01、logs_2026_02、……
example-partition-strategy-yearly-partition-configuration = 按年分区配置：
example-partition-strategy-table-name-pattern-logs-2026 = 表名模式 = logs_2026、logs_2027、……
example-partition-strategy-example-3-partition-strategy = 示例 3：分区策略对比
example-partition-strategy-default-strategy = 默认策略：{ $value }
example-partition-strategy-example-4-toml-configuration = 示例 4：TOML 配置文件格式
example-partition-strategy-monthly-partition-default = 按月分区（默认）：
example-partition-strategy-yearly-partition = 按年分区：
example-partition-strategy-note-when-the-partition-field = 注意：partition 字段省略时默认为 "monthly"。
example-partition-strategy-example-5-how-to-choose-a = 示例 5：如何选择分区策略
example-partition-strategy-when-to-choose-monthly = 何时选择 Monthly：
example-partition-strategy-more-than-100k-log-records-per = - 每天日志记录超过 10 万条
example-partition-strategy-you-archive-or-purge-history-by = - 按月份归档或清理历史数据
example-partition-strategy-queries-are-usually-limited-to = - 查询通常限定在特定月份
example-partition-strategy-you-use-postgresql-partitioned = - 使用 PostgreSQL 分区表或 MySQL 分区
example-partition-strategy-when-to-choose-yearly = 何时选择 Yearly：
example-partition-strategy-fewer-than-10k-log-records-per = - 每天日志记录少于 1 万条
example-partition-strategy-retention-is-measured-in-years = - 留存以年为单位
example-partition-strategy-management-simplicity-comes = - 优先考虑运维简单
example-partition-strategy-you-do-not-purge-history = - 不常清理历史数据
example-partition-strategy-partitioned-table-management = 分区表管理建议：
example-partition-strategy-drop-expired-partitions = - 定期删除过期分区（DROP PARTITION）
example-partition-strategy-watch-per-partition-sizes-to = - 关注各分区大小，避免数据倾斜
example-partition-strategy-index-the-fields-that-queries = - 为查询命中的字段建索引
example-partition-strategy-archive-cold-data-as-parquet = - 冷数据归档为 Parquet（见 parquet_archive 示例）

# ring-buffered-file
example-ring-buffered-file-inklog-channelbufferedfilesink = === inklog ChannelBufferedFileSink 示例 ===
example-ring-buffered-file-all-channelbufferedfilesink = ✓ ChannelBufferedFileSink 示例全部演示完毕
example-ring-buffered-file-1-channelbufferedconfig-defaults = 1. ChannelBufferedConfig 默认值
example-ring-buffered-file-1-1-default-default = 1.1 Default::default()
example-ring-buffered-file-channel-capacity = channel_capacity    = { $channel_capacity }
example-ring-buffered-file-flush-batch-size = flush_batch_size    = { $flush_batch_size }
example-ring-buffered-file-flush-interval-ms = flush_interval_ms   = { $flush_interval_ms }
example-ring-buffered-file-backpressure-strategy = backpressure_strategy = { $backpressure_strategy }
example-ring-buffered-file-1-2-backpressurestrategy-enum = 1.2 BackpressureStrategy 枚举取值
example-ring-buffered-file-block = Block       = { $value }
example-ring-buffered-file-dropoldest = DropOldest  = { $value }
example-ring-buffered-file-dropnewest = DropNewest  = { $value }
example-ring-buffered-file-1-3-custom-configuration-small = 1.3 自定义配置（小容量 + DropNewest）
example-ring-buffered-file-2-block-backpressure-strategy = 2. Block 背压策略（通道满时阻塞）
example-ring-buffered-file-2-1-construct-a = 2.1 构造 ChannelBufferedFileSink
example-ring-buffered-file-sink-created-channel-capacity = sink 已创建，channel_capacity = { $value }
example-ring-buffered-file-2-2-write-20-log-records = 2.2 写入 20 条日志记录
example-ring-buffered-file-2-3-flush-shutdown-to-ensure = 2.3 flush + shutdown 确保数据落盘
example-ring-buffered-file-file-size-bytes = 文件大小：{ $value } 字节
example-ring-buffered-file-all-20-log-records-written-the = ✓ 20 条日志全部写入（Block 策略不丢数据）
example-ring-buffered-file-metrics-after-shutdown-dropped = shutdown 后指标：dropped_count = { $dropped_count }
example-ring-buffered-file-it-counts-incomplete-batches = IO 线程上未凑满的批次（recv_count < batch_size）也会计入，
example-ring-buffered-file-3-dropnewest-backpressure = 3. DropNewest 背压策略（满时丢弃最新）
example-ring-buffered-file-3-1-small-channel-dropnewest = 3.1 小容量通道 + DropNewest
example-ring-buffered-file-3-2-write-64-records-quickly = 3.2 快速写入 64 条记录（超出容量）
example-ring-buffered-file-metrics-dropped-count = metrics.dropped_count = { $dropped_count }
example-ring-buffered-file-dropnewest-drops-new-records = ✓ 通道满时 DropNewest 丢弃新记录，dropped_count > 0
example-ring-buffered-file-4-dropoldest-backpressure = 4. DropOldest 背压策略（满时逐出最旧）
example-ring-buffered-file-4-1-small-channel-dropoldest = 4.1 小容量通道 + DropOldest
example-ring-buffered-file-4-2-write-64-records-quickly = 4.2 快速写入 64 条记录（超出容量）
example-ring-buffered-file-dropoldest-evicts-old-records = ✓ 通道满时 DropOldest 逐出旧记录，dropped_count > 0
example-ring-buffered-file-5-channelbufferedmetrics = 5. ChannelBufferedMetrics 运行期统计
example-ring-buffered-file-5-1-construct-the-sink-channel = 5.1 构造 sink（channel_capacity=16，flush_interval=10ms）
example-ring-buffered-file-5-2-write-10-log-records = 5.2 写入 10 条日志记录
example-ring-buffered-file-5-3-wait-for-the-io-thread-and = 5.3 等待 IO 线程并读取指标
example-ring-buffered-file-channel-capacity-2 = channel_capacity = { $channel_capacity }
example-ring-buffered-file-channel-len = channel_len      = { $channel_len }
example-ring-buffered-file-bytes-written = bytes_written    = { $bytes_written }
example-ring-buffered-file-flush-count = flush_count      = { $flush_count }
example-ring-buffered-file-dropped-count = dropped_count    = { $dropped_count }
example-ring-buffered-file-the-file-contains-all-10 = ✓ 文件包含全部 10 条记录，指标正确

# rotation
example-rotation-inklog-log-rotation-examples = === inklog 日志轮转示例 ===
example-rotation-all-rotation-examples-completed = ✓ 轮转示例全部演示完毕
example-rotation-1-size-based-rotation = 1. 按大小轮转（SizeBasedRotation）
example-rotation-1-1-from-size-string-100mb = 1.1 from_size_string("100MB") 构造
example-rotation-max-size-bytes-mb = max_size = { $value } 字节（{ $value2 } MB）
example-rotation-1-2-file-size-within-the-limit = 1.2 文件大小未超限 → 不轮转
example-rotation-current-size-mb-limit-100-mb = current_size = { $value } MB，上限 = 100 MB
example-rotation-should-rotate = should_rotate = { $should_rotate }
example-rotation-1-3-file-size-over-the-limit = 1.3 文件大小超限 → 触发轮转
example-rotation-reason = reason        = { $reason }
example-rotation-2-time-based-rotation = 2. 按时间轮转（TimeBasedRotation）
example-rotation-2-1-the-four-built-in-intervals = 2.1 四种内置间隔
example-rotation-interval-secs-sec-hours = { $interval } interval_secs = { $value } 秒（{ $value2 } 小时）
example-rotation-2-2-an-unknown-interval-returns = 2.2 未知间隔返回错误
example-rotation-error-message = 错误信息：{ $err }
example-rotation-2-3-interval-not-reached-no = 2.3 未到间隔 → 不轮转（daily，已过 1 小时）
example-rotation-elapsed-sec-interval-sec = 已过 { $value } 秒，间隔 { $value2 } 秒
example-rotation-2-4-interval-reached-rotation = 2.4 已到间隔 → 触发轮转（daily，已过 24 小时）
example-rotation-3-rotated-file-naming-rules = 3. 轮转文件命名规则
example-rotation-3-1-generate-next-path-default = 3.1 generate_next_path 默认格式
example-rotation-base-path = 基础路径：{ $value }
example-rotation-next-path = 下一路径：{ $value }
example-rotation-3-2-different-sequence-values = 3.2 不同序号生成不同文件名
example-rotation-seq = seq={ $i } → { $p }
example-rotation-3-3-a-path-without-an-extension = 3.3 无扩展名路径回退为 .log
example-rotation-4-composite-rotation-strategy = 4. 组合轮转策略（CompositeRotation）
example-rotation-4-1-composite-size-time-neither = 4.1 组合 大小 + 时间，两个条件都不满足
example-rotation-strategy-name = 策略名称 = { $value }
example-rotation-size-100b-1kb-elapsed-1h-24h = size=100B(<1KB), elapsed=1h(<24h) → should_rotate={ $should_rotate }
example-rotation-4-2-the-size-condition-is-met = 4.2 满足大小条件 → 触发轮转
example-rotation-size-2048b-1kb-elapsed-1h-24h = size=2048B(>1KB), elapsed=1h(<24h) → should_rotate={ $should_rotate }
example-rotation-4-3-the-time-condition-is-met = 4.3 满足时间条件 → 触发轮转
example-rotation-size-100b-1kb-elapsed-24h-24h = size=100B(<1KB), elapsed=24h(>=24h) → should_rotate={ $should_rotate }
example-rotation-5-filesinkconfig-rotation-fields = 5. FileSinkConfig 轮转字段
example-rotation-5-1-default-configuration = 5.1 默认配置
example-rotation-max-size = max_size      = { $max_size }
example-rotation-rotation-time = rotation_time = { $rotation_time }
example-rotation-keep-files = keep_files    = { $keep_files }
example-rotation-compress = compress      = { $compress }
example-rotation-5-2-custom-rotation = 5.2 自定义轮转配置（小文件 + 每小时）
example-rotation-path = path          = { $value }
example-rotation-compression-level = compression_level = { $compression_level }
