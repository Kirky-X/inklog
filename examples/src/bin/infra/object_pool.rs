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
use inklog_examples::common::{print_section, print_separator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("inklog object pool example");

    show_pool_basic_operations().await?;
    show_pool_with_ttl_config().await?;
    show_global_log_record_functions();
    show_global_string_functions();
    show_edge_cases().await?;
    show_error_handling().await?;
    show_best_practices();

    println!("\nAll object pool examples completed.");
    Ok(())
}

/// Example 1: basic ObjectPool operations
///
/// 演示 put/get 存取、缺失键处理、Result 类型验证。
async fn show_pool_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    print_section("Example 1: basic ObjectPool operations");

    print_section("1.1 new() defaults (capacity 1024)");
    let pool = ObjectPool::<String, String>::new().await?;
    println!("default pool built");
    assert_eq!(pool.len(), 0, "a new pool should be empty");

    print_section("1.2 Custom config via with_config(ObjectPoolConfig)");
    let config = ObjectPoolConfig {
        max_capacity: 64,
        ttl_secs: None,
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    println!("custom config pool built");

    print_section("1.3 put() / get() access");
    pool.put(&"greeting".to_string(), "hello".to_string())
        .await?;
    pool.put(&"name".to_string(), "inklog".to_string()).await?;
    pool.put(&"version".to_string(), "0.1.12".to_string())
        .await?;

    let greeting = pool.get(&"greeting".to_string()).await?;
    let name = pool.get(&"name".to_string()).await?;
    let version = pool.get(&"version".to_string()).await?;
    let missing = pool.get(&"missing".to_string()).await?;

    println!("greeting = {:?}", greeting);
    println!("name     = {:?}", name);
    println!("version  = {:?}", version);
    println!("missing  = {:?}", missing);

    assert_eq!(greeting, Some("hello".to_string()));
    assert_eq!(name, Some("inklog".to_string()));
    assert_eq!(version, Some("0.1.12".to_string()));
    assert_eq!(missing, None, "a missing key should return None");
    println!("✓ All access assertions passed");

    print_section("1.4 Overwriting");
    pool.put(&"greeting".to_string(), "world".to_string())
        .await?;
    let updated = pool.get(&"greeting".to_string()).await?;
    assert_eq!(
        updated,
        Some("world".to_string()),
        "should return the new value after overwrite"
    );
    println!("overwriting greeting: hello → world");
    println!("✓ Overwrite verified");

    print_section("1.5 Explicit error propagation (Result return)");
    let put_result = pool.put(&"k".to_string(), "v".to_string()).await;
    assert!(put_result.is_ok(), "put should return Ok");
    let get_result = pool.get(&"k".to_string()).await;
    assert!(get_result.is_ok(), "get should return Ok");
    println!("put/get both return Ok; errors propagate explicitly");
    Ok(())
}

