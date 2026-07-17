// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! ConsoleSink 配置构造与测试用例写入工具。
//!
//! 从 `examples/src/bin/console.rs` 提取共享逻辑：
//!
//! - [`create_console_config`]：构造 [`ConsoleSinkConfig`]，控制颜色与 stderr 分流。
//! - [`write_test_cases`]：把 `(level, message)` 列表写入 sink 并 flush。

use anyhow::Result;
use inklog::chrono::Utc;
use inklog::domain::config::ConsoleSinkConfig;
use inklog::support::io::sink::LogSink;
use inklog::LogRecord;

/// 构造 ConsoleSink 配置。
///
/// - `colored`：是否启用 ANSI 颜色。
/// - `stderr_levels`：哪些级别输出到 stderr（小写形式，例如 `["error", "warn"]`）。
/// - `masking_enabled`：固定为 `false`，与示例保持一致。
pub fn create_console_config(colored: bool, stderr_levels: Vec<String>) -> ConsoleSinkConfig {
    ConsoleSinkConfig {
        enabled: true,
        colored,
        stderr_levels,
        masking_enabled: false,
    }
}

/// 把 `(level, message)` 测试用例依次写入 sink 并 flush。
///
/// 返回写入的用例数，便于调用方做完整性检查。`target` 统一使用
/// `"console_ops"`，便于在输出中识别来源。
pub async fn write_test_cases(sink: &dyn LogSink, cases: &[(&str, &str)]) -> Result<usize> {
    let mut written = 0;
    for (level, message) in cases {
        let record = LogRecord {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.to_string(),
            target: "console_ops".to_string(),
            fields: Default::default(),
            file: None,
            line: None,
            thread_id: "main".to_string(),
        };
        sink.write(&record).await?;
        written += 1;
    }
    sink.flush().await?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inklog::support::io::sink::console::ConsoleSink;
    use inklog::LogTemplate;

    #[test]
    fn test_create_console_config_basic() {
        // 验证：禁用颜色、空 stderr_levels 时的字段。
        let config = create_console_config(false, vec![]);
        assert!(config.enabled);
        assert!(!config.colored);
        assert!(config.stderr_levels.is_empty());
        assert!(!config.masking_enabled);
    }

    #[test]
    fn test_create_console_config_colored_with_stderr() {
        // 验证：启用颜色并配置 stderr 分流时字段正确。
        let levels = vec!["error".to_string(), "warn".to_string()];
        let config = create_console_config(true, levels.clone());
        assert!(config.colored);
        assert_eq!(config.stderr_levels, levels);
    }

    #[tokio::test]
    async fn test_write_test_cases() {
        // 验证：写入 5 个用例后返回 5，且不报错（ConsoleSink 写 stdout）。
        let config = create_console_config(false, vec![]);
        let sink = ConsoleSink::new(config, LogTemplate::default());
        let cases = [
            ("INFO", "这是 INFO 日志"),
            ("DEBUG", "这是 DEBUG 日志"),
            ("TRACE", "这是 TRACE 日志"),
            ("WARN", "这是 WARN 日志"),
            ("ERROR", "这是 ERROR 日志"),
        ];
        let written = write_test_cases(&sink, &cases).await.expect("写入失败");
        assert_eq!(written, 5);
    }

    #[tokio::test]
    async fn test_write_test_cases_empty() {
        // 验证：空用例列表返回 0。
        let config = create_console_config(false, vec![]);
        let sink = ConsoleSink::new(config, LogTemplate::default());
        let written = write_test_cases(&sink, &[])
            .await
            .expect("写入空列表应成功");
        assert_eq!(written, 0);
    }
}
