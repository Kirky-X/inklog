# 🧪 TEST - inklog 测试文档 (Test Specification)

## 1. 测试策略

### 1.1 项目简介

inklog 是一个企业级Rust日志基础设施，提供高性能、高可靠、可扩展的日志记录能力。本文档详细描述了 inklog 的测试策略和用例。

### 1.1 测试金字塔

```
         /\
        /  \  E2E测试 (5%)
       /────\
      / 集成  \ 集成测试 (15%)
     /  测试   \
    /──────────\
   /   单元测试  \ 单元测试 (80%)
  /──────────────\
```

### 1.2 测试环境

| 环境     | 用途         | 数据库         | S3     |
| -------- | ------------ | -------------- | ------ |
| 单元测试 | 组件隔离测试 | Mock           | Mock   |
| 集成测试 | 多组件协作   | TestContainers | MinIO  |
| 性能测试 | 压力测试     | 真实DB         | 真实S3 |

------

## 2. 单元测试用例

### 2.1 Console Sink测试

| 用例ID    | 测试场景   | 输入             | 预期输出         | 优先级 |
| --------- | ---------- | ---------------- | ---------------- | ------ |
| UT-CS-001 | 基础输出   | `info!("test")`  | stdout包含"test" | P0     |
| UT-CS-002 | 彩色渲染   | `error!("fail")` | 红色输出         | P1     |
| UT-CS-003 | stderr分流 | `error!("err")`  | 输出到stderr     | P1     |
| UT-CS-004 | 格式模板   | 自定义模板       | 按模板格式化     | P0     |
| UT-CS-005 | 非TTY环境  | 重定向到文件     | 无彩色代码       | P1     |
| UT-CS-006 | 并发安全   | 10线程同时写     | 无数据混乱       | P0     |

**测试代码示例**：

```rust
#[test]
fn test_console_colored_output() {
    let mut sink = ConsoleSink::new(config);
    let record = LogRecord {
        level: Level::ERROR,
        message: "test error".into(),
        ..Default::default()
    };
    
    let output = capture_stdout(|| {
        sink.write(&record).unwrap();
    });
    
    assert!(output.contains("\x1b[31m")); // 红色ANSI码
    assert!(output.contains("test error"));
}

#[test]
fn test_config_validation_success() {
    let config = InklogConfig {
        global: GlobalConfig {
            level: "info".into(),
            enable_console: true,
            enable_file: Some("logs/app.log".into()),
            ..Default::default()
        },
        performance: PerformanceConfig {
            channel_capacity: 1000,
            worker_threads: 3,
            ..Default::default()
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_failure() {
    let config = InklogConfig {
        global: GlobalConfig {
            level: "invalid_level".into(), // 无效级别
            ..Default::default()
        },
        performance: PerformanceConfig {
            channel_capacity: 50, // 太小
            ..Default::default()
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_err());
    assert!(matches!(config.validate(), Err(InklogError::ConfigError(_))));
}

#[test]
fn test_builder_mode() {
    let logger = LoggerManager::builder()
        .level("debug")
        .enable_console(true)
        .enable_file("test.log")
        .channel_capacity(5000)
        .build()
        .unwrap();
    
    assert!(logger.is_initialized());
}

#[test]
fn test_dual_initialization() {
    // 方式1: 直接初始化（零依赖）
    let logger1 = LoggerManager::new().unwrap();
    assert!(logger1.is_initialized());
    
    // 方式2: 配置文件初始化（需要confers特性）
    #[cfg(feature = "confers")]
    {
        let logger2 = LoggerManager::from_file("test_config.toml").unwrap();
        assert!(logger2.is_initialized());
    }
}

#[test]
fn test_feature_flag_compilation() {
    // 测试条件编译是否正确
    #[cfg(feature = "confers")]
    {
        // confers特性启用时的测试
        let config = InklogConfig::load_from("config.toml").unwrap();
        assert!(config.validate().is_ok());
    }
    
    #[cfg(not(feature = "confers"))]
    {
        // confers特性禁用时的测试
        let config = InklogConfig::default();
        assert!(config.validate().is_ok());
    }
}
```

