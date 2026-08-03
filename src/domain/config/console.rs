// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Console sink configuration.

use super::global::default_true;
use serde::{Deserialize, Serialize};

// ============================================================================
// ConsoleSinkConfig - Console output settings
// ============================================================================

/// Console sink configuration.
///
/// Controls logging output to stdout/stderr with optional colored output
/// and level-based stream routing.
///
/// # Example TOML Configuration
///
/// ```toml
/// [console_sink]
/// enabled = true
/// colored = true
/// stderr_levels = ["error", "warn"]
/// masking_enabled = true
/// ```
///
/// # Stream Routing
///
/// Log levels specified in `stderr_levels` are written to stderr,
/// all other levels go to stdout. This enables:
/// - Separating errors from normal output
/// - Piping stdout to files while keeping errors visible
/// - Integration with monitoring tools that parse stderr
///
/// # Environment Variable Overrides
///
/// ```bash
/// export INKLOG_CONSOLE_SINK_ENABLED=true
/// export INKLOG_CONSOLE_SINK_COLORED=false
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSinkConfig {
    /// Enable console logging.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Enable colored output using ANSI escape codes.
    #[serde(default = "default_true")]
    pub colored: bool,

    /// Log levels to write to stderr instead of stdout.
    #[serde(default = "default_stderr_levels")]
    pub stderr_levels: Vec<String>,

    /// Enable sensitive data masking for console output.
    ///
    /// Defaults to `true` for security consistency with [`GlobalConfig`].
    /// When enabled, PII patterns (emails, phone numbers, etc.) are
    /// automatically redacted from console log output.
    #[serde(default = "default_true")]
    pub masking_enabled: bool,
}

fn default_stderr_levels() -> Vec<String> {
    vec!["error".to_string(), "warn".to_string()]
}

impl Default for ConsoleSinkConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            colored: default_true(),
            stderr_levels: default_stderr_levels(),
            masking_enabled: default_true(),
        }
    }
}

impl ConsoleSinkConfig {
    /// Validate console sink configuration.
    ///
    /// Ensures `stderr_levels` contains only valid log level names.
    /// Invalid entries are removed with a warning.
    pub fn validate(&mut self) {
        let original_len = self.stderr_levels.len();
        self.stderr_levels.retain(|level| {
            if !crate::LogLevel::is_valid_level(level) {
                tracing::warn!(level = %level, "Invalid stderr_levels entry, removing");
                false
            } else {
                true
            }
        });
        if self.stderr_levels.len() != original_len {
            tracing::info!(
                remaining = self.stderr_levels.len(),
                "Removed invalid stderr_levels entries"
            );
        }
    }
}
