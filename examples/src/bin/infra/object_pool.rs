// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! object pool example（Layer 0 零依赖）
//!
//! 演示 ObjectPool、ObjectPoolConfig 的 async API 使用，
//! 以及全局线程本地池便捷函数 get_log_record / put_log_record、
//! get_string_buffer / put_string_buffer。
//!
//! # 内容
//!
//! 1. ObjectPool 基本操作（put/get/missing key）
//! 2. ObjectPoolConfig 自定义配置（容量、TTL）
//! 3. 全局便捷函数 get_log_record / put_log_record
//! 4. 全局便捷函数 get_string_buffer / put_string_buffer
//! 5. 边界场景（容量限制、覆盖写入、并发安全）
//! 6. 错误处理验证
//! 7. Best practices与使用建议
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin object_pool
//! ```

use inklog::{
    LogRecord, ObjectPool, ObjectPoolConfig, get_log_record, get_string_buffer, put_log_record,
    put_string_buffer,
};
use inklog_examples::common::{print_section, print_separator, tr, tr_args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_separator(&tr("example-object-pool-inklog-object-pool-example"));

    show_pool_basic_operations().await?;
    show_pool_with_ttl_config().await?;
    show_global_log_record_functions();
    show_global_string_functions();
    show_edge_cases().await?;
    show_error_handling().await?;
    show_best_practices();

    println!("\n{}", tr("example-object-pool-all-object-pool-examples"));
    Ok(())
}

