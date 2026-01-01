# ❌ UAT - inklog 验收测试文档 (User Acceptance Testing)

## 1. 验收标准

### 1.1 项目简介

inklog 是一个企业级Rust日志基础设施，提供高性能、高可靠、可扩展的日志记录能力。本文档详细描述了 inklog 的验收测试标准。

### 1.1 功能验收清单

| 功能模块          | 验收条件                             | 测试方法                             | 状态 |
| ----------------- | ------------------------------------ | ------------------------------------ | ---- |
| **基础日志**      |                                      |                                      |      |
| 日志宏支持        | info!/error!/warn!/debug!/trace!可用 | 手动调用各宏                         | ✅ 已实现 (通过log crate或tracing crate) |
| 日志级别过滤      | 只输出>=配置级别的日志               | 设置level=warn,验证info不输出        | ✅ 已实现    |
| 结构化字段        | key=value格式正常解析                | info!(user=123, "test")              | ✅ 已实现    |
| **Console Sink**  |                                      |                                      |      |
| 彩色输出          | ERROR红色,WARN黄色,INFO绿色          | 目视检查终端                         | ✅ 已实现 (通过owo-colors库) |
| stderr分流        | ERROR/WARN输出到stderr               | 重定向测试                           | ✅ 已实现 (stderr_levels配置) |
| 非TTY兼容         | 管道输出时无彩色代码                 | `app | cat`                         | ✅ 已实现 (is_terminal()检测) |
| 格式模板          | 自定义格式正确渲染                   | 配置template验证                         | ✅ 已实现 |
| **File Sink**     |                                      |                                      |      |
| 基础写入          | 日志写入文件                         | 检查文件内容                         | ✅ 已实现 (基础文件写入) |
| 大小轮转          | 达到100MB时轮转                      | 写入大量日志验证                     | ✅ 已实现 (基于size_threshold配置) |
| 时间轮转          | 定时器驱动的跨天自动轮转             | 修改系统时间测试                     | ✅ 已实现 (定时器驱动精确时间轮转) |
| 文件压缩          | 生成.zst压缩文件                     | 检查文件扩展名和大小                 | ✅ 已实现 (zstd压缩) |
| 文件加密          | 加密文件无法直接读取                 | cat查看文件乱码                      | ✅ 已实现 (AES-256-GCM加密) |
| 加密文件格式      | Magic Header正确                     | hexdump验证                          | ✅ 已实现 (MAGIC_HEADER + VERSION + ALGO) |
| 解密兼容性        | 旧版本能解密v1.0文件                 | 向后兼容测试                         | ✅ 已实现 (decrypt_file_compatible自动检测版本格式) |
| 解密工具          | 能正确解密日志                       | `inklog-cli decrypt`                 | ✅ 已实现 (inklog-cli decrypt命令已实现,支持文件/目录/批量解密) |
| 历史清理          | 保留N个文件,删除旧文件               | 验证文件数量                         | ✅ 已实现 (多维度保留策略:时间+数量+总大小) |
| **Database Sink** |                                      |                                      |      |
| 超时刷新          | 500ms触发flush                       | 写少量日志等待                       | ✅ 已实现    |
| 跨库兼容          | SQLite/PG/MySQL都能用                | 切换driver测试                       | ✅ 已实现 (代码已完整支持3种数据库) |
| 表分区            | 按月自动分区                         | 检查表结构                           | ✅ 已实现    |
| S3归档            | 30天后归档到S3                       | 模拟时间流逝                         | ✅ 已实现 (需要aws feature flag) |
| **可靠性**        |                                      |                                      |      |
| 故障降级          | DB失败时写fallback文件               | 断开数据库连接                       | ✅ 已实现    |
| 健康恢复          | DB恢复后自动重连                     | 重启数据库观察                       | ✅ 已实现    |
| 背压控制          | Channel满时阻塞不丢失                | 压力测试                             | ✅ 已实现    |
| **配置管理**      |                                      |                                      |      |
| 双初始化方式      | new()和from_file()都可用             | 分别测试两种方式                     | ✅ 已实现    |
| Builder模式       | 链式调用构建配置                     | 测试链式调用                         | ✅ 已实现    |
| 配置验证          | 无效配置返回错误                     | 输入无效参数                         | ✅ 已实现    |
| 环境变量配置      | 环境变量覆盖文件配置                 | 设置环境变量测试                     | ✅ 已实现 (完整环境变量覆盖支持) |
| CLI工具           | generate/validate/decrypt命令可用    | 运行CLI命令                          | ✅ 已实现 (inklog-cli完整支持)
| **敏感信息过滤**  | **字段脱敏（如`password=***`）**     | **配置测试**                         | **✅ 已实现** |
|                   | Regex脱敏（邮箱/手机/身份证/银行卡） | 配置enable_regex=true                | ✅ 已实现 (完整正则脱敏支持)

