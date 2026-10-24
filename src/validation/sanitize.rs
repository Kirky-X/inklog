// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Log content sanitization for security.
//!
//! This module provides log content sanitization to prevent log injection attacks
//! and ensure safe log output for SIEM systems.

use regex::Regex;
use std::sync::LazyLock;

/// Pre-compiled regex for ANSI SGR (Select Graphic Rendition) escape sequences.
static ANSI_SGR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b\[[0-9;]*m")
        .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-ansi_sgr_invariant")))
});

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

/// Default sensitive patterns compiled once and shared across all LogSanitizer instances.
/// Using LazyLock avoids recompiling these stable regex patterns on every constructor call.
static DEFAULT_SENSITIVE_PATTERNS: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"\b\d{13,16}\b").unwrap_or_else(|_| {
                panic!("{}", crate::i18n::tr("sanitize-card_number_invariant"))
            }),
            "[CARD_NUM]".to_string(),
        ),
        (
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
                .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-email_invariant"))),
            "[EMAIL]".to_string(),
        ),
        (
            Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")
                .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-ssn_invariant"))),
            "[SSN]".to_string(),
        ),
        // 标记保护在「值」一级执行（见 sanitize() 的标记契约说明）：值已是
        // 脱敏/掩码标记时不重复脱敏——上游标记不被改写成另一种标记。保护
        // 判定不依赖环视（regex crate 不支持），由替换闭包按值判定。
        (
            Regex::new(r"(?i)password\s*[=:]\s*\S+")
                .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-password_invariant"))),
            "password=[REDACTED]".to_string(),
        ),
        (
            Regex::new(r"(?i)token\s*[=:]\s*\S+")
                .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-token_invariant"))),
            "token=[REDACTED]".to_string(),
        ),
        (
            Regex::new(r"(?i)api[_-]?key\s*[=:]\s*\S+")
                .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-api_key_invariant"))),
            "api_key=[REDACTED]".to_string(),
        ),
        (
            Regex::new(r"Bearer\s+[A-Za-z0-9\-\.]+").unwrap_or_else(|_| {
                panic!("{}", crate::i18n::tr("sanitize-bearer_token_invariant"))
            }),
            "Bearer [TOKEN]".to_string(),
        ),
        (
            Regex::new(r"Basic\s+[A-Za-z0-9+/=]+")
                .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("sanitize-basic_auth_invariant"))),
            "Basic [AUTH]".to_string(),
        ),
    ]
});

impl LogSanitizer {
    /// Create a new LogSanitizer with default configuration.
    pub fn new() -> Self {
        Self {
            config: SanitizerConfig::default(),
            sensitive_regexes: Self::default_sensitive_patterns(),
        }
    }

    /// Create with custom configuration.
    /// User-provided `sensitive_patterns` are merged with default patterns.
    pub fn with_config(mut config: SanitizerConfig) -> Self {
        let mut sensitive_regexes = Self::default_sensitive_patterns();
        sensitive_regexes.append(&mut config.sensitive_patterns);
        Self {
            config,
            sensitive_regexes,
        }
    }

    /// Clone from the shared LazyLock cache to avoid recompiling regexes.
    /// Each caller gets its own owned copy so mutations (add_pattern) don't affect others.
    fn default_sensitive_patterns() -> Vec<(Regex, String)> {
        DEFAULT_SENSITIVE_PATTERNS.clone()
    }

