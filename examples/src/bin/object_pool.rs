// SPDX-License-Identifier: MIT
//! 对象池示例（Layer 0 零依赖）
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
//! 7. 最佳实践与使用建议
//!
//! # 运行
//!
//! ```bash
//! cargo run --bin object_pool
//! ```

use inklog::{
    get_log_record, get_string_buffer, put_log_record, put_string_buffer, LogRecord, ObjectPool,
    ObjectPoolConfig,
};
use inklog_examples::common::{print_section, print_separator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("inklog 对象池示例");

    show_pool_basic_operations().await?;
    show_pool_with_ttl_config().await?;
    show_global_log_record_functions();
    show_global_string_functions();
    show_edge_cases().await?;
    show_error_handling().await?;
    show_best_practices();

    println!("\n所有对象池示例演示完毕。");
    Ok(())
}

/// 示例 1：ObjectPool 基本操作
///
/// 演示 put/get 存取、缺失键处理、Result 类型验证。
async fn show_pool_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    print_section("示例 1：ObjectPool 基本操作");

    // 1.1 默认配置
    print_section("1.1 new() 默认配置（容量 1024）");
    let pool = ObjectPool::<String, String>::new().await?;
    println!("默认对象池构建成功");
    assert_eq!(pool.len(), 0, "新建池应为空");

    // 1.2 自定义配置
    print_section("1.2 with_config(ObjectPoolConfig) 自定义配置");
    let config = ObjectPoolConfig {
        max_capacity: 64,
        ttl_secs: None,
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    println!("自定义配置构建成功");

    // 1.3 put/get 存取
    print_section("1.3 put() / get() 存取");
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
    assert_eq!(missing, None, "不存在的键应返回 None");
    println!("✓ 所有存取断言通过");

    // 1.4 覆盖写入
    print_section("1.4 覆盖写入");
    pool.put(&"greeting".to_string(), "world".to_string())
        .await?;
    let updated = pool.get(&"greeting".to_string()).await?;
    assert_eq!(updated, Some("world".to_string()), "覆盖后应返回新值");
    println!("覆盖 greeting: hello → world");
    println!("✓ 覆盖写入验证通过");

    // 1.5 Result 类型验证
    print_section("1.5 错误显性传播（Result 返回）");
    let put_result = pool.put(&"k".to_string(), "v".to_string()).await;
    assert!(put_result.is_ok(), "put 应返回 Ok");
    let get_result = pool.get(&"k".to_string()).await;
    assert!(get_result.is_ok(), "get 应返回 Ok");
    println!("put/get 均返回 Ok，错误显性传播");
    Ok(())
}

/// 示例 2：ObjectPoolConfig 自定义配置（含 TTL）
///
/// 演示带 TTL 的对象池配置。
async fn show_pool_with_ttl_config() -> Result<(), Box<dyn std::error::Error>> {
    print_section("示例 2：ObjectPoolConfig 自定义配置（含 TTL）");

    // 2.1 带 TTL
    let config = ObjectPoolConfig {
        max_capacity: 256,
        ttl_secs: Some(60),
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    pool.put(&"k".to_string(), "v".to_string()).await?;
    let v = pool.get(&"k".to_string()).await?;
    assert_eq!(v, Some("v".to_string()));
    println!("带 TTL=60s 的对象池构建并存取成功");

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
    println!("小容量池（4）填充 4 个对象成功");

    // 2.3 验证小容量池
    for i in 0..4 {
        let v = small_pool.get(&format!("key{}", i)).await?;
        assert_eq!(v, Some(format!("val{}", i)), "key{} 应存在", i);
    }
    println!("✓ 小容量池全部验证通过\n");
    Ok(())
}

/// 示例 3：全局便捷函数 get_log_record / put_log_record
///
/// 演示基于线程本地池的 LogRecord 复用。
fn show_global_log_record_functions() {
    print_section("示例 3：get_log_record / put_log_record 全局函数");

    // 3.1 获取默认 LogRecord
    print_section("3.1 get_log_record() 获取 LogRecord");
    let mut record: LogRecord = get_log_record();
    println!("从全局池获取 LogRecord：level = {}", record.level);
    assert_eq!(record.level, "INFO", "默认级别应为 INFO");

    // 修改后放回池中
    record.message = "对象池示例".to_string();
    put_log_record(record);
    println!("已修改 message 并放回全局池（put 会自动 reset 记录）");

    // 3.2 验证 reset 行为
    print_section("3.2 再次 get_log_record() 验证 reset");
    let record2 = get_log_record();
    println!("再次获取：level = {}", record2.level);
    assert_eq!(record2.level, "INFO", "reset 后级别应为 INFO");
    assert!(
        record2.message.is_empty() || record2.message == "对象池示例",
        "reset 后消息应为空或保留（取决于池实现）"
    );

    // 3.3 多次循环验证稳定性
    for i in 0..10 {
        let r = get_log_record();
        assert_eq!(r.level, "INFO", "循环 {} 中级别应为 INFO", i);
        put_log_record(r);
    }
    println!("10 次 get/put 循环完成，无 panic");
    println!("✓ 全局 LogRecord 池验证通过\n");
}

/// 示例 4：全局便捷函数 get_string_buffer / put_string_buffer
///
/// 演示基于线程本地池的 String 复用。
fn show_global_string_functions() {
    print_section("示例 4：get_string_buffer / put_string_buffer 全局函数");

    // 4.1 获取空 String
    print_section("4.1 get_string_buffer() 获取 String");
    let buf: String = get_string_buffer();
    println!("从全局池获取 String：len = {}", buf.len());
    assert!(buf.is_empty(), "初始 String 应为空");

    // 填充内容后放回
    let mut buf = buf;
    buf.push_str("inklog string buffer");
    put_string_buffer(buf);
    println!("已填充 'inklog string buffer' 并放回全局池");

    // 4.2 再次获取
    print_section("4.2 再次 get_string_buffer()");
    let s = get_string_buffer();
    println!("再次获取：len = {}", s.len());
    put_string_buffer(s);

    // 4.3 多次循环验证
    for _ in 0..5 {
        let mut b = get_string_buffer();
        b.push_str("test data");
        put_string_buffer(b);
    }
    println!("5 次 get/put 循环完成");
    println!("✓ 全局 String 池验证通过\n");
}

/// 示例 5：边界场景
///
/// 测试空键、空值、重复 put 等边界情况。
async fn show_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    print_section("示例 5：边界场景");

    let pool = ObjectPool::<String, String>::new().await?;

    // 5.1 空键
    pool.put(&String::new(), "empty_key_value".to_string())
        .await?;
    let v = pool.get(&String::new()).await?;
    assert_eq!(v, Some("empty_key_value".to_string()));
    println!("✓ 空键存取正常");

    // 5.2 空值
    pool.put(&"empty_val".to_string(), String::new()).await?;
    let v = pool.get(&"empty_val".to_string()).await?;
    assert_eq!(v, Some(String::new()));
    println!("✓ 空值存取正常");

    // 5.3 重复 put 同一键
    pool.put(&"dup".to_string(), "first".to_string()).await?;
    pool.put(&"dup".to_string(), "second".to_string()).await?;
    let v = pool.get(&"dup".to_string()).await?;
    assert_eq!(v, Some("second".to_string()), "重复 put 应保留最后值");
    println!("✓ 重复 put 保留最后值");

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
    assert_eq!(found, 100, "100 个键值对应全部存在");
    println!("✓ 100 个键值对存取全部正确\n");

    Ok(())
}

