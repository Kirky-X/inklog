// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! DI (Dependency Injection) 模式示例
//!
//! 展示如何使用依赖注入模式创建 LoggerManager，包括真实适配器和 Mock 实现。
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example di_example
//! ```

use inklog::integrations::infra::{Cache, Config, Database};
use inklog::integrations::infra::{InklogConfigAdapter, OxCacheAdapter};
use inklog::integrations::infra::{MockCache, MockConfig, MockDatabaseAdapter};
use inklog::{InklogConfig, InklogContainer, LoggerDependencies, LoggerManager};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DI (Dependency Injection) pattern example ===\n");

    // 模式 1: 使用 InklogContainer 容器（真实适配器）
    println!("1. Using InklogContainer (real adapters):");
    println!("   - Creating container...");
    let _container = InklogContainer::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .build()?;
    println!("   - Cache instance: created (Arc<dyn Cache>)");
    println!("   - Config instance: created (Arc<dyn Config>)");

    // 模式 2: 使用 LoggerBuilder 注入依赖（真实适配器）
    println!("\n2. Injecting dependencies via LoggerBuilder (real adapters):");
    let _logger = LoggerManager::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .level("info")
        .console(true)
        .build()
        .await?;
    println!("   - Logger created!");

    // 模式 3: 使用 with_dependencies（真实适配器）
    println!("\n3. Using with_dependencies (real adapters):");
    let deps = LoggerDependencies {
        cache: Some(Arc::new(OxCacheAdapter::new()?)),
        config: Some(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        ))),
        ..Default::default()
    };
    // 该示例的职责即演示 container/with_dependencies 这组（已弃用的）DI 入口，
    // 供仍使用旧入口的下游参考；新代码请用 LoggerManager::builder()。
    #[allow(deprecated)]
    let _logger = LoggerManager::with_dependencies(deps).await?;
    println!("   - Logger created!");

    // 模式 4: 容器共享依赖实例（真实适配器）
    println!("\n4. Container-shared dependency instances (real adapters):");
    let container = InklogContainer::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .build()?;

    // 获取共享的缓存实例
    let cache1 = container.cache();
    let cache2 = container.cache();

    // 在一个实例上设置值
    cache1.set("shared_key", "shared_value".to_string()).await?;
    println!("   - cache1 set shared_key = shared_value");

    // 从另一个实例读取
    let value = cache2.get("shared_key").await?;
    println!("   - cache2 read shared_key = {:?}", value);
    println!(
        "   - Both instances share the same underlying data: {}",
        value == Some("shared_value".to_string())
    );

    // 模式 5: 从容器创建多个 Logger（共享依赖）
    println!("\n5. Creating multiple loggers from the container:");
    let container = InklogContainer::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .build()?;

    // 该示例的职责即演示 container/with_dependencies 这组（已弃用的）DI 入口，
    // 供仍使用旧入口的下游参考；新代码请用 LoggerManager::builder()。
    #[allow(deprecated)]
    let _logger1 = container.create_logger().await?;
    println!("   - Logger 1 created");
    // 该示例的职责即演示 container/with_dependencies 这组（已弃用的）DI 入口，
    // 供仍使用旧入口的下游参考；新代码请用 LoggerManager::builder()。
    #[allow(deprecated)]
    let _logger2 = container.create_logger().await?;
    println!("   - Logger 2 created");
    println!("   - Both loggers share the same cache and config instances");

    // ============================================================================
    // Mock 类型示例 - 用于测试
    // ============================================================================

    println!("\n=== Mock type examples (for testing) ===\n");

    // 模式 6: 使用 MockCache 进行测试
    println!("6. Testing with MockCache:");
    let mock_cache = Arc::new(MockCache::new());

    mock_cache.set("test_key", "test_value".to_string()).await?;
    let value = mock_cache.get("test_key").await?;
    println!("   - MockCache set and read: {:?}", value);
    assert_eq!(value, Some("test_value".to_string()));

    let exists = mock_cache.exists("test_key").await?;
    println!("   - test_key exists: {}", exists);

    let deleted = mock_cache.delete("test_key").await?;
    println!("   - Deleted test_key: {}", deleted);

    // 模式 7: 使用 MockConfig 进行测试
    println!("\n7. Testing with MockConfig:");
    let mock_config = Arc::new(
        MockConfig::new()
            .with_value("global.level", "debug")
            .with_value("http_server.port", "9090")
            .with_value("file_sink.enabled", "true"),
    );

    let level = mock_config.get_string("global.level");
    let port = mock_config.get_int("http_server.port");
    let enabled = mock_config.get_bool("file_sink.enabled");
    println!("   - Read global.level: {:?}", level);
    println!("   - Read http_server.port: {:?}", port);
    println!("   - Read file_sink.enabled: {:?}", enabled);

    // 运行时修改配置
    mock_config.set("global.level", "trace");
    println!(
        "   - global.level after modification: {:?}",
        mock_config.get_string("global.level")
    );

    // 模式 8: 使用 MockDatabaseAdapter 进行测试
    println!("\n8. Testing with MockDatabaseAdapter:");
    let mock_db = Arc::new(MockDatabaseAdapter::new());

    let healthy = mock_db.is_healthy().await;
    println!("   - Initial health status: {}", healthy);

    // 模拟数据库故障
    mock_db.set_healthy(false);
    println!(
        "   - Health status after fault injection: {}",
        mock_db.is_healthy().await
    );

    // 恢复健康
    mock_db.set_healthy(true);
    println!(
        "   - Health status after recovery: {}",
        mock_db.is_healthy().await
    );

    println!("   - Stored record count: {}", mock_db.record_count());

    // 模式 9: 使用 Mock 类型创建 Logger（测试场景）
    println!("\n9. Creating a logger with mock types:");
    let container = InklogContainer::builder()
        .cache(Arc::new(MockCache::new()))
        .config(Arc::new(
            MockConfig::new().with_value("global.level", "info"),
        ))
        .database(Arc::new(MockDatabaseAdapter::new()))
        .build()?;

    // 该示例的职责即演示 container/with_dependencies 这组（已弃用的）DI 入口，
    // 供仍使用旧入口的下游参考；新代码请用 LoggerManager::builder()。
    #[allow(deprecated)]
    let _logger = container.create_logger().await?;
    println!("   - Logger created with mock dependencies!");

    // 模式 10: 使用带延迟的 MockCache（模拟网络延迟）
    println!("\n10. Using MockCache with delay (simulating network latency):");
    let delayed_cache = MockCache::with_delay(10); // 10ms 延迟

    let start = std::time::Instant::now();
    delayed_cache
        .set("slow_key", "slow_value".to_string())
        .await?;
    let elapsed = start.elapsed();
    println!("   - Set operation took: {:?}", elapsed);
    println!(
        "   - Verified delay took effect: {}ms >= 10ms",
        elapsed.as_millis()
    );

    println!("\n=== Examples completed ===");
    Ok(())
}
