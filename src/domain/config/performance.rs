// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Performance tuning configuration.

use serde::{Deserialize, Serialize};

// ============================================================================
// ChannelStrategy - Adaptive channel sizing strategy
// ============================================================================

/// Channel sizing strategy for log buffer management.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelStrategy {
    #[serde(rename = "fixed")]
    #[default]
    Fixed,
    #[serde(rename = "adaptive")]
    Adaptive,
}

impl std::str::FromStr for ChannelStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Use eq_ignore_ascii_case to avoid heap allocation from to_lowercase()
        if s.eq_ignore_ascii_case("fixed") {
            Ok(ChannelStrategy::Fixed)
        } else if s.eq_ignore_ascii_case("adaptive") {
            Ok(ChannelStrategy::Adaptive)
        } else {
            Err(format!("Unknown channel strategy: {}", s))
        }
    }
}

impl std::fmt::Display for ChannelStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelStrategy::Fixed => write!(f, "fixed"),
            ChannelStrategy::Adaptive => write!(f, "adaptive"),
        }
    }
}

// ============================================================================
// PerformanceConfig - Performance tuning parameters
// ============================================================================

/// Performance tuning configuration for log channel and worker management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PerformanceConfig {
    #[serde(default = "default_channel_capacity")]
    pub channel_capacity: usize,
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    #[serde(default)]
    pub channel_strategy: ChannelStrategy,
    #[serde(default = "default_expand_threshold")]
    pub expand_threshold_percent: u8,
    #[serde(default = "default_shrink_threshold")]
    pub shrink_threshold_percent: u8,
    #[serde(default = "default_shrink_wait")]
    pub shrink_wait_seconds: u64,
    #[serde(default = "default_min_capacity")]
    pub min_capacity: usize,
    #[serde(default = "default_max_capacity")]
    pub max_capacity: usize,
    /// Maximum log rate in logs/sec. `None` = unlimited.
    #[serde(default)]
    pub rate_limit: Option<u64>,
}

fn default_channel_capacity() -> usize {
    10000
}
fn default_worker_threads() -> usize {
    3
}
fn default_expand_threshold() -> u8 {
    80
}
fn default_shrink_threshold() -> u8 {
    20
}
fn default_shrink_wait() -> u64 {
    30
}
fn default_min_capacity() -> usize {
    1000
}
fn default_max_capacity() -> usize {
    50000
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            channel_capacity: default_channel_capacity(),
            worker_threads: default_worker_threads(),
            channel_strategy: ChannelStrategy::default(),
            expand_threshold_percent: default_expand_threshold(),
            shrink_threshold_percent: default_shrink_threshold(),
            shrink_wait_seconds: default_shrink_wait(),
            min_capacity: default_min_capacity(),
            max_capacity: default_max_capacity(),
            rate_limit: None,
        }
    }
}

impl PerformanceConfig {
    /// Validate performance tuning parameters.
    ///
    /// Ensures:
    /// - Percentage thresholds are within 0–100
    /// - `min_capacity <= max_capacity`
    /// - `shrink_threshold_percent < expand_threshold_percent`
    pub fn validate(&mut self) {
        self.expand_threshold_percent = self.expand_threshold_percent.min(100);
        self.shrink_threshold_percent = self.shrink_threshold_percent.min(100);

        if self.min_capacity > self.max_capacity {
            tracing::warn!(
                min = self.min_capacity,
                max = self.max_capacity,
                "min_capacity > max_capacity, swapping"
            );
            std::mem::swap(&mut self.min_capacity, &mut self.max_capacity);
        }
        if self.shrink_threshold_percent >= self.expand_threshold_percent {
            tracing::warn!(
                shrink = self.shrink_threshold_percent,
                expand = self.expand_threshold_percent,
                "shrink_threshold >= expand_threshold, resetting to defaults"
            );
            self.shrink_threshold_percent = default_shrink_threshold();
            self.expand_threshold_percent = default_expand_threshold();
        }

        // Validate rate_limit: Some(0) is invalid, reset to None
        if let Some(rate) = self.rate_limit
            && rate == 0
        {
            tracing::warn!("rate_limit = 0 is invalid, resetting to None (unlimited)");
            self.rate_limit = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_strategy_from_str() {
        assert_eq!(
            "fixed".parse::<ChannelStrategy>().unwrap(),
            ChannelStrategy::Fixed
        );
        assert_eq!(
            "FIXED".parse::<ChannelStrategy>().unwrap(),
            ChannelStrategy::Fixed
        );
        assert_eq!(
            "adaptive".parse::<ChannelStrategy>().unwrap(),
            ChannelStrategy::Adaptive
        );
        assert_eq!(
            "Adaptive".parse::<ChannelStrategy>().unwrap(),
            ChannelStrategy::Adaptive
        );
        assert!("unknown".parse::<ChannelStrategy>().is_err());
    }

    #[test]
    fn test_channel_strategy_display() {
        assert_eq!(ChannelStrategy::Fixed.to_string(), "fixed");
        assert_eq!(ChannelStrategy::Adaptive.to_string(), "adaptive");
    }

    #[test]
    fn test_performance_config_default() {
        let cfg = PerformanceConfig::default();
        assert_eq!(cfg.channel_capacity, 10000);
        assert_eq!(cfg.worker_threads, 3);
        assert_eq!(cfg.expand_threshold_percent, 80);
        assert_eq!(cfg.shrink_threshold_percent, 20);
        assert_eq!(cfg.min_capacity, 1000);
        assert_eq!(cfg.max_capacity, 50000);
    }

    #[test]
    fn test_validate_clamps_percentages() {
        let mut cfg = PerformanceConfig::default();
        cfg.expand_threshold_percent = 150;
        cfg.shrink_threshold_percent = 50;
        cfg.validate();
        // 150 clamped to 100; shrink 50 < expand 100, no reset
        assert_eq!(cfg.expand_threshold_percent, 100);
        assert_eq!(cfg.shrink_threshold_percent, 50);
    }

    #[test]
    fn test_validate_swaps_min_max() {
        let mut cfg = PerformanceConfig::default();
        cfg.min_capacity = 99999;
        cfg.max_capacity = 100;
        cfg.validate();
        assert_eq!(cfg.min_capacity, 100);
        assert_eq!(cfg.max_capacity, 99999);
    }

    #[test]
    fn test_validate_resets_shrink_ge_expand() {
        let mut cfg = PerformanceConfig::default();
        cfg.shrink_threshold_percent = 80;
        cfg.expand_threshold_percent = 50;
        cfg.validate();
        assert_eq!(cfg.shrink_threshold_percent, default_shrink_threshold());
        assert_eq!(cfg.expand_threshold_percent, default_expand_threshold());
    }

    #[test]
    fn test_validate_rate_limit_zero_reset_to_none() {
        let mut cfg = PerformanceConfig::default();
        cfg.rate_limit = Some(0);
        cfg.validate();
        assert_eq!(cfg.rate_limit, None);
    }

    #[test]
    fn test_validate_rate_limit_positive_preserved() {
        let mut cfg = PerformanceConfig::default();
        cfg.rate_limit = Some(10000);
        cfg.validate();
        assert_eq!(cfg.rate_limit, Some(10000));
    }

    #[test]
    fn test_validate_rate_limit_none_unchanged() {
        let mut cfg = PerformanceConfig::default();
        cfg.rate_limit = None;
        cfg.validate();
        assert_eq!(cfg.rate_limit, None);
    }
}
