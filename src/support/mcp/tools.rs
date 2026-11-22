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

        // field 键值经 sanitize 校验后解析为 FieldPredicate，交给 query.rs 共用过滤链
        let mut field_predicates: Vec<crate::support::query::FieldPredicate> = Vec::new();
        if let Some(field) = args.get("field").and_then(Value::as_str) {
            let sanitizer = crate::validation::sanitize::LogSanitizer::new();
            let cleaned = sanitizer.strip_ansi(field);
            if cleaned.contains(|c: char| c.is_control()) {
                return McpToolResult::error("field contains control characters");
            }
            let Some((key, value)) = cleaned.split_once('=') else {
                return McpToolResult::error("field must be k=v");
            };
            if key.is_empty() || value.is_empty() {
                return McpToolResult::error("field must be k=v");
            }
            // 数值字符串用 Number，否则用 String（matches_field 支持跨类型等值）
            let value_json = value
                .parse::<i64>()
                .map(serde_json::Value::from)
                .unwrap_or_else(|_| Value::String(value.to_string()));
            field_predicates.push(crate::support::query::FieldPredicate {
                key: key.to_string(),
                value: value_json,
            });
        }

        let raw_limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(self.cfg.max_limit as u64);
        let limit = usize::try_from(raw_limit)
            .unwrap_or(usize::MAX)
            .min(self.cfg.max_limit);

        let since = args.get("since").and_then(Value::as_str).and_then(|s| {
            time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
        });
        let until = args.get("until").and_then(Value::as_str).and_then(|s| {
            time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
        });

        let opts = QueryOptions {
            since,
            until,
            level,
            keyword: args.get("grep").and_then(Value::as_str).map(str::to_string),
            limit,
            fields: field_predicates,
        };
        match query_paths(&paths, &opts, None) {
            Ok(entries) => McpToolResult::ok(entries_to_text(&entries, self.cfg.redact)),
            Err(e) => McpToolResult::error(format!("query failed: {e}")),
        }
    }
}

/// `inklog_audit_verify` 工具：受策略约束的链自洽 + 封条 + 锚验证。
///
/// 密钥 env 名由服务端 `McpConfig.audit_key_env` 固定（工具参数不接受 env 名，
/// 防 Agent 指定任意 env 探测）；manifest 路径须在 allowed_manifest_dirs 内。
/// 返回与 verify-chain --json 同构的分段结论。
pub struct AuditVerifyTool {
    cfg: McpConfig,
}

impl AuditVerifyTool {
    pub fn new(cfg: McpConfig) -> Self {
        Self { cfg }
    }

