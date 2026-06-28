// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 日志级别枚举定义
//!
//! 提供类型安全的日志级别表示，消除硬编码字符串字面量

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// 日志级别枚举
///
/// 用于表示日志记录的严重程度，按从低到高的顺序排列
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum LogLevel {
    /// 跟踪级别 - 最详细的日志
    Trace = 0,
    /// 调试级别 - 开发时使用的详细信息
    Debug = 1,
    /// 信息级别 - 一般信息
    #[default]
    Info = 2,
    /// 警告级别 - 警告信息
    Warn = 3,
    /// 错误级别 - 错误信息
    Error = 4,
    /// 致命级别 - 导致程序终止的错误
    Fatal = 5,
}

impl LogLevel {
    /// 从字符串解析日志级别（大小写不敏感）
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            "FATAL" | "CRITICAL" => Some(LogLevel::Fatal),
            _ => None,
        }
    }

    /// 获取日志级别的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    /// 获取日志级别的短字符串表示（用于显示）
    pub fn as_short_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRC",
            LogLevel::Debug => "DBG",
            LogLevel::Info => "INF",
            LogLevel::Warn => "WRN",
            LogLevel::Error => "ERR",
            LogLevel::Fatal => "FTL",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for LogLevel {
    type Err = LogLevelParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| LogLevelParseError::Unknown(s.to_string()))
    }
}

/// 解析日志级别字符串的错误
#[derive(Error, Debug)]
pub enum LogLevelParseError {
    #[error("Unknown log level: {0}")]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid() {
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("WARN"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("FATAL"), Some(LogLevel::Fatal));
        assert_eq!(LogLevel::from_str("CRITICAL"), Some(LogLevel::Fatal));
    }

    #[test]
    fn test_from_str_invalid() {
        assert_eq!(LogLevel::from_str("INVALID"), None);
        assert_eq!(LogLevel::from_str(""), None);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_ordering() {
        assert!(LogLevel::Error > LogLevel::Debug);
        assert!(LogLevel::Info <= LogLevel::Warn);
        assert!(LogLevel::Trace < LogLevel::Fatal);
    }

    #[test]
    fn test_as_str_all_variants() {
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Fatal.as_str(), "FATAL");
    }

    #[test]
    fn test_as_short_str_all_variants() {
        assert_eq!(LogLevel::Trace.as_short_str(), "TRC");
        assert_eq!(LogLevel::Debug.as_short_str(), "DBG");
        assert_eq!(LogLevel::Info.as_short_str(), "INF");
        assert_eq!(LogLevel::Warn.as_short_str(), "WRN");
        assert_eq!(LogLevel::Error.as_short_str(), "ERR");
        assert_eq!(LogLevel::Fatal.as_short_str(), "FTL");
    }

    #[test]
    fn test_display_matches_as_str() {
        for level in [
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Fatal,
        ] {
            assert_eq!(format!("{}", level), level.as_str());
        }
    }

    #[test]
    fn test_from_str_trait_valid() {
        let level: LogLevel = "debug".parse().expect("valid level should parse");
        assert_eq!(level, LogLevel::Debug);

        let level: LogLevel = "WARNING".parse().expect("WARNING should parse to Warn");
        assert_eq!(level, LogLevel::Warn);

        let level: LogLevel = "CRITICAL".parse().expect("CRITICAL should parse to Fatal");
        assert_eq!(level, LogLevel::Fatal);
    }

    #[test]
    fn test_from_str_trait_invalid_returns_error() {
        let result: Result<LogLevel, _> = "invalid_level".parse();
        let err = result.expect_err("invalid level should error");
        assert!(format!("{}", err).contains("Unknown log level"));
        assert!(format!("{}", err).contains("invalid_level"));
    }

    #[test]
    fn test_from_str_trait_empty_string() {
        let result: Result<LogLevel, _> = "".parse();
        assert!(result.is_err());
    }
}
