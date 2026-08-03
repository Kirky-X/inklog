// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Global logger configuration.

use serde::{Deserialize, Serialize};

/// Returns `true` — shared default for several `bool` fields.
pub(crate) fn default_true() -> bool {
    true
}

// ============================================================================
// GlobalConfig - Global logger settings
// ============================================================================

/// Global logger configuration.
///
/// Controls the overall behavior of the logging system including log level,
/// format string, and fallback settings.
///
/// # Configuration Priority
///
/// Configuration values are loaded with the following priority (highest to lowest):
/// 1. Environment variables (prefix `INKLOG_GLOBAL_`)
/// 2. Configuration file values
/// 3. Default values
///
/// # Example TOML Configuration
///
/// ```toml
/// [global]
/// level = "debug"
/// format = "{timestamp} [{level}] {target} - {message}"
/// masking_enabled = true
/// auto_fallback = true
/// fallback_initial_delay_ms = 1000
/// fallback_max_delay_ms = 60000
/// fallback_max_retries = 10
/// ```
///
/// # Environment Variable Overrides
///
/// Any field can be overridden via environment variables:
/// ```bash
/// export INKLOG_GLOBAL_LEVEL=debug
/// export INKLOG_GLOBAL_MASKING_ENABLED=false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalConfig {
    /// Minimum log level to capture.
    ///
    /// Valid values (case-insensitive): `trace`, `debug`, `info`, `warn`, `error`, `fatal`.
    /// Logs below this level are ignored.
    ///
    /// # Default
    ///
    /// `"info"` - Captures INFO, WARN, ERROR, and FATAL logs.
    #[serde(default = "default_global_level")]
    pub level: String,

    /// Log message format template.
    ///
    /// Supports placeholders that are replaced with values from each log record:
    /// - `{timestamp}` - ISO 8601 timestamp
    /// - `{level}` - Log level (INFO, DEBUG, etc.)
    /// - `{target}` - Module path that emitted the log
    /// - `{message}` - Log message content
    /// - `{file}` - Source file path (optional)
    /// - `{line}` - Line number in source file (optional)
    /// - `{thread_id}` - Thread identifier
    /// - `{fields}` - Additional structured fields (JSON)
    ///
    /// # Default
    ///
    /// `"{timestamp} [{level}] {target} - {message}"`
    #[serde(default = "default_global_format")]
    pub format: String,

    /// Enable sensitive data masking.
    ///
    /// When enabled, sensitive patterns (passwords, API keys, credit cards, etc.)
    /// are automatically replaced with `[REDACTED]` placeholders.
    ///
    /// # Default
    ///
    /// `true` - Masking enabled by default for security.
    #[serde(default = "default_true")]
    pub masking_enabled: bool,

    /// Enable automatic fallback on sink failures.
    ///
    /// When a sink fails repeatedly, the system automatically falls back to
    /// alternative sinks (e.g., database → file → console).
    ///
    /// # Default
    ///
    /// `true` - Fallback enabled for reliability.
    #[serde(default = "default_true")]
    pub auto_fallback: bool,

    /// Initial delay before first retry (milliseconds).
    ///
    /// When a sink fails, the system waits this duration before attempting
    /// the first retry. Subsequent retries use exponential backoff.
    ///
    /// # Default
    ///
    /// `1000` ms (1 second)
    #[serde(default = "default_fallback_initial_delay")]
    pub fallback_initial_delay_ms: u64,

    /// Maximum delay between retries (milliseconds).
    ///
    /// Caps the exponential backoff delay to prevent excessive waiting.
    ///
    /// # Default
    ///
    /// `60000` ms (60 seconds)
    #[serde(default = "default_fallback_max_delay")]
    pub fallback_max_delay_ms: u64,

    /// Maximum number of retry attempts.
    ///
    /// After this many failures, the sink is marked as unhealthy and
    /// fallback mechanisms are activated.
    ///
    /// # Default
    ///
    /// `10` retries
    #[serde(default = "default_fallback_max_retries")]
    pub fallback_max_retries: u32,
}

fn default_global_level() -> String {
    "info".to_string()
}
fn default_global_format() -> String {
    "{timestamp} [{level}] {target} - {message}".to_string()
}
fn default_fallback_initial_delay() -> u64 {
    1000
}
fn default_fallback_max_delay() -> u64 {
    60000
}
fn default_fallback_max_retries() -> u32 {
    10
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            level: default_global_level(),
            format: default_global_format(),
            masking_enabled: default_true(),
            auto_fallback: default_true(),
            fallback_initial_delay_ms: default_fallback_initial_delay(),
            fallback_max_delay_ms: default_fallback_max_delay(),
            fallback_max_retries: default_fallback_max_retries(),
        }
    }
}

impl GlobalConfig {
    /// Validate fallback backoff parameters.
    ///
    /// Ensures:
    /// - `fallback_initial_delay_ms <= fallback_max_delay_ms`
    /// - `fallback_max_retries > 0`
    pub fn validate(&mut self) {
        if self.fallback_initial_delay_ms > self.fallback_max_delay_ms {
            tracing::warn!(
                initial = self.fallback_initial_delay_ms,
                max = self.fallback_max_delay_ms,
                "fallback_initial_delay_ms > fallback_max_delay_ms, clamping"
            );
            self.fallback_initial_delay_ms = self.fallback_max_delay_ms;
        }
        if self.fallback_max_retries == 0 {
            tracing::warn!("fallback_max_retries is 0, resetting to 1");
            self.fallback_max_retries = 1;
        }
    }
}