### 2.2 File Sink测试

| 用例ID    | 测试场景   | 输入         | 预期输出               | 优先级 |
| --------- | ---------- | ------------ | ---------------------- | ------ |
| UT-FS-001 | 基础写入   | 单条日志     | 文件存在且内容正确     | P0     |
| UT-FS-002 | 大小轮转   | 写入101MB    | 生成2个文件            | P0     |
| UT-FS-003 | 时间轮转   | 跨天写入     | 生成带日期的文件       | P0     |
| UT-FS-004 | 压缩功能   | 轮转后       | 生成.zst文件           | P0     |
| UT-FS-005 | 加密功能   | 轮转+加密    | 文件以magic header开头 | P1     |
| UT-FS-006 | 文件清理   | keep_files=3 | 仅保留3个最新文件      | P1     |
| UT-FS-007 | 磁盘满处理 | ENOSPC错误   | 返回错误不panic        | P0     |

**测试代码示例**：

```rust
#[test]
fn test_file_rotation_by_size() {
    let temp_dir = TempDir::new().unwrap();
    let config = FileSinkConfig {
        path: temp_dir.path().join("test.log"),
        max_size: "1MB".into(),
        ..Default::default()
    };
    
    let mut sink = FileSink::new(config).unwrap();
    
    // 写入2MB数据
    for _ in 0..2000 {
        let record = create_test_record(1024); // 1KB/条
        sink.write(&record).unwrap();
    }
    
    // 验证生成了2个文件
    let files = fs::read_dir(temp_dir.path()).unwrap();
    assert_eq!(files.count(), 2);
}
```

### 2.3 Database Sink测试

| 用例ID    | 测试场景     | 输入            | 预期输出        | 优先级 |
| --------- | ------------ | --------------- | --------------- | ------ |
| UT-DB-001 | 单条写入     | 1条日志         | 数据库有1条记录 | P0     |
| UT-DB-002 | 批量写入     | 100条日志       | 触发1次INSERT   | P0     |
| UT-DB-003 | 超时刷新     | 10条+等待600ms  | 触发flush       | P0     |
| UT-DB-004 | 事务回滚     | 插入失败        | 不丢失数据      | P0     |
| UT-DB-005 | 连接池耗尽   | 并发写入        | 阻塞等待连接    | P1     |
| UT-DB-006 | 跨数据库兼容 | SQLite/PG/MySQL | 都能正常写入    | P0     |

### 2.4 Config模块测试

| 用例ID    | 测试场景         | 输入                    | 预期输出           | 优先级 |
| --------- | ---------------- | ----------------------- | ------------------ | ------ |
| UT-CF-001 | 默认配置         | 无参数                  | 配置加载成功       | P0     |
| UT-CF-002 | 配置文件加载     | valid.toml              | 配置加载成功       | P0     |
| UT-CF-003 | 无效配置         | invalid.toml            | 返回错误           | P0     |
| UT-CF-004 | 环境变量覆盖     | LOG_LEVEL=debug         | 级别被覆盖         | P1     |
| UT-CF-005 | 配置验证成功     | 有效参数                | 验证通过           | P0     |
| UT-CF-006 | 配置验证失败     | 无效参数                | 返回ConfigError    | P0     |
| UT-CF-007 | Builder模式      | 链式调用                | 配置构建成功       | P0     |
| UT-CF-008 | 双初始化方式     | new() vs from_file()    | 两种都成功         | P0     |
| UT-CF-009 | Feature标志测试  | #[cfg(feature="confers")] | 条件编译正确   | P1     |

**测试代码示例**：

