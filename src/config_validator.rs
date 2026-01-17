// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 配置验证模块
//!
//! 提供统一的配置验证接口

use crate::error::InklogError;

/// 配置验证 trait
///
/// 所有需要验证的配置结构体都应该实现这个 trait
pub trait ConfigValidator {
    /// 验证配置是否有效
    ///
    /// # 返回值
    ///
    /// 返回 `Ok(())` 如果配置有效，否则返回 `Err(InklogError)`
    fn validate(&self) -> Result<(), InklogError>;
}

/// 日志级别验证器
pub fn validate_log_level(level: &str) -> Result<(), InklogError> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.to_lowercase().as_str()) {
        return Err(InklogError::ConfigError(format!(
            "Invalid log level: {}. Valid levels are: {}",
            level,
            valid_levels.join(", ")
        )));
    }
    Ok(())
}

/// 路径验证器
///
/// 验证路径是否有效，如果父目录不存在则尝试创建
pub fn validate_path(path: &std::path::Path) -> Result<(), InklogError> {
    if path.as_os_str().is_empty() {
        return Err(InklogError::ConfigError("Path cannot be empty".into()));
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                InklogError::ConfigError(format!("Cannot create directory {}: {}", parent.display(), e))
            })?;
        }
    }

    Ok(())
}

/// URL 验证器
pub fn validate_url(url: &str, field_name: &str) -> Result<(), InklogError> {
    if url.is_empty() {
        return Err(InklogError::ConfigError(format!("{} cannot be empty", field_name)));
    }
    Ok(())
}

/// 数值验证器
///
/// 验证数值是否大于 0
pub fn validate_positive<T>(value: T, field_name: &str) -> Result<(), InklogError>
where
    T: std::ops::Sub<T, Output = T>
        + std::cmp::PartialOrd<T>
        + std::fmt::Display
        + Copy
        + Into<i64>,
{
    if value <= 0.into() {
        return Err(InklogError::ConfigError(format!(
            "{} must be > 0, got {}",
            field_name, value
        )));
    }
    Ok(())
}

/// 字符串验证器
///
/// 验证字符串是否非空
pub fn validate_non_empty(value: &str, field_name: &str) -> Result<(), InklogError> {
    if value.is_empty() {
        return Err(InklogError::ConfigError(format!("{} cannot be empty", field_name)));
    }
    Ok(())
}
