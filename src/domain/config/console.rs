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
/// masking_enabled = false
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
    #[serde(default)]
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
            masking_enabled: false,
        }
    }
}
