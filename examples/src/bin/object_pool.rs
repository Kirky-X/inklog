// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 对象池示例（Layer 0 零依赖）
//!
//! 演示 ObjectPool、ObjectPoolBuilder、ObjectPoolConfig、PoolMetrics 的使用，
//! 以及全局 LOG_RECORD_POOL / STRING_POOL 池和便捷函数 get_log_record /
//! put_log_record、get_string_buffer / put_string_buffer。
//!
//! # 运行
//! ```bash
//! cargo run --bin object_pool
//! ```

use inklog::{
	get_log_record, get_string_buffer, put_log_record, put_string_buffer, LogRecord, ObjectPool,
	ObjectPoolBuilder, ObjectPoolConfig, LOG_RECORD_POOL, STRING_POOL,
};
use inklog_examples::common::{print_section, print_separator};

fn main() {
	println!("=== inklog 对象池示例 ===\n");

	// 1. ObjectPoolBuilder 链式构建
	show_builder_pattern();

	// 2. ObjectPool 基本操作与 ObjectPoolConfig
	show_pool_basic_operations();

	// 3. PoolMetrics 指标
	show_pool_metrics();

	// 4. 全局便捷函数 get_log_record / put_log_record（线程本地池）
	show_global_log_record_functions();

	// 5. 全局便捷函数 get_string_buffer / put_string_buffer（线程本地池）
	show_global_string_functions();

	// 6. 全局 LOG_RECORD_POOL / STRING_POOL（基于 oxcache）
	show_global_pools();

	println!("\n✓ 所有对象池示例演示完成");
}

/// 展示 ObjectPoolBuilder 链式构建
fn show_builder_pattern() {
	print_separator("1. ObjectPoolBuilder 链式构建");

	print_section("1.1 builder().capacity(n).build()");
	let pool = ObjectPool::<String, i32>::builder().capacity(256).build();
	println!("构建对象池：容量 = {}", pool.capacity());
	assert_eq!(pool.capacity(), 256);

	print_section("1.2 builder().capacity(n).ttl_secs(n).build()");
	let pool = ObjectPool::<String, String>::builder()
		.capacity(512)
		.ttl_secs(60)
		.build();
	println!("构建对象池：容量 = {}, TTL = 60s", pool.capacity());
	assert_eq!(pool.capacity(), 512);

	print_section("1.3 ObjectPoolBuilder::default().capacity(n).build()");
	let pool = ObjectPoolBuilder::<String, i32>::default()
		.capacity(128)
		.build();
	println!("通过 ObjectPoolBuilder::default() 构建：容量 = {}", pool.capacity());
	assert_eq!(pool.capacity(), 128);

	print_section("1.4 ObjectPoolBuilder::default().build() 默认容量");
	let pool = ObjectPoolBuilder::<String, String>::default().build();
	println!("默认构建：容量 = {}", pool.capacity());
	assert_eq!(pool.capacity(), 1024);
}

/// 展示 ObjectPool 基本操作与 ObjectPoolConfig
fn show_pool_basic_operations() {
	print_separator("2. ObjectPool 基本操作");

	print_section("2.1 new() 默认配置（容量 1024）");
	let pool = ObjectPool::<String, String>::new();
	println!("默认对象池：容量 = {}", pool.capacity());
	assert_eq!(pool.capacity(), 1024);

	print_section("2.2 with_config(ObjectPoolConfig) 自定义配置");
	let config = ObjectPoolConfig {
		max_capacity: 64,
		ttl_secs: Some(30),
	};
	let pool = ObjectPool::<String, i32>::with_config(config);
	println!("自定义配置：容量 = {}", pool.capacity());
	assert_eq!(pool.capacity(), 64);

	print_section("2.3 put() / get() 存取");
	let pool = ObjectPool::<String, String>::with_capacity(16);
	pool.put(&"greeting".to_string(), "hello".to_string());
	pool.put(&"name".to_string(), "inklog".to_string());
	let greeting = pool.get(&"greeting".to_string());
	let name = pool.get(&"name".to_string());
	let missing = pool.get(&"missing".to_string());
	println!("greeting = {:?}", greeting);
	println!("name = {:?}", name);
	println!("missing = {:?}", missing);
	assert_eq!(greeting, Some("hello".to_string()));
	assert_eq!(name, Some("inklog".to_string()));
	assert_eq!(missing, None);

	print_section("2.4 contains() / remove()");
	assert!(pool.contains(&"name".to_string()));
	let removed = pool.remove(&"name".to_string());
	println!("remove name = {:?}", removed);
	assert_eq!(removed, Some("inklog".to_string()));
	assert!(!pool.contains(&"name".to_string()));

	print_section("2.5 clear() 清空");
	pool.put(&"a".to_string(), "1".to_string());
	pool.put(&"b".to_string(), "2".to_string());
	pool.clear();
	assert_eq!(pool.get(&"a".to_string()), None);
	assert_eq!(pool.get(&"b".to_string()), None);
	println!("clear 后所有 key 均不可访问");
}