    fn verify(&self, args: &Value) -> Result<Value, String> {
        let env = self
            .cfg
            .audit_key_env
            .as_deref()
            .ok_or("audit verify not enabled by policy")?;
        let manifest_str = args
            .get("manifest")
            .and_then(Value::as_str)
            .ok_or("manifest must be a string")?;
        let manifest = PathBuf::from(manifest_str);
        if !path_allowed(&manifest, &self.cfg) {
            return Err("manifest path outside allowed_manifest_dirs".to_string());
        }
        if !self.cfg.allowed_manifest_dirs.is_empty() {
            let Ok(canonical) = manifest.canonicalize() else {
                return Err("manifest path outside allowed_manifest_dirs".to_string());
            };
            if !self.cfg.allowed_manifest_dirs.iter().any(|dir| {
                dir.canonicalize()
                    .map(|base| canonical.starts_with(base))
                    .unwrap_or(false)
            }) {
                return Err("manifest path outside allowed_manifest_dirs".to_string());
            }
        }
        let key = std::env::var(env).map_err(|_| "audit key env not set".to_string())?;
        let key = key.into_bytes();

        let body = std::fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
        let entries: Vec<crate::support::audit_chain::ArchiveChainEntry> = body
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                serde_json::from_str(l).map_err(|e| {
                    format!(
                        "{}: {e}",
                        crate::i18n::tr("cli-verify-chain-err-line-invalid")
                    )
                })
            })
            .collect::<Result<_, _>>()?;
        if entries.is_empty() {
            return Ok(serde_json::json!({
                "command": "verify-chain",
                "manifest": manifest.display().to_string(),
                "entries": 0,
                "status": "empty",
                "anchor_status": "no-anchor",
                "failover_markers": 0,
            }));
        }
        let seal_path = crate::support::audit_chain::seal_path_for_manifest(&manifest);
        let seal: crate::support::audit_chain::ArchiveChainSeal =
            serde_json::from_str(&std::fs::read_to_string(&seal_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("invalid seal: {e}"))?;
        let chain_ok = crate::support::audit_chain::ArchiveChain::verify_entries(&entries, &key);
        let seal_ok = crate::support::audit_chain::ArchiveChain::verify_seal(&seal, &entries, &key);
        let status = if chain_ok && seal_ok {
            "ok"
        } else {
            "tampered"
        };

        // 锚段：存在锚文件时用同把 audit 密钥验（MCP 工具不暴露独立锚密钥 env）。
        let anchor_path = crate::support::audit_chain::anchor_path_for_manifest(&manifest);
        let (anchor_status, anchors_valid) = if anchor_path.exists() {
            let records: Vec<crate::support::audit_chain::AnchorRecord> =
                std::fs::read_to_string(&anchor_path)
                    .map_err(|e| e.to_string())?
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| {
                        serde_json::from_str(l).map_err(|e| format!("invalid anchor line: {e}"))
                    })
                    .collect::<Result<_, _>>()?;
            let signer = crate::support::audit_chain::HmacAnchorSigner::new(key.clone());
            let verdict = crate::support::audit_chain::verify_anchors(&records, &entries, &signer);
            (
                if verdict.all_valid() { "ok" } else { "fail" },
                Some(format!("{}/{}", verdict.valid, verdict.total)),
            )
        } else {
            ("no-anchor", None)
        };

        let failover_markers = entries
            .iter()
            .filter(|e| {
                serde_json::from_str::<Value>(&e.event)
                    .ok()
                    .and_then(|v| v.get("op").and_then(|o| o.as_str()).map(str::to_string))
                    .is_some_and(|op| {
                        matches!(
                            op.as_str(),
                            "failover_begin" | "failover_end" | "failover_replay"
                        )
                    })
            })
            .count();

        let mut report = serde_json::json!({
            "command": "verify-chain",
            "manifest": manifest.display().to_string(),
            "entries": entries.len(),
            "status": status,
            "anchor_status": anchor_status,
            "failover_markers": failover_markers,
        });
        if let Some(valid) = anchors_valid {
            report["anchors_valid"] = Value::String(valid);
        }
        Ok(report)
    }
}

impl McpTool for AuditVerifyTool {
    fn name(&self) -> &'static str {
        "inklog_audit_verify"
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": { "manifest": { "type": "string" } },
            "required": ["manifest"]
        })
    }

    fn call(&self, args: Value) -> McpToolResult {
        match self.verify(&args) {
            Ok(report) => McpToolResult::ok(report.to_string()),
            Err(e) => McpToolResult::error(e),
        }
    }
}

/// `inklog_stats` 工具：只读观测快照（Metrics + 健康面），不触发新探测。
pub struct StatsTool {
    cfg: McpConfig,
}

impl StatsTool {
    pub fn new(cfg: McpConfig) -> Self {
        Self { cfg }
    }
}

