# Inklog 架构文档

## 概述

Inklog 是一个企业级 Rust 日志基础设施,为分布式和高性能环境设计。它提供了异步 I/O、多目标输出、结构化日志、压缩、加密、数据脱敏和健康监控等企业级特性。

### 设计目标

- **高性能**: 基于 Tokio 的异步架构,支持每秒数千条日志的吞吐量
- **可靠性**: 断路器、自动恢复、故障降级和批量重试确保日志不会丢失
- **安全性**: AES-256-GCM 加密、PII 数据自动脱敏、安全的密钥管理
- **可观测性**: Prometheus 指标、健康检查端点、延迟直方图统计
- **可扩展性**: 通过 `LogSink` trait 支持自定义输出目标

## 系统架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                    应用层                              │
│  (使用 log! 宏的 Rust 应用代码)                 │
└───────────────────────────┬─────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│                 Inklog API 层                      │
│  - LoggerManager: 核心协调器               │
│  - LoggerBuilder: 流式构建器模式         │
│  - 配置管理: 验证、环境变量覆盖       │
│  - 健康监控: Metrics、HealthStatus         │
└───────────────────────────┬─────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│              LoggerSubscriber 层                 │
│  - 实现 tracing::Subscriber                 │
│  - 格式化日志记录                     │
│  - 数据脱敏处理                      │
└───────────────────────────┬─────────────────────────────┘
                        │
        ┌───────────────┴───────────────┐
        ▼                            ▼
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│  日志通道队列      │      │  控制通道       │      │  停止通道       │
│  Crossbeam      │      │  控制消息       │      │  shutdown_tx     │
│  bounded channel│      │  恢复指令       │      │                 │
│  (默认容量     │      │  sink 状态查询   │      │                 │
│   10000)       │      │                 │      │                 │
└────────┬─────────┘      └────────┬─────────┘      └─────────┬─────────┘
         │                       │                         │
         ▼                       ▼                         ▼
    ┌──────────────────────────────────────────────────────────────┐
    │              工作线程池                      │
    │  (3 个专用 OS 线程)                  │
    ├──────────┬──────────┬──────────────────┬───────────┤
    │          │          │                  │          │
    ▼          ▼          ▼                  ▼          ▼
┌────────┐  ┌────────┐  ┌─────────────────┐  ┌──────────┐
│文件线程│  │ DB 线程│  │ 健康检查线程   │  │轮转线程   │
│Sink    │  │Sink    │  │                │  │          │
│(阻塞)  │  │(异步)  │  │每 10 秒检查   │  │(定时器)  │
└────┬───┘  └────┬───┘  └────────┬────────┘  └─────┬────┘
     │            │                  │                  │
     │            │                  │                  │
     ▼            ▼                  ▼                  ▼
┌────────────┐  ┌────────────┐  ┌──────────────────────┐
│FileSink   │  │DatabaseSink│  │控制台降级/错误日志  │
│          │  │            │  │                 │
│ - 文件    │  │ - Sea-ORM  │  │ConsoleSink (File) │
│ - 轮转    │  │ - 批量写入 │  │                 │
│ - 压缩    │  │ - 连接池   │  │                 │
│ - 加密    │  │ - 分区     │  │                 │
│ - 清理    │  │ - 断路器   │  │                 │
└────────────┘  └────────────┘  └──────────────────────┘
     │            │
     └────────────┴──────────────────────┐
                  │                        │
                  ▼                        ▼
           ┌────────────────────┐    ┌──────────────────┐
           │   存储后端        │    │  归档服务        │
           ├────────────────────┤    │                │
           │  - 文件系统      │    │ - S3ArchiveManager│
           │  - PostgreSQL     │    │ (AWS 特性)    │
           │  - MySQL         │    │                │
           │  - SQLite        │    │ - 定时调度       │
           │                 │    │ - Parquet 导出   │
           └────────────────────┘    └──────────────────┘
                  │                        │
                  └────────────┬───────────┘
                               │
                          ┌──────▼───────┐
                          │ HTTP 服务器   │
                          │              │
                          │  Axum       │
                          │  /health     │
                          │  /metrics    │
                          └──────────────┘
