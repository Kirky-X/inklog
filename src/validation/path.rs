// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Path validation for security.
//!
//! This module provides path validation to prevent path traversal attacks
//! and ensure safe file operations.

use std::path::{Path, PathBuf};
use tracing::warn;

/// Configuration for path validation.
#[derive(Debug, Clone)]
pub struct PathValidatorConfig {
    /// Allow absolute paths
    pub allow_absolute: bool,
    /// Restrict to specific base directory
    pub base_dir: Option<PathBuf>,
    /// Allow symlinks
    pub allow_symlinks: bool,
    /// Deny list of path components
    pub deny_components: Vec<String>,
}

impl Default for PathValidatorConfig {
    fn default() -> Self {
        Self {
            allow_absolute: true,
            base_dir: None,
            allow_symlinks: false,
            deny_components: vec![
                "..".to_string(),
                ".git".to_string(),
                ".ssh".to_string(),
                ".env".to_string(),
                "etc".to_string(),
                "passwd".to_string(),
                "shadow".to_string(),
            ],
        }
    }
}

/// Result of path validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the path is valid
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Sanitized path if applicable
    pub sanitized_path: Option<PathBuf>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            error: None,
            sanitized_path: None,
        }
    }

    pub fn invalid(message: &str) -> Self {
        Self {
            valid: false,
            error: Some(message.to_string()),
            sanitized_path: None,
        }
    }

    pub fn sanitized(path: PathBuf) -> Self {
        Self {
            valid: true,
            error: None,
            sanitized_path: Some(path),
        }
    }
}

/// Path validator for security checks.
#[derive(Debug, Clone)]
pub struct PathValidator {
    config: PathValidatorConfig,
}

impl PathValidator {
    /// Create a new PathValidator with default configuration.
    pub fn new() -> Self {
        Self {
            config: PathValidatorConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: PathValidatorConfig) -> Self {
        Self { config }
    }

    /// Validate a path.
    pub fn validate(&self, path: &Path) -> ValidationResult {
        let path_str = path.to_string_lossy();

        if path_str.contains("..") {
            warn!("Path traversal detected: {}", path_str);
            return ValidationResult::invalid("Path traversal detected");
        }

        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_string_lossy();
                if self.config.deny_components.iter().any(|d| name_str == *d) {
                    warn!("Dangerous path component detected: {}", name_str);
                    return ValidationResult::invalid(&format!(
                        "Dangerous path component: {}",
                        name_str
                    ));
                }
            }
        }

        if !self.config.allow_absolute && path.is_absolute() {
            return ValidationResult::invalid("Absolute paths are not allowed");
        }

        if !self.config.allow_symlinks {
            if let Ok(metadata) = std::fs::symlink_metadata(path) {
                if metadata.file_type().is_symlink() {
                    return ValidationResult::invalid("Symlinks are not allowed");
                }
            }
        }

        if let Some(ref base_dir) = self.config.base_dir {
            let canonical_path = match path.canonicalize() {
                Ok(p) => p,
                Err(_) => path.to_path_buf(),
            };
            let canonical_base = match base_dir.canonicalize() {
                Ok(p) => p,
                Err(_) => base_dir.clone(),
            };

            if !canonical_path.starts_with(&canonical_base) {
                return ValidationResult::invalid("Path is outside base directory");
            }
        }