### 1.2 性能验收标准

| 指标            | 目标值   | 测试方法             | 实测值  | 通过  |
| --------------- | -------- | -------------------- | ------- | ----- |
| Console延迟     | <50μs    | Criterion bench      | ~1.05μs | ✅ 已通过    |
| File写入延迟    | <2ms     | 异步测量             | N/A     | ⬜     |
| 吞吐量(常规)    | 5条/秒   | 持续60秒             | >900K   | ✅ 已通过    |
| 吞吐量(峰值)    | 500条/秒 | 持续10秒             | ~3.6M   | ✅ 已通过    |
| **压力测试**    | **长期高并发** | **8线程持续写入**    | **~3.6M ops/s** | **✅ 已通过** |
| **CPU占用**     | **<5%**  | **top命令**          | **___** | **⬜** |
| **内存占用**    | **<30MB**| **/proc/PID/status** | **___** | **⬜** |
| 压缩比(zstd)    | >70%     | 文件大小对比         | ___     | ⬜     |
| Channel使用率   | <80%     | 监控指标             | ___     | ⬜     |

------

## 2. 用户场景验收

### Scenario 1: 开发者快速集成

**用户角色**：后端开发工程师
**前置条件**：已安装Rust 1.70+

**操作步骤**：

1. 添加依赖到Cargo.toml
2. 创建inklog_config.toml
3. 在main.rs初始化Logger
4. 使用info!宏输出日志

**验收标准**：

-  5分钟内完成集成
-  首次编译成功
-  看到彩色控制台输出
-  生成日志文件

**实际结果**：

```
测试时间: ___分钟
遇到的问题: _______________
解决方案: _______________
```

------

### Scenario 2: 运维人员故障排查

**用户角色**：SRE工程师
**前置条件**：系统已运行24小时

**操作步骤**：

1. 连接数据库
2. 执行SQL查询错误日志：

```sql
   SELECT * FROM logs 
   WHERE level='ERROR' 
     AND timestamp > now() - INTERVAL '1 hour'
   ORDER BY timestamp DESC;
```

1. 根据thread_id追踪请求链路
2. 查看压缩日志文件（解密工具）

**验收标准**：

-  查询响应时间<100ms
-  结构化字段可单独过滤
-  解密工具能正常使用
-  日志内容完整可读

------

### Scenario 3: 合规审计

**用户角色**：安全审计员
**前置条件**：需要查看3个月前的日志

**操作步骤**：

1. 从S3下载归档文件
2. 解压Parquet文件
3. 使用工具查询特定用户的操作日志
4. 验证日志完整性（无篡改）

**验收标准**：

-  S3文件可正常下载
-  Parquet文件可用工具打开
-  日志内容与加密文件一致
-  提供审计报告模板

### Scenario 4: 配置管理验收

**用户角色**：系统管理员
**前置条件**：需要配置不同的日志策略

**操作步骤**：

1. **方式1 - 直接初始化（零依赖）**:
   ```rust
   let logger = LoggerManager::new()?;
   ```

2. **方式2 - Builder模式配置**:
   ```rust
   let logger = LoggerManager::builder()
       .level("info")
       .enable_console(true)
       .enable_file("app.log")
       .channel_capacity(5000)
       .build()?;
   ```

3. **方式3 - 配置文件初始化（需confers特性）**:
   ```rust
   let logger = LoggerManager::from_file("inklog.toml")?;
   ```

4. **CLI工具使用**:
   ```bash
   # 生成配置模板
   inklog generate -o config.toml
   
   # 验证配置文件
   inklog validate -c config.toml
   ```

**验收标准**：

-  三种初始化方式都正常工作
-  Builder模式链式调用流畅
-  配置验证能检测无效参数
-  CLI工具生成有效配置模板
-  环境变量能覆盖配置文件设置
-  配置变更后系统能正常响应

------

## 3. 压力测试验收

### Test 1: 持续高负载

**测试环境**：

- CPU: 4核
- 内存: 8GB
- 磁盘: SSD 100GB

**测试场景**：

```bash
# 启动应用
cargo run --release

# 压测工具发送日志
for i in {1..100000}; do
  echo "info!('test log {}')" | nc localhost 8080
done
```