```rust
#[tokio::test]
async fn test_database_batch_insert() {
    let db = setup_test_db().await;
    let mut sink = DatabaseSink::new(db, config);
    
    // 写入99条 (不触发batch_size=100)
    for _ in 0..99 {
        sink.write(&create_test_record()).unwrap();
    }
    assert_eq!(count_logs(&db).await, 0); // 未flush
    
    // 第100条触发批量写入
    sink.write(&create_test_record()).unwrap();
    assert_eq!(count_logs(&db).await, 100);
}
```

------

## 3. 集成测试用例

### 3.1 多Sink协作测试

| 用例ID    | 测试场景        | 验证点                  | 优先级 |
| --------- | --------------- | ----------------------- | ------ |
| IT-MS-001 | 同时启用3个Sink | Console+File+DB都有输出 | P0     |
| IT-MS-002 | DB失败降级      | 写入db_fallback.log     | P0     |
| IT-MS-003 | File失败降级    | 仅Console输出           | P0     |
| IT-MS-004 | Sink独立故障    | 一个失败不影响其他      | P0     |

### 3.2 配置集成测试

| 用例ID    | 测试场景           | 操作步骤                     | 预期结果               |
| --------- | ------------------ | ---------------------------- | ---------------------- |
| IT-CF-001 | 配置加载优先级     | env→file→default             | 环境变量优先级最高     |
| IT-CF-002 | 配置验证失败处理   | 提供无效配置                 | 优雅降级到默认配置     |
| IT-CF-003 | Builder模式集成    | 使用Builder构建并初始化      | 系统正常启动           |
| IT-CF-004 | Feature开关测试    | 切换confers特性编译          | 条件编译正确           |
| IT-CF-005 | 配置文件热重载     | 修改配置文件                 | 配置自动更新           |

**配置集成测试代码示例**:

```rust
#[test]
fn test_config_loading_priority() {
    // 设置环境变量
    env::set_var("INKLOG_LEVEL", "debug");
    env::set_var("INKLOG_ENABLE_CONSOLE", "true");
    
    // 加载配置（应该优先使用环境变量）
    #[cfg(feature = "confers")]
    {
        let config = InklogConfig::load_from_env_and_file("config.toml").unwrap();
        assert_eq!(config.global.level, "debug");
        assert_eq!(config.global.enable_console, true);
    }
    
    // 清理环境变量
    env::remove_var("INKLOG_LEVEL");
    env::remove_var("INKLOG_ENABLE_CONSOLE");
}

#[test]
fn test_config_validation_fallback() {
    // 提供无效的配置文件
    std::fs::write("invalid_config.toml", "invalid toml content").unwrap();
    
    // 应该优雅降级到默认配置
    #[cfg(feature = "confers")]
    {
        let result = InklogConfig::load_from("invalid_config.toml");
        assert!(result.is_err());
        
        // 使用默认配置
        let default_config = InklogConfig::default();
        assert!(default_config.validate().is_ok());
    }
    
    // 清理测试文件
    std::fs::remove_file("invalid_config.toml").ok();
}
```

### 3.3 端到端测试

| 用例ID  | 测试场景     | 操作步骤            | 预期结果        |
| ------- | ------------ | ------------------- | --------------- |
| E2E-001 | 完整生命周期 | 初始化→写入→关闭    | 所有日志落盘    |
| E2E-002 | 优雅关闭     | 写入中途发送SIGTERM | 等待30秒后关闭  |
| E2E-003 | S3归档流程   | 触发归档任务        | 文件上传+DB清理 |
| E2E-004 | 加密解密验证 | 加密后手动解密      | 内容一致        |
| E2E-005 | 双初始化方式 | new()和from_file()  | 两种都正常工作  |

### IT-ER-001: 数据库故障恢复

**测试步骤**：
1. 启动系统，验证DB写入正常
2. 停止数据库服务（模拟故障）
3. 观察系统行为：
   - 3秒内应自动降级到File
   - error.log记录降级事件