```

## 核心组件

### LoggerManager

`LoggerManager` 是 Inklog 的核心协调器,负责日志系统的初始化、运行和生命周期管理。

**主要职责**:

```rust
pub struct LoggerManager {
    config: InklogConfig,
    sender: Sender<LogRecord>,      // 主日志消息通道
    shutdown_tx: Sender<()>,        // 优雅关闭信号
    console_sink: Arc<Mutex<ConsoleSink>>,
    metrics: Arc<Metrics>,         // 健康监控
    worker_handles: Mutex<Vec<JoinHandle<()>>>, // 工作线程句柄
    control_tx: Sender<SinkControlMessage>, // 控制消息
    #[cfg(feature = "aws")]
    archive_service: Option<Arc<AsyncMutex<ArchiveService>>>,
    #[cfg(feature = "http")]
    http_server_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}
```

**初始化流程**:

1. 验证配置 (`config.validate()`)
2. 创建 Crossbeam 通道 (默认容量 10000)
3. 初始化 `ConsoleSink` 和 `LoggerSubscriber`
4. 启动 3 个工作线程 (文件、数据库、健康检查)
5. 安装全局 tracing subscriber
6. [可选] 启动 HTTP 健康检查服务器
7. [可选] 初始化 S3 归档服务

**工作线程**:

- **文件线程**: 处理文件日志写入,支持轮转、压缩、加密
- **数据库线程**: 处理批量数据库写入,使用独立 tokio 运行时
- **健康检查线程**: 每 10 秒检查一次 sink 状态,触发自动恢复

### LoggerBuilder

流式构建器模式,提供类型安全的配置 API:

```rust
LoggerBuilder::new()
    .level("debug")
    .format("{timestamp} {level} {message}")
    .file("/var/log/app.log")
    .database("postgres://localhost/logs")
    .channel_capacity(5000)
    .worker_threads(4)
    .http_server("0.0.0.0", 9090)
    .build()
    .await?
```

### 配置系统 (InklogConfig)

集中式配置结构,支持环境变量覆盖和 TOML 文件加载:

```rust
pub struct InklogConfig {
    pub global: GlobalConfig,                    // 全局设置
    pub console_sink: Option<ConsoleSinkConfig>,
    pub file_sink: Option<FileSinkConfig>,
    pub database_sink: Option<DatabaseSinkConfig>,
    pub s3_archive: Option<S3ArchiveConfig>,
    pub performance: PerformanceConfig,
    pub http_server: Option<HttpServerConfig>,
}
```

**配置优先级** (从高到低):

1. 环境变量 (`INKLOG_*`)
2. 配置文件 (`inklog_config.toml`)
3. 默认值

**验证规则**:

- 日志级别必须有效 (trace/debug/info/warn/error)
- 文件路径不能为空
- 数据库 URL 不能为空
- 加密必须提供密钥环境变量
- S3 bucket 和 region 必须配置

## Sink 系统

Sink 抽象层定义统一接口 `LogSink`:

```rust
pub trait LogSink: Send + Sync {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError>;
    fn flush(&mut self) -> Result<(), InklogError>;
    fn is_healthy(&self) -> bool { true }
    fn shutdown(&mut self) -> Result<(), InklogError>;
    
    // 可选: 轮转支持
    fn start_rotation_timer(&mut self) {}
    fn stop_rotation_timer(&mut self) {}
    
    // 可选: 磁盘空间检查
    fn check_disk_space(&self) -> Result<bool, InklogError> { Ok(true) }
}
```

### Console Sink

轻量级控制台输出,支持 ANSI 颜色和 stderr 分离:

**特性**:
- ANSI 颜色输出 (ERROR=红色, WARN=黄色, INFO=绿色, DEBUG=蓝色)
- 可配置的 stderr 级别 (默认: error, warn)
- 环境变量支持 (`NO_COLOR`, `CLICOLOR_FORCE`)
- 终端类型检测 (`TERM=dumb`)

**工作流程**:
```
LogRecord 
  ↓
检查是否为 stderr 级别
  ↓ [是]          [否]
stderr           stdout
  ↓                ↓
应用颜色       不着色
  ↓
