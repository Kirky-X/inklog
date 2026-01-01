# 📖 inklog 配置参考手册

## 配置文件结构

```toml
[global]                    # 全局配置
[console_sink]              # 控制台输出配置
[file_sink]                 # 文件输出配置
[database_sink]             # 数据库输出配置
[parquet_config]            # Parquet 导出配置
[archive]                   # S3 归档配置
[http_server]               # HTTP 监控配置
[performance]               # 性能配置
[masking]                   # 敏感信息过滤配置
```

---

## [global] 全局配置

```toml
[global]
# 日志级别: trace, debug, info, warn, error
level = "info"

# 日志格式模板
format = "{timestamp} [{level}] {target} - {message}"

# 可用模板变量:
# - {timestamp} : 时间戳
# - {level}     : 日志级别
# - {target}    : 模块名
# - {message}   : 日志消息
# - {file}      : 文件名
# - {line}      : 行号
# - {thread_id} : 线程ID
```

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| level | `INKLOG_GLOBAL_LEVEL` | "info" |
| format | `INKLOG_GLOBAL_FORMAT` | 见上方 |

---

## [console_sink] 控制台输出

```toml
[console_sink]
enabled = true              # 启用控制台输出
colored = true              # 启用彩色输出（非TTY自动禁用）
stderr_levels = ["error", "warn"]  # 输出到 stderr 的级别
```

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| enabled | `INKLOG_CONSOLE_ENABLED` | true |
| colored | `INKLOG_CONSOLE_COLORED` | true |
| stderr_levels | `INKLOG_CONSOLE_STDERR_LEVELS` | ["error", "warn"] |

---

## [file_sink] 文件输出

```toml
[file_sink]
enabled = true
path = "logs/app.log"           # 日志文件路径
max_size = "100MB"              # 文件大小阈值，支持 MB/GB
rotation_time = "daily"         # 时间轮转: hourly, daily, weekly
keep_files = 30                 # 保留历史文件数量
compress = true                 # 启用压缩
encrypt = true                  # 启用加密
encryption_key_env = "LOG_KEY"  # 密钥环境变量名
cleanup_interval_minutes = 60   # 清理间隔（分钟）
retention_days = 90             # 文件保留天数
```

### 加密说明

- 密钥必须为 Base64 编码的 32 字节
- 加密算法: AES-256-GCM
- 每个文件使用独立的随机 Nonce

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| enabled | `INKLOG_FILE_ENABLED` | false |
| path | `INKLOG_FILE_PATH` | "logs/app.log" |
| max_size | `INKLOG_FILE_MAX_SIZE` | "100MB" |
| rotation_time | `INKLOG_FILE_ROTATION_TIME` | "daily" |
| keep_files | `INKLOG_FILE_KEEP_FILES` | 30 |
| compress | `INKLOG_FILE_COMPRESS` | true |
| encrypt | `INKLOG_FILE_ENCRYPT` | false |
| encryption_key_env | `INKLOG_FILE_ENCRYPTION_KEY_ENV` | - |
| retention_days | `INKLOG_FILE_RETENTION_DAYS` | 90 |

---

## [database_sink] 数据库输出

```toml
[database_sink]
enabled = true
driver = "postgres"             # 数据库类型: sqlite, postgres, mysql
url = "postgres://user:pass@localhost/logs"  # 连接 URL
batch_size = 100                # 批量写入大小
flush_interval_ms = 500         # 超时刷新间隔（毫秒）
archive_to_s3 = true            # 启用 S3 归档
archive_after_days = 30         # 归档天数
partition_by_month = true       # 按月分区（PostgreSQL）
```

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| enabled | `INKLOG_DB_ENABLED` | false |
| driver | `INKLOG_DB_DRIVER` | "postgres" |
| url | `INKLOG_DB_URL` | - |
| batch_size | `INKLOG_DB_BATCH_SIZE` | 100 |
| flush_interval_ms | `INKLOG_DB_FLUSH_INTERVAL_MS` | 500 |
| archive_to_s3 | `INKLOG_DB_ARCHIVE_TO_S3` | false |
| archive_after_days | `INKLOG_DB_ARCHIVE_AFTER_DAYS` | 30 |

---

## [parquet_config] Parquet 导出配置

```toml
[parquet_config]
compression_level = 3           # 压缩级别 1-22
encoding = "PLAIN"              # 编码: PLAIN, RLE, DELTA
max_row_group_size = 10000      # 最大行组大小
max_page_size = 1048576         # 最大页面大小（字节）
```

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| compression_level | `INKLOG_DB_PARQUET_COMPRESSION_LEVEL` | 3 |
| encoding | `INKLOG_DB_PARQUET_ENCODING` | "PLAIN" |
| max_row_group_size | `INKLOG_DB_PARQUET_MAX_ROW_GROUP_SIZE` | 10000 |
| max_page_size | `INKLOG_DB_PARQUET_MAX_PAGE_SIZE` | 1048576 |

---

## [archive] S3 归档配置

```toml
[archive]
enabled = true
bucket = "logs-archive"         # S3 存储桶
region = "us-east-1"            # AWS 区域
archive_interval_days = 7       # 归档间隔（天）
schedule_expression = "0 2 * * *"  # Cron 表达式（可选）
local_retention_days = 30       # 本地保留天数
local_retention_path = "logs/archive_failures"
compression = "zstd"            # 压缩类型
storage_class = "standard_ia"   # 存储类别
prefix = "logs/"                # S3 前缀路径
max_file_size_mb = 100          # 单文件大小限制（MB）
endpoint_url = ""               # 自定义端点（MinIO 等）
force_path_style = false        # 强制路径样式访问
skip_bucket_validation = false  # 跳过存储桶验证
encryption_algorithm = "aes256" # 加密算法
```

