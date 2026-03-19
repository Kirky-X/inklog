# inklog 示例项目

本目录包含 inklog 企业级 Rust 日志基础设施的完整示例，按分层架构组织。

## 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                    inklog 示例项目                          │
├─────────────────────────────────────────────────────────────┤
│  Layer 0 - 零依赖示例（开箱即运行）                          │
│  ├── console.rs    控制台输出（基础、彩色、stderr 分流）       │
│  ├── template.rs   日志模板渲染（占位符、自定义格式）          │
│  ├── builder.rs    Builder 模式配置（链式 API 演示）          │
│  └── masking.rs    数据脱敏（邮箱、电话、身份证等）            │
├─────────────────────────────────────────────────────────────┤
│  Layer 1 - 本地资源示例（自动清理）                          │
│  ├── file.rs       文件输出、轮转、Zstd 压缩                  │
│  ├── encryption.rs 加密日志（AES-256-GCM）                    │
│  └── performance.rs 性能测试（吞吐量、延迟统计）              │
├─────────────────────────────────────────────────────────────┤
│  Layer 2 - 外部服务示例（可选依赖）                          │
│  ├── database.rs   SQLite 内存数据库连接                     │
│  ├── http.rs        HTTP 健康检查和指标端点                   │
│  ├── fallback.rs    Sink 降级机制（故障切换）                 │
│  └── s3_archive.rs  S3 日志归档（AWS/LocalStack）            │
└─────────────────────────────────────────────────────────────┘
```

## 快速开始

### 编译所有示例

```bash
cd examples
cargo build
```

### 运行单个示例

```bash
# Layer 0 - 零依赖示例
cargo run --bin console
cargo run --bin template
cargo run --bin builder
cargo run --bin masking

# Layer 1 - 本地资源示例
cargo run --bin file
cargo run --bin encryption
cargo run --bin performance

# Layer 2 - 外部服务示例
cargo run --bin database
cargo run --bin http
cargo run --bin fallback
cargo run --bin s3_archive

# 保留的原有示例
cargo run --bin basic
cargo run --bin production
cargo run --bin all_features
```

## Layer 0 详解

### console.rs - 控制台输出

演示三种控制台配置模式：

```rust
use inklog::config::ConsoleSinkConfig;
use inklog::sink::console::ConsoleSink;

// 基础输出（无颜色）
let config = ConsoleSinkConfig {
    enabled: true,
    colored: false,
    stderr_levels: vec![],
    masking_enabled: false,
};

// 彩色输出
let config = ConsoleSinkConfig {
    enabled: true,
    colored: true,
    stderr_levels: vec![],
    masking_enabled: false,
};

// stderr 分流
let config = ConsoleSinkConfig {
    enabled: true,
    colored: true,
    stderr_levels: vec!["error".to_string(), "warn".to_string()],
    masking_enabled: false,
};
```

**环境变量控制**：
- `NO_COLOR=1`: 禁用颜色输出
- `CLICOLOR_FORCE=1`: 强制启用颜色
- `TERM=dumb`: 禁用颜色

### template.rs - 日志模板

支持以下占位符：

| 占位符 | 说明 |
|--------|------|
| `{timestamp}` | ISO 8601 时间戳 |
| `{level}` | 日志级别（TRACE/DEBUG/INFO/WARN/ERROR） |
| `{target}` | 日志目标/模块 |
| `{message}` | 日志消息 |
| `{field:xxx}` | 自定义字段 |

### builder.rs - Builder 模式

链式 API 配置示例：

```rust
// 最简单配置
let logger = LoggerManager::builder()
    .level("debug")
    .console(true)
    .build()
    .await?;

// 多 Sink 配置
let logger = LoggerManager::builder()
    .level("info")
    .console(true)
    .file("logs/app.log")
    .build()
    .await?;
```

### masking.rs - 数据脱敏

支持的敏感数据类型：

| 类型 | 示例 | 脱敏结果 |
|------|------|----------|
| 邮箱 | `user@example.com` | `**@**.***` |
| 电话 | `13812345678` | `***-****-****` |
| 身份证 | `110101199001011234` | `******1234` |
| 银行卡 | `6222021234567890123` | `622202******0123` |

## Layer 1 详解

### file.rs - 文件输出

```rust
// 基础文件配置
FileSinkConfig {
    enabled: true,
    path: "logs/app.log".to_string(),
    rotation: Some(RotationPolicy::Size { max_size: "10MB".to_string() }),
    compress: false,
    ..Default::default()
}

