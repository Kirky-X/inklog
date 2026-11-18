// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP 工具实现（feature `mcp`）：inklog_query / inklog_audit_verify / inklog_stats。
//!
//! 安全策略（allowed_roots / allowed_levels / max_limit / redact）全部在工具
//! 内部强制执行——Agent 参数不可越权。见 design.md §4。

use crate::domain::config::mcp::McpConfig;
use crate::support::query::{LogEntry, QueryOptions, query_paths};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// MCP 工具调用结果（MCP content block 约定）。
#[derive(Debug, Clone)]
pub struct McpToolResult {
    /// 文本块列表（每块 type=text）
    pub content: Vec<Value>,
    /// 是否工具级错误（isError）
    pub is_error: bool,
}

impl McpToolResult {
    /// 成功文本结果。
    pub fn ok(text: String) -> Self {
        Self {
            content: vec![serde_json::json!({ "type": "text", "text": text })],
            is_error: false,
        }
    }

    /// 工具级错误结果。
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![serde_json::json!({ "type": "text", "text": text.into() })],
            is_error: true,
        }
    }
}

/// MCP 工具端口。
pub trait McpTool: Send + Sync {
    /// 工具名（tools/list 暴露）
    fn name(&self) -> &'static str;
    /// 输入 JSON Schema
    fn schema(&self) -> Value;
    /// 执行调用
    fn call(&self, args: Value) -> McpToolResult;
}

/// 对单个路径做 realpath 前缀校验：规范化后必须落在某个 allowed_root 内。
/// allowed_roots 为空 = 不限制（默认策略保持可用）。
fn path_allowed(path: &Path, cfg: &McpConfig) -> bool {
    if cfg.allowed_roots.is_empty() {
        return true;
    }
    let Ok(canonical) = path.canonicalize() else {
        return false;
    };
    cfg.allowed_roots.iter().any(|root| {
        root.canonicalize()
            .map(|base| canonical.starts_with(base))
            .unwrap_or(false)
    })
}

/// level 白名单校验（大小写不敏感）。
fn level_allowed(level: &str, cfg: &McpConfig) -> bool {
    match &cfg.allowed_levels {
        None => true,
        Some(allowed) => allowed.iter().any(|l| l.eq_ignore_ascii_case(level)),
    }
}

/// 把 LogEntry 转成 JSONL 文本块；redact=true 时对 message/target 过掩码。
fn entries_to_text(entries: &[LogEntry], redact: bool) -> String {
    let masker = if redact {
        Some(crate::DataMasker::new())
    } else {
        None
    };
    let mut out = String::new();
    for entry in entries {
        let mut value = serde_json::to_value(entry).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(masker) = &masker {
            if let Some(msg) = value.get("message").and_then(Value::as_str) {
                value["message"] = Value::String(masker.mask(msg));
            }
            if let Some(target) = value.get("target").and_then(Value::as_str) {
                value["target"] = Value::String(masker.mask(target));
            }
        }
        out.push_str(&value.to_string());
        out.push('\n');
    }
    out
}

/// `inklog_query` 工具：受策略约束的日志查询。
pub struct QueryTool {
    cfg: McpConfig,
}

impl QueryTool {
    pub fn new(cfg: McpConfig) -> Self {
        Self { cfg }
    }
}

