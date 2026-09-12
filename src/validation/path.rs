// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Path validation for security.
//!
//! This module provides path validation to prevent path traversal attacks
//! and ensure safe file operations.

use crate::error::InklogError;
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
        // Check for path traversal using component iteration instead of substring match.
        // `contains("..")` would reject legitimate filenames like "foo..bar".
        if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            // Log only the path display name, not the full path, to avoid leaking
            // sensitive directory information into application logs.
            warn!(
                "{} ({} components)",
                crate::i18n::tr("validation-path_traversal"),
                path.components().count()
            );
            return ValidationResult::invalid(&crate::i18n::tr("validation-path_traversal"));
        }

        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_string_lossy();
                if self.config.deny_components.iter().any(|d| name_str == *d) {
                    warn!(
                        "{}: {}",
                        crate::i18n::tr("validation-dangerous_component"),
                        name_str
                    );
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("component", name_str.to_string());
                    return ValidationResult::invalid(&crate::i18n::tr_args(
                        "validation-dangerous_component",
                        args,
                    ));
                }
            }
        }

        if !self.config.allow_absolute && path.is_absolute() {
            return ValidationResult::invalid(&crate::i18n::tr("validation-no_absolute"));
        }

        if !self.config.allow_symlinks
            && let Ok(metadata) = std::fs::symlink_metadata(path)
            && metadata.file_type().is_symlink()
        {
            return ValidationResult::invalid(&crate::i18n::tr("validation-no_symlinks"));
        }

        if let Some(ref base_dir) = self.config.base_dir {
            // TOCTOU caveat: `canonicalize()` resolves symlinks at validation
            // time only. When it fails, falling back to the raw (unchecked)
            // path widens the validate-then-use window — the filesystem may
            // change between this check and the actual file operation.
            let canonical_path = match path.canonicalize() {
                Ok(p) => p,
                Err(_) => path.to_path_buf(),
            };
            let canonical_base = match base_dir.canonicalize() {
                Ok(p) => p,
                Err(_) => base_dir.clone(),
            };

            // 仅 Windows 的 verbatim 前缀回退比较会重新赋值
            #[cfg_attr(not(windows), allow(unused_mut))]
            let mut inside_base = canonical_path.starts_with(&canonical_base);
            if !inside_base {
                // Windows：canonicalize() 返回 `\\?\` verbatim 前缀，而 canonicalize 失败
                // 时回退的原始路径没有此前缀，导致 starts_with 误判——比较前对齐两侧前缀。
                // 无 verbatim 前缀时上方标准 starts_with 比较即为权威结果，直接落到底部判定。
                #[cfg(windows)]
                if let Some(base_plain) = canonical_base
                    .to_string_lossy()
                    .strip_prefix(r"\\?\")
                    .map(std::path::PathBuf::from)
                {
                    inside_base = canonical_path.starts_with(&base_plain);
                }
            }

            if !inside_base {
                return ValidationResult::invalid(&crate::i18n::tr("validation-outside_base"));
            }
        }

        ValidationResult::valid()
    }

    /// Validate and sanitize a path.
    ///
    /// # Behavior
    ///
    /// This method first calls `validate()` which strictly rejects paths
    /// containing `..` (parent directory) traversal components. Only paths
    /// that pass validation are then sanitized.
    ///
    /// Since `validate()` already rejects all traversal attempts, the
    /// `sanitize()` step here primarily serves to normalize path separators
    /// and remove any redundant components from already-safe paths. It will
    /// **not** "fix" a traversal path — those are always rejected.
    pub fn validate_and_sanitize(&self, path: &Path) -> ValidationResult {
        let result = self.validate(path);
        if result.valid {
            match self.sanitize(path) {
                Ok(sanitized) => ValidationResult::sanitized(sanitized),
                Err(e) => ValidationResult::invalid(&e.to_string()),
            }
        } else {
            result
        }
    }

    /// Sanitize a path by removing dangerous components.
    ///
    /// `..` components pop the last remaining component and `.` components
    /// are dropped. A `..` that would escape past the start of the path
    /// (nothing left to pop, i.e. the path is pure traversal such as
    /// `../../etc/passwd` or `..`) is an error rather than a silent no-op:
    /// silently dropping the traversal would return a deceptively safe
    /// relative path.
    ///
    /// # Note
    ///
    /// Sanitization alone performs **no base-directory check**. For security
    /// sensitive use cases always use
    /// [`validate_and_sanitize()`](Self::validate_and_sanitize), which rejects
    /// traversal paths outright.
    ///
    /// # Errors
    ///
    /// Returns `InklogError::ConfigError` when a `..` component pops an empty
    /// component stack (the path escapes its virtual root).
    pub fn sanitize(&self, path: &Path) -> Result<PathBuf, InklogError> {
        let mut components: Vec<std::path::Component<'_>> = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    if components.pop().is_none() {
                        return Err(InklogError::ConfigError(
                            crate::i18n::tr("validation-path_traversal"),
                        ));
                    }
                }
                std::path::Component::CurDir => {}
                _ => components.push(component),
            }
        }
        Ok(components.iter().collect())
    }
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 以 `O_NOFOLLOW` 打开已校验的现有文件（只读）。
///
/// `PathValidator::validate` 与实际 `open` 之间存在 validate-then-use 窗口：
/// 攻击者可在校验通过后把路径末段替换为符号链接。本函数在内核打开时
/// 拒绝末段符号链接（`ELOOP`），把该竞态窗口压缩到中间目录组件被攻击者
/// 控制的场景。仅 Unix 平台支持 `O_NOFOLLOW`，其他平台退化为普通打开。
///
/// 应紧随 `validate`/`validate_file_path` 调用使用。
#[cfg(unix)]
pub fn open_validated_file(path: &Path) -> std::io::Result<std::fs::File> {
    use nix::fcntl::OFlag;
    use nix::sys::stat::Mode;

    let fd = nix::fcntl::open(
        path,
        OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    Ok(std::fs::File::from(fd))
}

/// 非 Unix 平台的退化实现：无 `O_NOFOLLOW`，仅普通打开。
#[cfg(not(unix))]
pub fn open_validated_file(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(path)
}

/// 以 `O_NOFOLLOW | O_CREAT | O_TRUNC` 创建/截断已校验的输出文件（写入，0600）。
///
/// 语义对齐 [`std::fs::File::create`]，但末段为符号链接时在内核层被拒绝
/// （`ELOOP`），关闭"校验后输出路径被替换为符号链接"的竞态。非 Unix
/// 平台退化为普通 `File::create`。
#[cfg(unix)]
pub fn create_validated_file(path: &Path) -> std::io::Result<std::fs::File> {
    use nix::fcntl::OFlag;
    use nix::sys::stat::Mode;

    let fd = nix::fcntl::open(
        path,
        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::from_bits_truncate(0o600),
    )
    .map_err(std::io::Error::from)?;
    Ok(std::fs::File::from(fd))
}

/// 非 Unix 平台的退化实现：无 `O_NOFOLLOW`，仅普通创建。
#[cfg(not(unix))]
pub fn create_validated_file(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::create(path)
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

        // 平台相关绝对路径：Windows 用驱动盘符形式，其余平台用 POSIX 绝对路径
        let abs = if cfg!(windows) {
            Path::new("C:\\Windows\\system32")
        } else {
            Path::new("/absolute/path")
        };
        assert!(!validator.validate(abs).valid);
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

        // 路径分隔符中性断言：Windows 输出 `\`，其余平台输出 `/`
        let norm = |p: &Path| p.to_string_lossy().replace('\\', "/").to_string();

        let sanitized = validator.sanitize(Path::new("foo/../bar")).unwrap();
        assert_eq!(norm(&sanitized), "bar");

        let sanitized = validator.sanitize(Path::new("foo/./bar")).unwrap();
        assert_eq!(norm(&sanitized), "foo/bar");

        let sanitized = validator.sanitize(Path::new("foo/../bar/../baz")).unwrap();
        assert_eq!(norm(&sanitized), "baz");
    }

    #[test]
    fn test_sanitize_rejects_pure_traversal() {
        let validator = PathValidator::new();

        // 前导 ".." 在空栈上弹出会逃逸虚拟根目录，必须报错而非静默丢弃
        assert!(validator.sanitize(Path::new("../../etc/passwd")).is_err());
        assert!(validator.sanitize(Path::new("..")).is_err());
        assert!(validator.sanitize(Path::new("../foo")).is_err());
        assert!(validator.sanitize(Path::new("../..")).is_err());
    }

    #[test]
    fn test_sanitize_allows_internal_traversal() {
        let validator = PathValidator::new();

        // 内部 ".." 有可弹出的组件，规范化后不逃逸
        let sanitized = validator.sanitize(Path::new("foo/..")).unwrap();
        assert!(sanitized.as_os_str().is_empty());
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
        let sanitized = validator.sanitize(Path::new("././foo")).unwrap();
        assert_eq!(sanitized.to_string_lossy(), "foo");
    }

    #[test]
    fn test_sanitize_empty_path() {
        let validator = PathValidator::new();
        let sanitized = validator.sanitize(Path::new("")).unwrap();
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

    #[cfg(unix)]
    #[test]
    fn test_symlink_detection_with_real_symlink() {
        // Fixed: std::fs::symlink_metadata() does NOT follow symlinks, so
        // metadata.file_type().is_symlink() correctly detects symlinks.
        // 仅 unix 平台语义（Windows 无普通用户 symlink 权限）。
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let target_file = temp_dir.path().join("target.log");
        std::fs::write(&target_file, "test").expect("failed to write target");
        let symlink_path = temp_dir.path().join("link.log");

        std::os::unix::fs::symlink(&target_file, &symlink_path).expect("failed to create symlink");

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: None,
            allow_symlinks: false,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);

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

    #[cfg(unix)]
    #[test]
    fn test_symlink_allowed() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let target_file = temp_dir.path().join("target.log");
        std::fs::write(&target_file, "test").expect("failed to write target");
        let symlink_path = temp_dir.path().join("link.log");

        std::os::unix::fs::symlink(&target_file, &symlink_path).expect("failed to create symlink");

        let config = PathValidatorConfig {
            allow_absolute: true,
            base_dir: None,
            allow_symlinks: true,
            deny_components: vec![],
        };
        let validator = PathValidator::with_config(config);

        let result = validator.validate(&symlink_path);
        assert!(result.valid);
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

    #[test]
    fn test_validate_accepts_filenames_with_double_dots() {
        // filenames containing ".." as part of the name should be accepted
        // e.g., "foo..bar" is a valid filename, not a path traversal
        let validator = super::PathValidator::new();
        let result = validator.validate(Path::new("foo..bar"));
        assert!(
            result.valid,
            "foo..bar should be accepted, got error: {:?}",
            result.error
        );

        // But actual parent directory traversal should still be rejected
        let result = validator.validate(Path::new("../etc/passwd"));
        assert!(!result.valid, "../etc/passwd should be rejected");
    }

    #[test]
    #[cfg(unix)]
    fn test_open_validated_file_rejects_symlink() {
        let dir = tempfile::TempDir::new().unwrap();
        let real = dir.path().join("real.log");
        std::fs::write(&real, b"data").unwrap();
        let link = dir.path().join("link.log");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        // 末段符号链接必须被内核拒绝（ELOOP）
        assert!(
            super::open_validated_file(&link).is_err(),
            "symlinked leaf must be rejected by O_NOFOLLOW"
        );
        // 普通文件正常打开
        let f = super::open_validated_file(&real).expect("regular file should open");
        drop(f);
    }

    #[test]
    #[cfg(unix)]
    fn test_create_validated_file_rejects_symlink_and_preserves_target() {
        use std::io::Write;

        let dir = tempfile::TempDir::new().unwrap();
        let victim = dir.path().join("victim.log");
        std::fs::write(&victim, b"do not touch").unwrap();
        let out_link = dir.path().join("out.log");
        std::os::unix::fs::symlink(&victim, &out_link).unwrap();

        // 输出路径为符号链接时创建必须失败，且目标内容不被破坏
        assert!(
            super::create_validated_file(&out_link).is_err(),
            "symlinked output must be rejected by O_NOFOLLOW"
        );
        assert_eq!(std::fs::read(&victim).unwrap(), b"do not touch");

        // 普通路径正常创建并可写
        let out = dir.path().join("out2.log");
        super::create_validated_file(&out)
            .unwrap()
            .write_all(b"x")
            .unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"x");
    }
}
