// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::support::io::sink::LogSink;
use crate::ConsoleSinkConfig;
use crate::DataMasker;
use crate::InklogError;
use crate::LogRecord;
use crate::LogTemplate;
use is_terminal::IsTerminal;
use owo_colors::OwoColorize;
use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

pub struct ConsoleSink {
    config: ConsoleSinkConfig,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    template: LogTemplate,
    masker: DataMasker,
}

impl fmt::Debug for ConsoleSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConsoleSink")
            .field("config", &self.config)
            .field("template", &self.template)
            .finish()
    }
}

impl ConsoleSink {
    pub fn new(config: ConsoleSinkConfig, template: LogTemplate) -> Self {
        Self {
            config,
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
            template,
            masker: DataMasker::new(),
        }
    }

    fn write_record<W: Write>(
        &self,
        writer: &mut W,
        record: &LogRecord,
        use_color: bool,
    ) -> io::Result<()> {
        let formatted_message = self.template.render(record);

        if use_color {
            let level_colored = match record.level.as_str() {
                "ERROR" | "error" => record.level.red().to_string(),
                "WARN" | "warn" => record.level.yellow().to_string(),
                "INFO" | "info" => record.level.green().to_string(),
                "DEBUG" | "debug" => record.level.blue().to_string(),
                "TRACE" | "trace" => record.level.magenta().to_string(),
                _ => record.level.clone(),
            };
            writeln!(
                writer,
                "{}",
                self.apply_color(&formatted_message, &level_colored)
            )
        } else {
            writeln!(writer, "{}", formatted_message)
        }
    }

    fn apply_color(&self, message: &str, level: &str) -> String {
        match level {
            "ERROR" | "error" => message.red().to_string(),
            "WARN" | "warn" => message.yellow().to_string(),
            "INFO" | "info" => message.green().to_string(),
            "DEBUG" | "debug" => message.blue().to_string(),
            "TRACE" | "trace" => message.magenta().to_string(),
            _ => message.green().to_string(),
        }
    }

    fn should_colorize(&self, is_stderr: bool) -> bool {
        if !self.config.colored {
            return false;
        }

        // NO_COLOR standard (https://no-color.org/)
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        // FORCE_COLOR standard
        if let Ok(val) = std::env::var("CLICOLOR_FORCE") {
            if val != "0" {
                return true;
            }
        }

        // TERM=dumb
        if let Ok(term) = std::env::var("TERM") {
            if term == "dumb" {
                return false;
            }
        }

        if is_stderr {
            io::stderr().is_terminal()
        } else {
            io::stdout().is_terminal()
        }
    }
}

impl LogSink for ConsoleSink {
    fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        // 应用数据脱敏（如果启用）
        let masked_record = if self.config.masking_enabled {
            let mut masked = record.clone();
            masked.message = self.masker.mask(&record.message);
            self.masker.mask_hashmap(&mut masked.fields);
            masked
        } else {
            record.clone()
        };

        // Stderr separation
        let is_stderr = self
            .config
            .stderr_levels
            .contains(&masked_record.level.to_lowercase());

        let use_color = self.should_colorize(is_stderr);

        if is_stderr {
            let mut stderr = io::stderr();
            self.write_record(&mut stderr, &masked_record, use_color)
                .map_err(InklogError::IoError)?;
        } else {
            let mut writer = self
                .writer
                .lock()
                .map_err(|_| InklogError::IoError(io::Error::other("Lock poisoned")))?;
            self.write_record(&mut *writer, &masked_record, use_color)
                .map_err(InklogError::IoError)?;
        }

        Ok(())
    }

    fn flush(&self) -> Result<(), InklogError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| InklogError::IoError(io::Error::other("Lock poisoned")))?;
        writer.flush().map_err(InklogError::IoError)
    }

    fn is_healthy(&self) -> bool {
        true
    }

    fn shutdown(&self) -> Result<(), InklogError> {
        self.flush()
    }
}

impl Clone for ConsoleSink {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            // Clone shares the same writer (Arc ensures reference counting)
            writer: Arc::clone(&self.writer),
            template: self.template.clone(),
            masker: DataMasker::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConsoleSinkConfig;
    use serial_test::serial;
    use std::env;