4. 重启数据库
5. 观察10秒内自动恢复

**预期结果**：
- 故障期间所有日志写入fallback文件
- 恢复后继续DB写入
- 无日志丢失（对比总数）

**测试代码示例**：

```rust
#[test]
fn test_graceful_shutdown() {
    let logger = LoggerManager::init("test_config.toml").unwrap();
    
    // 启动写入线程
    let handle = thread::spawn(|| {
        for i in 0..1000 {
            info!("log {}", i);
        }
    });
    
    // 等待100ms后发送关闭信号
    thread::sleep(Duration::from_millis(100));
    logger.shutdown(Duration::from_secs(30));
    
    handle.join().unwrap();
    
    // 验证所有日志都写入
    let log_count = count_file_lines("logs/app.log");
    assert_eq!(log_count, 1000);
}

#[test]
fn test_dual_initialization_e2e() {
    // 测试方式1: 直接初始化（零依赖）
    {
        let logger = LoggerManager::new().unwrap();
        info!("test direct initialization");
        logger.shutdown(Duration::from_secs(5));
    }
    
    // 测试方式2: 配置文件初始化（需要confers特性）
    #[cfg(feature = "confers")]
    {
        // 创建测试配置文件
        std::fs::write("test_dual.toml", r#"
            [global]
            level = "debug"
            enable_console = true
            
            [performance]
            channel_capacity = 1000
        "#).unwrap();
        
        let logger = LoggerManager::from_file("test_dual.toml").unwrap();
        info!("test file-based initialization");
        logger.shutdown(Duration::from_secs(5));
        
        // 清理测试文件
        std::fs::remove_file("test_dual.toml").ok();
    }
}
```

------

## 4. 性能测试

### 4.1 吞吐量测试

| 测试场景     | 目标QPS | 持续时间 | 通过标准          |
| ------------ | ------- | -------- | ----------------- |
| 仅Console    | 10,000  | 10秒     | CPU<10%, 无丢失   |
| Console+File | 5,000   | 30秒     | 延迟<5ms          |
| 全开(C+F+DB) | 500     | 60秒     | Channel使用率<80% |

**测试工具**：

```bash
# 使用criterion.rs进行基准测试
cargo bench --bench throughput

# 输出示例:
# console_only     time: [45.2 µs 46.1 µs 47.3 µs]
# with_file        time: [1.82 ms 1.89 ms 1.97 ms]
# all_sinks        time: [3.21 ms 3.34 ms 3.49 ms]
```

### 4.2 压力测试

| 测试场景   | 配置                    | 预期行为               |
| ---------- | ----------------------- | ---------------------- |
| Channel满  | 10,000容量,发送20,000条 | 发送线程阻塞,不丢失    |
| 磁盘满     | 写入到满盘              | 返回错误,降级到Console |
| DB连接断开 | 中途断开连接            | 自动重连,降级备份      |
| 内存泄漏   | 运行24小时              | 内存增长<50MB          |

**测试代码示例**：

```rust
#[test]
fn test_backpressure() {
    let logger = LoggerManager::init_with_capacity(1000);
    
    // 并发发送10,000条日志
    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..1000 {
                    info!("stress test");
                }
            })
        })
        .collect();
    
    for h in handles {
        h.join().unwrap();
    }
    
    logger.shutdown(Duration::from_secs(60));
    
    // 验证无丢失
    assert_eq!(count_all_logs(), 10_000);
}
```

### 4.3 并发安全测试

| 测试场景   | 线程数 | 操作             | 验证点       |
| ---------- | ------ | ---------------- | ------------ |
| 多线程写入 | 50     | 每个写入1000条   | 无数据竞争   |
| 竞争轮转   | 10     | 同时触发轮转     | 文件名不冲突 |
| 并发关闭   | 100    | 同时调用shutdown | 无panic      |

------

## 6. 兼容性测试用例