写入终端
```

### File Sink

最复杂的 sink (1350+ 行),提供完整的文件日志解决方案:

**核心功能**:

1. **自动轮转**:
   - **大小轮转**: 达到 `max_size` 时触发
   - **时间轮转**: `hourly`/`daily`/`weekly` 定期触发
   - **文件命名**: `app_YYYYMMDD_HHMMSS.log.zst.enc`

2. **压缩** (ZSTD, GZIP, Brotli, LZ4):
   ```rust
   fn compress_file(&self, path: &PathBuf) -> Result<PathBuf, InklogError> {
       let compressed_path = path.with_extension("zst");
       let encoder = zstd::stream::Encoder::new(output_file, compression_level)?;
       // 流式压缩避免大文件内存占用
       encoder.finish()?;
   }
   ```

3. **AES-256-GCM 加密**:
   ```rust
   fn encrypt_file(&self, path: &PathBuf) -> Result<PathBuf, InklogError> {
       let key = get_encryption_key_from_env()?;
       let nonce: [u8; 12] = rand::thread_rng().gen();
       let cipher = Aes256Gcm::new((&key).into());
       let ciphertext = cipher.encrypt(nonce_slice, plaintext.as_ref())?;
       // 格式: [nonce 12字节][密文 N字节]
   }
   ```

4. **断路器保护**:
   ```rust
   pub struct CircuitBreaker {
       state: CircuitState,  // Closed | Open | HalfOpen
       failure_count: u32,
       failure_threshold: u32,  // 默认 5
       reset_timeout: Duration,  // 默认 30 秒
   }
   ```

5. **磁盘空间管理**:
   - 检查可用空间 (< 5% 或 < 100MB = 警告)
   - 自动清理旧日志 (删除最旧的 20%)
   - 持久化策略: 按天数 (30 天) 和总大小 (1GB)

**写入流程**:
```
LogRecord
  ↓
检查断路器状态
  ↓ [开启]          [关闭]
写入 Console降级   继续处理
                    ↓
检查磁盘空间
  ↓ [不足]          [充足]
降级到 Console    继续处理
                    ↓
写入 BufWriter<File>
  ↓
检查轮转条件
  ↓ [需轮转]       [无需轮转]
执行轮转/压缩/加密   更新大小
```

### Database Sink

使用 Sea-ORM 的异步批量数据库写入:

**数据模型**:
```rust
pub struct Model {
    pub id: i64,                           // 自增主键
    pub timestamp: DateTimeUtc,               // ISO 8601 格式
    pub level: String,                      // trace/debug/info/warn/error
    pub target: String,                     // 模块路径
    pub message: String,                     // 日志内容
    pub fields: Option<serde_json::Value>,  // 结构化字段
    pub file: Option<String>,                // 源文件
    pub line: Option<i32>,                 // 源行号
    pub thread_id: String,                   // 线程标识
}
```

**批量处理策略**:

```rust
struct DatabaseSink {
    buffer: Vec<LogRecord>,              // 待写入缓冲区
    last_flush: Instant,                   // 上次刷新时间
    rt: Runtime,                          // 专用 tokio 运行时
    db: Option<DatabaseConnection>,         // 连接池
    circuit_breaker: CircuitBreaker,         // 故障保护
}
```

**刷新逻辑**:
```rust
fn flush_buffer(&mut self) -> Result<(), InklogError> {
    // 动态批大小: 半开启状态时减半
    let current_batch_size = if self.circuit_breaker.state() == HalfOpen {
        self.config.batch_size / 2
    } else {
        self.config.batch_size  // 默认 100
    };
    
    // 检查触发条件
    if self.buffer.len() >= current_batch_size 
       || self.last_flush.elapsed() >= flush_interval {
        
        // 执行批量 INSERT
        Entity::insert_many(logs).exec(&db).await?;
        
        self.circuit_breaker.record_success();
        self.buffer.clear();
    }
}
```

**分区表支持**:
- **PostgreSQL**: `CREATE TABLE logs_2026_01 PARTITION OF logs FOR VALUES FROM ('2026-01-01') TO ('2026-02-01')`
- **MySQL**: `CREATE TABLE logs_2026_01 PARTITION OF logs FOR VALUES IN (TO_DAYS('2026-01-01'))`
- **SQLite**: 无分区 (单表)

**Parquet 导出** (用于归档):
```rust
fn convert_logs_to_parquet(logs: &[Model], config: &ParquetConfig) -> Result<Vec<u8>> {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("timestamp", DataType::Utf8, false),
        Field::new("level", DataType::Utf8, false),
        // ...
    ]);
    
    let writer = ArrowWriter::try_new(
        cursor,
        schema,
        Some(writer_props)  // ZSTD 压缩, PLAIN/DICTIONARY 编码
    )?;
    
    writer.write(&batch)?;
    writer.close()?;
}
```

### 自定义 Sinks

实现 `LogSink` trait 即可创建自定义输出:

```rust
pub struct CustomSink {
    endpoint: String,
    client: reqwest::Client,
    buffer: Vec<LogRecord>,
}