    fn get_sink() -> ConsoleSink {
        ConsoleSink::new(
            ConsoleSinkConfig {
                enabled: true,
                colored: true,
                ..Default::default()
            },
            LogTemplate::default(),
        )
    }

    #[test]
    #[serial]
    fn test_no_color_env() {
        let sink = get_sink();
        env::set_var("NO_COLOR", "1");
        assert!(!sink.should_colorize(false));
        env::remove_var("NO_COLOR");
    }

    #[test]
    #[serial]
    fn test_force_color_env() {
        let sink = get_sink();
        // Remove NO_COLOR to ensure deterministic test result
        env::remove_var("NO_COLOR");
        env::set_var("CLICOLOR_FORCE", "1");
        assert!(sink.should_colorize(false));
        env::remove_var("CLICOLOR_FORCE");
    }

    #[test]
    #[serial]
    fn test_term_dumb() {
        let sink = get_sink();
        // Remove NO_COLOR to ensure deterministic test result
        env::remove_var("NO_COLOR");
        env::set_var("TERM", "dumb");
        // Ensure no other conflicting envs
        env::remove_var("CLICOLOR_FORCE");
        assert!(!sink.should_colorize(false));
        env::remove_var("TERM");
    }

    #[test]
    #[serial]
    fn test_config_disabled() {
        let mut sink = get_sink();
        // Remove NO_COLOR to ensure deterministic test result
        env::remove_var("NO_COLOR");
        sink.config.colored = false;
        env::set_var("CLICOLOR_FORCE", "1"); // Config should override force?
                                             // My logic: if !config.colored return false.
        assert!(!sink.should_colorize(false));
        env::remove_var("CLICOLOR_FORCE");
    }