**测试脚本示例**：

```bash
#!/bin/bash
# performance_test.sh

# 启动应用
cargo run --release &
PID=$!

# 预热5秒
sleep 5

# 发送500条/秒，持续10秒
for i in {1..5000}; do
  echo "test log $i" | nc localhost 8080
  sleep 0.002 # 2ms间隔 = 500QPS
done

# 采集性能数据
CPU=$(top -b -n 1 -p $PID | tail -1 | awk '{print $9}')
MEM=$(cat /proc/$PID/status | grep VmRSS | awk '{print $2/1024}')

echo "CPU: $CPU%"
echo "Memory: ${MEM}MB"

# 验证门禁
if (( $(echo "$CPU > 5" | bc -l) )); then
  echo "❌ CPU超标"
  exit 1
fi
```

**监控指标**：

- [ ] 吞吐量稳定在500QPS
- [ ] Channel使用率<80%
- [ ] 无内存泄漏（运行1小时）
- [ ] CPU占用<10%

---

### Test 2: 故障注入

| 故障类型   | 注入方法   | 预期行为       | 实际行为 | 通过 |
| ---------- | ---------- | -------------- | -------- | ---- |
| 数据库断开 | 停止DB服务 | 写fallback文件 | ___      | ⬜    |
| 磁盘满     | dd填满磁盘 | 降级到Console  | ___      | ⬜    |
| 网络抖动   | tc添加延迟 | 自动重连       | ___      | ⬜    |
| 进程崩溃   | kill -9    | 重启后恢复     | ___      | ⬜    |
| 配置错误   | 无效配置   | 降级到默认配置 | ___      | ⬜    |
| 配置热重载 | 修改配置   | 自动更新       | ___      | ⬜    |

### Test 3: 配置管理压力测试

**测试环境**：
- CPU: 4核
- 内存: 8GB
- 配置文件: 1MB复杂配置

**测试场景**：

1. **配置验证压力测试**:
   ```bash
   # 批量验证配置文件
   for i in {1..1000}; do
     echo "test_config_$i.toml"
     inklog validate -c "test_config_$i.toml" 
   done
   ```

2. **Builder模式性能测试**:
   ```rust
   #[test]
   fn test_builder_performance() {
       let start = Instant::now();
       
       for _ in 0..10000 {
           let _logger = LoggerManager::builder()
               .level("info")
               .enable_console(true)
               .enable_file("app.log")
               .channel_capacity(5000)
               .build()
               .unwrap();
       }
       
       let elapsed = start.elapsed();
       println!("Builder模式10000次构建耗时: {:?}", elapsed);
       assert!(elapsed < Duration::from_secs(1)); // 1秒内完成
   }
   ```

3. **双初始化方式对比测试**:
   ```rust
   #[test]
   fn test_dual_initialization_performance() {
       // 方式1: 直接初始化
       let start1 = Instant::now();
       for _ in 0..1000 {
           let _logger = LoggerManager::new().unwrap();
       }
       let elapsed1 = start1.elapsed();
       
       // 方式2: 配置文件初始化（需要confers特性）
       #[cfg(feature = "confers")]
       {
           let start2 = Instant::now();
           for _ in 0..1000 {
               let _logger = LoggerManager::from_file("config.toml").unwrap();
           }
           let elapsed2 = start2.elapsed();
           
           println!("直接初始化1000次耗时: {:?}", elapsed1);
           println!("文件初始化1000次耗时: {:?}", elapsed2);
           
           // 文件初始化应该只比直接初始化慢不超过10倍
           assert!(elapsed2 < elapsed1 * 10);
       }
   }
   ```

**验收标准**：
- 配置验证1000次耗时<5秒
- Builder模式10000次构建<1秒
- 文件初始化不比直接初始化慢10倍以上
- 配置热重载响应时间<1秒

---

## 4. 兼容性验收

### 4.1 操作系统兼容性

| OS      | 版本         | Console | File | DB   | 通过 |
| ------- | ------------ | ------- | ---- | ---- | ---- |
| Linux   | Ubuntu 22.04 | ⬜       | ⬜    | ⬜    | ⬜    |
| macOS   | Ventura 13+  | ⬜       | ⬜    | ⬜    | ⬜    |
| Windows | Win11        | ⬜       | ⬜    | ⬜    | ⬜    |

### 4.2 数据库兼容性