impl LogSink for CustomSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError> {
        self.buffer.push(record.clone());
        if self.buffer.len() >= 10 {
            self.flush()?;
        }
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), InklogError> {
        // 发送到远程 API
        let payload = serde_json::to_vec(&self.buffer)?;
        self.client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .await?;
        self.buffer.clear();
        Ok(())
    }
}
```

## 数据流

### 日志消息生命周期

```
1. 应用代码调用 log!() 宏
   ↓
2. tracing 层捕获事件
   ↓
3. LoggerSubscriber::event() 被调用
   ↓
4. 创建 LogRecord 结构:
   {
       timestamp: Utc::now(),
       level: "INFO",
       target: "my_crate::module",
       message: "User logged in",
       fields: HashMap {
           "user_id": 12345,
           "ip": "192.168.1.1"
       },
       file: Some("main.rs"),
       line: Some(42),
       thread_id: "tokio-runtime-worker"
   }
   ↓
5. [可选] 数据脱敏:
   - 检测敏感字段名 (password, token, etc.)
   - 应用正则模式 (邮箱, 电话, 信用卡等)
   ↓
6. 发送到 Crossbeam 通道:
   sender.send(LogRecord)?
   ↓
7. 工作线程接收记录:
   ├─ 文件线程: 写入文件 (带轮转/压缩/加密)
   ├─ 数据库线程: 添加到缓冲区 (批量刷新)
   └─ 控制线程: 写入 stdout/stderr
   ↓
8. 更新 Metrics:
   - metrics.inc_logs_written()
   - metrics.record_latency(latency)
   - metrics.update_sink_health()
   ↓
9. [可选] HTTP 端点暴露健康状态
   GET /health → HealthStatus JSON
   GET /metrics → Prometheus 格式文本
```

### 异步处理管道

```rust
// 应用线程 (异步 tokio 运行时)
log::info!("message");

// LoggerSubscriber (异步)
async fn event(&self, event: &Event) {
    let record = LogRecord::from(event);
    self.sender.send(record)?;  // 非阻塞发送
}

// 工作线程 1: 文件 (阻塞)
loop {
    match receiver.recv_timeout(Duration::from_millis(100)) {
        Ok(record) => {
            let start = Instant::now();
            file_sink.write(&record)?;
            let latency = start.elapsed();
            metrics.record_latency(latency);
        }
        Err(_) => {
            file_sink.flush()?;
        }
    }
}

// 工作线程 2: 数据库 (独立 tokio 运行时)
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(num_cpus::get())
    .thread_name("inklog-db-worker")
    .enable_all()  // 包括 I/O 和时间驱动器
    .build()?;

rt.block_on(async {
    while let Ok(record) = receiver.recv() {
        buffer.push(record);
        if buffer.len() >= batch_size || last_flush.elapsed() >= interval {
            Entity::insert_many(to_active_models(buffer)).exec(db).await?;
            buffer.clear();
        }
    }
});
```

### 错误处理流程

```
1. 错误发生 (file_sink.write() 返回 Err)
   ↓
2. 记录错误到 error.log:
   error_sink.write(&LogRecord {
       level: "ERROR",
       target: "inklog::file_sink",
       message: "Failed to write: {error}",
       ...
   })
   ↓
3. 更新断路器:
   circuit_breaker.record_failure();
   // 失败计数 +1
   // 如果 >= threshold, 状态 → Open
   ↓
4. 触发降级策略:
   [优先级顺序]
   1. 重试 (最多 3 次, 指数退避: 10ms, 20ms, 40ms)
   2. 降级到 ConsoleSink
   3. 记录失败指标
      metrics.inc_sink_error();
      metrics.update_sink_health("file", false, Some(error));
   ↓