impl McpTool for QueryTool {
    fn name(&self) -> &'static str {
        "inklog_query"
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "paths": { "type": "array", "items": { "type": "string" } },
                "since": { "type": "string" },
                "until": { "type": "string" },
                "level": { "type": "string" },
                "grep": { "type": "string" },
                "field": { "type": "string" },
                "limit": { "type": "integer", "minimum": 0 }
            },
            "required": ["paths"]
        })
    }

    fn call(&self, args: Value) -> McpToolResult {
        let Some(paths_val) = args.get("paths").and_then(Value::as_array) else {
            return McpToolResult::error("paths must be an array");
        };
        let paths: Vec<PathBuf> = paths_val
            .iter()
            .filter_map(|v| v.as_str().map(PathBuf::from))
            .collect();
        if paths.is_empty() {
            return McpToolResult::error("paths must not be empty");
        }
        for path in &paths {
            if !path_allowed(path, &self.cfg) {
                return McpToolResult::error("path outside allowed_roots");
            }
        }

        let level = args
            .get("level")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(level) = &level
            && !level_allowed(level, &self.cfg)
        {
            return McpToolResult::error("level not allowed by policy");
        }

        // field 键值经 sanitize 校验（去掉控制字符/ANSI 后作为 grep 补充语义）
        if let Some(field) = args.get("field").and_then(Value::as_str) {
            let sanitizer = crate::validation::sanitize::LogSanitizer::new();
            let cleaned = sanitizer.strip_ansi(field);
            if cleaned.contains(|c: char| c.is_control()) {
                return McpToolResult::error("field contains control characters");
            }
        }

        let raw_limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(self.cfg.max_limit as u64);
        let limit = usize::try_from(raw_limit)
            .unwrap_or(usize::MAX)
            .min(self.cfg.max_limit);

        let since = args
            .get("since")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));
        let until = args
            .get("until")
            .and_then(Value::as_str)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let opts = QueryOptions {
            since,
            until,
            level,
            keyword: args.get("grep").and_then(Value::as_str).map(str::to_string),
            limit,
        };
        match query_paths(&paths, &opts, None) {
            Ok(entries) => McpToolResult::ok(entries_to_text(&entries, self.cfg.redact)),
            Err(e) => McpToolResult::error(format!("query failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_log(dir: &Path) -> PathBuf {
        let path = dir.join("sample.log");
        std::fs::write(
            &path,
            "2026-09-11T01:00:00Z [INFO] app - user alice@example.com logged in\n\
             2026-09-11T01:00:01Z [ERROR] app - failure code 42\n",
        )
        .unwrap();
        path
    }

    fn cfg_with_roots(roots: Vec<PathBuf>) -> McpConfig {
        McpConfig {
            allowed_roots: roots,
            ..Default::default()
        }
    }

    #[test]
    fn test_query_tool_schema_and_name() {
        let tool = QueryTool::new(McpConfig::default());
        assert_eq!(tool.name(), "inklog_query");
        assert!(tool.schema().get("properties").is_some());
    }

    #[test]
    fn test_path_outside_allowed_roots_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let log = sample_log(dir.path());
        let outside = tempfile::tempdir().unwrap();
        let tool = QueryTool::new(cfg_with_roots(vec![outside.path().to_path_buf()]));
        let result = tool.call(serde_json::json!({"paths": [log.display().to_string()]}));
        assert!(
            result.is_error,
            "path outside allowed_roots must be rejected"
        );
    }

    #[test]
    fn test_limit_is_clamped_to_max_limit() {
        let dir = tempfile::tempdir().unwrap();
        let log = sample_log(dir.path());
        let tool = QueryTool::new(McpConfig {
            max_limit: 1,
            ..cfg_with_roots(vec![dir.path().to_path_buf()])
        });
        let result =
            tool.call(serde_json::json!({"paths": [log.display().to_string()], "limit": 1000}));
        assert!(!result.is_error);
        let text = result.content[0]["text"].as_str().unwrap();
        assert_eq!(
            text.lines().count(),
            1,
            "limit must be clamped to max_limit=1"
        );
    }

    #[test]
    fn test_redact_masks_email_in_message() {
        let dir = tempfile::tempdir().unwrap();
        let log = sample_log(dir.path());
        let tool = QueryTool::new(McpConfig {
            redact: true,
            ..cfg_with_roots(vec![dir.path().to_path_buf()])
        });
        let result = tool.call(serde_json::json!({"paths": [log.display().to_string()]}));
        assert!(!result.is_error);
        let text = result.content[0]["text"].as_str().unwrap();
        assert!(!text.contains("alice@example.com"), "email must be masked");
    }

    #[test]
    fn test_level_outside_whitelist_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let log = sample_log(dir.path());
        let tool = QueryTool::new(McpConfig {
            allowed_levels: Some(vec!["INFO".to_string()]),
            ..cfg_with_roots(vec![dir.path().to_path_buf()])
        });
        let result =
            tool.call(serde_json::json!({"paths": [log.display().to_string()], "level": "ERROR"}));
        assert!(result.is_error, "level outside whitelist must be rejected");
    }
}
