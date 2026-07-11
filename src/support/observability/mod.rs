// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Observability module - monitoring and health.

pub mod metrics;

pub use metrics::{
    FallbackConfig, FallbackState, GaugeF64, HealthStatus, Metrics, SinkHealth, SinkHealthMonitor,
    SinkStatus,
};
