// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 采样策略配置：限流压力下决定保留哪些记录。
//!
//! 未配置（`per_level` 与 `per_target_prefix` 均为空）时 Subscriber 走内置
//! 兜底（非关键级别丢弃，ERROR/FATAL 1-in-N 采样），与无采样策略的行为一致。

use crate::InklogError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 单条 target 前缀规则：级别豁免阈值 + N 取 1 采样率。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TargetSamplingRule {
    /// target 命中前缀且级别 ≥ 该级别的记录始终放行（None = 不按级别豁免）
    #[serde(default)]
    pub keep_level: Option<String>,
    /// 未豁免记录的 N 取 1 采样率（≥1；1 = 全放行）
    #[serde(default = "default_sample_every_n")]
    pub sample_every_n: u64,
}

fn default_sample_every_n() -> u64 {
    1
}

impl Default for TargetSamplingRule {
    fn default() -> Self {
        Self {
            keep_level: None,
            sample_every_n: default_sample_every_n(),
        }
    }
}

/// 采样策略配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SamplingConfig {
    /// 每级别采样率：级别名（大小写不敏感，含 warning/critical 别名）→ N
    #[serde(default)]
    pub per_level: HashMap<String, u64>,
    /// target 前缀 → 规则（命中时最长前缀优先）
    #[serde(default)]
    pub per_target_prefix: HashMap<String, TargetSamplingRule>,
}

impl SamplingConfig {
    /// 是否未配置任何规则（未配置 = Subscriber 走内置兜底采样）。
    pub fn is_empty(&self) -> bool {
        self.per_level.is_empty() && self.per_target_prefix.is_empty()
    }

    /// 校验配置：级别名合法、采样率 ≥ 1、前缀非空。
    ///
    /// 校验失败即拒绝加载（与 `Sampler` 基座的 ConfigError 约定一致），
    /// 不做静默修正——采样率与豁免阈值直接决定压力下的日志保留语义。
    pub fn validate(&self) -> Result<(), InklogError> {
        for (level, rate) in &self.per_level {
            if !crate::LogLevel::is_valid_level(level) {
                return Err(InklogError::ConfigError(format!(
                    "sampling.per_level[{level:?}] is not a valid level. Valid: {}",
                    crate::LogLevel::VALID_LEVEL_STRINGS.join(", ")
                )));
            }
            if *rate == 0 {
                return Err(InklogError::ConfigError(format!(
                    "sampling.per_level[{level:?}] rate must be >= 1, got 0"
                )));
            }
        }
        for (prefix, rule) in &self.per_target_prefix {
            if prefix.is_empty() {
                return Err(InklogError::ConfigError(
                    "sampling.per_target_prefix key must not be empty".to_string(),
                ));
            }
            if rule.sample_every_n == 0 {
                return Err(InklogError::ConfigError(format!(
                    "sampling.per_target_prefix[{prefix:?}].sample_every_n must be >= 1, got 0"
                )));
            }
            if let Some(keep_level) = &rule.keep_level
                && !crate::LogLevel::is_valid_level(keep_level)
            {
                return Err(InklogError::ConfigError(format!(
                    "sampling.per_target_prefix[{prefix:?}].keep_level {keep_level:?} is not a valid level. Valid: {}",
                    crate::LogLevel::VALID_LEVEL_STRINGS.join(", ")
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
    fn test_default_sampling_config_is_empty_and_valid() {
        let config = SamplingConfig::default();
        assert!(config.is_empty(), "default config must have no rules");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_is_empty_reflects_rule_presence() {
        let mut config = SamplingConfig::default();
        config.per_level.insert("info".to_string(), 10);
        assert!(!config.is_empty());

        let mut config = SamplingConfig::default();
        config
            .per_target_prefix
            .insert("app::".to_string(), TargetSamplingRule::default());
        assert!(!config.is_empty());
    }

    #[test]
    fn test_validate_accepts_valid_config() {
        let mut config = SamplingConfig::default();
        config.per_level.insert("info".to_string(), 10);
        config.per_level.insert("WARNING".to_string(), 5);
        config.per_target_prefix.insert(
            "app::audit".to_string(),
            TargetSamplingRule {
                keep_level: Some("debug".to_string()),
                sample_every_n: 3,
            },
        );
        config
            .per_target_prefix
            .insert("app::noise".to_string(), TargetSamplingRule::default());
        assert!(
            config.validate().is_ok(),
            "valid config must pass: {config:?}"
        );
    }

    #[test]
    fn test_validate_rejects_zero_per_level_rate() {
        let mut config = SamplingConfig::default();
        config.per_level.insert("info".to_string(), 0);
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("per_level"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_invalid_per_level_key() {
        let mut config = SamplingConfig::default();
        config.per_level.insert("verbose".to_string(), 10);
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("verbose"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_zero_rule_rate() {
        let mut config = SamplingConfig::default();
        config.per_target_prefix.insert(
            "app::noise".to_string(),
            TargetSamplingRule {
                keep_level: None,
                sample_every_n: 0,
            },
        );
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("sample_every_n"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_invalid_keep_level() {
        let mut config = SamplingConfig::default();
        config.per_target_prefix.insert(
            "app::audit".to_string(),
            TargetSamplingRule {
                keep_level: Some("verbose".to_string()),
                sample_every_n: 1,
            },
        );
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("keep_level") && err.to_string().contains("verbose"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_validate_rejects_empty_prefix() {
        let mut config = SamplingConfig::default();
        config
            .per_target_prefix
            .insert(String::new(), TargetSamplingRule::default());
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("prefix"),
            "unexpected error: {err}"
        );
    }
}
