// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 路径验证器示例（Layer 0 零依赖）
//!
//! 演示 PathValidator、PathValidatorConfig、ValidationResult 的使用，
//! 覆盖路径遍历检测、危险组件检测、符号链接检测、base_dir 约束与 sanitize 方法。
//!
//! # 运行
//! ```bash
//! cargo run --bin path_validator
//! ```

use inklog::{PathValidator, PathValidatorConfig, ValidationResult};
use inklog_examples::common::{print_section, print_separator};
use std::path::{Path, PathBuf};

fn main() {
	println!("=== inklog 路径验证器示例 ===\n");

	show_path_traversal_detection();
	show_dangerous_components();
	show_absolute_path_restriction();
	show_symlink_detection();
	show_base_dir_constraint();
	show_sanitize_method();
	show_validation_result_constructors();

	println!("\n✓ 所有路径验证器示例演示完成");
}

/// 展示路径遍历检测（".." 组件）
fn show_path_traversal_detection() {
	print_separator("1. 路径遍历检测（\"..\" 组件）");
	let validator = PathValidator::new();

	print_section("1.1 \"../etc/passwd\"");
	let result = validator.validate(Path::new("../etc/passwd"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("traversal"));

	print_section("1.2 \"foo/../../bar\"");
	let result = validator.validate(Path::new("foo/../../bar"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);

	print_section("1.3 安全相对路径 \"logs/app.log\"");
	let result = validator.validate(Path::new("logs/app.log"));
	println!("valid = {}", result.valid);
	assert!(result.valid);
}

/// 展示危险组件检测（/etc/passwd、.ssh、.env、.git 等）
fn show_dangerous_components() {
	print_separator("2. 危险组件检测（deny_components）");
	let validator = PathValidator::new();

	print_section("2.1 \"/etc/passwd\"");
	let result = validator.validate(Path::new("/etc/passwd"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("Dangerous path component"));

	print_section("2.2 \"~/.ssh/id_rsa\"");
	let result = validator.validate(Path::new("~/.ssh/id_rsa"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);

	print_section("2.3 \"./.env\"");
	let result = validator.validate(Path::new("./.env"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);

	print_section("2.4 \"project/.git/config\"");
	let result = validator.validate(Path::new("project/.git/config"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);

	print_section("2.5 自定义空 deny_components 后 \"etc/passwd\" 通过");
	let config = PathValidatorConfig {
		allow_absolute: true,
		base_dir: None,
		allow_symlinks: true,
		deny_components: vec![],
	};
	let validator = PathValidator::with_config(config);
	let result = validator.validate(Path::new("etc/passwd"));
	println!("valid = {}", result.valid);
	assert!(result.valid);
}

/// 展示绝对路径限制
fn show_absolute_path_restriction() {
	print_separator("3. 绝对路径限制（allow_absolute）");
	let config = PathValidatorConfig {
		allow_absolute: false,
		..Default::default()
	};
	let validator = PathValidator::with_config(config);

	print_section("3.1 禁用绝对路径后验证 \"/absolute/path\"");
	let result = validator.validate(Path::new("/absolute/path"));
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("Absolute"));

	print_section("3.2 相对路径 \"relative/path\" 仍可用");
	let result = validator.validate(Path::new("relative/path"));
	println!("valid = {}", result.valid);
	assert!(result.valid);
}

/// 展示符号链接检测
fn show_symlink_detection() {
	print_separator("4. 符号链接检测（allow_symlinks）");
	print_section("4.1 不存在的路径（无 symlink，验证通过）");
	let config = PathValidatorConfig {
		allow_symlinks: false,
		..Default::default()
	};
	let validator = PathValidator::with_config(config);
	let result = validator.validate(Path::new("/nonexistent/path"));
	println!("valid = {}", result.valid);
	assert!(result.valid);

	print_section("4.2 allow_symlinks = true 配置");
	let config = PathValidatorConfig {
		allow_symlinks: true,
		..Default::default()
	};
	let validator = PathValidator::with_config(config);
	let result = validator.validate(Path::new("/nonexistent/path"));
	println!("valid = {}（allow_symlinks=true 不拒绝）", result.valid);
	assert!(result.valid);
}

/// 展示 base_dir 约束
fn show_base_dir_constraint() {
	print_separator("5. base_dir 约束");

	// 创建临时基目录与子文件（Layer 0 仅使用本地文件系统）
	let temp_dir = std::env::temp_dir().join(format!(
		"inklog_path_example_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let nested = temp_dir.join("logs");
	std::fs::create_dir_all(&nested).expect("创建临时目录失败");
	let log_file = nested.join("app.log");
	std::fs::write(&log_file, "test").expect("写入临时文件失败");

	let config = PathValidatorConfig {
		allow_absolute: true,
		base_dir: Some(temp_dir.clone()),
		allow_symlinks: false,
		deny_components: vec![],
	};
	let validator = PathValidator::with_config(config);

	print_section("5.1 base_dir 内的路径（通过）");
	let result = validator.validate(&log_file);
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(result.valid);

	print_section("5.2 base_dir 外的路径（拒绝）");
	let outside = std::env::temp_dir().join("inklog_outside_file.log");
	let result = validator.validate(&outside);
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("base directory"));

	// 清理临时目录
	let _ = std::fs::remove_dir_all(&temp_dir);
	println!("\n已清理临时目录");
}

/// 展示 sanitize 方法
fn show_sanitize_method() {
	print_separator("6. sanitize 方法");
	let validator = PathValidator::new();

	print_section("6.1 \"foo/../bar\" → 移除 ParentDir");
	let sanitized: PathBuf = validator.sanitize(Path::new("foo/../bar"));
	println!("sanitized = {}", sanitized.display());
	assert_eq!(sanitized.to_string_lossy(), "bar");

	print_section("6.2 \"foo/./bar\" → 移除 CurDir");
	let sanitized = validator.sanitize(Path::new("foo/./bar"));
	println!("sanitized = {}", sanitized.display());
	assert_eq!(sanitized.to_string_lossy(), "foo/bar");

	print_section("6.3 \"foo/../bar/../baz\" → 多级移除");
	let sanitized = validator.sanitize(Path::new("foo/../bar/../baz"));
	println!("sanitized = {}", sanitized.display());
	assert_eq!(sanitized.to_string_lossy(), "baz");

	print_section("6.4 validate_and_sanitize() 组合方法");
	let result = validator.validate_and_sanitize(Path::new("logs/app.log"));
	println!(
		"valid = {}, sanitized = {:?}",
		result.valid, result.sanitized_path
	);
	assert!(result.valid);
	assert!(result.sanitized_path.is_some());
}

/// 展示 ValidationResult 构造器
fn show_validation_result_constructors() {
	print_separator("7. ValidationResult 构造器");

	print_section("7.1 ValidationResult::valid()");
	let result = ValidationResult::valid();
	println!(
		"valid = {}, error = {:?}, sanitized = {:?}",
		result.valid, result.error, result.sanitized_path
	);
	assert!(result.valid);
	assert!(result.error.is_none());

	print_section("7.2 ValidationResult::invalid(msg)");
	let result = ValidationResult::invalid("自定义错误信息");
	println!("valid = {}, error = {:?}", result.valid, result.error);
	assert!(!result.valid);
	assert_eq!(result.error.as_ref().unwrap(), "自定义错误信息");

	print_section("7.3 ValidationResult::sanitized(path)");
	let result = ValidationResult::sanitized(PathBuf::from("/safe/path/file.log"));
	println!(
		"valid = {}, sanitized = {:?}",
		result.valid, result.sanitized_path
	);
	assert!(result.valid);
	assert_eq!(
		result.sanitized_path.as_ref().unwrap(),
		&PathBuf::from("/safe/path/file.log")
	);
}