### CT-OS-001: Ubuntu运行验证
**环境**：Ubuntu 22.04, Rust 1.75  
**操作**：运行完整测试套件  
**预期**：所有测试通过，无平台特定错误

### CT-DB-001: PostgreSQL版本兼容
**环境**：PG 12 vs PG 16  
**操作**：同样的日志写入100条  
**预期**：两个版本表结构一致，数据完整

### CT-RT-001: Rust版本兼容
**环境**：Rust 1.70 (MSRV)  
**操作**：cargo build --release  
**预期**：编译成功，无deprecation警告

------

## 5. 测试覆盖率要求

| 模块          | 行覆盖率 | 分支覆盖率 | 门禁标准   |
| ------------- | -------- | ---------- | ---------- |
| LoggerManager | ≥90%     | ≥85%       | 阻断发布   |
| Console Sink  | ≥85%     | ≥80%       | 阻断发布   |
| File Sink     | ≥90%     | ≥85%       | 阻断发布   |
| Database Sink | ≥85%     | ≥80%       | 阻断发布   |
| Config模块    | ≥95%     | ≥90%       | 阻断发布   |
| **整体项目**  | **≥85%** | **≥80%**   | **CI门禁** |

### 5.1 配置测试特殊要求

**配置验证覆盖率**:
- 所有配置字段验证逻辑必须100%覆盖
- Builder模式的所有链式调用组合必须测试
- 双初始化方式的代码路径必须完全覆盖
- Feature标志的条件编译必须分别测试

**配置测试命令**:
```bash
# 测试默认配置（无confers特性）
cargo test --no-default-features

# 测试confers特性启用
cargo test --features confers

# 测试所有特性组合
cargo test --all-features
```

**测试命令**：

```bash
# 生成覆盖率报告
cargo tarpaulin --out Html --output-dir coverage/
```

**CI门禁规则**：

```yaml
# .github/workflows/test.yml
- name: Check Coverage
  run: |
    cargo tarpaulin --out Xml
    if [ $(grep 'line-rate' coverage.xml | awk -F'"' '{print $2*100}') -lt 85 ]; then
      echo "❌ Coverage below 85%"
      exit 1
    fi
```

### 6.1 CLI工具测试（需confers特性）

| 用例ID    | 测试场景         | 命令参数                     | 预期结果               |
| --------- | ---------------- | ---------------------------- | ---------------------- |
| UT-CLI-001 | 生成配置模板     | `inklog generate`            | 生成完整模板文件       |
| UT-CLI-002 | 生成最小模板     | `inklog generate --level minimal` | 生成最小配置模板   |
| UT-CLI-003 | 验证有效配置     | `inklog validate -c valid.toml` | 返回成功消息       |
| UT-CLI-004 | 验证无效配置     | `inklog validate -c invalid.toml` | 返回错误消息     |
| UT-CLI-005 | 自定义输出路径   | `inklog generate -o custom.toml` | 生成到指定路径    |

**CLI测试代码示例**:

```rust
#[test]
#[cfg(feature = "confers")]
fn test_cli_generate_template() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "inklog-cli", "--features", "confers", "--", "generate", "-o", "test_template.toml"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    assert!(Path::new("test_template.toml").exists());
    
    // 验证生成的文件内容
    let content = std::fs::read_to_string("test_template.toml").unwrap();
    assert!(content.contains("[global]"));
    assert!(content.contains("[performance]"));
    
    // 清理测试文件
    std::fs::remove_file("test_template.toml").ok();
}

#[test]
#[cfg(feature = "confers")]
fn test_cli_validate_config() {
    // 创建有效的配置文件
    std::fs::write("test_valid.toml", r#"
        [global]
        level = "info"
        enable_console = true
        
        [performance]
        channel_capacity = 1000
    "#).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "inklog-cli", "--features", "confers", "--", "validate", "-c", "test_valid.toml"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("✅ Configuration is valid"));
    
    // 清理测试文件
    std::fs::remove_file("test_valid.toml").ok();
}
```