5. 健康检查线程检测不健康 sink:
   - 每 10 秒检查一次
   - 如果连续失败 > 3 次且冷却时间 (30秒) 已过:
     * 发送 SinkControlMessage::RecoverSink("file")
   ↓
6. Sink 重新初始化:
   FileSink::new(config.clone())?
   // 创建新文件句柄
   - 重置断路器
   - 清除连续失败计数
   ↓
7. 恢复成功:
   metrics.update_sink_health("file", true, None)
```

## 并发模型

### Tokio 运行时

Inklog 使用混合的异步/阻塞架构:

| 组件          | 运行时类型      | 用途                        |
|---------------|----------------|---------------------------|
| 应用代码      | tokio (多线程) | 异步日志 API            |
| LoggerSubscriber | tokio          | 非阻塞通道发送         |
| DatabaseSink  | 专用 tokio RT  | 数据库批量操作          |
| FileSink      | 阻塞 (OS 线程) | 文件 I/O 和轮转        |
| ConsoleSink    | 阻塞 (OS 线程) | 终端输出               |

**DatabaseSink 独立运行时**:
```rust
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(std::cmp::max(2, num_cpus::get()))
    .thread_name("inklog-db-worker")
    .enable_all()  // 包括 I/O 和时间驱动器
    .build()?;
```

### Crossbeam 通道

高性能多生产者多消费者通道:

```rust
// 主通道
let (sender, receiver) = bounded(channel_capacity);

// 控制通道
let (control_tx, control_rx) = bounded(10);

// 停止通道
let (shutdown_tx, shutdown_rx) = bounded(1);
```

**通道使用**:
- **主日志通道**: `LogRecord` 从 subscriber 到工作线程
- **控制通道**: `SinkControlMessage::RecoverSink("file")` 从健康检查到工作线程
- **停止通道**: `()` 从 shutdown() 到工作线程

### 工作线程池

3 个专用 OS 线程:

```rust
struct WorkerParams {
    config: InklogConfig,
    receiver: Receiver<LogRecord>,      // 共享接收者
    shutdown_rx: Receiver<()>,         // 停止信号
    control_rx: Receiver<SinkControlMessage>,  // 控制命令
    control_tx: Sender<SinkControlMessage>,  // 控制发送
    metrics: Arc<Metrics>,             // 共享指标
    console_sink: Arc<Mutex<ConsoleSink>>,  // 共享控制台
    error_sink: Arc<Mutex<Option<FileSink>>>, // 错误日志
}

// 启动线程
let handles = start_workers(WorkerParams { ... })?;

// Thread 1: 文件 sink
let handle_file = thread::spawn(move || {
    metrics.active_workers.inc();
    // 接收循环...
    metrics.active_workers.dec();
});

// Thread 2: 数据库 sink
let handle_db = thread::spawn(move || {
    metrics.active_workers.inc();
    // 接收循环...
    metrics.active_workers.dec();
});

// Thread 3: 健康检查
let handle_health = thread::spawn(move || {
    loop {
        if shutdown_rx.recv_timeout(Duration::from_secs(10)).is_ok() {
            break;
        }
        // 检查 sink 健康状态
        // 触发恢复如果需要
    }
});
```

## 存储层

### 文件存储

**文件组织**:
```
logs/
├── app.log              # 当前活动日志文件
├── app_20250117_143022.log   # 已轮转日志
├── app_20250117_120000.log.zst  # 已压缩
├── app_20250116_080000.log.zst.enc  # 已加密
├── error.log            # 内部错误日志
└── archive/             # 本地归档目录
    └── 2026/
        └── 01/
            └── archive_20250117.parquet
