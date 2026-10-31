# 更新日志

本项目的所有显著变更将记录在此文件中。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<details open>
<summary>📑 目录</summary>

- [Unreleased](#unreleased)
- [0.3.0-rc.11](#030-rc11---2026-10-20)
- [0.3.0-rc.10](#030-rc10---2026-10-19)
- [0.3.0-rc.9](#030-rc9---2026-10-10)
- [0.3.0-rc.8](#030-rc8---2026-10-08)
- [0.3.0-rc.7](#030-rc7---2026-10-07)
- [0.3.0-rc.6](#030-rc6---2026-10-06)
- [0.3.0-rc.5](#030-rc5---2026-09-21)
- [0.3.0-rc.4](#030-rc4---2026-09-14)
- [0.3.0-rc.2](#030-rc2---2026-09-03)
- [0.2.0](#020---2026-08-05)
- [0.1.12](#0112---2026-07-22)
- [0.1.11](#0111---2026-07-17)
- [0.1.7](#017---2026-07-13)
- [0.1.6](#016---2026-07-12)
- [0.1.5](#015---2026-07-11)
- [0.1.2](#012---2026-07-05)
- [0.1.1](#011---2026-06-29)
- [0.1.0](#010---2026-01-18)
- [0.0.0](#000---2025-12-30)

</details>

## [Unreleased]

## [0.3.0-rc.12] - 2026-11-01

### Added

- `oxcache-audit` feature：承接 oxcache 审计事件（实现 oxcache `AuditEventPublisher` 端口，事件 → `LogRecord` 写入注入的 `LogSink`），保留 fail-open 计数语义（`dropped_count` / `write_failure_count`）。

### Added

- **feature `oxcache-audit`**：新增 `inklog::integrations::OxcacheAuditPublisher`——oxcache `AuditEventPublisher` 端口的 inklog 实现（原 oxcache 侧 `InklogAuditPublisher` 迁入消费侧）。缓存审计事件经有界通道交由单一 writer task 写入注入的 sink；通道过载 / 无 runtime 上下文计入 `dropped_count()`，sink 写失败计入 `write_failure_count()`（fail-open 观测面，不回压缓存主路径）。级别映射：Set/Delete/Clear → INFO，Hit/Miss/Evict/Expired → DEBUG；消息文案为 inklog 侧字面量 `"cache audit event"`。启用方式：`features = ["oxcache-audit"]`。
- **限流自持（BREAKING）**：新增仓内令牌桶（`support/processing/token_bucket.rs`，容量 = 速率、每秒补充速率个令牌，语义等价 limiteron `TokenBucketLimiter::new(rate, rate)`），按速率（`RateLimiter`）与按 target 前缀分组（`TargetRateLimiter`）的限流模块**默认构建即生效**。决策点为同步 O(1)，不再经 `futures::executor::block_on` 桥接异步判定（原实现把同步阻塞留在 async 写路径上，每条日志阻塞一次）。sink 级限流扩展统一走自有 `SinkRateLimit` 端口（`RateLimitedSink::new`），不再暴露第三方限流 trait。
- **突发配额计数器 `BurstLimiter`**（`inklog::BurstLimiter`）：原子 CAS 递减的配额计数器，无时钟、无补充、无速率语义——等价 limiteron `TokenBucketLimiter::new(capacity, 0)`（`refill_rate = 0`）的纯预算行为，供默认限速路径使用。

### Changed

- 依赖地板对齐（registry 已发布版本）：`dbnexus` 0.6.0-rc.11、`trait-kit`/`confers` 0.6.0-rc.12 / 0.5.0-rc.12、`oxcache` 0.5.0-rc.12、`parking_lot` 0.13（与 dbnexus rc.11 拉入版对齐，避免双版本同图）；examples 的 `confers` 钉版同步 rc.12。
- 限流能力自持：新增仓内令牌桶（`support/processing/token_bucket.rs`，容量 = 速率、每秒补充速率个令牌），订阅者级限流（`performance.rate_limit` / `rate_limit.rules`）在默认构建下即生效，不再随 feature 关闭。决策点为同步 O(1)，不再经 `futures::executor::block_on` 桥接异步判定（原实现把同步阻塞留在 async 写路径上，每条日志阻塞一次）。
- 移除 `limiteron-rate-limit` feature 与对 limiteron 的依赖：inklog 对 limiteron 依赖归零，公开 API 不含任何第三方限流 trait；sink 级限流扩展统一走 inklog 自有的 `SinkRateLimit` 端口。

### Changed

- `LimiteronRateLimit` 与 `RateLimitedSink::with_limiter` 转为内部实现并移除：其参数为第三方 trait，留在公开 API 会让下游在 path 依赖开发模式下遭遇 trait 身份分裂（同一 crate 两个实例）。外部扩展请实现 inklog 自有的 `SinkRateLimit` 端口（`RateLimitedSink::new` 注入）。
- 移除 `limiteron-rate-limit` feature：限流能力不再有「未编译即静默失效」的路径，原先依赖该 feature 的告警随之删除（该告警本身用 `tracing::warn!`，而此刻全局 subscriber 尚未安装，事件必被丢弃）。
- 移除对 limiteron 的 `retry` / `sync` / `test-clock` feature 引用（inklog 只用其限流器类型；retry 公式已自持，另两者为空特性）。
- 跨仓自研依赖改为 `path` + `version` 双重声明：本地恒走工作树，发布时仍按 version 解析 registry。
### Changed

- **依赖（BREAKING）**：`limiteron` 依赖**完全移除**（`cargo tree -p inklog -e normal` 中 limiteron 消失，inklog 依赖树归零于外部限流库）。迁移要点：
  - 默认限速路径的语义**不变**——sink 级突发配额限速由仓内 `BurstLimiter` 承担，初始放行 `capacity` 次、其后拒绝、无时间补充；
  - 按速率 / 按 target 分组限流（`RateLimiter` / `TargetRateLimiter` / `GlobalRateLimitMiddleware` / `TargetQuotaMiddleware` / `LoggerSubscriber::with_rate_limiter` / `with_target_rate_limiter`）**默认构建即生效**（内核为仓内令牌桶）——原先随 `limiteron-rate-limit` feature 提供的 limiteron 原生限流器注入路径（`RateLimitedSink::with_limiter`）已移除，改为实现自有 `SinkRateLimit` 端口注入——需要限流的用户请显式启用该 feature；未启用时若配置了 `performance.rate_limit` 或 `rate_limit.rules`，启动会打印显性告警（不静默忽略）；
  - sink worker 池的重试次数与退避数值改为 inklog 侧常量（数值与语义不变：`max_retries = 2`、退避 1s/2s/4s/8s/16s、封顶 30s）。
- **依赖地板**：`dbnexus` 由精确钉 `=0.6.0-rc.10` 放宽为 caret `0.6.0-rc.10`——精确钉是底层推进迫使顶层重发、以及下游双版本同图的唯一来源；地板仍是「已承诺兼容的最低版本」承诺面，允许滞后于实际可用版本。
- **跨仓地板基线**：7 仓（trait-kit / confers / dbnexus / inklog / oxcache / limiteron / sdforge）跨仓自研依赖声明共 26 条，机器扫描断言「版本串不得以 `=` 开头（精确钉）」且「地板必须带预发布前缀（`-rc` / `-alpha` / `-beta` / `-pre`）」——当前基线 **0 处违规**（同仓 path 依赖内联钉豁免）。本次仅固化断言与基线记录，未改动任何依赖声明。

### Removed

- **安全文档**：移除 `docs/SECURITY.md` 的「安全赏金计划」章节（含奖励金额表与排除范围清单）——当前不再设任何安全赏金计划。

### Security

- **错误文本出口全部脱敏**：`InklogError` 的 `Display`（`to_string()` / `format!("{err}")`）与 `Debug`（`{:?}`）此前原样内插细节，路径、DSN、SQL 片段与凭据会随 `?` 传播外带；`localized_message()` 同样未脱敏。现四个出口（`Display` / `Debug` / `localized_message()` / `safe_message()`）共用同一套脱敏规则，`Debug` 为手工实现（变体名保留，细节脱敏，`source` 只报存在性）。
- **连接池 DSN 不再进错误消息**：`driver_from_config` 判定驱动失败时回显的连接池 URL 改为遮蔽口令后的形态（`postgres://user:***@host/db`）。
- **归档链截尾可检出**：`verify_entries` 只做链内自洽校验，manifest 被截尾到非空前缀时仍自洽（判为完整、CLI 退出码 0）。新增链外锚点——`<stem>.chain.seal.json` 封条（条目总数 + 尾哈希 + 封条 HMAC，以链密钥签发、不可伪造），随 manifest 单独落盘；`inklog-cli verify-chain` 追加封条比对，**封条缺失或与 manifest 不符即判失败**（退出码 2）。
- **HTTP 暴露面守卫覆盖库入口**：`validate_http_exposure()` 此前只在 `InklogConfig::validate()` 与 `load_with_env_overrides()` 内生效，`LoggerManager::with_config` / `LoggerBuilder::build` 这类推荐入口不经 `validate()` 即可构建出「非回环绑定 + 无鉴权/白名单」的监控端点。守卫现落在所有构建入口的公共路径上（`build_detached_full`），配置期即拒绝。
- **出站 TLS 证书校验降级受编译期门控**：`danger_accept_invalid_certs`（跳过证书链与主机名校验）此前是无门控的公开开关，仅发一条 `tracing::warn!`——而告警在 sink 构造期发出，此刻全局 subscriber 尚未安装，事件必被丢弃。现新增 `insecure-tls` feature：仅调试构建（`debug_assertions`）或显式启用该 feature 时可用，发布构建无授权即构造失败（fail-closed）；放行时的告警改走标准错误，保证送达。
- **`LogRecord` 调试视图脱敏**：`Debug` 此前原样打印 message、全部字段值与来源路径。现手工实现：message 过 sanitizer（消息级敏感值）再过掩码器（PII 与敏感键字段值），来源路径同口径脱敏。`Serialize` 保持原文——落盘前的脱敏是 sink 的职责（`masking_enabled` / `secret_scan_enabled` 开关的语义面），在序列化层改写会静默改变使用方显式配置过的落盘数据。
- **脱敏标记判定收窄**：`value_is_redaction_marker` 此前允许标记后缀为任意大写字母（`***REDACTEDSECRET` 这类「标记 + 真实值」拼接被判为已脱敏而原样保留）。判定改为对 crate 实际产出的标记形态**整值精确匹配**；secret-scan 内建模式的替换串由单测兜底防词表漂移。
- **`LogSanitizer` 捕获组展开回归修复**：自定义敏感模式的替换串改用闭包替换时丢失了 `$1` / `${name}` 展开（`$1` 原样输出）。恢复为 `Captures::expand`，与 regex `&str` 替换器同语义。
- **驱动判定收敛为单一来源**：`DbNexusAdapter` 此前有两套语义不同的检测器——`with_full_config` / `ensure_table_exists` 走 `detect_driver_from_url`（大小写敏感，未知 scheme **回退 PostgreSQL**），`from_pool` / `from_connection_pool` 走 `driver_from_pool_url`（大小写敏感，判定失败即中止）。后果：`MYSQL://…` 这类 dbnexus 认可的有效 DSN 在主构造路径被判成 PostgreSQL（建表用 PG 方言、转义用 PG 语义——MySQL 后端下 `\'` 可闭合字符串字面量），`SQLite://…` / `DUCKDB://…` 同理。现四个构造器统一取自 dbnexus 的 `DbConfig::database_type()`（与 dbnexus 建池选后端同源，scheme 大小写不敏感），建表方言与写入转义同源于该驱动；未知 scheme 或不受支持的后端（Ladybug/Neo4j）构造直接失败，不再回退。
- **env 覆盖不再吞掉 DSN 口令**：`load_with_env_overrides()` 的 env 覆盖走「序列化→打补丁→反序列化」往返，而 `DatabaseSinkConfig` 的 `Serialize` 视图把口令遮蔽为 `***`——往返后配置文件里的口令变成占位符，连接必然认证失败。该路径现保留原 DSN（`INKLOG_DATABASE_SINK_URL` 提供的 DSN 仍按 env 原值生效）。
- **DSN 脱敏覆盖 query/keyword 形态**：口令此前只在 userinfo 形态被遮蔽；`?password=…`（sqlx 按 URL 解析 query 参数，该形态有效）与 keyword/value 形态（`host=… password=…`）的口令原样进入 `Debug` / `Serialize` 输出。现按凭据字段名（password/pwd/pass/token/secret/api_key/apikey/auth/key/sig/signature，大小写不敏感）遮蔽 query、fragment 与 keyword/value 形态的取值；authority 边界同时识别 `?` 与 `#`（此前只识别 `/`，query 里的 `@` 会把 host 一起吞掉）；口令含未编码 `/` 或形态不可判定时整体遮蔽，不回显疑似口令原文。
- **脱敏视图不可用于认证**：口令位是占位符（`***`）的 DSN 认证必然失败。`DatabaseSinkConfig::validate()` 与配置加载校验现显式拒绝该形态，`INKLOG_DATABASE_SINK_URL` 传入占位符口令时按非法值忽略并告警。

### Added

- **feature `insecure-tls`**：允许在发布构建中使用 `danger_accept_invalid_certs`（跳过 TLS 证书校验）。默认关闭——跳过证书校验后链路只加密不认证，中间人可完整读取并篡改外发日志，客户端侧无任何报错或指标信号。调试构建（`debug_assertions`）无需本特性即可使用该开关。
- **`DatabaseSinkConfig::to_lossless_json()` / `to_lossless_toml()`**：`Serialize` 输出的是脱敏视图（DSN 口令遮蔽为 `***`），序列化→反序列化往返会丢口令。两个新方法输出**不脱敏**的完整配置，供需要保真往返的场景（备份、迁移、跨环境搬运）使用——导出结果含原始口令，须按凭据同等对待。

### Fixed

- **HTTP 绑定地址判定与运行期绑定同源**：`HttpServerConfig::is_loopback_host()` 与 HTTP 服务器的实际绑定此前各按一套规则解析 `host`——判定侧接受主机名（`localhost`）与过度剥离的方括号（`[[::1]]`、`[::1`），运行期只接受 socket 地址字面量。结果是守卫放行一个起不来的端点（如 `host = "localhost"` 判为回环，运行期 `localhost:9090` 解析失败），也判错非回环绑定的暴露面。现抽出共享的地址解析（`config::http::bind_socket_addr`），判定侧与运行期绑定共用：只接受 IPv4 明文（`127.0.0.1`）与 IPv6 带方括号（`[::1]` / `[::]`）的 IP 字面量，主机名与畸形方括号写法判为不可绑定。配置校验新增可绑定性检查，`host` 不可绑定时在配置期即报错（此前要等启动失败才暴露）。
- **放行全部地址的白名单不再算访问控制**：`ip_whitelist` 含前缀长度为 0 的 CIDR（`0.0.0.0/0`）时，该条目对所有来源放行，与不加白名单无异——此前被计入访问控制，暴露面守卫因此放行实际裸露的端点。现此类白名单不构成访问控制。

## [0.3.0-rc.11] - 2026-10-20

### Changed

- **依赖**：`oxcache` 从 `=0.5.0-rc.8` 放宽为 `^0.5.0-rc.8`——vecboost 线要求 rc.10，精确 pin 与其区间无交集导致下游（fluxgate）解析冲突；rc.8~rc.10 已全量实测兼容（2162 项测试）。跨仓申报：oxcache 反向桥接（`inklog` feature）下次发布需同步将 `=0.3.0-rc.10` 升至 `=0.3.0-rc.11`。

### Fixed

- **文档**：还原 8de6cec 批量注释精简误删的 doc 示例空参数括号（347 行），修复 22 个 doctest 编译失败；`config_schema.json` 随 doc 文本回归一致（无需变更）。

## [0.3.0-rc.10] - 2026-10-19

### Changed

- **发布依赖传播**：自研上游钉升到刚发布版本——`dbnexus` `=0.6.0-rc.9` → `=0.6.0-rc.10`（精确钉）、`trait-kit` rc.9 → rc.10、`confers` rc.10 → rc.11；`oxcache` 保持 `=0.5.0-rc.8`、`limiteron` 保持 `0.3.0-rc.6`（反向可选边，断环约定）；examples `confers` 钉同步 rc.11

### Internal

- **CI 发布**：publish 凭证切换 crates.io trusted publishing（OIDC auth-action）+ 发布幂等三元组
- **安全/仓库卫生**：gitleaks 降噪配置与误报指纹；gitignore 忽略设计文档；README 锚点修复

---

## [0.3.0-rc.9] - 2026-10-10

### Fixed

- **降级 sink 的 ops 事件自激循环**：file/db 通道工厂持续失败进入降级模式后，`handle_sink_unavailable` 对每条到达记录广播 `sink_degraded` ops 事件，而该事件经 ops hub 回流到同一 sink 通道 → 再次降级 → 再广播，形成自激循环（实测无用户日志时 `sink_errors`/`logs_dropped` 以约 2k/s 无界增长并持续刷 error sink）。现每轮降级只广播一次并跳过 ops 事件自身的回流，sink 重建（工厂重试/手动恢复/自动恢复）后复位。
- **自定义 sink worker 写失败通知的自激放大**：失败路径逐次 `tracing::error!`，事件经 subscriber 回流到同一批 sink 通道，失败的 sink 收到自身通知后再失败，3 次尝试各自再产生通知——单条记录被放大到数十次尝试且持续增长。现改为直写 console sink（与内置 worker 的 error sink 直写路径同款隔离），尝试次数稳定为 3。
- **DatabaseSink 配置未透传**：manager 的 db sink 工厂以 `DatabaseSink::new(db)` 构造，`config.database_sink` 的 `secret_scan_enabled`/`batch_size` 被静默忽略；现经 `new_with_config` 透传，`secret_scan_enabled = true` 的落库前出站脱敏门对 sk-/AKIA 等裸 secret 形态生效。
- **CLI 错误输出丢失底层明细**：密钥格式非法时只打印外层 context（`Failed to get encryption key from env var: <VAR>`），`base64_wrong_length`/`password_too_short`/`key_wrong_length` 的长度诊断被吞。现沿 anyhow 因果链逐层本地化展开，且 decrypt 保留 `InklogError` 类型透传；文本与 `--json` 两种输出都给出长度信息。
- **基准 logger helper 的 `None` 配置误启用 sink**：`benches/inklog_bench.rs` 的 `setup_noop_logger`/`setup_console_logger`/`setup_file_logger` 用 `file_sink: None` / `console_sink: None` 表达"不启用"，但 manager 语义下 `None` 均表示默认启用——"noop" 基准实际向 stdout 洪泛输出（实测全量约 145 万行）并启用默认路径 `logs/app.log` 的 file 通道（stdout 洪泛阻塞 console worker，每次采样 teardown 排水巨量积压），`noop_sink` 与 `console_sink_latency` 默认参数下各耗时 10 分 42 秒，合计占 `inklog_bench` 全量的 74%，并让 console 基准中位值被 file 通道背压主导（3.51 ms）。现三个 helper 显式 `enabled: false` 关闭非被测 sink：全量默认参数由 28 分 42 秒降到 9 分 01 秒（同机实测），console 中位值回到 4.30 µs（本征路径）；口径与数字修正见 `docs/PERFORMANCE.md` 复现章节续记。

### Changed

- **feature 拆分 `cache`（default 含）**：oxcache 的 icu/fluent-bundle 依赖非可选，此前 `default-features = false` 仍会经 oxcache 引入 icu/fluent，与「关闭 i18n 即裁剪 icu/fluent 依赖树」的文档承诺漂移。现 oxcache 改为可选依赖并新增 `cache` feature（`default = ["i18n", "cache"]`）：`default-features = false` 同时关闭两者，真正裁掉 icu/fluent；`ObjectPool`/`OxCacheAdapter` 随 `cache` 门控，default 组合 API 面不变，无缓存组合下容器回退空缓存。
- **三方依赖升到最新稳定版**：`opentelemetry` 0.30 → 0.33（`otel` feature 的宿主对齐版本随之升到 0.33，配套 `tracing-opentelemetry` 0.34；对启用 `otel` 的宿主是破坏性对齐，见 README feature 表）；examples 的 `uuid` 1.26 → 1.28；锁文件另有 20 个三方包刷新到最新稳定版（`toml` 1.1.6 → 1.1.8、`zeroize` 1.9.0 → 1.9.1、`zstd` 0.14.0 → 0.14.1、`hyper` 1.11.1 → 1.12.0、`h2` 0.4.19 → 0.4.20、`syn` 3.0.6 → 3.0.7 等）。自研依赖（dbnexus/confers/trait-kit/oxcache）版本号不变，留给发布链统一定值。
- **依赖图裁剪**：`tokio` 去掉 `full`，按实际用量显式声明 `rt-multi-thread`/`macros`/`sync`/`time`/`fs`/`io-util`/`net`/`signal`（io-std/process/parking_lot 路径不再进入产物，tokio 对 parking_lot 的依赖随之消失）；`rayon`、`futures` 在 src/ 零引用、仅测试与基准使用，从 `[dependencies]` 移入 `[dev-dependencies]`，不再进入库消费者依赖图；`ctor` 关闭隐式默认特性并显式声明 `std`/`proc_macro`（未使用的 `priority` 一并关闭，`link-section` 退出依赖图）；全部三方依赖统一 `default-features = false` + 显式特性声明。
- **CI actions 升到最新**：`taiki-e/install-action` v2.87.27、`codecov/codecov-action` v7.1.1、`actions/upload-artifact` v7.0.2、`github/codeql-action` v4.38.3、`docker/setup-compose-action` v2.4.0；`actions/checkout` v7.0.1、`Swatinem/rust-cache` v2.9.2、`actions/cache` v6.1.0、`softprops/action-gh-release` v3.0.3 经解析提交 SHA 复核已是最新，注释精确到语义化版本。

## [0.3.0-rc.8] - 2026-10-08

### Removed

- **生产 API 收缩：删除测试专用公共函数**：`support::io::sink::ring_buffered_file` 公开链上的 `decrypt_file` 与 observability 面的 `reset_ops_hub_for_tests` 等仅测试使用的 `pub` 函数从生产代码移除——公共 API 面收缩，外部消费者如引用需迁移至既有入口或 `test-utils` feature。

### Changed

- **复杂热点收敛与死代码清理**：敏感键/采样/模板热点抽取辅助函数，query 解码过滤路径抽公共助手，压缩守卫按特性隔离；测试面死委托删除并以特性隔离替代 `allow` 标记。
- **治理与注释卫生**：specmark 目录与治理复核记录移出 git 跟踪；注释清理（变更史叙述与任务编号残留）；漏洞报告渠道统一为 GH 安全通告首选、邮箱备选。

## [0.3.0-rc.7] - 2026-10-07

### Fixed

- **非 unix 平台在零压缩特性下无法编译（`E0433`）**：`support/io/sink/compression.rs` 的 `create_compressed_output` 函数体不受 `zstd`/`gzip` 门控（仅 `allow(dead_code)`），而其 `#[cfg(not(unix))]` 分支使用的裸 `File` 来自 `#[cfg(any(feature = "zstd", feature = "gzip"))]` 门控导入——因此以 `default-features = false` 依赖 inklog 的 Windows 消费者（limiteron 0.3.0-rc.6 起）会直接编译失败。改为全限定 `std::fs::File::create`，并在注释中记录该 cfg 组合成因。

### Changed

- **CI 构建腿补零特性检查**：`build` job 增加 `cargo check --workspace --no-default-features`，使 windows/macos 腿也能覆盖"未启用压缩特性"的编译组合（原仅 ubuntu 的 check job 跑零特性，而平台分支错误只有非 unix 才触发）。

## [0.3.0-rc.6] - 2026-10-06

### Added

- **全出口 PII 掩码覆盖**：net 转发、OTLP 导出、`ChannelBufferedFileSink` 三个出口此前绕过 DataMasker（仅上游弱版 LogSanitizer 兜底），现统一接入 21 条规则掩码（`masking_enabled` 门控，默认开启）；DB sink 改键值遍历式掩码——`password` 等敏感键整值替换，不再依赖值正则的偶然命中
- **Sink masker 注入点**：console/file/net/otlp/ChannelBufferedFileSink 新增 `with_masker()`，支持经 `DataMasker::builder()` 注入自定义规则（`fast-masking` feature 下 literal 规则自动走 Aho-Corasick 加速路径，接线从死代码转为可用）
- **脱敏正则修复**：phone/id_card/bank_card/ssn 等数字类规则以负向断言替代 `\b`（CJK 汉字紧贴数字不再漏报）；身份证支持小写 `x` 校验位；bank_card 收紧为 16–19 位（13 位毫秒时间戳不再误伤）；passport 排除混合 hex 形态（git 短 SHA 不再误掩）；脱敏递归深度 16 层上限（深嵌套 JSON 防栈溢出）、单条输入 1 MiB 上限；JSON Number/Bool 形态的敏感值参与掩码
- **ERROR/FATAL 兜底运行期补发**：async 通道打满时进入兜底缓冲的关键日志，在通道恢复半满时立即补发，并有 60s 周期任务兜底——不再依赖进程退出时的 Drop drain
- **崩溃一致性**：`FileSinkConfig.fsync`（默认 false）每批写盘后 `sync_all`；空闲期批量缓冲由轮转定时线程代为落盘（不再依赖"下一条写入"触发）
- **文件权限加固**：FileSink 活动日志、轮转加密产物（`.enc`）、压缩产物（`.zst`/`.gz`）以 0600 创建；新建日志目录 0700（已存在目录不回改）
- **留存合规**：年龄清理与大小清理拆为独立阶段（`max_total_size` 不可解析时过期清理仍执行）；`encrypt=true` 且 `retention_days < 180` 输出等保 2.0 留存提示
- **审计链接入轮转管线**：`audit_chain_enabled = true` 后每次轮转向 `<stem>.chain.jsonl` 追加 `{path, sha256, timestamp}` 条目；新增 `inklog-cli verify-chain` 子命令（链完整退出码 0、篡改退出码 2，密钥读 `INKLOG_AUDIT_KEY`）
- **log 门面补齐**：桥接当前 tracing span 的 trace_id/span_id（与 tracing 路径共享派生逻辑）；per-target 级别过滤（共享 manager 指令集，`RUST_LOG`/`target_levels` 对 log 来源生效）
- **OTLP 导出补链路字段**：编码输出 `traceId`/`spanId`/`severityNumber`（标准映射 DEBUG=5/INFO=9/WARN=13/ERROR=17/FATAL=21）；https endpoint 经 rustls 传输（`net-sink` feature，CA PEM 或显式跳过校验二选一；feature 未启用时构造期报错）
- **net 重连指数退避**：connect 失败按 `min(2^n × 100ms, 30s)` 退避，窗口内 write 直接入缓冲不再发起连接，成功清零
- **解压炸弹防护**：查询与内存解压路径施加 1 GiB 输出上限（流式读取，超限报 i18n 错误）
- **内部故障 ops 事件**：轮转 rename 失败、轮转产物压缩/加密失败、sink 降级与恢复路径自动发布 `sink_degraded`/`sink_recovered` 事件（全局 hub 复用 manager ops 通道）
- **服务身份静态字段注入**：`GlobalConfig` 新增 `service_name`/`service_instance`/`service_env`/`service_version` 与 `static_fields`（附加键值）；配置后由 subscriber 在记录入通道前注入每条日志的 `fields`（键名同名，全部出口一致，事件显式同名字段优先），未配置时热路径零开销；支持 TOML（`[global]`）、环境变量（`INKLOG_GLOBAL_SERVICE_*`、`INKLOG_GLOBAL_STATIC_FIELDS=k=v,...`，沿用既有前缀语义）与 confers/DI 键路径（`global.service_*`）；三条配置链统一身份校验：TOML 由 `InklogConfig::validate` 硬拒绝、env 覆盖点对空白值/控制字符告警忽略、DI 装配链拒绝构建；adapter 键路径对未设置的身份字段透传 None（不产生空串注入）
- **deferred-capabilities 四项遗留能力**：中文姓名掩码（`NAME_FIELD_PATTERNS` 姓名族键词 + 2-4 汉字值形态，保留首字脱敏）；磁盘持久化 fallback 队列 `FallbackJournal`（JSONL 10MiB 上限丢最旧、启动重放防循环、`global.fallback_journal` 默认关）；ChannelBufferedFileSink 转正为主 file sink 路径（简单配置走 CBFS，高级配置回落 FileSink，USER_GUIDE 双路径能力矩阵）；真 OTel 链路上下文（新 `otel` feature，opentelemetry 0.30，span extensions → W3C traceparent → 根 span 派生三级提取，版本未对齐静默回退）
- **masking API 扩展**：detect-only 检测面（`MaskMatch`/`detect`/`has_match` 与 pattern 访问器）；`MaskRuleRegistry` 全量快照 `rules()` 与构造器 `from_rules()`；行级 KV 脱敏 `DataMasker::mask_kv_lines`
- **confers 审计桥（`confers-audit` feature）**：实现 confers `AuditSink` 端口（已发布 confers 0.6.0-rc.5 起）——`ConfersAuditSink` 把 confers 配置审计事件（KeyAccess/KeyRotation/Decrypt/LoadSuccess/ReloadTrigger）批量转换为结构化 `LogRecord`（`target = "confers::audit"`、message 为事件 JSON、变体载荷平铺进 fields）写入注入的 inklog `LogSink`，与 console/file/db sink 走同一落盘管道；同步端口 → 异步 sink 经 mpsc + 惰性 writer task 桥接，通道满/无 runtime 上下文按丢弃计数绝不阻塞 confers 审计路径，`accepted`/`dropped`/`write_failures` 计数可观测；级别映射循 confers `AuditLevel` 分类（Durable → WARN，BestEffort → INFO）；confers 为既有 optional 依赖新增 `confers/audit` feature 联动（默认 feature 零依赖零行为变化），示例 `confers_audit` + 7 项单测（批量落盘/变体平铺/级别映射/时间戳透传/失败计数/无 runtime 丢弃/通道满不阻塞）
- **性能基线入库与 CI bench job**：`inklog_bench` 全组正式基线数字入库 docs/PERFORMANCE.md（2026-10-01，与 rc4_pipeline_bench 基线同载体续记）；`.github/workflows/ci.yml` 新增 `benchmark` job（`workflow_dispatch` 显式开关 + 宽松阈值对照，默认不阻塞 PR）
- **fallback journal 加密与推送**：`global.fallback_journal_encrypt` 可选 AES-256-GCM 加密——文件头 `INKJRN1\0` magic + 版本 + 算法 + 16 字节随机盐，逐段 nonce(96-bit 随机，唯一性策略实现前固化并由单测断言同 key 跨段/跨文件不重复) + 密文；key 来源 **env 优先、其次 key 文件**（unix 权限强制 0600，组/其他可读显性拒绝），加密启用而 key 缺失/格式无效/熵不足时**构建期显性失败，禁止静默明文落盘**；`replay` 新旧格式自适应（旧明文 JSONL 可被加密实例重放，明文实例遇加密文件按段计数跳过）；`global.fallback_journal_push_addr` 接线推送端口 `JournalPushSender`（无网络环境可注入 mock），内置 `TcpJournalPusher` 复用 net-sink 断线缓冲范式（有界 FIFO 满则丢最旧、指数退避重连、按序补发、半开连接探测），`TransportSecurity` 预留 TLS 扩展点（non_exhaustive）。密码模式按文件头盐 PBKDF2 确定性派生（与 FileSink v2 加密头同范式），密钥解析/熵校验收敛为 `encryption::resolve_key_material`/`validate_key_entropy` 单一事实源
- **ChannelBufferedFileSink 补齐 FileSink 高级能力**：CBFS 写路径接入按大小/时间轮转（`max_size`/`rotation_time`；时间为滚动间隔近似——自上次轮转起算，与 FileSink 的日历对齐语义不同，如实标注不宣称一致）、压缩/加密归档（与 FileSink 共用同一后处理单一事实源 `process_rotated`，加密密钥构造期显性解析——key 缺失/无效构建期报错，来源 env 优先、其次 `encryption_key_file` 0600）、审计链（轮转即登记 `<stem>.chain.jsonl`，复用 `FileSink` manifest 范式）与留存清理（复用 `FileSink::perform_cleanup`）
- **配置 JSON Schema 导出**：新增 `schema` optional feature（schemars 1.2）——全部配置类型（`InklogConfig` 及 global/console/file/database/http/performance/sampling/rate_limit 子配置与枚举）derive `JsonSchema`；`InklogConfig::json_schema()` 程序化导出；`inklog-cli generate --schema` 输出 schema 文件（默认 `config_schema.json`）；`SinkMetadata.config_schema` 由 factory 填充真实 schema（`FileSinkFactory` 先行，未启用 feature 时维持 `None`）；产物 `config_schema.json` 入库并由漂移单测（`include_str` 对比）与 CI 步骤（重新生成 + `git diff --exit-code`）双重防漂移

- **per-target 分级限流**：`rate_limit.rules`（target 前缀 → 该组每秒令牌数，最长前缀优先、ASCII 大小写不敏感）配置按前缀分组的配额限流；命中组由组桶独立裁决（放行后不再进入 `performance.rate_limit` 全局限流），未命中 target 维持既有全局路径；组预算耗尽与全局限流拒绝共用同一关键级别救援（ERROR/FATAL 按 1-in-N 采样保留），非关键级别计为 `logs_dropped`；加载期校验（前缀非空、速率 ≥ 1），未配置时整体不接线、默认全局行为不变；查找路径为无分配线性扫描（基准 `target_rate_limiter_lookup`：最长前缀命中 ≈6ns、全表未命中 ≈4ns、含组桶裁决 ≈41ns）
- **可配采样策略**：`InklogConfig.sampling`（`SamplingConfig`：`per_level` 每级别采样率 + `per_target_prefix` 按 target 前缀规则，最长前缀优先、大小写不敏感）配置限流压力下的保留规则，加载期校验（级别合法、采样率 ≥ 1、前缀非空）；规则未命中或未配置时回退内置兜底（非关键级别丢弃、ERROR/FATAL 保留 1/100），行为与既有语义一致
- **采样细分指标**：新增 `inklog_sampled_out_total`（`Metrics::sampled_out()`），仅统计采样决策淘汰的记录（Subscriber 压力路径采样与 `SamplingSink` 淘汰），限流非采样丢弃与通道满丢弃不计入；`MetricsSnapshot` 同步携带 `sampled_out`

### Changed

- **脱敏规则引擎切换 fancy-regex**：正则环视（lookbehind/lookahead）是 CJK 友好边界的前提，`regex` crate 不支持；新增 `fancy-regex` 依赖（规则热路径行为不变，编译期 expect 守卫）
- **FileSink 写路径**：文件句柄改 64 KiB `BufWriter`（每批一次 flush 摊销逐条 syscall）；`inner` 状态升级 `Arc<RwLock<>>` 供定时线程执行空闲 flush；批末 flush 失败整批回填重试（at-least-once）
- **对象池闭环**：worker 消费端 `Arc::try_unwrap` 成功即归还 `LogRecord`（池此前只取不还，零分配目标落空）
- **`set_level` 双门面同步**：运行时热调级别时同步 `log::set_max_level`（此前 log 门面被旧级别拦截）
- **行为变更：console sink `enabled=false` 不再输出**：`ConsoleSink::write` 此前不检查 `enabled` 字段，显式禁用组合仍全量写 stdout/stderr；现禁用态在 write 处丢弃记录（默认组合 `enabled=true` 行为逐字节不变）。该语义落实使性能基准等声明禁用 console 的场景不再向 stdout 泄漏海量输出
- **CI 数据库后端矩阵补齐 duckdb**：docker 数据库流水线分组矩阵扩为 sqlite/postgres/mysql/duckdb 四后端（互斥 feature 逐一验证）；duckdb 为 embedded 后端不起 compose 服务，经 `duckdb:///` 文件库直跑集成测试；`tests/docker` 测试目标 crate cfg 同步纳入 duckdb（此前该 feature 下整个目标被 cfg 掉，矩阵项只会空跑）。随真跑暴露并修复 duckdb 链路两处缺陷：`DbNexusAdapter` 建表此前走 SeaORM 通道（DuckDB 连接直接报错），现按驱动分派至 `execute_duckdb_raw`；DuckDB 建表 DDL 去除自增 id 列（duckdb-rs 绑定对 AUTOINCREMENT/IDENTITY 约束报 "Constraint not implemented"，行标识由隐式 rowid 提供）；评审收尾：DuckDB DDL 分派显式限定 CREATE TABLE 模板（`execute_duckdb_raw` 的 DdlGuard 仅对可识别 DDL 关键字设防，其余语句会以 admin 绕过 guard 策略），并清理 docker 测试遗留的死辅助 DDL 代码
- **i18n feature 化**：icu/fluent 依赖树可裁剪（不启用 i18n feature 的消费方不再承担其编译成本），default 行为零变化
- **console 着色构造期缓存**：着色判定构造期缓存与禁用分支零克隆
- **error sink 延迟创建**：error sink 延迟到首次 error 写入时才创建文件
- **ChannelBufferedFileSink 落盘格式**：缓冲落盘记录间补换行分隔

---
- **BREAKING：`MiddlewareVerdict` 由两态扩为三态**：新增 `Reround` 变体（治理轮终审放行，短路链上剩余治理件，记录继续进入改写与发送阶段；语义来源为 target 前缀配额组放行后不再进入全局限流）。对下游 `match` 穷尽匹配该枚举的代码为破坏性变更，升级时需补全新变体分支；有意不加 `#[non_exhaustive]`——`RecordMiddleware` 的下游实现者需要构造裁决值，non_exhaustive 会同时封死构造面，破坏面反而更大
- **file sink 回落判定收窄**：轮转/压缩/加密/审计链配置不再回落 `FileSink`——`ChannelBufferedFileSink` 已全能力覆盖，仅 `fsync = true` 与 `output_format = "Json"`（NDJSON 出站）仍走 `FileSink`；默认配置落盘路径由 FileSink 变为 CBFS（独立 flush 线程 + 可配背压），能力矩阵见 docs/USER_GUIDE.md

### Fixed

- **TcpJournalPusher 推送健壮化**：`push` 收敛为锁内入队（O(1)），连接/半开探测/补发/重连全部移入专职投递线程——关键日志发送路径不再被网络停滞串行化或冻结；新增 `write_timeout`（默认 3s），对端不排空时写超时即弃用连接走既有重连缓冲路径（此前同步 `write_all` 会无限阻塞）；构造期地址校验收敛为 `host:port` 语法检查（瞬时 DNS 故障不再让「尽力而为」端口以 ConfigError 终止构建），解析延迟到投递线程
- **journal 加密 replay 显性化**：不可识别/不可解密（版本或算法不支持、明文实例遇加密文件的配置回滚、错误密钥全军覆没）时 journal 文件原样保留并按段计数上报——此前无条件清空会销毁不可重放的记录；明文实例对加密 journal 的 spill 显性拒绝（不再追加明文破坏段结构）；加密 journal 全部写路径（头创建/段追加/截断 tmp+rename）以 0600 创建（对齐 FileSink 加密产物与 key 文件约束）；`manager::shutdown` 接线推送端口 `flush_pending` 尽力补发并文档化丢失窗口；加密 spill 头部读取收敛为 O(1)（只读头 28 字节，此前每次 spill 整文件读取）
- **ChannelBufferedFileSink 加固对齐与轮转判定补全**：shutdown drain 路径补齐 size/time 轮转判定与 `current_file_bytes` 记账（此前 shutdown 时通道积压可无限越过 `max_size`，size 轮转断言偶发失败的根因）；活动文件打开对齐 FileSink 三处审计级加固（0600 / O_NOFOLLOW / 新建父目录 0700）；轮转 rename+copy 双失败显性上报 `sink_degraded`（此前静默吞掉，持续失败环境轮转永久失效无告警）；审计链登记（sha256 + manifest 写穿）移入既有归档后台线程并与 FileSink 收敛为共享单一事实源 `register_rotated_archive`，sha256 失败以 ops 事件显性告警；时间轮转语义如实标注为滚动间隔近似（与 FileSink 日历对齐不同）
- **加密后明文残留显性上报**：轮转产物加密后 `remove_file` 源文件失败升级为 `sink_degraded` ops 事件（此前仅 warn 日志——明文残留属安全相关降级状态）；加密密钥环境变量为空时报错显性提示「移除该变量以回退 key 文件」（此前落入泛化长度错误丢失处置指引）

## [0.3.0-rc.5] - 2026-09-21

### 新增

- **通用业务指标 registry**：Prometheus 文本导出（收编并行会话改动）

### 变更

- **i18n 整改**：统一错误与消息文案管理并清理零引用死键
- **特性守卫**：补四数据库后端互斥守卫，引入 database/zstd 谓词收敛
- **依赖与发布**：跨仓 path 依赖改走 crates.io；rustls 升 0.23.45；为 path-only 依赖补全 version 字段；release 工作流 publish 步骤幂等容错
- **工程加固**：detect-secrets 基线、pre-commit 门禁、typos 白名单、CI examples serde 依赖修复

### 修复

- **logger-install**：依赖注入构建路径补装全局 tracing/log 前端
- **dbnexus 依赖特性**：补 failover/replica-routing

---

## [0.3.0-rc.4] - 2026-09-14

> 注：0.3.0-rc.3 未单独发布（无 tag、未上 crates.io），本节内容含原 rc.3 开发批次，随 0.3.0-rc.4 一并发布。

### Added

- **可配置日志门面**：新增 `init_inklog_logger()` 与 `init_inklog_logger_with_config(config)` 便捷初始化函数，进程级单例语义（重复初始化返回明确错误）
- **Per-crate target 级别预设**：`InklogConfig` 新增 `target_levels: HashMap<String, String>` 字段，按 crate target 设置独立日志级别，合并到 EnvFilter（优先级低于 RUST_LOG、高于全局默认）
- **dbnexus 池指标接入**：`DbNexusAdapter` 新增 `pool_status()` 透传连接池状态快照；`Metrics` 新增 `record_pool_metrics(total, active, idle)` 方法及对应 Prometheus 导出（`inklog_db_pool_total/active/idle`）
- **动态 Sink 注册**：`LoggerBuilder::add_sink(Arc<dyn AsyncSink>)` + 通用 SinkWorker——第三方 Sink 实现 `LogSink` 即可零核心改动接入，每 sink 独立通道互不抢占；`AsyncSink` 为 `LogSink` 子 trait（blanket impl，trait 上转型直通）
- **运行时级别热调**：`LoggerManager::set_level(target, level)` 经 `tracing_subscriber::reload` 换装 EnvFilter，进程内即时生效；指令集支持全局/per-target upsert，RUST_LOG 附加指令跨重建保留
- **dbnexus AuditStorage 适配器（`dbnexus-audit` feature）**：`InklogAuditStorage` 实现 dbnexus 审计存储端口——审计事件 → 结构化日志（`audit::<entity_type>`）→ inklog DB 落库
- **追踪 ID 关联**：`LogRecord` 新增 `trace_id`/`span_id`（从当前 tracing span 提取；无 OTel 时沿 parent 链以根 span id 派生 32/16 位 hex），模板新增 `{trace_id}`/`{span_id}` 占位符，serde 兼容旧数据
- **日志查询 CLI**：`inklog-cli query` 按时间/级别/关键词检索本地日志（含 `ENCLOG1` v1/v2 解密与 zstd/gzip 解包，目录递归），`--json` 输出，退出码 0/2/1
- **性能基线门禁**：新增 `rc4_pipeline_bench`（写入/序列化/加密路径）与 `docs/PERFORMANCE.md` 首份正式基线
- **日志采样器**：`Sampler`（级别阈值 + N 取 1 + 关键词白名单豁免）与 `SamplingSink` 装饰器
- **confers 配置集成（`config-confers` feature）**：`InklogConfig` 经 confers 加载 + `ConfersConfigWatcher` watch 热更新级别/轮转参数（非法 TOML 保持旧配置）
- **KMS 密钥提供者（`kms` feature）**：`KeyProvider` 端口 + `EnvKeyProvider`/`ConfersKeyProvider`（confers AsyncKeyProvider 适配）+ Vault transit MVP（mock server 测试）
- **按目标限流端口**：`SinkRateLimit`（对象安全）+ `NoOpRateLimit` 默认 + `TokenBucketRateLimit` 基线 + `RateLimitedSink` 装饰器——供 limiteron 上层实现
- **内部审计事件流 + 归档防篡改链**：`publish_ops_event` 广播 ops 事件到全部 sink 通道；`ArchiveChain` 归档 HMAC-SHA256 链（随机链首盐，防篡改/删除/重排/伪造）
- **网络转发 Sink（`net-sink` feature）**：`TcpSink`（可 TLS，rustls 客户端）+ `UdpSink`（NDJSON），断线缓冲 + 半开探测 + 自动重连按序补发
- **直方图/中间件/参数化批量/OTLP**：Prometheus 原生 `inklog_write_latency_us` histogram 导出；`MiddlewareChain`/`MiddlewareSink`（filter/transform 组合）；DuckDB 批量插入参数化（预编译语句绑定，消除转义拼接）；`otlp` feature OTLP/HTTP JSON 导出 MVP（mock collector 测试）

### Changed

- **kit feature 显式包含 `trait-kit/observer`**：消除编译定时炸弹（下游项目不再依赖传递依赖意外启用 observer）
- **oxcache 死使能清理**：移除零消费的 `macros`/`serialization`/`metrics`/`batch` feature，仅保留 `memory`
- **dbnexus 死使能清理**：移除 `failover`/`replica-routing` feature（`with_full_config` 硬编码 `None`，无行为）
- 依赖升级：dbnexus → 0.6.0-rc.3、oxcache → 0.5.0-rc.4、trait-kit → 0.5.0-rc.3
- 新增依赖：confers 0.6.0-rc.3（optional，`config-confers`/`kms` feature）+ `[patch.crates-io]` 本地路径

---

## [0.3.0-rc.2] - 2026-09-03

### ⚠️ BREAKING CHANGES

- **版本号升级**: 0.2.0 → 0.3.0-rc.2，反映 trait-kit 0.5.0-rc.2 集成与 i18n 重构

### Changed

- **依赖升级**: trait-kit 0.4 → 0.5.0-rc.2
- **S3 归档功能移除**: 文档中清理 S3 归档相关描述（0.1.1 后已统一走本地/对象存储抽象层）

### Documentation

- 同步 docs/ 下版本号至 0.3.0-rc.2
- 同步 src/lib.rs html_root_url 至 0.3.0-rc.2
- 同步 ARCHITECTURE.md 文档版本至 3.0
- uat.md: MSRV 1.70 → 1.94

## [0.2.0] - 2026-08-05

### ⚠️ BREAKING CHANGES

- **版本号升级**: 0.1.12 → 0.2.0，反映项目成熟度和 API 稳定性提升
- **依赖升级**: dbnexus 0.4 → 0.5, sea-orm 保持 2.0, oxcache 0.3 → 0.4, thiserror 1.x → 2.0, axum 0.6 → 0.8, trait-kit 0.3 → 0.4

### Added

- **compression feature**: 可选 ZSTD 压缩支持（避免 zstd-sys 符号冲突）
- **parquet feature**: Parquet/Arrow 导出支持（数据库 Sink 归档）
- **fast-masking feature**: Aho-Corasick 多模式加速脱敏
- **i18n 核心模块**: fluent-bundle + ICU 国际化支持（始终编译，非 feature flag）
- **通道缓冲文件 Sink**: ChannelBufferedFileSink 与背压策略
- **断路器保护**: CircuitBreaker 机制保护 Sink 故障恢复
- **环形缓冲**: RingBufferedFileSink 高吞吐场景

### Changed

- **模块重构**: `infrastructure` 模块重命名为 `integrations`
- **公共 API 统一**: 所有集成类型通过根级别 re-export 访问
- **edition 升级**: Rust edition 2024, MSRV 1.94
- **依赖特性化**: 所有依赖显式声明 `default-features = false`

### Removed

- **不存在的 feature 清理**: 文档中移除 `test-local`、`debug`、`metrics`、`i18n` 等从未定义的 feature flag
- **压缩算法简化**: 文档中移除 Brotli/LZ4 引用（实际仅支持 ZSTD 和 GZIP）

### Fixed

- 文档版本号、功能标志表、模块路径与实际代码同步
- README.md 许可证信息修正为 MIT（与 Cargo.toml 一致）
- README_EN.md Rust 版本徽章修正为 1.94+

## [0.1.12] - 2026-07-22

### 测试

- 新增 `tests/e2e_advanced.rs`（226 个测试）：覆盖 CORE/ROT/PIPE/SEC/REL/PERF/OBS/CFG/compression/i18n/Integration 共 19 个模块的边界与异常场景

### 维护

- 移除未使用依赖：parking_lot（examples）
- 更新 sea-orm 到 2.0 稳定版

## [0.1.11] - 2026-07-17

### 安全修复

- **[vuln-0001]** 对 `table_name` 做 SQL 注入防护清洗
- **[vuln-0002]** `FileSink` 集成 `PathValidator` 防路径穿越；收紧敏感组件黑名单（新增 `.bashrc`/`.profile`/`.zshrc`/`.netrc`/`id_rsa`/`id_ed25519` 等），禁止落到用户主目录与密钥文件
- **[vuln-0003]** HTTP auth token 改为启动期缓存，获取失败 fail-closed（不再降级为无认证）

### 修复

- `compress_file`/`encrypt_file` 参数改 `&Path` 修 clippy `ptr_arg`
- 已安装的全局 logger/subscriber 不再重复 warn

### 重构

- examples/tests/benches：扩展 L1 重导出隔离 + 补 `tracing`/`chrono`/`tokio`/`serde` re-export

## [0.1.7] - 2026-07-13

### ⚠️ BREAKING CHANGES（仅影响启用 db 后端 feature 的用户）

- dbnexus 0.3 → 0.4（pre-1.0 minor bump，Cargo 视为不兼容）；启用 `sqlite`/`postgres`/`mysql`/`duckdb`/`kit` feature 的用户需同步升级
- trait-kit 0.2 → 0.3（同上，仅影响 `kit` feature 用户）

### Dependencies

- dbnexus 0.3 → 0.4（对齐下游 sdforge/limiteron 依赖链）
- trait-kit 0.2 → 0.3（dbnexus 0.4 依赖 trait-kit 0.3）
- oxcache 0.3.4 → 0.3.8

### Fixed

- `src/error.rs`: Moved `InklogResult<T>` type alias from after `#[cfg(test)] mod tests` to before it, fixing clippy `items_after_test_module` lint
- `Cargo.toml`: MSRV updated from `1.85` to `1.94` to match actual requirement (sea-orm 2.0.0-rc + sqlx 0.9)

## [0.1.6] - 2026-07-12

### ⚠️ BREAKING CHANGES

- `error` module moved from `src/domain/types/error.rs` to `src/error.rs`, import path `crate::domain::types::error::` → `crate::error::`
- Added `InklogResult<T>` type alias

### Changed

- 文件级 import 扁平化为 mod 级 import
- `pub mod` 收敛为 `mod`（lib.rs 收敛）
- `mod.rs` 拆分为 `mod.rs` + `<module>.rs`（impl 提取）
- 导入路径统一为最浅 re-export 路径
- README 翻译为中文，新增 README_EN 英文版，徽章下添加语言切换链接
- AGENTS.md 添加到 .gitignore（禁止 AI 文件入库）

## [0.1.5] - 2026-07-11

### Changed

- 无代码变更，版本号对齐 workspace 同步升级

### Changed

- **edition 升级**: 从 edition 2021 升级至 edition 2024
- **MSRV 声明**: `rust-version = "1.85"` 显式声明最低支持 Rust 版本
- **MIT license 统一**: 许可证从 `"MIT OR Apache-2.0"` 统一为 `"MIT"`，移除 Apache-2.0 选项

### Security

- **edition 2024 unsafe 要求**: `std::env::set_var` / `std::env::remove_var` 调用包裹 `unsafe` 块（edition 2024 将其标记为 unsafe 操作）

## [0.1.2] - 2026-07-05

### BREAKING CHANGES

- **LogSink trait async 化**: `LogSink` trait 的 `write`/`flush`/`shutdown` 方法签名改为 `async`
  - `FileSink`、`ConsoleSink`、`RingBufferedFileSink`、`DatabaseSink` 全部迁移至 async
  - `DatabaseSink` 移除 `block_in_place`，使用原生 async
  - `LoggerManager` 中所有 sink 调用更新为 `.await`
  - 自定义 Sink 实现需要相应改为 async

- **ObjectPool API 重构**:
  - `ObjectPool::new`/`with_config`/`get`/`put` 改为 `async` 并返回 `Result<_, InklogError>`
  - 移除内部 `SHARED_RUNTIME`（内部 tokio runtime）
  - ThreadLocal 池（`get_log_record`/`put_log_record`/`get_string_buffer`/`put_string_buffer`）保留为同步便捷函数

- **Cache trait 错误处理显性化**: `Cache` trait 的 `get`/`delete`/`exists` 方法返回 `Result<_, InklogError>`（此前为 `Option`/`bool`，错误被 `tracing::warn!` 静默吞掉）
  - 移除 `OxCacheAdapter::default()`（原本通过 `.expect()` 触发 panic）
  - `ObjectPool::with_config` 不再静默回退到 `Cache::default()`

- **dbnexus feature 拆分**: `dbnexus` feature 拆分为四个独立 feature：`sqlite`、`postgres`、`mysql`、`duckdb`（`duckdb` 仅用于 `--all-features` 测试场景，DatabaseSink 不直接支持 duckdb 驱动）

### Changed

#### 依赖重构

- `oxcache` 升级 0.2.0 → 0.3.3，启用 features `memory`/`serialization`/`tracing`/`macros`
- `dbnexus` 升级 0.2.0 → 0.3.1，新增 `duckdb` feature 用于 `--all-features` 测试场景（dbnexus 0.3.1 规则 2 例外：sqlite+duckdb+postgres+mysql 全部 4 个后端启用时允许共存）
- 移除直接的 `moka` 和 `dashmap` 依赖（现仅通过 oxcache 间接依赖）
- `sea-orm` TLS 后端从 `runtime-tokio-native-tls` 切换至 `runtime-tokio-rustls`

#### 代码清理

- 移除源码注释中所有 `confers` 引用
- 清理 `object_pool.rs` 中的死代码（1507 → ~650 行）

### Removed

- `SHARED_RUNTIME`（内部 tokio runtime）
- `ObjectPoolBuilder`、`PoolMetrics` 类型
- `ObjectPool` 死代码方法：`with_capacity`/`remove`/`contains`/`capacity`/`metrics`/`clear`/`execute_async`
- `LOG_RECORD_POOL`/`STRING_POOL` 静态变量
- `OxCacheAdapter::default()` 实现
- 直接的 `moka` 和 `dashmap` 依赖

### Security

- 在 `deny.toml` 忽略列表中添加 `RUSTSEC-2026-0173`（proc-macro-error2 unmaintained，transitive via dbnexus-macros/sea-bae，无安全升级路径）

## [0.1.1] - 2026-06-29

### BREAKING
- 完全移除 S3/AWS 代码和依赖（aws feature、aws-sdk-s3、aws-config 等）
- 移除 ArchiveConfig、S3Error、ArchiveError 类型
- 移除 archive 模块和 S3 归档服务
- **MSRV 提升**: Rust 1.75+ → 1.85+（因 rand 0.10 要求 MSRV 1.85）
- **rand 0.9 → 0.10**: `RngCore` trait 重命名为 `Rng`，`Rng` 重命名为 `RngExt`，`OsRng` 重命名为 `SysRng`
- **aes-gcm 0.10 → 0.11**: `Nonce::from_slice` 已废弃，改用 `Nonce::from`；`cipher.encrypt/decrypt` 接受 `&Nonce` 而非 `Nonce`

### Added
- 新增 10 个示例：object_pool、path_validator、log_sanitizer、log_adapter、compression、rotation、ring_buffered_file、config_file、metrics、circuit_breaker
- 测试覆盖率达 90.12%（3291/3652 行）
- 新增 Docker 测试基础设施（SQLite/PostgreSQL/MySQL）
- 新增 CI 流水线（test-docker.yml）
- Cargo.toml 新增 `rust-version = "1.85"` 显式声明 MSRV

### Changed
- **依赖升级**:
  - `toml` 0.9 → 1.1（major 版本升级）
  - `validator` 0.19 → 0.20
  - `aes-gcm` 0.10 → 0.11（破坏性 API 变更已适配）
  - `rand` 0.9 → 0.10（破坏性 API 变更已适配，MSRV 1.85+）
  - `parquet` 57.3 → 59.0（major 版本升级）
  - `arrow-array` 57.3 → 59.0
  - `arrow-schema` 57.3 → 59.0
  - `cron` 0.15 → 0.17
  - `sha2` 0.10 → 0.11
  - `pbkdf2` 0.12 → 0.13
- CI MSRV 矩阵从 1.70.0 提升至 1.85.0
- README/README_zh Rust 版本徽章从 1.75+ 更新为 1.85+
- docs/CONTRIBUTING.md MSRV 与版本引用同步更新
- examples/Cargo.toml 移除 s3_archive [[bin]] 定义
- lib.rs 移除 "S3 归档" 描述

### Fixed
- 修复 examples/Cargo.toml s3_archive 残留导致编译失败
- 修复 lib.rs S3 归档描述残留
- 适配 rand 0.10 破坏性 API：`RngCore` → `Rng`，`Rng` → `RngExt`（src/support/io/sink/encryption.rs、src/support/io/sink/file.rs、src/cli/decrypt.rs、benches/inklog_bench.rs）
- 适配 aes-gcm 0.11 破坏性 API：`Nonce::from_slice` → `Nonce::from`，`cipher.encrypt/decrypt` 接受引用（src/support/io/sink/file.rs、src/cli/decrypt.rs、tests/cli_integration.rs）

## [0.1.0] - 2026-01-18

### Added

#### 核心功能
- **LoggerManager**: 异步日志管理器，支持多种初始化方式
  - `LoggerManager::new()` 默认初始化
  - `LoggerManager::builder()` 构建器模式
  - `LoggerManager::with_config()` 自定义配置

- **多输出目标支持**: 基于trait的可扩展sink架构
  - ConsoleSink: 控制台输出，支持彩色显示
  - FileSink: 文件输出，支持轮转和压缩
  - DatabaseSink: 数据库输出，支持批量写入

- **配置系统**: 完整的TOML配置支持
  - 全局配置、性能配置、HTTP服务器配置
  - 环境变量覆盖
  - 配置验证和错误处理

- **性能优化**: 基于crossbeam-channel的异步架构
  - 有界通道，支持背压控制
  - 多线程工作池
  - 内存池优化

- **监控和指标**: 内置健康检查和性能指标
  - HTTP健康检查端点
  - Prometheus兼容指标
  - 实时状态监控

#### 功能特性
- **日志轮转**: 基于大小和时间的自动轮转
- **数据掩码**: 敏感信息自动掩码功能
- **S3归档**: AWS S3云存储归档（可选功能）
- **CLI工具**: 配置生成、验证、日志解密命令行工具

#### 技术栈
- **异步运行时**: tokio 1.32+
- **日志框架**: tracing 0.1
- **序列化**: serde 1.0
- **并发**: crossbeam-channel 0.5
- **HTTP服务**: axum 0.6（可选）

### 兼容性
- **Rust**: 1.70+
- **平台**: Linux, macOS, Windows
- **数据库**: SQLite, PostgreSQL, MySQL（通过SeaORM）
- **云存储**: AWS S3兼容存储

### 示例用法

```rust
use inklog::LoggerManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _logger = LoggerManager::new().await?;

    tracing::info!("Hello, inklog!");
    Ok(())
}
```

---

## [0.0.0] - 2025-12-30

### Added

- 初始项目结构
- 基础Cargo配置
- CI/CD工作流

<!-- Links -->
[Unreleased]: https://github.com/Kirky-X/inklog/compare/v0.3.0-rc.10...HEAD
[0.3.0-rc.7]: https://github.com/Kirky-X/inklog/compare/v0.3.0-rc.6...v0.3.0-rc.7
[0.3.0-rc.6]: https://github.com/Kirky-X/inklog/compare/v0.3.0-rc.5...v0.3.0-rc.6
[0.3.0-rc.5]: https://github.com/Kirky-X/inklog/compare/v0.3.0-rc.4...v0.3.0-rc.5
[0.3.0-rc.4]: https://github.com/Kirky-X/inklog/compare/v0.3.0-rc.2...v0.3.0-rc.4
[0.3.0-rc.2]: https://github.com/Kirky-X/inklog/compare/v0.2.0...v0.3.0-rc.2
[0.2.0]: https://github.com/Kirky-X/inklog/compare/v0.1.12...v0.2.0
[0.1.12]: https://github.com/Kirky-X/inklog/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/Kirky-X/inklog/compare/v0.1.7...v0.1.11
[0.1.7]: https://github.com/Kirky-X/inklog/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/Kirky-X/inklog/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/Kirky-X/inklog/compare/v0.1.2...v0.1.5
[0.1.2]: https://github.com/Kirky-X/inklog/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Kirky-X/inklog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Kirky-X/inklog/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Kirky-X/inklog/releases/tag/v0.0.0