# CI要求
# - 单元测试必须全部通过
# - 覆盖率不低于85%
# - 性能测试不退化>5%
# - 配置测试覆盖率不低于95%

------

## 7. 迁移指南

### 7.1 版本对比

| 功能 | 旧版本 | 新版本（默认） | 新版本（confers） |
|------|--------|---------------|------------------|
| 默认配置 | `init(None)` | `new()` | `load()` |
| 指定配置文件 | `init("config.toml")` | N/A | `from_file("config.toml")` |
| Builder模式 | ❌ 不支持 | ✅ `builder()` | ✅ `builder()` |
| 零依赖 | ❌ | ✅ | ✅ |
| 配置文件支持 | ✅ | ❌ | ✅ |
| 环境变量配置 | ✅ | ❌ | ✅ |
| CLI工具 | ❌ | ❌ | ✅ |

### 7.2 测试迁移步骤

#### 测试双重初始化

```rust
#[test]
fn test_dual_initialization_migration() {
    // 旧版本测试方式
    // let logger = LoggerManager::init(None).unwrap();
    
    // 新版本测试方式1: 直接初始化（零依赖）
    let logger1 = LoggerManager::new().unwrap();
    assert!(logger1.is_initialized());
    
    // 新版本测试方式2: 配置文件初始化（需要confers特性）
    #[cfg(feature = "confers")]
    {
        let logger2 = LoggerManager::from_file("test_config.toml").unwrap();
        assert!(logger2.is_initialized());
    }
}
```

#### 测试Builder模式

```rust
#[test]
fn test_builder_mode_migration() {
    // 旧版本不支持Builder模式
    // 新版本支持链式配置
    let logger = LoggerManager::builder()
        .level("debug")
        .enable_console(true)
        .enable_file("test.log")
        .channel_capacity(5000)
        .build()
        .unwrap();
    
    assert!(logger.is_initialized());
}
```

#### 测试特性标志

```rust
#[test]
fn test_feature_flag_migration() {
    // 测试条件编译是否正确
    #[cfg(feature = "confers")]
    {
        // confers特性启用时的测试
        let config = InklogConfig::load_from("config.toml").unwrap();
        assert!(config.validate().is_ok());
    }
    
    #[cfg(not(feature = "confers"))]
    {
        // confers特性禁用时的测试
        let config = InklogConfig::default();
        assert!(config.validate().is_ok());
    }
}
```

### 7.3 CLI工具测试迁移

```rust
#[test]
#[cfg(feature = "confers")]
fn test_cli_tools_migration() {
    // 新版本添加了CLI工具测试
    let output = Command::new("cargo")
        .args(&["run", "--bin", "inklog-cli", "--features", "confers", "--", "generate"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
}
```

### 7.4 测试覆盖率迁移要求

**双重初始化覆盖率**:
- `LoggerManager::new()` 必须100%覆盖
- `LoggerManager::from_file()` 必须100%覆盖
- Builder模式的所有方法必须100%覆盖

**特性标志覆盖率**:
- `#[cfg(feature = "confers")]` 代码路径必须完全覆盖
- `#[cfg(not(feature = "confers"))]` 代码路径必须完全覆盖

**CLI工具覆盖率**:
- 所有CLI命令必须测试
- 错误处理路径必须测试

### 7.5 特性配置

在 `Cargo.toml` 中配置测试依赖：

```toml
[dev-dependencies]
# 测试零依赖版本
cargo test --no-default-features

# 测试confers特性版本
cargo test --features confers

# 测试所有特性组合
cargo test --all-features
```

### 7.6 注意事项

1. **零依赖版本**测试不需要配置文件相关测试
2. **confers特性**版本需要额外的CLI工具测试
3. **Builder模式**测试在两个版本中都可以运行
4. 迁移后测试更加模块化，区分了不同特性的测试场景