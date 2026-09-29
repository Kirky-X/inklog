// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 按 target 前缀分组的限流配额配置（R7：per-target 分级限流）。
//!
//! 与 `performance.rate_limit`（订阅器级全局令牌桶）的关系：命中
//! `rules` 前缀的 target 由其配额组独立裁决，未命中的 target 维持
//! 既有全局路径。未配置（`rules` 为空）时整体零介入，默认全局行为不变。

use crate::InklogError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 按 target 前缀分组的限流配额：前缀 → 该组每秒令牌数（组内共享预算）。
///
/// 查找为按记录执行的线性扫描（最长前缀优先），规则数建议 ≤ 64。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RateLimitConfig {
    /// target 前缀 → 每秒令牌数；最长前缀优先，未命中规则的 target 不受限
    #[serde(default)]
    pub rules: HashMap<String, u64>,
}

impl RateLimitConfig {
    /// 是否未配置任何规则（未配置 = 订阅器不接线，走既有全局限流路径）。
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 校验配置：前缀非空、速率 ≥ 1。
    ///
    /// 校验失败即拒绝加载（配额直接决定压力下的日志保留，不做静默修正——
    /// 与采样配置的硬拒绝语义一致）。
    pub fn validate(&self) -> Result<(), InklogError> {
        for (prefix, rate) in &self.rules {
            if prefix.is_empty() {
                return Err(InklogError::ConfigError(
                    "rate_limit.rules key must not be empty".to_string(),
                ));
            }
            if *rate == 0 {
                return Err(InklogError::ConfigError(format!(
                    "rate_limit.rules[{prefix:?}] rate must be >= 1, got 0"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rate_limit_config_is_empty_and_valid() {
        let config = RateLimitConfig::default();
        assert!(config.is_empty(), "default config must have no rules");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_is_empty_reflects_rule_presence() {
        let mut config = RateLimitConfig::default();
        config.rules.insert("app::audit".to_string(), 100);
        assert!(!config.is_empty());
    }

    #[test]
    fn test_validate_accepts_valid_config() {
        let mut config = RateLimitConfig::default();
        config.rules.insert("app::audit".to_string(), 100);
        config.rules.insert("app::noise".to_string(), 5);
        assert!(
            config.validate().is_ok(),
            "valid config must pass: {config:?}"
        );
    }

    #[test]
    fn test_validate_rejects_empty_prefix() {
        let mut config = RateLimitConfig::default();
        config.rules.insert(String::new(), 10);
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("empty"), "unexpected error: {err}");
    }

    #[test]
    fn test_validate_rejects_zero_rate() {
        let mut config = RateLimitConfig::default();
        config.rules.insert("app::noise".to_string(), 0);
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains(">= 1") && err.to_string().contains("app::noise"),
            "unexpected error: {err}"
        );
    }
}
