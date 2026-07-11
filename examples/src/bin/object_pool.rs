// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 对象池示例（Layer 0 零依赖）
//!
//! 演示 ObjectPool、ObjectPoolConfig 的 async API 使用，
//! 以及全局线程本地池便捷函数 get_log_record / put_log_record、
//! get_string_buffer / put_string_buffer。
//!
//! # 运行
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
    println!("=== inklog 对象池示例 ===\n");

    // 1. ObjectPool async 构建与基本操作
    show_pool_basic_operations().await?;

    // 2. ObjectPoolConfig 自定义配置（含 TTL）
    show_pool_with_ttl_config().await?;

    // 3. 全局便捷函数 get_log_record / put_log_record（线程本地池）
    show_global_log_record_functions();

    // 4. 全局便捷函数 get_string_buffer / put_string_buffer（线程本地池）
    show_global_string_functions();

    println!("\n✓ 所有对象池示例演示完成");
    Ok(())
}

/// 展示 ObjectPool async 基本操作
async fn show_pool_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("1. ObjectPool async 基本操作");

    print_section("1.1 new() 默认配置（容量 1024）");
    let pool = ObjectPool::<String, String>::new().await?;
    println!("默认对象池构建成功");
    assert_eq!(pool.len(), 0);

    print_section("1.2 with_config(ObjectPoolConfig) 自定义配置");
    let config = ObjectPoolConfig {
        max_capacity: 64,
        ttl_secs: None,
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    println!("自定义配置构建成功");

    print_section("1.3 put() / get() 存取");
    pool.put(&"greeting".to_string(), "hello".to_string())
        .await?;
    pool.put(&"name".to_string(), "inklog".to_string()).await?;
    let greeting = pool.get(&"greeting".to_string()).await?;
    let name = pool.get(&"name".to_string()).await?;
    let missing = pool.get(&"missing".to_string()).await?;
    println!("greeting = {:?}", greeting);
    println!("name = {:?}", name);
    println!("missing = {:?}", missing);
    assert_eq!(greeting, Some("hello".to_string()));
    assert_eq!(name, Some("inklog".to_string()));
    assert_eq!(missing, None);

    print_section("1.4 错误显性传播（Result 返回）");
    // put/get 返回 Result<(), InklogError> / Result<Option<V>, InklogError>
    let put_result = pool.put(&"k".to_string(), "v".to_string()).await;
    assert!(put_result.is_ok());
    let get_result = pool.get(&"k".to_string()).await;
    assert!(get_result.is_ok());
    println!("put/get 均返回 Ok，错误显性传播");
    Ok(())
}

/// 展示 ObjectPoolConfig 自定义配置（含 TTL）
async fn show_pool_with_ttl_config() -> Result<(), Box<dyn std::error::Error>> {
    print_separator("2. ObjectPoolConfig 自定义配置（含 TTL）");
    let config = ObjectPoolConfig {
        max_capacity: 256,
        ttl_secs: Some(60),
    };
    let pool = ObjectPool::<String, String>::with_config(config).await?;
    pool.put(&"k".to_string(), "v".to_string()).await?;
    let v = pool.get(&"k".to_string()).await?;
    assert_eq!(v, Some("v".to_string()));
    println!("带 TTL 的对象池构建并存取成功");
    Ok(())
}

/// 展示全局便捷函数 get_log_record / put_log_record（基于线程本地池）
fn show_global_log_record_functions() {
    print_separator("3. get_log_record / put_log_record 全局函数");

    print_section("3.1 get_log_record() 获取 LogRecord");
    let mut record: LogRecord = get_log_record();
    println!("从全局池获取 LogRecord：level = {}", record.level);
    assert_eq!(record.level, "INFO");

    // 修改后放回池中（put 会自动 reset）
    record.message = "对象池示例".to_string();
    put_log_record(record);
    println!("已修改 message 并放回全局池（put 会自动 reset 记录）");

    print_section("3.2 再次 get_log_record() 验证 reset");
    let record2 = get_log_record();
    println!("再次获取：level = {}", record2.level);
    assert_eq!(record2.level, "INFO");

    // 多次循环验证 API 稳定性
    for _ in 0..5 {
        let r = get_log_record();
        put_log_record(r);
    }
    println!("5 次 get/put 循环完成，无 panic");
}

/// 展示全局便捷函数 get_string_buffer / put_string_buffer（基于线程本地池）
fn show_global_string_functions() {
    print_separator("4. get_string_buffer / put_string_buffer 全局函数");

    print_section("4.1 get_string_buffer() 获取 String");
    let buf: String = get_string_buffer();
    println!("从全局池获取 String：len = {}", buf.len());
    assert!(buf.is_empty());

    // 使用后放回池中
    let mut buf = buf;
    buf.push_str("inklog string buffer");
    put_string_buffer(buf);
    println!("已填充内容并放回全局池");

    print_section("4.2 再次 get_string_buffer()");
    let s = get_string_buffer();
    println!("再次获取：len = {}", s.len());
    // 不对内容做断言（线程本地池可能返回池化值或默认值）
    put_string_buffer(s);
}