### 存储类别

- `standard` - 标准存储
- `intelligent_tiering` - 智能分层
- `standard_ia` - 标准-不频繁访问
- `onezone_ia` - 单区-不频繁访问
- `glacier` - Glacier
- `glacier_deep_archive` - Glacier 深度归档

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| enabled | `INKLOG_S3_ENABLED` | false |
| bucket | `INKLOG_S3_BUCKET` | - |
| region | `INKLOG_S3_REGION` | "us-east-1" |
| archive_interval_days | `INKLOG_S3_INTERVAL_DAYS` | 7 |
| schedule_expression | `INKLOG_S3_SCHEDULE` | - |
| local_retention_days | `INKLOG_S3_LOCAL_RETENTION_DAYS` | 30 |
| compression | `INKLOG_S3_COMPRESSION` | "zstd" |
| storage_class | `INKLOG_S3_STORAGE_CLASS` | "standard" |
| access_key_id | `AWS_ACCESS_KEY_ID` | - |
| secret_access_key | `AWS_SECRET_ACCESS_KEY` | - |

---

## [http_server] HTTP 监控配置

```toml
[http_server]
enabled = true
host = "0.0.0.0"              # 监听地址
port = 8080                   # 监听端口
health_path = "/health"       # 健康检查路径
metrics_path = "/metrics"     # 指标路径
```

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| enabled | `INKLOG_HTTP_ENABLED` | false |
| host | `INKLOG_HTTP_HOST` | "127.0.0.1" |
| port | `INKLOG_HTTP_PORT` | 8080 |
| health_path | `INKLOG_HTTP_HEALTH_PATH` | "/health" |
| metrics_path | `INKLOG_HTTP_METRICS_PATH` | "/metrics" |

### 健康检查响应

```json
{
  "overall": true,
  "uptime_seconds": 3600,
  "channel_usage": 0.15,
  "sinks": {
    "console": { "healthy": true, "last_error": null },
    "file": { "healthy": true, "last_error": null },
    "database": { "healthy": false, "last_error": "Connection timeout" }
  },
  "metrics": {
    "logs_written_total": 125000,
    "logs_dropped_total": 0
  }
}
```

### Prometheus 指标

```
# HELP inklog_logs_written_total Total number of logs written
# TYPE inklog_logs_written_total counter
inklog_logs_written_total 125000

# HELP inklog_channel_usage_ratio Channel usage ratio
# TYPE inklog_channel_usage_ratio gauge
inklog_channel_usage_ratio 0.15
```

---

## [performance] 性能配置

```toml
[performance]
channel_capacity = 10000       # Channel 容量
worker_threads = 3             # 工作线程数
channel_capacity = 10000       # 日志通道容量
```

### 环境变量覆盖

| 配置项 | 环境变量 | 默认值 |
|--------|----------|--------|
| channel_capacity | `INKLOG_PERFORMANCE_CHANNEL_CAPACITY` | 10000 |
| worker_threads | `INKLOG_PERFORMANCE_WORKER_THREADS` | 3 |

---

## [masking] 敏感信息过滤

```toml
[masking]
enabled = true
# 字段名匹配（精确）
mask_fields = ["password", "secret", "token", "api_key", "credential"]
# 启用正则脱敏
enable_regex = true
# 正则模式
regex_patterns = [
    "email",
    "phone",
    "id_card",
    "credit_card"
]
```

### 默认掩码字段

- password, passwd, pwd
- secret, token
- api_key, apikey
- credential, auth
- access_key, secret_key

### 默认正则模式

| 模式 | 匹配示例 |
|------|----------|
| email | `***@example.com` |
| phone | `138****8888` |
| id_card | `110***********1234` |
| credit_card | `**** **** **** 1234` |

---

## 完整配置示例

```toml
[global]
level = "info"
format = "{timestamp} [{level}] {target} - {message}"

[console_sink]
enabled = true
colored = true
stderr_levels = ["error", "warn"]

[file_sink]
enabled = true
path = "logs/app.log"
max_size = "100MB"
rotation_time = "daily"
keep_files = 30
compress = true
encrypt = true
encryption_key_env = "LOG_ENCRYPTION_KEY"
retention_days = 90

[database_sink]
enabled = true
driver = "postgres"
url = "postgres://user:pass@localhost/logs"
batch_size = 100
flush_interval_ms = 500
archive_to_s3 = true
archive_after_days = 30

[parquet_config]
compression_level = 3
max_row_group_size = 10000

[archive]
enabled = true
bucket = "my-logs"
region = "us-east-1"
archive_interval_days = 7
schedule_expression = "0 2 * * *"
compression = "zstd"
storage_class = "standard_ia"

[http_server]
enabled = true
host = "0.0.0.0"
port = 8080

[performance]
channel_capacity = 10000
worker_threads = 3

[masking]
enabled = true
enable_regex = true
```

## 配置优先级

1. **环境变量** (最高)
2. **配置文件**
3. **代码配置** (Builder 模式)
4. **默认值** (最低)

## 验证配置

```bash
# 使用 CLI 工具验证配置
inklog validate -c inklog.toml
```