/// 展示 PoolMetrics 指标
fn show_pool_metrics() {
	print_separator("3. PoolMetrics 指标");
	let pool = ObjectPool::<String, i32>::with_capacity(32);

	// 触发一次 miss
	let _ = pool.get(&"missing".to_string());

	// put 后再 get 触发一次 hit
	pool.put(&"key".to_string(), 100);
	let _ = pool.get(&"key".to_string());

	let metrics = pool.metrics();
	println!("current_size   = {}", metrics.current_size);
	println!("max_capacity   = {}", metrics.max_capacity);
	println!("total_requests = {}", metrics.total_requests);
	println!("hits           = {}", metrics.hits);
	println!("misses         = {}", metrics.misses);
	println!("hit_rate       = {:.2}%", metrics.hit_rate);
	println!("items_created  = {}", metrics.items_created);
	println!("items_reused   = {}", metrics.items_reused);
	assert_eq!(metrics.max_capacity, 32);
	assert_eq!(metrics.total_requests, 2);
	assert_eq!(metrics.hits, 1);
	assert_eq!(metrics.misses, 1);
}

/// 展示全局便捷函数 get_log_record / put_log_record（基于线程本地池）
fn show_global_log_record_functions() {
	print_separator("4. get_log_record / put_log_record 全局函数");

	print_section("4.1 get_log_record() 获取 LogRecord");
	let mut record: LogRecord = get_log_record();
	println!("从全局池获取 LogRecord：level = {}", record.level);
	assert_eq!(record.level, "INFO");

	// 修改后放回池中（put 会自动 reset）
	record.message = "对象池示例".to_string();
	put_log_record(record);
	println!("已修改 message 并放回全局池（put 会自动 reset 记录）");

	print_section("4.2 再次 get_log_record() 验证 reset");
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
	print_separator("5. get_string_buffer / put_string_buffer 全局函数");

	print_section("5.1 get_string_buffer() 获取 String");
	let buf: String = get_string_buffer();
	println!("从全局池获取 String：len = {}", buf.len());
	assert!(buf.is_empty());

	// 使用后放回池中
	let mut buf = buf;
	buf.push_str("inklog string buffer");
	put_string_buffer(buf);
	println!("已填充内容并放回全局池");

	print_section("5.2 再次 get_string_buffer()");
	let s = get_string_buffer();
	println!("再次获取：len = {}", s.len());
	// 不对内容做断言（线程本地池可能返回池化值或默认值）
	put_string_buffer(s);
}

/// 展示全局 LOG_RECORD_POOL / STRING_POOL（基于 oxcache）
fn show_global_pools() {
	print_separator("6. LOG_RECORD_POOL / STRING_POOL 全局池");

	print_section("6.1 LOG_RECORD_POOL.get() / put()");
	let record = LOG_RECORD_POOL.get();
	println!("LOG_RECORD_POOL.get() level = {}", record.level);
	assert_eq!(record.level, "INFO");
	LOG_RECORD_POOL.put(LogRecord::default());
	println!("LOG_RECORD_POOL.put() 完成");

	print_section("6.2 LOG_RECORD_POOL.metrics()");
	let metrics = LOG_RECORD_POOL.metrics();
	println!("max_capacity = {}", metrics.max_capacity);

	print_section("6.3 STRING_POOL.get() / put()");
	let s = STRING_POOL.get();
	println!("STRING_POOL.get() len = {}", s.len());
	STRING_POOL.put("reuse me".to_string());
	println!("STRING_POOL.put() 完成");

	print_section("6.4 STRING_POOL.metrics()");
	let metrics = STRING_POOL.metrics();
	println!("max_capacity = {}", metrics.max_capacity);
}
