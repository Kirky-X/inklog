// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Observability module - monitoring and health.

pub mod metrics;

pub use metrics::{
    FallbackAction, FallbackConfig, FallbackState, GaugeF64, HealthStatus, Metrics, SinkHealth,
    SinkHealthMonitor, SinkStatus,
};

// 通用业务指标 registry（Prometheus 文本格式导出；`metrics-registry` feature）
#[cfg(feature = "metrics-registry")]
pub mod metrics_registry;