    /// Sanitize a log message.
    ///
    /// # Processing Order Contract
    ///
    /// 1. ANSI escape sequences are stripped first.
    /// 2. Custom string replacements run next.
    /// 3. Sensitive-pattern redaction runs **last** (before escape-mode
    ///    encoding). This ordering is deliberate: a custom replacement could
    ///    otherwise re-introduce sensitive content into the output after
    ///    redaction had already run, so redaction is positioned as the final
    ///    content transform.
    ///
    /// # Marker Idempotency Contract
    ///
    /// Redaction markers (`***REDACTED`、`***MASKED`、`[REDACTED]`) never
    /// disable redaction for the whole message: the marker vocabulary is
    /// attacker-controllable free text, so a message-level short-circuit
    /// would let anyone exempt an entire record by injecting a marker.
    /// Marker protection therefore lives at the *value* level — a key=value
    /// rule whose value already is a marker is left untouched, while every
    /// other part of the message is still redacted. Injection protection
    /// (ANSI stripping, escape encoding) is unaffected.
    pub fn sanitize(&self, message: &str) -> String {
        // Strip ANSI escape sequences before any other processing
        let mut result = self.strip_ansi(message).into_owned();

        for (from, to) in &self.config.custom_replacements {
            result = result.replace(from, to);
        }

        for (pattern, replacement) in &self.sensitive_regexes {
            result = pattern
                .replace_all(&result, |caps: &regex::Captures<'_>| {
                    let whole = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
                    // 值已是脱敏/掩码标记：原样保留（幂等，不改写上游标记）
                    if value_is_redaction_marker(matched_value(whole)) {
                        whole.to_string()
                    } else {
                        // 替换串中的捕获组引用（`$1` / `${name}`）必须展开——
                        // 与 regex 的 `&str` 替换器同语义；直接 clone 会把
                        // `$1` 原样输出（用户自定义模式因此失效）。
                        let mut expanded = String::new();
                        caps.expand(replacement, &mut expanded);
                        expanded
                    }
                })
                .to_string();
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
            // Find a safe UTF-8 char boundary to avoid splitting multi-byte characters
            let mut end = self.config.max_length;
            while end > 0 && !result.is_char_boundary(end) {
                end -= 1;
            }
            result.truncate(end);
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

    /// Strict escape mode: converts control characters and quotes to `\uXXXX` form.
    ///
    /// This method is idempotent: calling it twice on the same input produces
    /// the same output, because already-escaped `\uXXXX` sequences are detected
    /// and not re-escaped.
    fn escape_strict(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            // Check if this backslash starts an existing \uXXXX escape sequence
            if c == '\\' {
                if chars.peek() == Some(&'u') {
                    // Pass through existing \uXXXX sequences unchanged
                    result.push(c);
                } else {
                    // Escape standalone backslashes
                    result.push_str(&format!("\\u{:04x}", c as u32));
                }
            } else if c.is_control() || c == '"' {
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

    /// Strip ANSI SGR escape sequences from the input string.
    ///
    /// Fast path: if the input contains no `\x1b` (ESC) character,
    /// returns the original string without any allocation.
    pub fn strip_ansi<'a>(&self, input: &'a str) -> std::borrow::Cow<'a, str> {
        if !input.contains('\x1b') {
            return std::borrow::Cow::Borrowed(input);
        }
        std::borrow::Cow::Owned(ANSI_SGR_REGEX.replace_all(input, "").into_owned())
    }
}

impl Default for LogSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

/// 已脱敏/已掩码标记（脱敏幂等契约）——**精确匹配**词表。
///
/// 三处脱敏入口共享同一分工与契约：
/// - `InklogError::safe_message`（src/error.rs）：错误消息出口脱敏
/// - [`LogSanitizer`]（本模块）：日志注入防护（CWE-117）与转义
/// - `DataMasker::mask`（src/support/processing/masking.rs）：PII 掩码
///
/// 双开关叠加时同一条消息会先后经过多个入口。标记保护下沉到「值」一级：
/// 值已是标记时不重复脱敏，避免产生 REDACTED 套 REDACTED 的嵌套标记。
/// 判定不落在「整条消息」一级——标记词表可被自由文本伪造，整条消息级
/// 短路会让注入一个标记的记录整体免于脱敏。
///
/// 词表只收**本 crate 实际产出的标记形态**，判定为整值精确匹配：形态
/// 之外的拼接（如 `***REDACTEDSECRET`——标记前缀后直接跟真实值）不再
/// 构成免脱敏旁路。词表须与产出点同步登记：
/// `src/support/processing/secret_patterns.rs` 的内建替换串由
/// `test_builtin_secret_markers_are_registered` 兜底防漂移。
pub(crate) const REDACTION_MARKERS: &[&str] = &[
    "***MASKED***",
    "***REDACTED***",
    "***REDACTED_API_KEY***",
    "***REDACTED_AWS_KEY***",
    "***REDACTED_CORP***",
    "***REDACTED_ENGINE_ERROR***",
    "***REDACTED_GITHUB***",
    "***REDACTED_GOOGLE***",
    "***REDACTED_JWT***",
    "***REDACTED_OVERSIZED***",
    "***REDACTED_PEM_KEY***",
    "***REDACTED_SLACK***",
    "***REDACTED_STRIPE***",
    "***HIGH_ENTROPY***",
    "[REDACTED]",
];

/// 取匹配文本中的「值」部分：首个 `=`/`:` 之后；无分隔符时为整体。
///
/// `key=value` 形态的敏感正则据此判定「值是否已是脱敏标记」；无分隔符
/// 的形态（邮箱、卡号、SSN、Bearer 等）值即整体。
pub(crate) fn matched_value(whole: &str) -> &str {
    match whole.find(['=', ':']) {
        Some(pos) => &whole[pos + 1..],
        None => whole,
    }
}

/// 值部分是否已是脱敏/掩码标记（幂等判定）。
///
/// 判定为**整值精确匹配**词表（见 [`REDACTION_MARKERS`）：标记形态之外的
/// 值一律照常脱敏——「以标记字样开头再拼真实值」（如
/// `***REDACTEDSECRET`）不构成免脱敏旁路。
pub(crate) fn value_is_redaction_marker(value: &str) -> bool {
    let trimmed = value.trim();
    REDACTION_MARKERS.contains(&trimmed)
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

    #[test]
    fn test_default_log_sanitizer() {
        let sanitizer = LogSanitizer::default();
        let result = sanitizer.sanitize("test\nmessage");
        assert!(result.contains("\\n"));
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_strict_escape_mode() {
        let config = SanitizerConfig {
            mode: EscapeMode::Strict,
            custom_replacements: Vec::new(),
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("Hello\nWorld");
        assert!(result.contains("\\u000a"));
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_escape_strict_with_backslash_and_quote() {
        let config = SanitizerConfig {
            mode: EscapeMode::Strict,
            custom_replacements: Vec::new(),
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("path\\to\"file");
        assert!(result.contains("\\u005c"));
        assert!(result.contains("\\u0022"));
        // The raw backslash and quote should be replaced by escape sequences
        // Note: escape sequences themselves contain backslashes, so we verify
        // the escape sequences are present rather than checking for absence of '\'
        assert!(!result.contains("\""));
    }

    #[test]
    fn test_escape_strict_preserves_printable() {
        let config = SanitizerConfig {
            mode: EscapeMode::Strict,
            custom_replacements: Vec::new(),
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("Hello World 123");
        assert_eq!(result, "Hello World 123");
    }

    #[test]
    fn test_escape_minimal_with_newline_tab_carriage_return() {
        let config = SanitizerConfig {
            mode: EscapeMode::Minimal,
            custom_replacements: Vec::new(),
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("line1\nline2\r\ttabbed");
        assert_eq!(result, "line1\\nline2\\r\\ttabbed");
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
        assert!(!result.contains('\t'));
    }

    #[test]
    fn test_escape_json_all_special_chars() {
        let config = SanitizerConfig {
            mode: EscapeMode::JsonSafe,
            custom_replacements: Vec::new(),
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let input = "quote\"backslash\\newline\ncarriage\rtab\t";
        let result = sanitizer.sanitize(input);
        assert!(result.contains("\\\""));
        assert!(result.contains("\\\\"));
        assert!(result.contains("\\n"));
        assert!(result.contains("\\r"));
        assert!(result.contains("\\t"));
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
        assert!(!result.contains('\t'));
    }

    #[test]
    fn test_escape_json_control_character() {
        let config = SanitizerConfig {
            mode: EscapeMode::JsonSafe,
            custom_replacements: Vec::new(),
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let result = sanitizer.sanitize("null\x00byte");
        assert!(result.contains("\\u0000"));
    }

    #[test]
    fn test_add_pattern() {
        let mut sanitizer = LogSanitizer::new();
        let pattern = Regex::new(r"SECRET-\d+").expect("valid regex");
        sanitizer.add_pattern(pattern, "[SECRET]".to_string());

        let result = sanitizer.sanitize("found SECRET-12345 here");
        assert!(result.contains("[SECRET]"));
        assert!(!result.contains("SECRET-12345"));
    }

    #[test]
    fn test_add_replacement() {
        let mut sanitizer = LogSanitizer::new();
        sanitizer.add_replacement("foo".to_string(), "bar".to_string());

        let result = sanitizer.sanitize("hello foo world");
        assert!(result.contains("bar"));
        assert!(!result.contains("foo"));
    }

    #[test]
    fn test_add_pattern_and_replacement_combined() {
        let mut sanitizer = LogSanitizer::new();
        let pattern = Regex::new(r"\bPHONE-\d+\b").expect("valid regex");
        sanitizer.add_pattern(pattern, "[PHONE]".to_string());
        sanitizer.add_replacement("internal".to_string(), "external".to_string());

        let result = sanitizer.sanitize("call PHONE-555 internal line");
        assert!(result.contains("[PHONE]"));
        assert!(result.contains("external"));
        assert!(!result.contains("PHONE-555"));
        assert!(!result.contains("internal"));
    }

    #[test]
    fn test_custom_replacement_reintroduced_content_is_redacted() {
        // 顺序契约：自定义替换先执行、敏感正则脱敏最后执行（安全后置）。
        // 若正则先执行，自定义替换重新拼出的敏感内容将绕过脱敏直接泄漏。
        let mut sanitizer = LogSanitizer::new();
        sanitizer.add_replacement("MYSECRET".to_string(), "password".to_string());

        // 原文不命中任何敏感正则；替换后 "password=..." 必须被再掩码
        let result = sanitizer.sanitize("MYSECRET=opensesame123");
        assert!(
            result.contains("[REDACTED]"),
            "reassembled sensitive content must be redacted: {}",
            result
        );
        assert!(!result.contains("opensesame123"));
    }

    #[test]
    fn test_email_redaction_does_not_swallow_pipe_suffix() {
        // 修复前 TLD 字符类 [A-Z|a-z] 中的 "|" 是字面量，会把邮箱后的
        // "|" 及后续字母吞进 [EMAIL]；修复后管道符保持原样
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.sanitize("mail user@example.com|tail");
        assert_eq!(result, "mail [EMAIL]|tail");
    }

    #[test]
    fn test_sanitize_truncate_respects_utf8_boundaries() {
        let mut config = super::SanitizerConfig::default();
        config.max_length = 7;
        let sanitizer = super::LogSanitizer::with_config(config);
        let result = sanitizer.sanitize("你好世界");
        assert!(result.starts_with("你好"));
        assert!(result.contains("...[truncated]"));
    }

    #[test]
    fn test_strip_ansi_sgr_sequence() {
        let sanitizer = LogSanitizer::new();
        assert_eq!(sanitizer.strip_ansi("\x1b[31mERROR\x1b[0m"), "ERROR");
    }

    #[test]
    fn test_strip_ansi_multiple_sequences() {
        let sanitizer = LogSanitizer::new();
        assert_eq!(sanitizer.strip_ansi("\x1b[1;32mOK\x1b[0m"), "OK");
    }

    #[test]
    fn test_strip_ansi_fast_path_no_esc() {
        let sanitizer = LogSanitizer::new();
        let input = "no ansi here";
        let result = sanitizer.strip_ansi(input);
        assert_eq!(result, "no ansi here");
        assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn test_strip_ansi_nested_sequences() {
        let sanitizer = LogSanitizer::new();
        assert_eq!(
            sanitizer.strip_ansi("\x1b[1m\x1b[31mBOLD RED\x1b[0m\x1b[0m"),
            "BOLD RED"
        );
    }

    #[test]
    fn test_strip_ansi_empty_input() {
        let sanitizer = LogSanitizer::new();
        assert_eq!(sanitizer.strip_ansi(""), "");
    }

    #[test]
    fn test_strip_ansi_only_esc_char() {
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.strip_ansi("\x1b");
        assert_eq!(result, "\x1b");
    }

    #[test]
    fn test_sanitize_strips_ansi_before_processing() {
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.sanitize("\x1b[31mERROR\x1b[0m");
        assert_eq!(result, "ERROR");
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn test_sanitize_marker_injection_does_not_disable_rules() {
        // 回归：整条消息级标记短路可被自由文本伪造——注入一个标记即可让
        // 整条记录免于敏感正则。标记保护只在「值」一级。
        let sanitizer = LogSanitizer::new();
        for marker in ["[REDACTED]", "***REDACTED***", "***MASKED***"] {
            let text = format!("login user={marker} token=supersecretvalue123");
            let out = sanitizer.sanitize(&text);
            assert!(
                !out.contains("supersecretvalue123"),
                "injected marker {marker} must not exempt the record: {out}"
            );
        }
    }

    /// 用户自定义模式的替换串里的捕获组引用必须展开（`$1` / `${name}`）。
    #[test]
    fn test_sanitize_expands_capture_group_in_custom_replacement() {
        let mut sanitizer = LogSanitizer::new();
        sanitizer.add_pattern(
            Regex::new(r"secret-(\d+)").expect("valid regex"),
            "secret-[REDACTED:$1]".to_string(),
        );

        let result = sanitizer.sanitize("token secret-42 end");
        assert!(
            result.contains("secret-[REDACTED:42]"),
            "capture group must be expanded: {result}"
        );
        assert!(
            !result.contains("$1"),
            "raw $1 must not be emitted: {result}"
        );
    }

    /// 标记判定不得过宽：「标记前缀 + 真实值」拼接不是标记形态。
    #[test]
    fn test_sanitize_marker_prefix_glued_to_real_value_is_redacted() {
        let sanitizer = LogSanitizer::new();
        let result = sanitizer.sanitize("password=***REDACTEDSECRET");
        assert!(
            !result.contains("REDACTEDSECRET"),
            "marker prefix glued to a real value must still be redacted: {result}"
        );
        assert!(result.contains("[REDACTED]"), "unexpected output: {result}");
    }

    /// 标记词表与 secret-scan 内建模式的替换串必须同步（防漂移）。
    #[cfg(feature = "secret-scan")]
    #[test]
    fn test_builtin_secret_markers_are_registered() {
        use crate::support::processing::secret_patterns::SecretPatternRegistry;

        for registry in [
            SecretPatternRegistry::with_builtins(),
            SecretPatternRegistry::with_extended(),
        ] {
            for pattern in registry.patterns() {
                assert!(
                    REDACTION_MARKERS.contains(&pattern.replacement()),
                    "secret pattern '{}' emits unregistered marker '{}'",
                    pattern.name(),
                    pattern.replacement()
                );
            }
        }
    }

    #[test]
    fn test_sanitize_marker_idempotency() {
        let sanitizer = LogSanitizer::new();

        // 已含 ***MASKED*** 标记的输入：跳过敏感正则再次脱敏，标记原样保留
        // （修复前 token=***MASKED*** 会被 token= 规则改写为 token=[REDACTED]）
        let marked = "token=***MASKED***";
        assert_eq!(
            sanitizer.sanitize(marked),
            marked,
            "masking marker must pass through unchanged"
        );

        // 已脱敏消息再次进入本入口：幂等，不产生嵌套标记
        let once = sanitizer.sanitize("user password=supersecret123 login");
        assert!(once.contains("[REDACTED]"));
        assert_eq!(
            sanitizer.sanitize(&once),
            once,
            "re-sanitizing already redacted text must be a no-op"
        );

        // 短路只跳过脱敏：注入防护（换行转义）对标记输入仍然执行
        let mixed = sanitizer.sanitize("password=[REDACTED]\nnext line");
        assert!(
            mixed.contains("[REDACTED]"),
            "marker must be preserved, got: {mixed}"
        );
        assert!(
            !mixed.contains('\n'),
            "injection protection must still run for marked input"
        );
    }
}