| 数据库     | 版本  | 连接 | 批量写入 | 分区 | 通过 |
| ---------- | ----- | ---- | -------- | ---- | ---- |
| SQLite     | 3.35+ | ⬜    | ⬜        | N/A  | ⬜    |
| PostgreSQL | 12+   | ⬜    | ⬜        | ⬜    | ⬜    |
| MySQL      | 8.0+  | ⬜    | ⬜        | ⬜    | ⬜    |

### 4.3 配置管理兼容性

| 配置方式     | 零依赖模式 | confers特性 | Builder模式 | 通过 |
| ------------ | ---------- | ----------- | ----------- | ---- |
| 直接初始化   | ✅ 支持     | N/A         | N/A         | ⬜    |
| 文件配置     | ❌ 不支持   | ✅ 支持      | N/A         | ⬜    |
| 环境变量     | ✅ 支持     | ✅ 支持      | N/A         | ⬜    |
| Builder模式  | ✅ 支持     | ✅ 支持      | ✅ 支持      | ⬜    |
| CLI工具      | ❌ 不支持   | ✅ 支持      | N/A         | ⬜    |

**配置兼容性测试脚本**:

```bash
#!/bin/bash
# config_compatibility_test.sh

echo "=== 配置兼容性测试 ==="

# 测试1: 零依赖模式
echo "测试1: 零依赖模式"
cargo test --no-default-features --test config_test
echo ""

# 测试2: confers特性模式
echo "测试2: confers特性模式"
cargo test --features confers --test config_test
echo ""

# 测试3: Builder模式
echo "测试3: Builder模式"
cargo test --test builder_test
echo ""

# 测试4: CLI工具（需要confers特性）
echo "测试4: CLI工具"
#[cfg(feature = "confers")]
{
    echo "生成配置模板:"
    cargo run --bin inklog-cli --features confers -- generate -o test_config.toml
    
    echo "验证配置:"
    cargo run --bin inklog-cli --features confers -- validate -c test_config.toml
    
    rm -f test_config.toml
}
echo ""

echo "=== 配置兼容性测试完成 ==="
```

---

## 5. 文档验收

| 文档类型     | 完整性 | 准确性 | 可读性 | 通过 |
| ------------ | ------ | ------ | ------ | ---- |
| 快速开始指南 | ⬜      | ⬜      | ⬜      | ⬜    |
| 配置参考     | ⬜      | ⬜      | ⬜      | ⬜    |
| API文档      | ⬜      | ⬜      | ⬜      | ⬜    |
| 故障排查     | ⬜      | ⬜      | ⬜      | ⬜    |
| 示例代码     | ⬜      | ⬜      | ⬜      | ⬜    |
| 配置管理指南 | ⬜      | ⬜      | ⬜      | ⬜    |
| Builder模式文档 | ⬜   | ⬜      | ⬜      | ⬜    |
| CLI工具文档  | ⬜      | ⬜      | ⬜      | ⬜    |

**文档审查清单**：

- [ ] 所有配置项有说明
- [ ] 代码示例可运行
- [ ] 常见问题有解答
- [ ] 有性能调优建议
- [ ] 双初始化方式有对比说明
- [ ] Builder模式使用指南
- [ ] 配置验证规则说明
- [ ] CLI工具使用文档
- [ ] Feature标志编译说明
- [ ] 配置优先级说明（env→file→default）

---

## 6. 发布检查清单

### 6.1 代码质量验收

| Task 5.2 | Worker线程       | 可靠性-背压控制           | 压力测试      |
| Task 5.3 | 故障降级         | 可靠性-故障降级           | 集成测试      |
| Task 6.1 | 双初始化方式     | 配置管理-零依赖初始化     | 单元测试      |
| Task 6.2 | Builder模式      | 配置管理-链式构建         | 单元测试      |

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

### 7.2 验收测试迁移步骤

#### 配置管理验收迁移

**旧版本验收方式**:
```rust
// 单一初始化方式
let logger = LoggerManager::init("config.toml")?;
```

**新版本验收方式**:
```rust
// 方式1: 直接初始化（零依赖）
let logger = LoggerManager::new()?;

// 方式2: Builder模式配置
let logger = LoggerManager::builder()
    .level("info")
    .enable_console(true)
    .enable_file("app.log")
    .channel_capacity(5000)
    .build()?;

// 方式3: 配置文件初始化（需confers特性）
let logger = LoggerManager::from_file("inklog.toml")?;
```

#### CLI工具验收迁移

**新版本添加了CLI工具验收**:
```bash
# 生成配置模板
inklog generate -o config.toml

# 验证配置文件
inklog validate -c config.toml
```

