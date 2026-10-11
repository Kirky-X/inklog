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
use inklog_examples::common::{tr, tr_args};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}\n",
        tr("example-di-example-di-dependency-injection-pattern")
    );

    // 模式 1: 使用 InklogContainer 容器（真实适配器）
    println!("{}", tr("example-di-example-1-using-inklogcontainer-real"));
    println!("   {}", tr("example-di-example-creating-container"));
    let _container = InklogContainer::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .build()?;
    println!(
        "   {}",
        tr("example-di-example-cache-instance-created-arc-dyn")
    );
    println!(
        "   {}",
        tr("example-di-example-config-instance-created-arc-dyn")
    );

    // 模式 2: 使用 LoggerBuilder 注入依赖（真实适配器）
    println!(
        "\n{}",
        tr("example-di-example-2-injecting-dependencies-via")
    );
    let _logger = LoggerManager::builder()
        .cache(Arc::new(OxCacheAdapter::new()?))
        .config(Arc::new(InklogConfigAdapter::from_config(
            InklogConfig::default(),
        )))
        .level("info")
        .console(true)
        .build()
        .await?;
    println!("   {}", tr("example-di-example-logger-created"));

    // 模式 3: 使用 with_dependencies（真实适配器）
    println!(
        "\n{}",
        tr("example-di-example-3-using-with-dependencies-real")
    );
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
    println!("   {}", tr("example-di-example-logger-created"));

    // 模式 4: 容器共享依赖实例（真实适配器）
    println!(
        "\n{}",
        tr("example-di-example-4-container-shared-dependency")
    );
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
    println!(
        "   {}",
        tr("example-di-example-cache1-set-shared-key-shared")
    );

    // 从另一个实例读取
    let value = cache2.get("shared_key").await?;
    println!(
        "   {}",
        tr_args(
            "example-di-example-cache2-read-shared-key",
            &[("value", &format!("{:?}", value))]
        )
    );
    println!(
        "   {}",
        tr_args(
            "example-di-example-both-instances-share-the-same",
            &[("value", &(value == Some("shared_value".to_string())))]
        )
    );

    // 模式 5: 从容器创建多个 Logger（共享依赖）
    println!("\n{}", tr("example-di-example-5-creating-multiple-loggers"));
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
    println!("   {}", tr("example-di-example-logger-1-created"));
    // 该示例的职责即演示 container/with_dependencies 这组（已弃用的）DI 入口，
    // 供仍使用旧入口的下游参考；新代码请用 LoggerManager::builder()。
    #[allow(deprecated)]
    let _logger2 = container.create_logger().await?;
    println!("   {}", tr("example-di-example-logger-2-created"));
    println!(
        "   {}",
        tr("example-di-example-both-loggers-share-the-same")
    );

    // ============================================================================
    // Mock 类型示例 - 用于测试
    // ============================================================================

    println!(
        "\n{}\n",
        tr("example-di-example-mock-type-examples-for-testing")
    );

    // 模式 6: 使用 MockCache 进行测试
    println!("{}", tr("example-di-example-6-testing-with-mockcache"));
    let mock_cache = Arc::new(MockCache::new());

    mock_cache.set("test_key", "test_value".to_string()).await?;
    let value = mock_cache.get("test_key").await?;
    println!(
        "   {}",
        tr_args(
            "example-di-example-mockcache-set-and-read",
            &[("value", &format!("{:?}", value))]
        )
    );
    assert_eq!(value, Some("test_value".to_string()));

    let exists = mock_cache.exists("test_key").await?;
    println!(
        "   {}",
        tr_args("example-di-example-test-key-exists", &[("exists", &exists)])
    );

    let deleted = mock_cache.delete("test_key").await?;
    println!(
        "   {}",
        tr_args(
            "example-di-example-deleted-test-key",
            &[("deleted", &deleted)]
        )
    );

    // 模式 7: 使用 MockConfig 进行测试
    println!("\n{}", tr("example-di-example-7-testing-with-mockconfig"));
    let mock_config = Arc::new(
        MockConfig::new()
            .with_value("global.level", "debug")
            .with_value("http_server.port", "9090")
            .with_value("file_sink.enabled", "true"),
    );

    let level = mock_config.get_string("global.level");
    let port = mock_config.get_int("http_server.port");
    let enabled = mock_config.get_bool("file_sink.enabled");
    println!(
        "   {}",
        tr_args(
            "example-di-example-read-global-level",
            &[("level", &format!("{:?}", level))]
        )
    );
    println!(
        "   {}",
        tr_args(
            "example-di-example-read-http-server-port",
            &[("port", &format!("{:?}", port))]
        )
    );
    println!(
        "   {}",
        tr_args(
            "example-di-example-read-file-sink-enabled",
            &[("enabled", &format!("{:?}", enabled))]
        )
    );

    // 运行时修改配置
    mock_config.set("global.level", "trace");
    println!(
        "   {}",
        tr_args(
            "example-di-example-global-level-after-modification",
            &[(
                "value",
                &format!("{:?}", mock_config.get_string("global.level"))
            )]
        )
    );

    // 模式 8: 使用 MockDatabaseAdapter 进行测试
    println!("\n{}", tr("example-di-example-8-testing-with"));
    let mock_db = Arc::new(MockDatabaseAdapter::new());

    let healthy = mock_db.is_healthy().await;
    println!(
        "   {}",
        tr_args(
            "example-di-example-initial-health-status",
            &[("healthy", &healthy)]
        )
    );

    // 模拟数据库故障
    mock_db.set_healthy(false);
    println!(
        "   {}",
        tr_args(
            "example-di-example-health-status-after-fault",
            &[("value", &(mock_db.is_healthy().await))]
        )
    );

    // 恢复健康
    mock_db.set_healthy(true);
    println!(
        "   {}",
        tr_args(
            "example-di-example-health-status-after-recovery",
            &[("value", &(mock_db.is_healthy().await))]
        )
    );

    println!(
        "   {}",
        tr_args(
            "example-di-example-stored-record-count",
            &[("value", &(mock_db.record_count()))]
        )
    );

    // 模式 9: 使用 Mock 类型创建 Logger（测试场景）
    println!(
        "\n{}",
        tr("example-di-example-9-creating-a-logger-with-mock")
    );
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
    println!("   {}", tr("example-di-example-logger-created-with-mock"));

    // 模式 10: 使用带延迟的 MockCache（模拟网络延迟）
    println!(
        "\n{}",
        tr("example-di-example-10-using-mockcache-with-delay")
    );
    let delayed_cache = MockCache::with_delay(10); // 10ms 延迟

    let start = std::time::Instant::now();
    delayed_cache
        .set("slow_key", "slow_value".to_string())
        .await?;
    let elapsed = start.elapsed();
    println!(
        "   {}",
        tr_args(
            "example-di-example-set-operation-took",
            &[("elapsed", &format!("{:?}", elapsed))]
        )
    );
    println!(
        "   {}",
        tr_args(
            "example-di-example-verified-delay-took-effect-ms",
            &[("value", &(elapsed.as_millis()))]
        )
    );

    println!("\n{}", tr("example-di-example-examples-completed"));
    Ok(())
}
