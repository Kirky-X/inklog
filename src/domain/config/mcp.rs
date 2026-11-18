// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP 工具面策略配置（feature `mcp`）。
//!
//! 全部约束在服务端配置中声明：Agent 传入的工具参数不可越权。
//! 结构字段与语义见 specmark/changes/mcp-log-tools/design.md §3。

use crate::InklogError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 已知工具名（allow 白名单只接受这些值）。
pub const KNOWN_TOOLS: &[&str] = &["inklog_query", "inklog_audit_verify", "inklog_stats"];

fn default_max_limit() -> usize {
    1000
}

fn default_true() -> bool {
    true
}

/// MCP 工具面策略。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpConfig {
    /// 允许暴露的工具名；空 = 全部启用
    #[serde(default)]
    pub allow: Vec<String>,
    /// inklog_query 返回条数上限（工具参数 limit 钳制到该值）
    #[serde(default = "default_max_limit")]
    pub max_limit: usize,
    /// 允许的 level 集合；None = 不限
    #[serde(default)]
    pub allowed_levels: Option<Vec<String>>,
    /// 允许的 target 前缀集合；None = 不限
    #[serde(default)]
    pub allowed_target_prefixes: Option<Vec<String>>,
    /// 查询结果是否强制过 DataMasker（默认 true；false 需显式声明）
    #[serde(default = "default_true")]
    pub redact: bool,
    /// 允许读取的日志根目录列表（realpath 前缀校验）
    #[serde(default)]
    pub allowed_roots: Vec<PathBuf>,
    /// inklog_audit_verify 使用的密钥 env 名（工具参数不接受 env 名）
    #[serde(default)]
    pub audit_key_env: Option<String>,
    /// verify-chain manifest 允许的父目录白名单；空 = 不限
    #[serde(default)]
    pub allowed_manifest_dirs: Vec<PathBuf>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            max_limit: default_max_limit(),
            allowed_levels: None,
            allowed_target_prefixes: None,
            redact: default_true(),
            allowed_roots: Vec::new(),
            audit_key_env: None,
            allowed_manifest_dirs: Vec::new(),
        }
    }
}

impl McpConfig {
    /// 校验策略：level 合法、工具名白名单、audit_key_env 须 INKLOG_ 前缀。
    pub fn validate(&self) -> Result<(), InklogError> {
        if self.max_limit == 0 {
            return Err(InklogError::ConfigError(
                "mcp.max_limit must be >= 1".to_string(),
            ));
        }
        if let Some(levels) = &self.allowed_levels {
            for level in levels {
                if !crate::LogLevel::is_valid_level(level) {
                    let mut args = crate::i18n::MsgArgs::new();
                    args.set("level", level);
                    args.set("valid", crate::LogLevel::VALID_LEVEL_STRINGS.join(", "));
                    return Err(InklogError::ConfigError(crate::i18n::tr_args(
                        "config-mcp_level_invalid",
                        args,
                    )));
                }
            }
        }
        for tool in &self.allow {
            if !KNOWN_TOOLS.contains(&tool.as_str()) {
                let mut args = crate::i18n::MsgArgs::new();
                args.set("tool", tool);
                return Err(InklogError::ConfigError(crate::i18n::tr_args(
                    "config-mcp_unknown_tool",
                    args,
                )));
            }
        }
        if let Some(env) = &self.audit_key_env
            && !env.starts_with("INKLOG_")
        {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("env", env);
            return Err(InklogError::ConfigError(crate::i18n::tr_args(
                "config-mcp_audit_key_env_prefix",
                args,
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mcp_config_valid() {
        let cfg = McpConfig::default();
        assert_eq!(cfg.max_limit, 1000);
        assert!(cfg.redact);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_mcp_rejects_invalid_level() {
        let cfg = McpConfig {
            allowed_levels: Some(vec!["not-a-level".to_string()]),
            ..Default::default()
        };
        assert!(cfg.validate().is_err(), "invalid level must be rejected");
    }

    #[test]
    fn test_mcp_rejects_unknown_tool() {
        let cfg = McpConfig {
            allow: vec!["inklog_delete_all".to_string()],
            ..Default::default()
        };
        assert!(cfg.validate().is_err(), "unknown tool must be rejected");
    }

    #[test]
    fn test_mcp_rejects_non_inklog_audit_env() {
        let cfg = McpConfig {
            audit_key_env: Some("AWS_SECRET".to_string()),
            ..Default::default()
        };
        assert!(cfg.validate().is_err(), "non-INKLOG_ env must be rejected");
    }

    #[test]
    fn test_mcp_accepts_valid_policy() {
        let cfg = McpConfig {
            allow: vec!["inklog_query".to_string()],
            allowed_levels: Some(vec!["INFO".to_string(), "error".to_string()]),
            audit_key_env: Some("INKLOG_AUDIT_KEY".to_string()),
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }
}