```

**轮转策略**:
- **大小轮转**: 达到 `max_size` 时立即轮转
- **时间轮转**: `hourly`/`daily`/`weekly` 定时轮转
- **文件名格式**: `{stem}_{timestamp}{ext}`
  - 原始: `app_20250117_143022.log`
  - 压缩: `app_20250117_143022.log.zst`
  - 加密: `app_20250117_143022.log.zst.enc`

**压缩算法**:
| 算法  | 优势                | 压缩比 | 速度    |
|--------|---------------------|--------|----------|
| ZSTD   | 最佳压缩比, 解压快   | ~3.5x  | 快      |
| GZIP   | 最广泛的兼容性        | ~2.5x  | 中等    |
| Brotli | 良好的 Web 兼容性     | ~3.0x  | 慢      |
| LZ4    | 最快的压缩/解压       | ~2.0x  | 最快    |

### 数据库存储

**支持的数据库**:

| 数据库   | 特性                    | Sea-ORM 后端   |
|---------|------------------------|---------------|
| PostgreSQL | 分区表, JSON 字段, 索引 | `DatabaseBackend::Postgres` |
| MySQL     | 分区表, JSON 支持        | `DatabaseBackend::MySql` |
| SQLite    | 单表, 轻量级          | `DatabaseBackend::Sqlite` |

**表结构** (PostgreSQL):
```sql
CREATE TABLE logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMP(3) NOT NULL,
    level VARCHAR(20) NOT NULL,
    target VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    fields JSONB,
    file VARCHAR(512),
    line INTEGER,
    thread_id VARCHAR(100) NOT NULL
);

CREATE INDEX idx_logs_timestamp ON logs(timestamp);
CREATE INDEX idx_logs_level ON logs(level);
CREATE INDEX idx_logs_target ON logs(target);

-- 分区表 (每月)
CREATE TABLE logs_2026_01 PARTITION OF logs
FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
```

**批量写入流程**:
```
LogRecords [100条]
  ↓
转换 ActiveModel
  ↓
Entity::insert_many(active_models).exec(db).await
  ↓
Sea-ORM 批量 INSERT (单次事务)
  ↓
清空缓冲区
```

### S3 云归档

**归档架构**:
```
DatabaseSink/FileSink
  ↓
ArchiveService (tokio-cron-scheduler)
  ↓
每夜 02:00 执行归档任务
  ├─ 从数据库查询日志
  ├─ 转换为 Parquet/JSON
  ├─ 上传到 S3 (multipart upload)
  └─ 记录归档元数据
  ↓
S3 对象存储
  ├─ 前缀: logs/YYYY/MM/
  ├─ 文件名: logs_YYYYMMDD_HHMMSS.parquet
  └─ 存储类别: Glacier (低成本长期存储)
