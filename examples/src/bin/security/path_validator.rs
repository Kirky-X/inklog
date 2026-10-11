// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
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
use inklog_examples::common::{print_section, print_separator, tr, tr_args};
use std::path::{Path, PathBuf};

fn main() {
    println!(
        "{}\n",
        tr("example-path-validator-inklog-path-validator-examples")
    );

    show_path_traversal_detection();
    show_dangerous_components();
    show_absolute_path_restriction();
    show_symlink_detection();
    show_base_dir_constraint();
    show_sanitize_method();
    show_validation_result_constructors();

    println!(
        "\n{}",
        tr("example-path-validator-all-path-validator-examples")
    );
}

/// 展示路径遍历检测（".." 组件）
fn show_path_traversal_detection() {
    print_separator(&tr("example-path-validator-1-path-traversal-detection"));
    let validator = PathValidator::new();

    print_section(&tr("example-path-validator-1-1-etc-passwd"));
    let result = validator.validate(Path::new("../etc/passwd"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);
    assert!(result.error.as_ref().unwrap().contains("traversal"));

    print_section(&tr("example-path-validator-1-2-foo-bar"));
    let result = validator.validate(Path::new("foo/../../bar"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);

    print_section(&tr(
        "example-path-validator-1-3-the-safe-relative-path-logs",
    ));
    let result = validator.validate(Path::new("logs/app.log"));
    println!(
        "{}",
        tr_args("example-path-validator-valid", &[("valid", &result.valid)])
    );
    assert!(result.valid);
}

/// 展示危险组件检测（/etc/passwd、.ssh、.env、.git 等）
fn show_dangerous_components() {
    print_separator(&tr(
        "example-path-validator-2-dangerous-component-detection",
    ));
    let validator = PathValidator::new();

    print_section(&tr("example-path-validator-2-1-etc-passwd"));
    let result = validator.validate(Path::new("/etc/passwd"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);
    assert!(
        result
            .error
            .as_ref()
            .unwrap()
            .contains("Dangerous path component")
    );

    print_section(&tr("example-path-validator-2-2-ssh-id-rsa"));
    let result = validator.validate(Path::new("~/.ssh/id_rsa"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);

    print_section(&tr("example-path-validator-2-3-env"));
    let result = validator.validate(Path::new("./.env"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);

    print_section(&tr("example-path-validator-2-4-project-git-config"));
    let result = validator.validate(Path::new("project/.git/config"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);

    print_section(&tr("example-path-validator-2-5-etc-passwd-passes-with-an"));
    let config = PathValidatorConfig {
        allow_absolute: true,
        base_dir: None,
        allow_symlinks: true,
        deny_components: vec![],
    };
    let validator = PathValidator::with_config(config);
    let result = validator.validate(Path::new("etc/passwd"));
    println!(
        "{}",
        tr_args("example-path-validator-valid", &[("valid", &result.valid)])
    );
    assert!(result.valid);
}

/// 展示绝对路径限制
fn show_absolute_path_restriction() {
    print_separator(&tr("example-path-validator-3-absolute-path-restriction"));
    let config = PathValidatorConfig {
        allow_absolute: false,
        ..Default::default()
    };
    let validator = PathValidator::with_config(config);

    print_section(&tr("example-path-validator-3-1-validating-absolute-path"));
    let result = validator.validate(Path::new("/absolute/path"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);
    assert!(result.error.as_ref().unwrap().contains("Absolute"));

    print_section(&tr("example-path-validator-3-2-the-relative-path-relative"));
    let result = validator.validate(Path::new("relative/path"));
    println!(
        "{}",
        tr_args("example-path-validator-valid", &[("valid", &result.valid)])
    );
    assert!(result.valid);
}

/// 展示符号链接检测
fn show_symlink_detection() {
    print_separator(&tr("example-path-validator-4-symlink-detection-allow"));
    print_section(&tr("example-path-validator-4-1-a-nonexistent-path-no"));
    let config = PathValidatorConfig {
        allow_symlinks: false,
        ..Default::default()
    };
    let validator = PathValidator::with_config(config);
    let result = validator.validate(Path::new("/nonexistent/path"));
    println!(
        "{}",
        tr_args("example-path-validator-valid", &[("valid", &result.valid)])
    );
    assert!(result.valid);

    print_section(&tr("example-path-validator-4-2-the-allow-symlinks-true"));
    let config = PathValidatorConfig {
        allow_symlinks: true,
        ..Default::default()
    };
    let validator = PathValidator::with_config(config);
    let result = validator.validate(Path::new("/nonexistent/path"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-allow-symlinks-true-does",
            &[("valid", &result.valid)]
        )
    );
    assert!(result.valid);
}

/// 展示 base_dir 约束
fn show_base_dir_constraint() {
    print_separator(&tr("example-path-validator-5-base-dir-constraint"));

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
    std::fs::create_dir_all(&nested).expect("failed to create the temp dir");
    let log_file = nested.join("app.log");
    std::fs::write(&log_file, "test").expect("failed to write the temp file");

    let config = PathValidatorConfig {
        allow_absolute: true,
        base_dir: Some(temp_dir.clone()),
        allow_symlinks: false,
        deny_components: vec![],
    };
    let validator = PathValidator::with_config(config);

    print_section(&tr("example-path-validator-5-1-a-path-inside-base-dir"));
    let result = validator.validate(&log_file);
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(result.valid);

    print_section(&tr("example-path-validator-5-2-a-path-outside-base-dir"));
    let outside = std::env::temp_dir().join("inklog_outside_file.log");
    let result = validator.validate(&outside);
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);
    assert!(result.error.as_ref().unwrap().contains("base directory"));

    let _ = std::fs::remove_dir_all(&temp_dir);
    println!(
        "\n{}",
        tr("example-path-validator-temporary-directory-cleaned")
    );
}

/// 展示 sanitize 方法
fn show_sanitize_method() {
    print_separator(&tr("example-path-validator-6-the-sanitize-method"));
    let validator = PathValidator::new();

    print_section(&tr("example-path-validator-6-1-foo-bar-remove-parentdir"));
    let sanitized: PathBuf = validator
        .sanitize(Path::new("foo/../bar"))
        .expect("a path that is not pure traversal should succeed");
    println!(
        "{}",
        tr_args(
            "example-path-validator-sanitized",
            &[("value", &(sanitized.display()))]
        )
    );
    assert_eq!(sanitized.to_string_lossy(), "bar");

    print_section(&tr("example-path-validator-6-2-foo-bar-remove-curdir"));
    let sanitized = validator
        .sanitize(Path::new("foo/./bar"))
        .expect("a path that is not pure traversal should succeed");
    println!(
        "{}",
        tr_args(
            "example-path-validator-sanitized",
            &[("value", &(sanitized.display()))]
        )
    );
    assert_eq!(sanitized.to_string_lossy(), "foo/bar");

    print_section(&tr("example-path-validator-6-3-foo-bar-baz-multi-level"));
    let sanitized = validator
        .sanitize(Path::new("foo/../bar/../baz"))
        .expect("a path that is not pure traversal should succeed");
    println!(
        "{}",
        tr_args(
            "example-path-validator-sanitized",
            &[("value", &(sanitized.display()))]
        )
    );
    assert_eq!(sanitized.to_string_lossy(), "baz");

    print_section(&tr(
        "example-path-validator-6-3-1-etc-passwd-pure-traversal",
    ));
    assert!(validator.sanitize(Path::new("../../etc/passwd")).is_err());

    print_section(&tr("example-path-validator-6-4-the-validate-and-sanitize"));
    let result = validator.validate_and_sanitize(Path::new("logs/app.log"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-sanitized",
            &[
                ("valid", &result.valid),
                ("sanitized_path", &format!("{:?}", result.sanitized_path))
            ]
        )
    );
    assert!(result.valid);
    assert!(result.sanitized_path.is_some());
}

/// 展示 ValidationResult 构造器
fn show_validation_result_constructors() {
    print_separator(&tr(
        "example-path-validator-7-validationresult-constructors",
    ));

    print_section(&tr("example-path-validator-7-1-validationresult-valid"));
    let result = ValidationResult::valid();
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error-sanitized",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error)),
                ("sanitized_path", &format!("{:?}", result.sanitized_path))
            ]
        )
    );
    assert!(result.valid);
    assert!(result.error.is_none());

    print_section(&tr(
        "example-path-validator-7-2-validationresult-invalid-msg",
    ));
    let result = ValidationResult::invalid("custom error message");
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-error",
            &[
                ("valid", &result.valid),
                ("error", &format!("{:?}", result.error))
            ]
        )
    );
    assert!(!result.valid);
    assert_eq!(result.error.as_ref().unwrap(), "custom error message");

    print_section(&tr("example-path-validator-7-3-validationresult-sanitized"));
    let result = ValidationResult::sanitized(PathBuf::from("/safe/path/file.log"));
    println!(
        "{}",
        tr_args(
            "example-path-validator-valid-sanitized",
            &[
                ("valid", &result.valid),
                ("sanitized_path", &format!("{:?}", result.sanitized_path))
            ]
        )
    );
    assert!(result.valid);
    assert_eq!(
        result.sanitized_path.as_ref().unwrap(),
        &PathBuf::from("/safe/path/file.log")
    );
}