impl McpTool for StatsTool {
    fn name(&self) -> &'static str {
        "inklog_stats"
    }

    fn schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    fn call(&self, _args: Value) -> McpToolResult {
        let metrics = crate::Metrics::new();
        let report = serde_json::json!({
            "logs_written": metrics.logs_written(),
            "logs_dropped": metrics.logs_dropped(),
            "sampled_out": metrics.sampled_out(),
            "sink_errors": metrics.sink_errors(),
            "pool_hit_rate": metrics.pool_hit_rate(),
        });
        let _ = &self.cfg;
        McpToolResult::ok(report.to_string())
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

    #[cfg(unix)]
    #[test]
    fn test_symlink_escape_from_allowed_root_is_rejected() {
        let allowed = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        // 白名单目录内放一个软链，指向白名单外的真实日志文件
        let outside_log = outside.path().join("secret.log");
        std::fs::write(&outside_log, "2026-09-11T01:00:00Z [INFO] app - secret\n").unwrap();
        let link = allowed.path().join("link.log");
        std::os::unix::fs::symlink(&outside_log, &link).unwrap();

        let tool = QueryTool::new(cfg_with_roots(vec![allowed.path().to_path_buf()]));
        let result = tool.call(serde_json::json!({"paths": [link.display().to_string()]}));
        assert!(
            result.is_error,
            "symlink escaping allowed_roots must be rejected (realpath check)"
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

    // ---- T006: audit_verify / stats ----

    #[test]
    fn test_audit_verify_without_env_is_error() {
        let tool = AuditVerifyTool::new(McpConfig::default());
        let result = tool.call(serde_json::json!({"manifest": "/tmp/x.chain.jsonl"}));
        assert!(result.is_error, "missing audit_key_env must be rejected");
    }

    #[test]
    #[serial_test::serial]
    fn test_audit_verify_valid_manifest_fields() {
        use crate::support::audit_chain::{
            ArchiveChain, HmacAnchorSigner, anchor_path_for_manifest, create_anchor_record,
            seal_path_for_manifest,
        };

        let dir = tempfile::tempdir().unwrap();
        let mut chain = ArchiveChain::new(b"mcp-chain-key");
        chain.append(r#"{"path":"r.log","sha256":"aa","timestamp":"t"}"#);
        chain.append(r#"{"op":"failover_begin","sink":"r.log","ts":1}"#);
        let manifest = dir.path().join("mcp.chain.jsonl");
        std::fs::write(
            &manifest,
            chain
                .entries()
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        std::fs::write(
            seal_path_for_manifest(&manifest),
            serde_json::to_string(&chain.seal().unwrap()).unwrap(),
        )
        .unwrap();
        let signer = HmacAnchorSigner::new(b"mcp-chain-key".to_vec());
        let anchors: Vec<String> = chain
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                serde_json::to_string(
                    &create_anchor_record(&signer, i as u64, &e.hmac, i as u64 + 1).unwrap(),
                )
                .unwrap()
            })
            .collect();
        std::fs::write(anchor_path_for_manifest(&manifest), anchors.join("\n")).unwrap();

        unsafe {
            std::env::set_var("INKLOG_MCP_TEST_KEY", "mcp-chain-key");
        }
        let tool = AuditVerifyTool::new(McpConfig {
            audit_key_env: Some("INKLOG_MCP_TEST_KEY".to_string()),
            allowed_manifest_dirs: vec![dir.path().to_path_buf()],
            ..Default::default()
        });
        let result = tool.call(serde_json::json!({"manifest": manifest.display().to_string()}));
        unsafe {
            std::env::remove_var("INKLOG_MCP_TEST_KEY");
        }
        assert!(!result.is_error);
        let report: Value =
            serde_json::from_str(result.content[0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(report["status"], "ok");
        assert_eq!(report["anchor_status"], "ok");
        assert_eq!(report["anchors_valid"], "2/2");
        assert_eq!(report["failover_markers"], 1);
    }

    #[test]
    fn test_stats_returns_parseable_json() {
        let tool = StatsTool::new(McpConfig::default());
        let result = tool.call(serde_json::Value::Null);
        assert!(!result.is_error);
        let report: Value =
            serde_json::from_str(result.content[0]["text"].as_str().unwrap()).unwrap();
        assert!(report.get("logs_written").is_some());
        assert!(report.get("logs_dropped").is_some());
    }
}
