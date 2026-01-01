<div align="center">

# 📚 API 参考

### Inklog 的完整 API 文档

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🏗️ 架构](ARCHITECTURE.md)

---

## 核心 API

### LoggerManager

日志记录系统的主要入口点。

```rust
pub struct LoggerManager {
    // 私有字段
}
```

#### 构造函数

```rust
impl LoggerManager {
    /// 使用默认配置创建新的日志管理器
    pub async fn new() -> Result<Self, InklogError>
    
    /// 使用自定义配置创建日志管理器
    pub async fn with_config(config: InklogConfig) -> Result<Self, InklogError>
    
    /// 构建分离的日志记录器（不设置全局订阅者）
    pub async fn build_detached(config: InklogConfig) -> Result<(Self, Subscriber, EnvFilter), InklogError>
}
```

#### 方法

```rust
impl LoggerManager {
    /// 获取当前健康状态
    pub fn get_health_status(&self) -> HealthStatus
    
    /// 获取指标快照
    pub fn get_metrics(&self) -> MetricsSnapshot
    
    /// 启动 HTTP 服务器（如果启用了 http 功能）
    #[cfg(feature = "http")]
    pub async fn start_http_server(&self, config: &HttpServerConfig) -> Result<(), InklogError>
    
    /// 启动归档服务（如果启用了 aws 功能）
    #[cfg(feature = "aws")]
    pub async fn start_archive_service(&self) -> Result<(), InklogError>
    
    /// 停止归档服务
    #[cfg(feature = "aws")]
    pub async fn stop_archive_service(&self) -> Result<(), InklogError>
    
    /// 触发手动归档
    #[cfg(feature = "aws")]
    pub async fn trigger_archive(&self) -> Result<(), InklogError>
}
```

### 错误处理模式

```rust
match operation() {
    Ok(result) => {
        println!("成功: {:?}", result);
    }
    Err(InklogError::ConfigError) => {
        eprintln!("无效配置");
    }
    Err(InklogError::LoggerError) => {
        eprintln!("日志记录器初始化失败");
    }
    Err(e) => {
        eprintln!("错误: {:?}", e);
    }
}
```

### 💡 常见使用模式

### 示例 1: 基本日志记录

```rust
use inklog::{LoggerManager, InklogConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用默认配置初始化日志记录器
    let _logger = LoggerManager::new().await?;
    
    // 开始日志记录
    log::info!("应用程序已启动");
    log::warn!("这是一个警告");
    log::error!("出现错误");
    
    Ok(())
}
```

### 示例 2: 自定义配置

```rust
use inklog::{LoggerManager, InklogConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = InklogConfig::default();
    config.global.level = "debug".to_string();
    config.global.masking_enabled = true;
    
    let _logger = LoggerManager::with_config(config).await?;
    
    log::info!("使用自定义配置的日志消息");
    
    Ok(())
}
```

### 示例 3: 高级配置

```rust
use inklog::{LoggerManager, InklogConfig};
use inklog::config::{GlobalConfig, ConsoleSinkConfig, FileSinkConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = InklogConfig::default();
    
    // 全局配置
    config.global.level = "info".to_string();
    config.global.channel_capacity = 10000;
    config.global.worker_threads = 4;
    
    // 控制台配置
    config.console_sink.enabled = true;
    config.console_sink.colored = true;
    
    // 文件配置
    config.file_sink.enabled = true;
    config.file_sink.path = "logs/app.log".to_string();
    config.file_sink.max_size = "100MB".to_string();
    
    let _logger = LoggerManager::with_config(config).await?;
    
    log::info!("使用高级配置的日志消息");
    
    Ok(())
}
```

---

<div align="center">

**[📖 用户指南](USER_GUIDE.md)** • **[🏗️ 架构](ARCHITECTURE.md)** • **[🏠 首页](../README.md)**

由文档团队用 ❤️ 制作

[⬆ 返回顶部](#-api-参考)

</div>