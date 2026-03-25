// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Observability module - monitoring and health.

pub mod metrics;

pub use metrics::{
    FallbackConfig, FallbackState, GaugeF64, HealthStatus, Metrics, SinkHealthMonitor, SinkStatus,
};