// 带压缩配置
FileSinkConfig {
    enabled: true,
    path: "logs/app.log".to_string(),
    rotation: Some(RotationPolicy::Size { max_size: "10MB".to_string() }),
    compress: true,  // 启用 Zstd 压缩
    ..Default::default()
}
```

**轮转策略**：
- `Size`: 按文件大小轮转
- `Time`: 按时间轮转（ hourly/daily/weekly ）
- `SizeAndTime`: 同时满足大小和时间条件

### encryption.rs - 加密日志

```bash
# 设置加密密钥（32 字节，Base64 编码）
export LOG_ENCRYPTION_KEY=<base64_encoded_32_bytes>

# 运行加密示例
cargo run --bin encryption
```

**加密文件格式**：

```
┌─────────────────────────────────────────┐
│ Magic Header (8 bytes)  - "ENCLOG1\0"  │
├─────────────────────────────────────────┤
│ Version (2 bytes)       - 0x0001        │
├─────────────────────────────────────────┤
│ Algorithm ID (2 bytes)  - 0x0001 (AES)  │
├─────────────────────────────────────────┤
│ Nonce (12 bytes)        - 随机/文件唯一  │
├─────────────────────────────────────────┤
│ Encrypted Data (可变)    - AES-GCM 密文  │
├─────────────────────────────────────────┤
│ Auth Tag (16 bytes)     - GCM 认证标签   │
└─────────────────────────────────────────┘
```

### performance.rs - 性能测试

性能基准数据（仅供参考，实际数据因硬件而异）：

| Sink 类型 | 吞吐量 | 延迟 P99 |
|-----------|--------|----------|
| Console Sink | ~200,000 条/秒 | ~200μs |
| File Sink | ~500 条/秒 | ~2ms |

## Layer 2 详解

### database.rs - 数据库日志

使用 SQLite 内存数据库，无需文件管理：

```bash
cargo run --bin database --features dbnexus
```

**前提条件**：需要 dbnexus 功能启用。

### http.rs - HTTP 监控

启动 HTTP 服务器提供健康检查和指标：

```bash
cargo run --bin http
```

**端点**：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查（JSON） |
| `/metrics` | GET | Prometheus 格式指标 |

**示例响应**：

```bash
# 健康检查
curl http://localhost:8080/health
# {"status":"healthy","sinks":[{"type":"Console","healthy":true}],"uptime_seconds":3600}

# Prometheus 指标
curl http://localhost:8080/metrics
# inklog_logs_total{level="INFO"} 1234
# inklog_errors_total 5
```

### fallback.rs - 降级机制

Sink 降级策略：

```
Database Sink → File Sink → Console Sink → 系统告警
```

故障场景：

| 故障场景 | 降级策略 |
|----------|----------|
| DB 连接失败 | 降级到 FileSink |
| 磁盘满 | 降级到 Console Sink |
| S3 不可达 | 本地保留，网络恢复后重试 |

### s3_archive.rs - S3 归档

```bash
# 无凭据时显示配置指南
cargo run --bin s3_archive

# 有 AWS 凭据时尝试归档
AWS_ACCESS_KEY_ID=xxx AWS_SECRET_ACCESS_KEY=yyy \
  AWS_DEFAULT_REGION=us-east-1 \
  cargo run --bin s3_archive

# 使用 LocalStack
AWS_ACCESS_KEY_ID=test AWS_SECRET_ACCESS_KEY=test \
  AWS_ENDPOINT_URL=http://localhost:4566 \
  cargo run --bin s3_archive
```

## 外部依赖配置

部分示例需要额外的 Cargo 特性：

| 特性 | 示例 | 说明 |
|------|------|------|
| `dbnexus` | database.rs | 数据库连接池 |
| `http` | http.rs | HTTP 指标端点 |
| `aws` | s3_archive.rs | S3 归档功能 |

在 `examples/Cargo.toml` 中启用：

```toml
[features]
default = ["dbnexus", "http", "aws"]
```

## 验证

### 编译验证

```bash
cargo build --all-targets
```

### Clippy 检查

```bash
cargo clippy -- -D warnings
```

### 运行所有示例

```bash
# Layer 0（零依赖）
for bin in console template builder masking; do
    cargo run --bin $bin
done

# Layer 1（本地资源）
for bin in file encryption performance; do
    cargo run --bin $bin
done

# Layer 2（外部服务）
for bin in database http fallback s3_archive; do
    cargo run --bin $bin
done
```

## 许可证

MIT License - 参见项目根目录 LICENSE 文件