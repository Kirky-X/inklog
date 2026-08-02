// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Performance tuning configuration.

use serde::{Deserialize, Serialize};

// ============================================================================
// ChannelStrategy - Adaptive channel sizing strategy
// ============================================================================

/// Channel sizing strategy for log buffer management.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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
        match s.to_lowercase().as_str() {
            "fixed" => Ok(ChannelStrategy::Fixed),
            "adaptive" => Ok(ChannelStrategy::Adaptive),
            _ => Err(format!("Unknown channel strategy: {}", s)),
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        }
    }
}