        ValidationResult::valid()
    }

    /// Validate and sanitize a path.
    pub fn validate_and_sanitize(&self, path: &Path) -> ValidationResult {
        let result = self.validate(path);
        if result.valid {
            let sanitized = self.sanitize(path);
            ValidationResult::sanitized(sanitized)
        } else {
            result
        }
    }

    /// Sanitize a path by removing dangerous components.
    pub fn sanitize(&self, path: &Path) -> PathBuf {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                _ => components.push(component),
            }
        }
        components.iter().collect()
    }
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_detection() {
        let validator = PathValidator::new();

        assert!(!validator.validate(Path::new("../etc/passwd")).valid);
        assert!(!validator.validate(Path::new("foo/../bar")).valid);
        assert!(!validator.validate(Path::new("foo/../../bar")).valid);
    }

    #[test]
    fn test_dangerous_components() {
        let validator = PathValidator::new();

        assert!(!validator.validate(Path::new("/etc/passwd")).valid);
        assert!(!validator.validate(Path::new("~/.ssh/id_rsa")).valid);
        assert!(!validator.validate(Path::new("./.env")).valid);
    }

    #[test]
    fn test_absolute_path_restriction() {
        let config = PathValidatorConfig {
            allow_absolute: false,
            ..Default::default()
        };
        let validator = PathValidator::with_config(config);

        assert!(!validator.validate(Path::new("/absolute/path")).valid);
        assert!(validator.validate(Path::new("relative/path")).valid);
    }

    #[test]
    fn test_symlink_detection() {
        let config = PathValidatorConfig {
            allow_symlinks: false,
            ..Default::default()
        };
        let validator = PathValidator::with_config(config);

        let result = validator.validate(Path::new("/nonexistent"));
        assert!(result.valid);
    }

    #[test]
    fn test_sanitize() {
        let validator = PathValidator::new();

        let sanitized = validator.sanitize(Path::new("foo/../bar"));
        assert_eq!(sanitized.to_string_lossy(), "bar");

        let sanitized = validator.sanitize(Path::new("foo/./bar"));
        assert_eq!(sanitized.to_string_lossy(), "foo/bar");

        let sanitized = validator.sanitize(Path::new("foo/../bar/../baz"));
        assert_eq!(sanitized.to_string_lossy(), "baz");
    }

    #[test]
    fn test_safe_paths() {
        let validator = PathValidator::new();

        assert!(validator.validate(Path::new("logs/app.log")).valid);
        assert!(validator.validate(Path::new("/var/log/app.log")).valid);
    }

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult::valid();
        assert!(result.valid);
        assert!(result.error.is_none());
        assert!(result.sanitized_path.is_none());
    }

    #[test]
    fn test_validation_result_invalid() {
        let result = ValidationResult::invalid("test error message");
        assert!(!result.valid);
        assert_eq!(result.error.as_ref().unwrap(), "test error message");
        assert!(result.sanitized_path.is_none());
    }

    #[test]
    fn test_validation_result_sanitized() {
        let path = PathBuf::from("/safe/path/file.log");
        let result = ValidationResult::sanitized(path.clone());
        assert!(result.valid);
        assert!(result.error.is_none());
        assert_eq!(result.sanitized_path.as_ref().unwrap(), &path);
    }

    #[test]
    fn test_path_validator_default() {
        let validator = PathValidator::default();
        let result = validator.validate(Path::new("safe/path.log"));
        assert!(result.valid);
    }

    #[test]
    fn test_path_validator_with_config() {
        let config = PathValidatorConfig {
            allow_absolute: false,
            base_dir: None,
            allow_symlinks: true,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);
        // With empty deny_components, paths that would normally be denied are now valid
        assert!(validator.validate(Path::new("etc/passwd")).valid);
    }

    #[test]
    fn test_validate_and_sanitize_valid() {
        let validator = PathValidator::new();
        let result = validator.validate_and_sanitize(Path::new("logs/app.log"));
        assert!(result.valid);
        assert!(result.sanitized_path.is_some());
        let sanitized = result.sanitized_path.unwrap();
        assert!(sanitized.to_string_lossy().contains("app.log"));
    }

    #[test]
    fn test_validate_and_sanitize_invalid() {
        let validator = PathValidator::new();
        let result = validator.validate_and_sanitize(Path::new("../etc/passwd"));
        assert!(!result.valid);
        assert!(result.error.is_some());
        assert!(result.sanitized_path.is_none());
    }

    #[test]
    fn test_validate_and_sanitize_removes_parent_dir() {
        let validator = PathValidator::new();
        // This path has no ".." so it passes validation, then sanitize removes parent dirs
        let result = validator.validate_and_sanitize(Path::new("foo/../bar"));
        // Wait - "foo/../bar" contains ".." so it will be rejected by validate()
        assert!(!result.valid);
    }

    #[test]
    fn test_sanitize_with_curdir_only() {
        let validator = PathValidator::new();
        let sanitized = validator.sanitize(Path::new("././foo"));
        assert_eq!(sanitized.to_string_lossy(), "foo");
    }

    #[test]
    fn test_sanitize_empty_path() {
        let validator = PathValidator::new();
        let sanitized = validator.sanitize(Path::new(""));
        assert_eq!(sanitized.to_string_lossy(), "");
    }

    #[test]
    fn test_base_dir_validation_inside() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let base_dir = temp_dir.path().to_path_buf();
        let nested_dir = base_dir.join("logs");
        std::fs::create_dir_all(&nested_dir).expect("failed to create nested dir");
        let log_file = nested_dir.join("app.log");
        std::fs::write(&log_file, "test").expect("failed to write file");

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: Some(base_dir.clone()),
            allow_symlinks: false,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);
        let result = validator.validate(&log_file);
        assert!(result.valid);
    }

    #[test]
    fn test_base_dir_validation_outside() {
        let base_temp = tempfile::tempdir().expect("failed to create base temp dir");
        let outside_temp = tempfile::tempdir().expect("failed to create outside temp dir");
        let base_dir = base_temp.path().to_path_buf();
        // Create a file in a completely different temp directory (outside base_dir)
        let outside_file = outside_temp.path().join("outside.log");
        std::fs::write(&outside_file, "test").expect("failed to write file");

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: Some(base_dir.clone()),
            allow_symlinks: false,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);
        let result = validator.validate(&outside_file);
        assert!(!result.valid);
        assert!(result.error.as_ref().unwrap().contains("base directory"));
    }

    #[test]
    fn test_symlink_detection_with_real_symlink() {
        // Fixed: std::fs::symlink_metadata() does NOT follow symlinks, so
        // metadata.file_type().is_symlink() correctly detects symlinks.
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let target_file = temp_dir.path().join("target.log");
        std::fs::write(&target_file, "test").expect("failed to write target");
        let symlink_path = temp_dir.path().join("link.log");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_file, &symlink_path)
                .expect("failed to create symlink");
        }

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: None,
            allow_symlinks: false,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);

        #[cfg(unix)]
        {
            let result = validator.validate(&symlink_path);
            // Now symlinks ARE detected and rejected
            assert!(!result.valid, "Symlink should be detected and rejected");
            assert!(
                result
                    .error
                    .as_ref()
                    .is_some_and(|m| m.contains("Symlinks are not allowed"))
            );
        }
    }

    #[test]
    fn test_symlink_allowed() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let target_file = temp_dir.path().join("target.log");
        std::fs::write(&target_file, "test").expect("failed to write target");
        let symlink_path = temp_dir.path().join("link.log");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_file, &symlink_path)
                .expect("failed to create symlink");
        }

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: None,
            allow_symlinks: true,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);

        #[cfg(unix)]
        {
            let result = validator.validate(&symlink_path);
            assert!(result.valid);
        }
    }

    #[test]
    fn test_absolute_path_allowed_by_default() {
        let validator = PathValidator::new();
        // Default config allows absolute paths
        let result = validator.validate(Path::new("/var/log/app.log"));
        assert!(result.valid);
    }

    #[test]
    fn test_dangerous_component_passwd() {
        let validator = PathValidator::new();
        let result = validator.validate(Path::new("/some/passwd/file"));
        assert!(!result.valid);
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("Dangerous path component")
        );
    }

    #[test]
    fn test_dangerous_component_shadow() {
        let validator = PathValidator::new();
        let result = validator.validate(Path::new("/etc/shadow"));
        assert!(!result.valid);
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("Dangerous path component")
        );
    }

    #[test]
    fn test_dangerous_component_git() {
        let validator = PathValidator::new();
        let result = validator.validate(Path::new("project/.git/config"));
        assert!(!result.valid);
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("Dangerous path component")
        );
    }

    // ========================================================================
    // canonicalize 失败分支（行 139、143）
    // ========================================================================

    #[test]
    fn test_base_dir_validation_path_canonicalize_fails() {
        // 覆盖行 139：path.canonicalize() 失败时回退到 path.to_path_buf()
        // 构造一个存在的 base_dir，但 path 本身不存在（canonicalize 失败）
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let base_dir = temp_dir.path().to_path_buf();
        // path 不存在 → canonicalize 失败 → 走 Err 分支使用 path.to_path_buf()
        // 注意：path 不含 ".." 且不在 deny_components 中，因此能通过前面的检查
        let nonexistent_path = base_dir.join("nonexistent_subdir").join("app.log");

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: Some(base_dir.clone()),
            allow_symlinks: false,
            // 允许 nonexistent_subdir 通过，避免被 deny 拦截
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);
        let result = validator.validate(&nonexistent_path);
        // nonexistent_path 的 canonicalize 失败，回退到原路径
        // 原路径 starts_with base_dir（字符串前缀匹配）→ valid
        // 关键：覆盖了 Err 分支，且验证了回退路径的处理逻辑
        assert!(
            result.valid,
            "path under base_dir should be valid even if canonicalize fails, got: {:?}",
            result.error
        );
    }

    #[test]
    fn test_base_dir_validation_base_dir_canonicalize_fails() {
        // 覆盖行 143：base_dir.canonicalize() 失败时回退到 base_dir.clone()
        // 构造一个不存在的 base_dir，但 path 存在且为相对路径
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        // base_dir 指向不存在的路径
        let nonexistent_base = temp_dir.path().join("nonexistent_base_dir");
        // path 是一个相对路径（不存在的 base_dir 的子路径，相对形式）
        // 使用相对路径避免绝对路径的 canonicalize 影响
        let relative_path = Path::new("logs/app.log");

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: Some(nonexistent_base.clone()),
            allow_symlinks: false,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);
        let result = validator.validate(relative_path);
        // relative_path.canonicalize() 会失败（相对路径不存在）
        // base_dir.canonicalize() 也会失败（不存在）
        // 两者都回退到原路径，relative_path 不 starts_with nonexistent_base（绝对路径）
        // 因此返回 invalid "Path is outside base directory"
        // 关键：覆盖了 base_dir canonicalize 的 Err 分支
        assert!(!result.valid);
        assert!(
            result
                .error
                .as_ref()
                .is_some_and(|m| m.contains("base directory")),
            "expected base directory error, got: {:?}",
            result.error
        );
    }
}
