// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Log content sanitization for security.
//!
//! This module provides log content sanitization to prevent log injection attacks
//! and ensure safe log output for SIEM systems.

use regex::Regex;

/// Escape mode for log content sanitization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EscapeMode {
    /// Minimal escaping - only escape newlines and control characters
    #[default]
    Minimal,
    /// Strict escaping - escape all non-printable characters
    Strict,
    /// JSON-safe escaping - escape for JSON output
    JsonSafe,
}

/// Configuration for log sanitization.
#[derive(Debug, Clone)]
pub struct SanitizerConfig {
    /// Escape mode
    pub mode: EscapeMode,
    /// Maximum message length (0 = unlimited)
    pub max_length: usize,
    /// Replace patterns with replacement
    pub sensitive_patterns: Vec<(Regex, String)>,
    /// Custom replacements for specific strings
    pub custom_replacements: Vec<(String, String)>,
}

impl Default for SanitizerConfig {
    fn default() -> Self {
        Self {
            mode: EscapeMode::Minimal,
            max_length: 0,
            sensitive_patterns: Vec::new(),
            custom_replacements: vec![
                ("\r\n".to_string(), "\\n".to_string()),
                ("\n".to_string(), "\\n".to_string()),
                ("\r".to_string(), "\\r".to_string()),
            ],
        }
    }
}

/// Log sanitizer for preventing log injection.
#[derive(Debug, Clone)]
pub struct LogSanitizer {
    config: SanitizerConfig,
    sensitive_regexes: Vec<(Regex, String)>,
}

impl LogSanitizer {
    /// Create a new LogSanitizer with default configuration.
    pub fn new() -> Self {
        Self {
            config: SanitizerConfig::default(),
            sensitive_regexes: Self::default_sensitive_patterns(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: SanitizerConfig) -> Self {
        Self {
            config,
            sensitive_regexes: Self::default_sensitive_patterns(),
        }
    }

    fn default_sensitive_patterns() -> Vec<(Regex, String)> {
        vec![
            (
                Regex::new(r"\b\d{13,16}\b").expect("hardcoded card number regex is valid"),
                "[CARD_NUM]".to_string(),
            ),
            (
                Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")
                    .expect("hardcoded email regex is valid"),
                "[EMAIL]".to_string(),
            ),
            (
                Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("hardcoded SSN regex is valid"),
                "[SSN]".to_string(),
            ),
            (
                Regex::new(r"(?i)password\s*[=:]\s*\S+")
                    .expect("hardcoded password regex is valid"),
                "password=[REDACTED]".to_string(),
            ),
            (
                Regex::new(r"(?i)token\s*[=:]\s*\S+").expect("hardcoded token regex is valid"),
                "token=[REDACTED]".to_string(),
            ),
            (
                Regex::new(r"(?i)api[_-]?key\s*[=:]\s*\S+")
                    .expect("hardcoded api_key regex is valid"),
                "api_key=[REDACTED]".to_string(),
            ),
            (
                Regex::new(r"Bearer\s+[A-Za-z0-9\-_\.]+")
                    .expect("hardcoded Bearer token regex is valid"),
                "Bearer [TOKEN]".to_string(),
            ),
            (
                Regex::new(r"Basic\s+[A-Za-z0-9+/=]+")
                    .expect("hardcoded Basic auth regex is valid"),
                "Basic [AUTH]".to_string(),
            ),
        ]
    }

    /// Sanitize a log message.
    pub fn sanitize(&self, message: &str) -> String {
        let mut result = message.to_string();

        for (pattern, replacement) in &self.sensitive_regexes {
            result = pattern
                .replace_all(&result, replacement.as_str())
                .to_string();
        }

        for (from, to) in &self.config.custom_replacements {
            result = result.replace(from, to);
        }

        match self.config.mode {
            EscapeMode::Minimal => {
                result = self.escape_minimal(&result);
            }
            EscapeMode::Strict => {
                result = self.escape_strict(&result);
            }
            EscapeMode::JsonSafe => {
                result = self.escape_json(&result);
            }
        }

        if self.config.max_length > 0 && result.len() > self.config.max_length {
            result.truncate(self.config.max_length);
            result.push_str("...[truncated]");
        }

        result
    }

    fn escape_minimal(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                c if c.is_control() && c != '\n' && c != '\r' && c != '\t' => {
                    result.push_str(&format!("\\x{:02x}", c as u8));
                }
                _ => result.push(c),
            }
        }
        result
    }

    fn escape_strict(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            if c.is_control() || c == '\\' || c == '"' {
                result.push_str(&format!("\\u{:04x}", c as u32));
            } else {
                result.push(c);
            }
        }
        result
    }

    fn escape_json(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                c if c.is_control() => {
                    result.push_str(&format!("\\u{:04x}", c as u32));
                }
                _ => result.push(c),
            }
        }
        result
    }

    /// Add a custom sensitive pattern.
    pub fn add_pattern(&mut self, pattern: Regex, replacement: String) {
        self.sensitive_regexes.push((pattern, replacement));
    }

    /// Add a custom string replacement.
    pub fn add_replacement(&mut self, from: String, to: String) {
        self.config.custom_replacements.push((from, to));
    }
}

impl Default for LogSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newline_escaping() {
        let sanitizer = LogSanitizer::new();

        let result = sanitizer.sanitize("Hello\nWorld");
        assert!(result.contains("\\n"));
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_sensitive_data_redaction() {
        let sanitizer = LogSanitizer::new();

        let result = sanitizer.sanitize("User password=secret123");
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("secret123"));

        let result = sanitizer.sanitize("api_key=sk-1234567890");
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("sk-1234567890"));
    }

    #[test]
    fn test_email_redaction() {
        let sanitizer = LogSanitizer::new();

        let result = sanitizer.sanitize("Contact user@example.com");
        assert!(result.contains("[EMAIL]"));
        assert!(!result.contains("user@example.com"));
    }

    #[test]
    fn test_escape_modes() {
        let config = SanitizerConfig {
            mode: EscapeMode::JsonSafe,
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("Hello\"World");
        assert!(result.contains("\\\""));
    }

    #[test]
    fn test_max_length() {
        let config = SanitizerConfig {
            max_length: 10,
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("This is a very long message");
        assert!(result.len() <= 10 + "...[truncated]".len());
        assert!(result.contains("...[truncated]"));
    }

    #[test]
    fn test_control_character_escaping() {
        let sanitizer = LogSanitizer::new();

        let result = sanitizer.sanitize("Hello\x00World");
        assert!(result.contains("\\x00"));
    }
}