    #[test]
    fn test_console_sink_new() {
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        };
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);
        assert!(sink.config.enabled);
    }

    #[test]
    fn test_console_sink_disabled() {
        let config = ConsoleSinkConfig {
            enabled: false,
            colored: true,
            ..Default::default()
        };
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);
        assert!(!sink.config.enabled);
    }

    #[test]
    #[serial]
    fn test_should_colorize_defaults() {
        env::remove_var("CLICOLOR_FORCE");
        env::remove_var("TERM");
        env::set_var("NO_COLOR", "1");
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: true,
            ..Default::default()
        };
        let template = LogTemplate::default();
        let sink = ConsoleSink::new(config, template);
        let result = sink.should_colorize(false);
        assert!(
            !result,
            "should_colorize should return false when NO_COLOR is set"
        );
        env::remove_var("NO_COLOR");
    }

    #[test]
    fn test_should_colorize_when_allowed() {
        let sink = get_sink();
        let colored = sink.apply_color("test message", "ERROR");
        assert!(colored.contains("test message"));
    }

    #[test]
    fn test_apply_color_info() {
        let sink = get_sink();
        let colored = sink.apply_color("test message", "INFO");
        assert!(colored.contains("test message"));
    }

    #[test]
    fn test_apply_color_unknown() {
        let sink = get_sink();
        let colored = sink.apply_color("test message", "UNKNOWN");
        assert!(colored.contains("test message"));
    }

    // ========================================================================
    // Test helpers
    // ========================================================================

    /// A Write implementation that buffers output for inspection in tests.
    #[derive(Default, Clone)]
    struct TestWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.buf.lock().unwrap().flush()
        }
    }

    impl TestWriter {
        fn output(&self) -> String {
            let buf = self.buf.lock().unwrap();
            String::from_utf8_lossy(&buf).to_string()
        }

        fn is_empty(&self) -> bool {
            self.buf.lock().unwrap().is_empty()
        }
    }

    /// A Write implementation that always fails, for error-path testing.
    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }

    /// Creates a LogRecord with the given level and message, with sane defaults.
    fn make_record(level: &str, message: &str) -> LogRecord {
        LogRecord {
            level: level.to_string(),
            message: message.to_string(),
            target: "test::module".to_string(),
            ..Default::default()
        }
    }

    /// Creates a ConsoleSink wired to a TestWriter for output inspection.
    /// Returns (sink, writer) so tests can assert on the captured output.
    fn sink_with_test_writer(config: ConsoleSinkConfig) -> (ConsoleSink, TestWriter) {
        let writer = TestWriter::default();
        let mut sink = ConsoleSink::new(config, LogTemplate::default());
        sink.writer = Arc::new(Mutex::new(Box::new(writer.clone())));
        (sink, writer)
    }

    // ========================================================================
    // apply_color: cover remaining branches (WARN, DEBUG, TRACE, lowercase)
    // ========================================================================

    #[test]
    fn test_apply_color_warn() {
        let sink = get_sink();
        let colored = sink.apply_color("warn message", "WARN");
        assert!(colored.contains("warn message"));
    }

    #[test]
    fn test_apply_color_debug() {
        let sink = get_sink();
        let colored = sink.apply_color("debug message", "DEBUG");
        assert!(colored.contains("debug message"));
    }

    #[test]
    fn test_apply_color_trace() {
        let sink = get_sink();
        let colored = sink.apply_color("trace message", "TRACE");
        assert!(colored.contains("trace message"));
    }

    #[test]
    fn test_apply_color_lowercase_levels() {
        let sink = get_sink();
        // Covers lowercase branches ("error", "warn", "info", "debug", "trace")
        // in apply_color's match arms.
        for level in &["error", "warn", "info", "debug", "trace"] {
            let colored = sink.apply_color("payload", level);
            assert!(colored.contains("payload"), "level {} lost message", level);
        }
    }

    #[test]
    #[serial]
    fn test_apply_color_emits_ansi_codes() {
        // Force color emission regardless of terminal detection so we can
        // verify the actual color mapping is correct.
        owo_colors::set_override(true);
        let sink = get_sink();

        let red = sink.apply_color("msg", "ERROR");
        let yellow = sink.apply_color("msg", "WARN");
        let green = sink.apply_color("msg", "INFO");
        let blue = sink.apply_color("msg", "DEBUG");
        let magenta = sink.apply_color("msg", "TRACE");

        // Unset before assertions so global state is clean even if an
        // assertion fails.
        owo_colors::unset_override();

        assert!(
            red.contains("\x1b[31m"),
            "ERROR must be red, got: {:?}",
            red
        );
        assert!(
            yellow.contains("\x1b[33m"),
            "WARN must be yellow, got: {:?}",
            yellow
        );
        assert!(
            green.contains("\x1b[32m"),
            "INFO must be green, got: {:?}",
            green
        );
        assert!(
            blue.contains("\x1b[34m"),
            "DEBUG must be blue, got: {:?}",
            blue
        );
        assert!(
            magenta.contains("\x1b[35m"),
            "TRACE must be magenta, got: {:?}",
            magenta
        );
    }

    // ========================================================================
    // write_record: cover use_color true/false, all level branches, errors
    // ========================================================================

    #[test]
    fn test_write_record_without_color() {
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("INFO", "hello world");
        sink.write_record(&mut buf, &record, false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // Formatted by default template: {timestamp} [{level}] {target} - {message}
        assert!(output.contains("[INFO]"));
        assert!(output.contains("test::module"));
        assert!(output.contains("hello world"));
        assert!(output.ends_with('\n'), "writeln should append newline");
        // No ANSI escape codes when color is off.
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn test_write_record_with_color_error() {
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("ERROR", "boom");
        sink.write_record(&mut buf, &record, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("boom"));
        assert!(output.contains("[ERROR]"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_write_record_with_color_warn() {
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("WARN", "careful");
        sink.write_record(&mut buf, &record, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("careful"));
        assert!(output.contains("[WARN]"));
    }

    #[test]
    fn test_write_record_with_color_debug() {
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("DEBUG", "details");
        sink.write_record(&mut buf, &record, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("details"));
        assert!(output.contains("[DEBUG]"));
    }

    #[test]
    fn test_write_record_with_color_trace() {
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("TRACE", "verbose");
        sink.write_record(&mut buf, &record, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("verbose"));
        assert!(output.contains("[TRACE]"));
    }

    #[test]
    fn test_write_record_with_color_unknown_level() {
        // Covers the `_ => record.level.clone()` fallback arm in write_record's
        // level_colored match.
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("FATAL", "critical");
        sink.write_record(&mut buf, &record, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("critical"));
        assert!(output.contains("[FATAL]"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_write_record_lowercase_level_with_color() {
        // Lowercase level strings should also match color branches.
        let sink = get_sink();
        let mut buf: Vec<u8> = Vec::new();
        let record = make_record("error", "lowercase boom");
        sink.write_record(&mut buf, &record, true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("lowercase boom"));
        assert!(output.contains("[error]"));
    }

    #[test]
    fn test_write_record_propagates_write_error() {
        let sink = get_sink();
        let mut writer = FailingWriter;
        let record = make_record("INFO", "will fail");
        let result = sink.write_record(&mut writer, &record, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("write failed"),
            "error should carry the underlying message, got: {}",
            err
        );
    }

    // ========================================================================
    // should_colorize: cover CLICOLOR_FORCE=0 and is_stderr=true branches
    // ========================================================================

    #[test]
    #[serial]
    fn test_should_colorize_clicolor_force_zero_falls_through() {
        // CLICOLOR_FORCE=0 means "do not force", so it should fall through to
        // the TERM check. Setting TERM=dumb makes the result deterministically
        // false, exercising the val != "0" false branch.
        env::remove_var("NO_COLOR");
        env::set_var("CLICOLOR_FORCE", "0");
        env::set_var("TERM", "dumb");
        let sink = get_sink();
        assert!(!sink.should_colorize(false));
        env::remove_var("CLICOLOR_FORCE");
        env::remove_var("TERM");
    }

    #[test]
    #[serial]
    fn test_should_colorize_stderr_path_with_force() {
        // CLICOLOR_FORCE=1 forces true, exercising the is_stderr=true branch
        // of the final terminal check (short-circuited by the force).
        env::remove_var("NO_COLOR");
        env::set_var("CLICOLOR_FORCE", "1");
        let sink = get_sink();
        assert!(sink.should_colorize(true));
        env::remove_var("CLICOLOR_FORCE");
    }

    #[test]
    #[serial]
    fn test_should_colorize_term_not_dumb_falls_through() {
        // TERM set to a non-dumb value exercises the `term == "dumb"` false
        // branch. Combined with CLICOLOR_FORCE=1 to make the result
        // deterministically true.
        env::remove_var("NO_COLOR");
        env::set_var("TERM", "xterm-256color");
        env::set_var("CLICOLOR_FORCE", "1");
        let sink = get_sink();
        assert!(sink.should_colorize(false));
        env::remove_var("TERM");
        env::remove_var("CLICOLOR_FORCE");
    }

    // ========================================================================
    // LogSink::write: masking, stderr/stdout routing, color interaction
    // ========================================================================

    #[test]
    fn test_log_sink_write_stdout_no_masking() {
        // masking_enabled=false covers the `else` branch of the masking if/else.
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            masking_enabled: false,
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let record = make_record("INFO", "plain message");
        sink.write(&record).unwrap();
        let output = writer.output();
        assert!(output.contains("plain message"));
        assert!(output.contains("[INFO]"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_log_sink_write_with_masking_redacts_sensitive_data() {
        // masking_enabled=true covers the `if` branch: mask message + fields.
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            masking_enabled: true,
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let record = make_record("INFO", "email=test@example.com");
        sink.write(&record).unwrap();
        let output = writer.output();
        // Original sensitive data must not appear.
        assert!(
            !output.contains("test@example.com"),
            "masked output must not contain the original email, got: {}",
            output
        );
        // Masked email retains the @ separator (partial masking pattern).
        assert!(output.contains('@'), "masked email should retain @");
    }

    #[test]
    fn test_log_sink_write_stderr_level_writes_to_stderr_not_stdout() {
        // is_stderr=true in write() routes to io::stderr(), so the stdout
        // TestWriter should remain empty.
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            stderr_levels: vec!["error".to_string()],
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let record = make_record("ERROR", "stderr-only message");
        sink.write(&record).unwrap();
        assert!(
            writer.is_empty(),
            "stdout writer must be empty when level routes to stderr"
        );
    }

    #[test]
    fn test_log_sink_write_warn_routes_to_stderr_by_default() {
        // Default stderr_levels is ["error", "warn"]; WARN should go to stderr.
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let record = make_record("WARN", "warning via stderr");
        sink.write(&record).unwrap();
        assert!(
            writer.is_empty(),
            "WARN should route to stderr by default, not stdout"
        );
    }

    #[test]
    fn test_log_sink_write_info_routes_to_stdout_by_default() {
        // INFO is not in default stderr_levels, so it goes to stdout writer.
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let record = make_record("INFO", "info via stdout");
        sink.write(&record).unwrap();
        let output = writer.output();
        assert!(output.contains("info via stdout"));
    }

    #[test]
    fn test_log_sink_write_case_insensitive_stderr_match() {
        // stderr_levels contains lowercase "error"; record level "ERROR"
        // should still match after to_lowercase().
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            stderr_levels: vec!["error".to_string()],
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let record = make_record("ERROR", "uppercase level");
        sink.write(&record).unwrap();
        assert!(
            writer.is_empty(),
            "uppercase ERROR must match lowercase stderr_levels"
        );
    }

    // ========================================================================
    // LogSink trait: flush, is_healthy, shutdown
    // ========================================================================

    #[test]
    fn test_log_sink_flush_succeeds() {
        let (sink, _writer) = sink_with_test_writer(ConsoleSinkConfig::default());
        assert!(sink.flush().is_ok());
    }

    #[test]
    fn test_log_sink_is_healthy_always_true() {
        let sink = get_sink();
        // ConsoleSink is always healthy (no persistent state to fail).
        assert!(sink.is_healthy());
    }

    #[test]
    fn test_log_sink_shutdown_flushes_without_error() {
        let (sink, _writer) = sink_with_test_writer(ConsoleSinkConfig::default());
        // shutdown delegates to flush, so it should succeed.
        assert!(sink.shutdown().is_ok());
    }

    // ========================================================================
    // Clone impl: config preserved, writer shared via Arc
    // ========================================================================

    #[test]
    fn test_console_sink_clone_preserves_config() {
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: true,
            stderr_levels: vec!["error".to_string(), "warn".to_string()],
            masking_enabled: true,
        };
        let sink = ConsoleSink::new(config, LogTemplate::default());
        let cloned = sink.clone();
        assert!(cloned.config.enabled);
        assert!(cloned.config.colored);
        assert!(cloned.config.masking_enabled);
        assert_eq!(cloned.config.stderr_levels, vec!["error", "warn"]);
        // Both original and clone should remain healthy.
        assert!(sink.is_healthy());
        assert!(cloned.is_healthy());
    }

    #[test]
    fn test_console_sink_clone_shares_writer_buffer() {
        // Arc Clone shares the same underlying writer, so writes via the clone
        // should be visible through the original's writer reference.
        let config = ConsoleSinkConfig {
            enabled: true,
            colored: false,
            ..Default::default()
        };
        let (sink, writer) = sink_with_test_writer(config);
        let cloned = sink.clone();
        let record = make_record("INFO", "written via clone");
        cloned.write(&record).unwrap();
        let output = writer.output();
        assert!(
            output.contains("written via clone"),
            "clone shares writer via Arc, output should be visible"
        );
    }

    // ========================================================================
    // Debug impl: only config and template fields
    // ========================================================================

    #[test]
    fn test_console_sink_debug_format() {
        let sink = get_sink();
        let debug_str = format!("{:?}", sink);
        // Debug impl only exposes config and template fields.
        assert!(debug_str.contains("ConsoleSink"));
        assert!(debug_str.contains("config"));
        assert!(debug_str.contains("template"));
        // writer and masker are deliberately omitted by the Debug impl.
        assert!(!debug_str.contains("writer"));
        assert!(!debug_str.contains("masker"));
    }
}