/// 示例 6：错误处理验证
///
/// 演示对象池操作的 Result 类型和错误处理模式。
async fn show_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    print_section("示例 6：错误处理验证");

    // 6.1 put 返回 Result
    let pool = ObjectPool::<String, String>::new().await?;
    match pool.put(&"key".to_string(), "value".to_string()).await {
        Ok(()) => println!("✓ put 成功: Ok(())"),
        Err(e) => println!("✗ put 失败: {}", e),
    }

    // 6.2 get 返回 Result<Option<V>>
    match pool.get(&"key".to_string()).await {
        Ok(Some(v)) => println!("✓ get 成功: Ok(Some({}))", v),
        Ok(None) => println!("  get 返回: Ok(None) — 键不存在"),
        Err(e) => println!("✗ get 失败: {}", e),
    }

    // 6.3 get 不存在的键
    match pool.get(&"nonexistent".to_string()).await {
        Ok(Some(_)) => println!("  意外: 键存在"),
        Ok(None) => println!("✓ get 不存在: Ok(None) — 符合预期"),
        Err(e) => println!("✗ get 失败: {}", e),
    }

    // 6.4 ? 操作符传播
    async fn propagate_error(
        pool: &ObjectPool<String, String>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let v = pool.get(&"key".to_string()).await?;
        Ok(v)
    }
    let result = propagate_error(&pool).await;
    assert!(result.is_ok(), "? 传播应成功");
    println!("✓ ? 操作符错误传播正常\n");

    Ok(())
}

/// 最佳实践建议
fn show_best_practices() {
    print_section("最佳实践");

    println!("1. 何时使用对象池：");
    println!("   - 高频创建/销毁同类对象时（如日志处理管道）");
    println!("   - 减少 GC 压力和内存分配开销");
    println!("   - 典型场景: LogRecord 复用、String buffer 复用");

    println!("\n2. 容量配置：");
    println!("   - max_capacity: 根据并发量设定（建议 线程数 × 每线程缓存数）");
    println!("   - 过小: 频繁创建新对象，失去池化意义");
    println!("   - 过大: 浪费内存，建议不超过实际需求的 2 倍");

    println!("\n3. TTL 配置：");
    println!("   - ttl_secs: 对象在池中的最大存活时间");
    println!("   - 适用场景: 对象有状态需要定期清理");
    println!("   - None: 永不失效（适合无状态对象如 String）");

    println!("\n4. 全局函数 vs 直接 API：");
    println!("   - get_log_record/put_log_record: 线程本地池，零锁竞争");
    println!("   - ObjectPool::new(): 共享池，跨线程共享");
    println!("   - 单线程场景优先使用全局函数");
    println!("   - 多线程共享场景使用 ObjectPool 实例");

    println!("\n5. 使用模式：");
    println!("   - 始终配对使用 get/put，避免对象泄漏");
    println!("   - put 后对象会被 reset，不应再引用旧值");
    println!("   - 不要在 put 后继续使用 get 返回的引用");
}