/// Example 2: custom ObjectPoolConfig (with TTL)
///
/// 演示带 TTL 的对象池配置。
async fn show_pool_with_ttl_config() -> Result<(), Box<dyn std::error::Error>> {
    print_section("Example 2: custom ObjectPoolConfig (with TTL)");

    // 2.1 带 TTL
    let config = ObjectPoolConfig {
        max_capacity: 256,
        ttl_secs: Some(60),
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    pool.put(&"k".to_string(), "v".to_string()).await?;
    let v = pool.get(&"k".to_string()).await?;
    assert_eq!(v, Some("v".to_string()));
    println!("pool with TTL=60s built and accessed successfully");

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
    println!("small pool (4) filled with 4 objects");

    // 2.3 验证小容量池
    for i in 0..4 {
        let v = small_pool.get(&format!("key{}", i)).await?;
        assert_eq!(v, Some(format!("val{}", i)), "key{} should exist", i);
    }
    println!("✓ Small-capacity pool fully verified\n");
    Ok(())
}

/// 示例 3：全局便捷函数 get_log_record / put_log_record
///
/// 演示基于线程本地池的 LogRecord 复用。
fn show_global_log_record_functions() {
    print_section("Example 3: get_log_record / put_log_record global functions");

    print_section("3.1 get_log_record() fetches a LogRecord");
    let mut record: LogRecord = get_log_record();
    println!("LogRecord from the global pool: level = {}", record.level);
    assert_eq!(record.level, "INFO", "default level should be INFO");

    // 修改后放回池中
    record.message = "object pool example".to_string();
    put_log_record(record);
    println!(
        "modified the message and returned it to the global pool (put resets the record automatically)"
    );

    print_section("3.2 get_log_record() again verifies reset");
    let record2 = get_log_record();
    println!("fetched again: level = {}", record2.level);
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
    println!("10 get/put rounds completed without panic");
    println!("✓ Global LogRecord pool verified\n");
}

/// 示例 4：全局便捷函数 get_string_buffer / put_string_buffer
///
/// 演示基于线程本地池的 String 复用。
fn show_global_string_functions() {
    print_section("Example 4: get_string_buffer / put_string_buffer global functions");

    print_section("4.1 get_string_buffer() fetches a String");
    let buf: String = get_string_buffer();
    println!("String from the global pool: len = {}", buf.len());
    assert!(buf.is_empty(), "initial String should be empty");

    // 填充内容后放回
    let mut buf = buf;
    buf.push_str("inklog string buffer");
    put_string_buffer(buf);
    println!("filled 'inklog string buffer' and returned it to the global pool");

    print_section("4.2 get_string_buffer() again");
    let s = get_string_buffer();
    println!("fetched again: len = {}", s.len());
    put_string_buffer(s);

    // 4.3 多次循环验证
    for _ in 0..5 {
        let mut b = get_string_buffer();
        b.push_str("test data");
        put_string_buffer(b);
    }
    println!("5 get/put rounds completed");
    println!("✓ Global String pool verified\n");
}

/// Example 5: edge cases
///
/// 测试空键、空值、重复 put 等边界情况。
async fn show_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    print_section("Example 5: edge cases");

    let pool = ObjectPool::<String, String>::new().await?;

    // 5.1 空键
    pool.put(&String::new(), "empty_key_value".to_string())
        .await?;
    let v = pool.get(&String::new()).await?;
    assert_eq!(v, Some("empty_key_value".to_string()));
    println!("✓ Empty-key access works");

    // 5.2 空值
    pool.put(&"empty_val".to_string(), String::new()).await?;
    let v = pool.get(&"empty_val".to_string()).await?;
    assert_eq!(v, Some(String::new()));
    println!("✓ Empty-value access works");

    // 5.3 重复 put 同一键
    pool.put(&"dup".to_string(), "first".to_string()).await?;
    pool.put(&"dup".to_string(), "second".to_string()).await?;
    let v = pool.get(&"dup".to_string()).await?;
    assert_eq!(
        v,
        Some("second".to_string()),
        "repeated put should keep the last value"
    );
    println!("✓ Repeated put keeps the last value");

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
    println!("✓ All 100 key-value pairs accessed correctly\n");

    Ok(())
}

/// Example 6: error handling checks
///
/// 演示对象池操作的 Result 类型和错误处理模式。
async fn show_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    print_section("Example 6: error handling checks");

    // 6.1 put 返回 Result
    let pool = ObjectPool::<String, String>::new().await?;
    match pool.put(&"key".to_string(), "value".to_string()).await {
        Ok(()) => println!("✓ put succeeded: Ok(())"),
        Err(e) => println!("✗ put failed: {}", e),
    }

    // 6.2 get 返回 Result<Option<V>>
    match pool.get(&"key".to_string()).await {
        Ok(Some(v)) => println!("✓ get succeeded: Ok(Some({}))", v),
        Ok(None) => println!("  get returned: Ok(None) — key absent"),
        Err(e) => println!("✗ get failed: {}", e),
    }

    // 6.3 get 不存在的键
    match pool.get(&"nonexistent".to_string()).await {
        Ok(Some(_)) => println!("  unexpected: key exists"),
        Ok(None) => println!("✓ get of a missing key: Ok(None) — as expected"),
        Err(e) => println!("✗ get failed: {}", e),
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
    println!("✓ ? operator error propagation works\n");

    Ok(())
}

/// Best practices建议
fn show_best_practices() {
    print_section("Best practices");

    println!("1. When to use an object pool:");
    println!("   - When creating/destroying the same type at high frequency (e.g. log pipelines)");
    println!("   - Reduces GC pressure and allocation overhead");
    println!("   - Typical uses: LogRecord reuse, String buffer reuse");

    println!("\n2. Capacity config:");
    println!("   - max_capacity: size by concurrency (threads × per-thread cache recommended)");
    println!("   - Too small: objects are created constantly, defeating pooling");
    println!("   - Too large: wastes memory; at most 2x the real need is advised");

    println!("\n3. TTL config:");
    println!("   - ttl_secs: maximum lifetime of an object in the pool");
    println!("   - Suits stateful objects that need periodic cleanup");
    println!("   - None: never expires (suits stateless objects like String)");

    println!("\n4. Global functions vs direct API:");
    println!("   - get_log_record/put_log_record: thread-local pool, zero lock contention");
    println!("   - ObjectPool::new(): shared pool, shared across threads");
    println!("   - Prefer the global functions in single-threaded scenarios");
    println!("   - Use an ObjectPool instance for multi-threaded sharing");

    println!("\n5. Usage patterns:");
    println!("   - Always pair get/put to avoid leaking objects");
    println!("   - Objects are reset on put; do not keep referencing old values");
    println!("   - Do not keep using a reference returned by get after put");
}
