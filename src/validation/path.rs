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
            if let Ok(metadata) = std::fs::metadata(path) {
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
}
