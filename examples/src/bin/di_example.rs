// Copyright (c) 2026 Kirky.X
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
    println!("=== DI (Dependency Injection) 模式示例 ===\n");

    // 模式 1: 使用 InklogContainer 容器（真实适配器）
    println!("1. 使用 InklogContainer (真实适配器):");
    println!("   - 创建容器...");
    let _container = InklogContainer::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .build()?;
    println!("   - 缓存实例: 已创建 (Arc<dyn Cache>)");
    println!("   - 配置实例: 已创建 (Arc<dyn Config>)");

    // 模式 2: 使用 LoggerBuilder 注入依赖（真实适配器）
    println!("\n2. 使用 LoggerBuilder 注入依赖 (真实适配器):");
    let _logger = LoggerManager::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .level("info")
        .console(true)
        .build()
        .await?;
    println!("   - Logger 创建成功!");

    // 模式 3: 使用 with_dependencies（真实适配器）
    println!("\n3. 使用 with_dependencies (真实适配器):");
    let deps = LoggerDependencies {
        cache: Some(Arc::new(OxCacheAdapter::new()?)),
        config: Some(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        ))),
        ..Default::default()
    };
    let _logger = LoggerManager::with_dependencies(deps).await?;
    println!("   - Logger 创建成功!");

    // 模式 4: 容器共享依赖实例（真实适配器）
    println!("\n4. 容器共享依赖实例演示 (真实适配器):");
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
    println!("   - cache1 设置 shared_key = shared_value");

    // 从另一个实例读取
    let value = cache2.get("shared_key").await?;
    println!("   - cache2 读取 shared_key = {:?}", value);
    println!(
        "   - 两个实例共享同一底层数据: {}",
        value == Some("shared_value".to_string())
    );

    // 模式 5: 从容器创建多个 Logger（共享依赖）
    println!("\n5. 从容器创建多个 Logger:");
    let container = InklogContainer::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .build()?;

    let _logger1 = container.create_logger().await?;
    println!("   - Logger 1 创建成功");
    let _logger2 = container.create_logger().await?;
    println!("   - Logger 2 创建成功");
    println!("   - 两个 Logger 共享相同的 cache 和 config 实例");

    // ============================================================================
    // Mock 类型示例 - 用于测试
    // ============================================================================

    println!("\n=== Mock 类型示例（用于测试）===\n");

    // 模式 6: 使用 MockCache 进行测试
    println!("6. 使用 MockCache 进行测试:");
    let mock_cache = Arc::new(MockCache::new());

    // 设置和读取值
    mock_cache.set("test_key", "test_value".to_string()).await?;
    let value = mock_cache.get("test_key").await?;
    println!("   - MockCache 设置并读取: {:?}", value);
    assert_eq!(value, Some("test_value".to_string()));

    // 检查 exists
    let exists = mock_cache.exists("test_key").await?;
    println!("   - test_key 存在: {}", exists);

    // 删除键
    let deleted = mock_cache.delete("test_key").await?;
    println!("   - 删除 test_key: {}", deleted);

    // 模式 7: 使用 MockConfig 进行测试
    println!("\n7. 使用 MockConfig 进行测试:");
    let mock_config = Arc::new(
        MockConfig::new()
            .with_value("global.level", "debug")
            .with_value("http_server.port", "9090")
            .with_value("file_sink.enabled", "true"),
    );

    let level = mock_config.get_string("global.level");
    let port = mock_config.get_int("http_server.port");
    let enabled = mock_config.get_bool("file_sink.enabled");
    println!("   - 读取 global.level: {:?}", level);
    println!("   - 读取 http_server.port: {:?}", port);
    println!("   - 读取 file_sink.enabled: {:?}", enabled);

    // 运行时修改配置
    mock_config.set("global.level", "trace");
    println!(
        "   - 修改后 global.level: {:?}",
        mock_config.get_string("global.level")
    );

    // 模式 8: 使用 MockDatabaseAdapter 进行测试
    println!("\n8. 使用 MockDatabaseAdapter 进行测试:");
    let mock_db = Arc::new(MockDatabaseAdapter::new());

    // 健康检查
    let healthy = mock_db.is_healthy().await;
    println!("   - 初始健康状态: {}", healthy);

    // 模拟数据库故障
    mock_db.set_healthy(false);
    println!("   - 设置故障后健康状态: {}", mock_db.is_healthy().await);

    // 恢复健康
    mock_db.set_healthy(true);
    println!("   - 恢复后健康状态: {}", mock_db.is_healthy().await);

    // 查看存储的记录数
    println!("   - 存储的记录数: {}", mock_db.record_count());

    // 模式 9: 使用 Mock 类型创建 Logger（测试场景）
    println!("\n9. 使用 Mock 类型创建 Logger:");
    let container = InklogContainer::builder()
        .cache(Arc::new(MockCache::new()))
        .config(Arc::new(
            MockConfig::new().with_value("global.level", "info"),
        ))
        .database(Arc::new(MockDatabaseAdapter::new()))
        .build()?;

    let _logger = container.create_logger().await?;
    println!("   - 使用 Mock 依赖创建 Logger 成功!");

    // 模式 10: 使用带延迟的 MockCache（模拟网络延迟）
    println!("\n10. 使用带延迟的 MockCache (模拟网络延迟):");
    let delayed_cache = MockCache::with_delay(10); // 10ms 延迟

    let start = std::time::Instant::now();
    delayed_cache
        .set("slow_key", "slow_value".to_string())
        .await?;
    let elapsed = start.elapsed();
    println!("   - 设置操作耗时: {:?}", elapsed);
    println!("   - 验证延迟生效: {}ms >= 10ms", elapsed.as_millis());

    println!("\n=== 示例完成 ===");
    Ok(())
}
