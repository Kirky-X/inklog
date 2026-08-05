// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Console sink configuration.

use super::global::default_true;
use crate::support::processing::template::OutputFormat;
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

    /// Output format: text (template-based) or JSON (NDJSON).
    ///
    /// When `Json`, colored output is automatically disabled.
    #[serde(default)]
    pub output_format: OutputFormat,
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
            output_format: OutputFormat::default(),
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

    /// Return invalid `stderr_levels` entries without mutating.
    ///
    /// Useful for strict validation before normalization auto-corrects them.
    pub fn invalid_stderr_levels(&self) -> Vec<&str> {
        self.stderr_levels
            .iter()
            .filter(|level| !crate::LogLevel::is_valid_level(level))
            .map(|s| s.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_console_sink_config() {
        let cfg = ConsoleSinkConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.colored);
        assert_eq!(
            cfg.stderr_levels,
            vec!["error".to_string(), "warn".to_string()]
        );
        assert!(cfg.masking_enabled);
    }

    #[test]
    fn test_validate_removes_invalid_levels() {
        let mut cfg = ConsoleSinkConfig::default();
        cfg.stderr_levels = vec![
            "error".into(),
            "invalid_level".into(),
            "warn".into(),
            "bogus".into(),
        ];
        cfg.validate();
        assert_eq!(
            cfg.stderr_levels,
            vec!["error".to_string(), "warn".to_string()]
        );
    }

    #[test]
    fn test_validate_keeps_valid_levels() {
        let mut cfg = ConsoleSinkConfig::default();
        cfg.stderr_levels = vec![
            "trace".into(),
            "debug".into(),
            "info".into(),
            "warn".into(),
            "warning".into(),
            "error".into(),
            "fatal".into(),
            "critical".into(),
        ];
        let original_len = cfg.stderr_levels.len();
        cfg.validate();
        assert_eq!(cfg.stderr_levels.len(), original_len);
    }

    #[test]
    fn test_invalid_stderr_levels_returns_invalid() {
        let cfg = ConsoleSinkConfig {
            stderr_levels: vec![
                "error".into(),
                "bogus".into(),
                "warn".into(),
                "typo_err".into(),
            ],
            ..Default::default()
        };
        let invalid = cfg.invalid_stderr_levels();
        assert_eq!(invalid, vec!["bogus", "typo_err"]);
    }

    #[test]
    fn test_invalid_stderr_levels_empty_when_all_valid() {
        let cfg = ConsoleSinkConfig {
            stderr_levels: vec!["error".into(), "warn".into()],
            ..Default::default()
        };
        assert!(cfg.invalid_stderr_levels().is_empty());
    }
}