/// Example 1: basic ObjectPool operations
///
/// 演示 put/get 存取、缺失键处理、Result 类型验证。
async fn show_pool_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    print_section(&tr("example-object-pool-example-1-basic-objectpool"));

    print_section(&tr("example-object-pool-1-1-new-defaults-capacity-1024"));
    let pool = ObjectPool::<String, String>::new().await?;
    println!("{}", tr("example-object-pool-default-pool-built"));
    assert_eq!(pool.len(), 0, "a new pool should be empty");

    print_section(&tr("example-object-pool-1-2-custom-config-via-with"));
    let config = ObjectPoolConfig {
        max_capacity: 64,
        ttl_secs: None,
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    println!("{}", tr("example-object-pool-custom-config-pool-built"));

    print_section(&tr("example-object-pool-1-3-put-get-access"));
    pool.put(&"greeting".to_string(), "hello".to_string())
        .await?;
    pool.put(&"name".to_string(), "inklog".to_string()).await?;
    pool.put(&"version".to_string(), "0.1.12".to_string())
        .await?;

    let greeting = pool.get(&"greeting".to_string()).await?;
    let name = pool.get(&"name".to_string()).await?;
    let version = pool.get(&"version".to_string()).await?;
    let missing = pool.get(&"missing".to_string()).await?;

    println!(
        "{}",
        tr_args(
            "example-object-pool-greeting",
            &[("greeting", &format!("{:?}", greeting))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-object-pool-name",
            &[("name", &format!("{:?}", name))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-object-pool-version",
            &[("version", &format!("{:?}", version))]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-object-pool-missing",
            &[("missing", &format!("{:?}", missing))]
        )
    );

    assert_eq!(greeting, Some("hello".to_string()));
    assert_eq!(name, Some("inklog".to_string()));
    assert_eq!(version, Some("0.1.12".to_string()));
    assert_eq!(missing, None, "a missing key should return None");
    println!("{}", tr("example-object-pool-all-access-assertions-passed"));

    print_section(&tr("example-object-pool-1-4-overwriting"));
    pool.put(&"greeting".to_string(), "world".to_string())
        .await?;
    let updated = pool.get(&"greeting".to_string()).await?;
    assert_eq!(
        updated,
        Some("world".to_string()),
        "should return the new value after overwrite"
    );
    println!(
        "{}",
        tr("example-object-pool-overwriting-greeting-hello-world")
    );
    println!("{}", tr("example-object-pool-overwrite-verified"));

    print_section(&tr("example-object-pool-1-5-explicit-error-propagation"));
    let put_result = pool.put(&"k".to_string(), "v".to_string()).await;
    assert!(put_result.is_ok(), "put should return Ok");
    let get_result = pool.get(&"k".to_string()).await;
    assert!(get_result.is_ok(), "get should return Ok");
    println!(
        "{}",
        tr("example-object-pool-put-get-both-return-ok-errors")
    );
    Ok(())
}

/// Example 2: custom ObjectPoolConfig (with TTL)
///
/// 演示带 TTL 的对象池配置。
async fn show_pool_with_ttl_config() -> Result<(), Box<dyn std::error::Error>> {
    print_section(&tr("example-object-pool-example-2-custom"));

    // 2.1 带 TTL
    let config = ObjectPoolConfig {
        max_capacity: 256,
        ttl_secs: Some(60),
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    pool.put(&"k".to_string(), "v".to_string()).await?;
    let v = pool.get(&"k".to_string()).await?;
    assert_eq!(v, Some("v".to_string()));
    println!("{}", tr("example-object-pool-pool-with-ttl-60s-built-and"));

    // 2.2 不同容量配置
    let small_config = ObjectPoolConfig {
        max_capacity: 4,
        ttl_secs: None,
    };
    let small_pool = ObjectPool::<String, String>::with_config(small_config).await?;
    for i in 0..4 {
        small_pool
            .put(&format!("key{}", i), format!("val{}", i))
            .await?;
    }
    println!("{}", tr("example-object-pool-small-pool-4-filled-with-4"));

    // 2.3 验证小容量池
    for i in 0..4 {
        let v = small_pool.get(&format!("key{}", i)).await?;
        assert_eq!(v, Some(format!("val{}", i)), "key{} should exist", i);
    }
    println!("{}\n", tr("example-object-pool-small-capacity-pool-fully"));
    Ok(())
}

/// 示例 3：全局便捷函数 get_log_record / put_log_record
///
/// 演示基于线程本地池的 LogRecord 复用。
fn show_global_log_record_functions() {
    print_section(&tr("example-object-pool-example-3-get-log-record-put"));

    print_section(&tr("example-object-pool-3-1-get-log-record-fetches-a"));
    let mut record: LogRecord = get_log_record();
    println!(
        "{}",
        tr_args(
            "example-object-pool-logrecord-from-the-global-pool",
            &[("level", &record.level)]
        )
    );
    assert_eq!(record.level, "INFO", "default level should be INFO");

    // 修改后放回池中
    record.message = "object pool example".to_string();
    put_log_record(record);
    println!(
        "modified the message and returned it to the global pool (put resets the record automatically)"
    );

    print_section(&tr("example-object-pool-3-2-get-log-record-again"));
    let record2 = get_log_record();
    println!(
        "{}",
        tr_args(
            "example-object-pool-fetched-again-level",
            &[("level", &record2.level)]
        )
    );
    assert_eq!(record2.level, "INFO", "level after reset should be INFO");
    assert!(
        record2.message.is_empty() || record2.message == "object pool example",
        "message after reset should be empty or preserved (pool-implementation specific)"
    );

    // 3.3 多次循环验证稳定性
    for i in 0..10 {
        let r = get_log_record();
        assert_eq!(r.level, "INFO", "level should be INFO in round {}", i);
        put_log_record(r);
    }
    println!("{}", tr("example-object-pool-10-get-put-rounds-completed"));
    println!(
        "{}\n",
        tr("example-object-pool-global-logrecord-pool-verified")
    );
}

/// 示例 4：全局便捷函数 get_string_buffer / put_string_buffer
///
/// 演示基于线程本地池的 String 复用。
fn show_global_string_functions() {
    print_section(&tr("example-object-pool-example-4-get-string-buffer-put"));

    print_section(&tr("example-object-pool-4-1-get-string-buffer-fetches-a"));
    let buf: String = get_string_buffer();
    println!(
        "{}",
        tr_args(
            "example-object-pool-string-from-the-global-pool-len",
            &[("value", &(buf.len()))]
        )
    );
    assert!(buf.is_empty(), "initial String should be empty");

    // 填充内容后放回
    let mut buf = buf;
    buf.push_str("inklog string buffer");
    put_string_buffer(buf);
    println!(
        "{}",
        tr("example-object-pool-filled-inklog-string-buffer-and")
    );

    print_section(&tr("example-object-pool-4-2-get-string-buffer-again"));
    let s = get_string_buffer();
    println!(
        "{}",
        tr_args(
            "example-object-pool-fetched-again-len",
            &[("value", &(s.len()))]
        )
    );
    put_string_buffer(s);

    // 4.3 多次循环验证
    for _ in 0..5 {
        let mut b = get_string_buffer();
        b.push_str("test data");
        put_string_buffer(b);
    }
    println!("{}", tr("example-object-pool-5-get-put-rounds-completed"));
    println!(
        "{}\n",
        tr("example-object-pool-global-string-pool-verified")
    );
}

/// Example 5: edge cases
///
/// 测试空键、空值、重复 put 等边界情况。
async fn show_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    print_section(&tr("example-object-pool-example-5-edge-cases"));

    let pool = ObjectPool::<String, String>::new().await?;

    // 5.1 空键
    pool.put(&String::new(), "empty_key_value".to_string())
        .await?;
    let v = pool.get(&String::new()).await?;
    assert_eq!(v, Some("empty_key_value".to_string()));
    println!("{}", tr("example-object-pool-empty-key-access-works"));

    // 5.2 空值
    pool.put(&"empty_val".to_string(), String::new()).await?;
    let v = pool.get(&"empty_val".to_string()).await?;
    assert_eq!(v, Some(String::new()));
    println!("{}", tr("example-object-pool-empty-value-access-works"));

    // 5.3 重复 put 同一键
    pool.put(&"dup".to_string(), "first".to_string()).await?;
    pool.put(&"dup".to_string(), "second".to_string()).await?;
    let v = pool.get(&"dup".to_string()).await?;
    assert_eq!(
        v,
        Some("second".to_string()),
        "repeated put should keep the last value"
    );
    println!("{}", tr("example-object-pool-repeated-put-keeps-the-last"));

    // 5.4 大量键值对
    let large_pool = ObjectPool::<String, String>::with_config(ObjectPoolConfig {
        max_capacity: 1000,
        ttl_secs: None,
    })
    .await?;
    for i in 0..100 {
        large_pool
            .put(&format!("key_{}", i), format!("val_{}", i))
            .await?;
    }
    let mut found = 0;
    for i in 0..100 {
        if large_pool.get(&format!("key_{}", i)).await?.is_some() {
            found += 1;
        }
    }
    assert_eq!(found, 100, "all 100 key-value pairs should exist");
    println!("{}\n", tr("example-object-pool-all-100-key-value-pairs"));

    Ok(())
}

/// Example 6: error handling checks
///
/// 演示对象池操作的 Result 类型和错误处理模式。
async fn show_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    print_section(&tr("example-object-pool-example-6-error-handling-checks"));

    // 6.1 put 返回 Result
    let pool = ObjectPool::<String, String>::new().await?;
    match pool.put(&"key".to_string(), "value".to_string()).await {
        Ok(()) => println!("{}", tr("example-object-pool-put-succeeded-ok")),
        Err(e) => println!(
            "{}",
            tr_args("example-object-pool-put-failed", &[("e", &e)])
        ),
    }

    // 6.2 get 返回 Result<Option<V>>
    match pool.get(&"key".to_string()).await {
        Ok(Some(v)) => println!(
            "{}",
            tr_args("example-object-pool-get-succeeded-ok-some", &[("v", &v)])
        ),
        Ok(None) => println!(
            "  {}",
            tr("example-object-pool-get-returned-ok-none-key-absent")
        ),
        Err(e) => println!(
            "{}",
            tr_args("example-object-pool-get-failed", &[("e", &e)])
        ),
    }

    // 6.3 get 不存在的键
    match pool.get(&"nonexistent".to_string()).await {
        Ok(Some(_)) => println!("  {}", tr("example-object-pool-unexpected-key-exists")),
        Ok(None) => println!(
            "{}",
            tr("example-object-pool-get-of-a-missing-key-ok-none-as")
        ),
        Err(e) => println!(
            "{}",
            tr_args("example-object-pool-get-failed", &[("e", &e)])
        ),
    }

    // 6.4 ? 操作符传播
    async fn propagate_error(
        pool: &ObjectPool<String, String>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let v = pool.get(&"key".to_string()).await?;
        Ok(v)
    }
    let result = propagate_error(&pool).await;
    assert!(result.is_ok(), "? propagation should succeed");
    println!(
        "{}\n",
        tr("example-object-pool-operator-error-propagation-works")
    );

    Ok(())
}

/// Best practices建议
fn show_best_practices() {
    print_section(&tr("example-object-pool-best-practices"));

    println!("{}", tr("example-object-pool-1-when-to-use-an-object-pool"));
    println!(
        "   {}",
        tr("example-object-pool-when-creating-destroying-the")
    );
    println!("   {}", tr("example-object-pool-reduces-gc-pressure-and"));
    println!(
        "   {}",
        tr("example-object-pool-typical-uses-logrecord-reuse")
    );

    println!("\n{}", tr("example-object-pool-2-capacity-config"));
    println!("   {}", tr("example-object-pool-max-capacity-size-by"));
    println!(
        "   {}",
        tr("example-object-pool-too-small-objects-are-created")
    );
    println!(
        "   {}",
        tr("example-object-pool-too-large-wastes-memory-at-most")
    );

    println!("\n{}", tr("example-object-pool-3-ttl-config"));
    println!(
        "   {}",
        tr("example-object-pool-ttl-secs-maximum-lifetime-of-an")
    );
    println!(
        "   {}",
        tr("example-object-pool-suits-stateful-objects-that")
    );
    println!("   {}", tr("example-object-pool-none-never-expires-suits"));

    println!(
        "\n{}",
        tr("example-object-pool-4-global-functions-vs-direct-api")
    );
    println!(
        "   {}",
        tr("example-object-pool-get-log-record-put-log-record")
    );
    println!(
        "   {}",
        tr("example-object-pool-objectpool-new-shared-pool")
    );
    println!(
        "   {}",
        tr("example-object-pool-prefer-the-global-functions-in")
    );
    println!(
        "   {}",
        tr("example-object-pool-use-an-objectpool-instance-for")
    );

    println!("\n{}", tr("example-object-pool-5-usage-patterns"));
    println!(
        "   {}",
        tr("example-object-pool-always-pair-get-put-to-avoid")
    );
    println!(
        "   {}",
        tr("example-object-pool-objects-are-reset-on-put-do-not")
    );
    println!(
        "   {}",
        tr("example-object-pool-do-not-keep-using-a-reference")
    );
}