**验收标准迁移**:
- ✅ 三种初始化方式都正常工作
- ✅ Builder模式链式调用流畅
- ✅ 配置验证能检测无效参数
- ✅ CLI工具生成有效配置模板
- ✅ 环境变量能覆盖配置文件设置
- ✅ 配置变更后系统能正常响应

### 7.3 性能验收标准迁移

| 指标 | 旧版本 | 新版本（默认） | 新版本（confers） | 说明 |
|------|--------|---------------|------------------|------|
| Console延迟 | <50μs | <50μs | <50μs | 保持一致 |
| File写入延迟 | <2ms | <2ms | <2ms | 保持一致 |
| 吞吐量(常规) | 5条/秒 | 5条/秒 | 5条/秒 | 保持一致 |
| 吞吐量(峰值) | 500条/秒 | 500条/秒 | 500条/秒 | 保持一致 |
| **配置加载时间** | N/A | **<100ms** | **<200ms** | **新增指标** |
| **Builder构建时间** | N/A | **<10ms** | **<10ms** | **新增指标** |

### 7.4 兼容性验收迁移

**配置管理兼容性新增测试**:

| 配置方式 | 零依赖模式 | confers特性 | Builder模式 | 验收要求 |
|----------|------------|-------------|-------------|----------|
| 直接初始化 | ✅ 支持 | N/A | N/A | 必须验收 |
| 文件配置 | ❌ 不支持 | ✅ 支持 | N/A | 必须验收 |
| 环境变量 | ✅ 支持 | ✅ 支持 | N/A | 必须验收 |
| Builder模式 | ✅ 支持 | ✅ 支持 | ✅ 支持 | 必须验收 |
| CLI工具 | ❌ 不支持 | ✅ 支持 | N/A | 新增验收 |

### 7.5 特性配置

在验收测试中配置不同特性：

```bash
# 验收零依赖版本
cargo test --no-default-features --test uat_test

# 验收confers特性版本
cargo test --features confers --test uat_test

# 验收所有特性组合
cargo test --all-features --test uat_test
```

### 7.6 验收注意事项

1. **零依赖版本**验收重点：
   - 快速初始化性能
   - Builder模式易用性
   - 无需配置文件的环境适应性

2. **confers特性**版本验收重点：
   - 配置文件加载正确性
   - CLI工具功能完整性
   - 环境变量优先级验证
   - 配置验证准确性

3. **迁移后验收更加全面**:
   - 覆盖了两种初始化方式
   - 增加了Builder模式验收
   - 添加了CLI工具功能验收
   - 强化了配置管理兼容性验收
| Task 6.3 | 配置验证         | 配置管理-参数验证         | 集成测试      |
| Task 6.4 | 环境变量配置     | 配置管理-环境变量支持     | 集成测试      |
| Task 7.1 | CLI工具          | 功能验收-配置生成         | inklog-cli    |
| Task 7.2 | CLI工具          | 功能验收-配置验证         | inklog-cli    |

**验收门禁规则**：

- [ ] 所有单元测试通过
- [ ] 集成测试通过
- [ ] 覆盖率≥85%
- [ ] Clippy无警告
- [ ] 无unsafe代码（或已充分审查）
- [ ] 整体覆盖率≥85%（以CI报告为准）
- [ ] 核心模块覆盖率满足标准要求
- [ ] 覆盖率报告已存档到文档库

### 6.2 安全审查

- [ ] 密钥不落盘
- [ ] 文件权限正确（600）
- [ ] 无SQL注入风险
- [ ] 依赖无已知漏洞（cargo audit）

### 6.3 性能验证

- [ ] Benchmark基准测试通过
- [ ] 无性能退化（vs上个版本）
- [ ] 压力测试通过
- [ ] 内存泄漏检查（valgrind）

### 6.4 发布物清单

- [ ] Crates.io发布
- [ ] GitHub Release Tag
- [ ] CHANGELOG.md更新
- [ ] 文档网站部署
- [ ] Docker镜像（可选）

---

## 7. 验收签字

| 角色       | 姓名 | 签字 | 日期 |
| ---------- | ---- | ---- | ---- |
| 产品经理   | ___  | ___  | ___  |
| 技术负责人 | ___  | ___  | ___  |
| 测试负责人 | ___  | ___  | ___  |
| 安全审计   | ___  | ___  | ___  |

**备注**：

```
所有验收项通过后，产品方可发布到生产环境。
如有未通过项，需在备注中说明原因和计划。
```