```

**S3 配置**:
```rust
pub struct S3ArchiveConfig {
    pub enabled: bool,
    pub bucket: String,           // 存储桶名称
    pub region: String,           // AWS 区域
    pub archive_interval_days: u32, // 归档间隔
    pub local_retention_days: u32,  // 本地保留
    pub prefix: String,           // 对象键前缀
    pub compression: CompressionType, // ZSTD/GZIP/LZ4/Brotli
    pub storage_class: StorageClass, // Standard/IntelligentTiering/Glacier
    pub encryption: Option<EncryptionConfig>,
    pub max_file_size_mb: u64,
}
```

**重试策略**:
```rust
async fn retry_with_backoff<T, F, Fut>(mut attempt: F) -> Result<T, InklogError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, InklogError>>,
{
    let mut retries = 0;
    let max_retries = 3;
    let base_delay = Duration::from_secs(1);
    
    loop {
        match attempt().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                retries += 1;
                let delay = base_delay * 2_u32.pow(retries - 1); // 1s, 2s, 4s
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## 安全架构

### 加密层

**AES-256-GCM 流程**:

```rust
// 1. 密钥加载 (从环境变量, 使用 zeroize)
let env_value = Zeroizing::new(std::env::var("INKLOG_ENCRYPTION_KEY")?);

// 2. Base64 解码或直接使用原始字节
let key: [u8; 32] = if let Ok(decoded) = base64_decode(env_value) {
    // Base64 格式: 32 字节
    decoded.try_into()?
} else {
    // 原始格式: 截取/填充到 32 字节
    let mut result = [0u8; 32];
    result.copy_from_slice(&env_value.as_bytes()[..32]);
    result
};

// 3. 文件加密
let nonce: [u8; 12] = rand::thread_rng().gen();
let cipher = Aes256Gcm::new((&key).into());
let ciphertext = cipher.encrypt(&nonce, plaintext)?;

// 4. 写入加密文件 (格式: [nonce][密文])
output_file.write_all(&nonce)?;
output_file.write_all(&ciphertext)?;
```

**解密验证** (CLI 工具):
```rust
// 读取文件
let mut file = File::open("encrypted.log.enc")?;
let mut nonce = [0u8; 12];
file.read_exact(&mut nonce)?;
let ciphertext = file.read_to_end()?;

// 解密
let cipher = Aes256Gcm::new((&key).into());
let plaintext = cipher.decrypt(&nonce, &ciphertext)?;
```

### 数据脱敏

**敏感字段检测**:
```rust
static SENSITIVE_FIELDS: &[&str] = &[
    "password", "token", "secret", "api_key", "access_key",
    "aws_secret", "jwt", "oauth_token", "credit_card", "ssn",
    // ... 30+ 字段名模式
];

pub fn is_sensitive_field(field_name: &str) -> bool {
    let lower_name = field_name.to_lowercase();
    SENSITIVE_FIELDS.iter().any(|s| lower_name.contains(*s))
}
```

**正则模式**:

| 类型      | 模式                          | 示例输入          | 输出               |
|---------|-------------------------------|-------------------|-------------------|
| 邮箱    | `[a-zA-Z0-9._%+-]+@.*`     | user@example.com | **@**.***        |
| 电话    | `\b1[3-9]\d{9}\b`         | 13812345678      | ***-****-****    |
| 身份证  | `^\d{6})(\d{8})(\d{3}[\dX])$` | 11010112345678 | ******1234        |
| 信用卡  | `(\d{4})(\d+)(\d{4})`      | 6222001234567890 | ****-****-****    |
| API Key | `api[_-]?key[^\s:=]*\s*[=:]\s*[a-zA-Z0-9_-]{20,}` | api_key=abc123 | ${1}***REDACTED***${3} |
| AWS Key | `(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}` | AKIAIOS...      | ***REDACTED***   |

**脱敏逻辑**:
```rust
fn mask(&self, text: &str) -> String {
    let mut result = text.to_string();
    for rule in &self.rules {
        result = rule.apply(&result);  // 顺序应用所有规则
    }
    result
}
```

### 内存安全

**密钥保护**:
```rust
use zeroize::Zeroizing;

// 环境变量值自动清零内存
let env_value = Zeroizing::new(std::env::var("KEY")?);
// ... 使用密钥 ...
// ... env_value 离开作用域时自动清零内存
```

**文件权限** (Unix):
```rust
use std::os::unix::fs::PermissionsExt;

let mut perms = metadata.permissions();
perms.set_mode(0o600);  // 仅所有者读写
file.set_permissions(perms)?;
```

## 性能考虑

### 批量处理

**FileSink**: 
- 行级写入 (每条日志立即写入)
- 使用 `BufWriter` 减少系统调用

**DatabaseSink**:
```rust
pub struct DatabaseSink {
    buffer: Vec<LogRecord>,
    last_flush: Instant,
}

// 批量刷新条件
if buffer.len() >= batch_size           // 默认 100
   || last_flush.elapsed() >= interval {  // 默认 500ms
    Entity::insert_many(active_models).exec(db).await?;
    buffer.clear();
}
```

**性能对比**:
| 策略         | 数据库事务   | I/O 开销    | 吞吐量    |
|---------------|------------|------------|---------|
| 逐条插入      | N (自动提交)  | 高         | ~100/s  |
| 批量 100 条    | Y (单次)      | 低         | ~10,000/s |

### 压缩策略

**ZSTD 压缩配置**:
```rust
let compression_level = config.compression_level; // 默认 3 (范围 0-22)

let encoder = zstd::stream::Encoder::new(output_file, compression_level)?;
// 写入流式避免内存占用
```

**压缩级别选择**:
- **Level 0**: 最快, 压缩比 ~2.5x
- **Level 3**: 平衡 (默认), 压缩比 ~3.5x
- **Level 19**: 最大压缩, 压缩比 ~4.5x, 慢

### 队列管理

**Crossbeam 通道配置**:
```rust
pub struct PerformanceConfig {
    pub channel_capacity: usize,  // 默认 10000
    pub worker_threads: usize,  // 默认 3
}
```

**队列行为**:
- **有界通道**: 防止内存溢出 (backpressure)
- **发送者阻塞**: `sender.send(record)?` 队列满时阻塞
- **接收者超时**: `recv_timeout(Duration::from_millis(100))`

**指标监控**:
```rust
let channel_len = sender.len();
let channel_cap = sender.capacity()?;
let usage = channel_len as f64 / channel_cap as f64;

// 更新指标
metrics.update_channel_usage(usage);

// Prometheus 导出
inklog_channel_usage 0.45  // 45% 使用率
```

## 扩展点

### 自定义 Sink 开发

**步骤 1**: 实现 `LogSink` trait

```rust
pub struct SlackSink {
    webhook_url: String,
    buffer: Vec<LogRecord>,
}

impl LogSink for SlackSink {
    fn write(&mut self, record: &LogRecord) -> Result<(), InklogError> {
        self.buffer.push(record.clone());
        if self.buffer.len() >= 10 {
            self.flush()?;
        }
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), InklogError> {
        let payload = serde_json::json!({
            "text": self.buffer.iter()
                .filter(|r| r.level == "ERROR" || r.level == "WARN")
                .map(|r| format!("{}: {}", r.target, r.message))
                .collect::<Vec<_>>()
                .join("\n")
        });
        
        reqwest::Client::new()
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?;
        
        self.buffer.clear();
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        self.webhook_url.len() > 0
    }
    
    fn shutdown(&mut self) -> Result<(), InklogError> {
        self.flush()
    }
}
```

**步骤 2**: 注册到 LoggerManager (需修改核心)

```rust
// 在 LoggerManager::start_workers() 中添加
let handle_slack = thread::spawn(move || {
    metrics.active_workers.inc();
    if let Ok(mut sink) = SlackSink::new(config.slack_config) {
        loop {
            if shutdown_rx.try_recv().is_ok() {
                // 排空队列, 最多 30 秒
                let deadline = Instant::now() + Duration::from_secs(30);
                while let Ok(record) = rx_slack.try_recv() {
                    let _ = sink.write(&record);
                    if Instant::now() > deadline {
                        break;
                    }
                }
                let _ = sink.shutdown();
                break;
            }
        }
    }
    metrics.active_workers.dec();
});
```

**步骤 3**: 配置支持

```rust
pub struct SlackSinkConfig {
    pub enabled: bool,
    pub webhook_url: String,
    pub batch_size: usize,
}

// 添加到 InklogConfig
pub struct InklogConfig {
    // ...
    pub slack_sink: Option<SlackSinkConfig>,
}
```

## 依赖

### 核心依赖

| 依赖              | 版本      | 用途                           |
|-------------------|----------|------------------------------|
| tokio             | 1.x     | 异步运行时                      |
| tracing           | 0.1.x    | 结构化日志层                   |
| tracing-subscriber| 0.3.x    | 实现 Subscriber                  |
| serde             | 1.x     | 序列化                         |
| chrono            | 0.4.x    | 时间戳                         |
| crossbeam         | 0.8      | 高性能通道和同步原语         |
| regex             | 1.10     | 数据脱敏                       |
| thiserror          | 1.x     | 错误类型                       |
| anyhow            | 1.x     | 错误上下文                     |

### 可选依赖 (特性门控)

**AWS 特性** (`aws`):
| 依赖                    | 用途                      |
|-----------------------|--------------------------|
| aws-sdk-s3            | S3 对象存储               |
| aws-config             | AWS 配置加载             |
| aws-types              | AWS 类型定义               |
| tokio-cron-scheduler   | 定时归档调度             |

**HTTP 特性** (`http`):
| 依赖   | 用途              |
|--------|------------------|
| axum   | HTTP 服务器      |
| serde  | JSON 响应       |

**数据库特性** (默认):
| 依赖      | 用途                    |
|----------|------------------------|
| sea-orm  | ORM 层               |
| 数据库驱动 | sqlx-postgres/sqlx-mysql/sqlx-sqlite |

**压缩特性**:
| 依赖    | 用途          |
|---------|-------------|
| zstd    | ZSTD 压缩   |
| flate2  | GZIP 压缩    |
| brotli  | Brotli 压缩  |
| lz4     | LZ4 压缩     |

**安全特性**:
| 依赖      | 用途          |
|----------|-------------|
| aes-gcm | AES-256-GCM 加密 |
| zeroize  | 安全内存清零    |
| owo-colors| ANSI 颜色输出   |

### 测试依赖

| 依赖         | 用途              |
|-------------|------------------|
| serial_test | 测试隔离          |
| tempfile   | 临时文件          |
| assert_cmd | CLI 命令测试     |

---

**文档版本**: 1.0  
**最后更新**: 2026-01-17  
**代码基准**: commit b7c5e6e
