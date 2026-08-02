// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 脱敏规则注册中心
//!
//! 提供 [`MaskRuleRegistry`] 用于管理内置和自定义脱敏规则。
//! 支持规则的增删改查、启用/禁用、优先级排序，以及从 TOML 配置加载自定义规则。

use super::masking::MaskRule;
use crate::error::InklogError;

/// 脱敏规则注册中心，管理内置和自定义规则。
///
/// # Example
///
/// ```rust
/// use inklog::MaskRuleRegistry;
///
/// let mut registry = MaskRuleRegistry::with_builtins();
/// assert!(registry.active_rules().len() >= 21);
///
/// // 禁用某规则
/// registry.set_enabled("email", false);
///
/// // 获取活跃规则（按优先级排序）
/// let active = registry.active_rules();
/// ```
#[derive(Debug, Clone, Default)]
pub struct MaskRuleRegistry {
    rules: Vec<MaskRule>,
}

impl MaskRuleRegistry {
    /// 创建包含所有内置规则的注册中心。
    ///
    /// 内置规则包含 21 条预定义脱敏规则，涵盖邮箱、电话、身份证、
    /// 银行卡、信用卡、IP 地址、MAC 地址、护照号、SSN、数据库连接串、
    /// API 密钥、AWS 密钥、JWT、GitHub/Slack/Stripe/Google 令牌、私钥等。
    pub fn with_builtins() -> Self {
        use super::masking::DataMasker;
        let masker = DataMasker::new();
        Self {
            rules: masker.into_rules(),
        }
    }

    /// 注册自定义规则。
    ///
    /// # Errors
    /// 若已存在同名规则，返回 `Err(InklogError)`。
    pub fn register(&mut self, rule: MaskRule) -> Result<(), InklogError> {
        if self.rules.iter().any(|r| r.name() == rule.name()) {
            return Err(InklogError::ConfigError(format!(
                "Masking rule '{}' already registered",
                rule.name()
            )));
        }
        self.rules.push(rule);
        Ok(())
    }

    /// 按名称移除规则，返回被移除的规则。
    pub fn remove(&mut self, name: &str) -> Option<MaskRule> {
        if let Some(pos) = self.rules.iter().position(|r| r.name() == name) {
            Some(self.rules.remove(pos))
        } else {
            None
        }
    }

    /// 启用或禁用指定规则。
    ///
    /// 返回操作是否成功（规则是否存在）。
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.name() == name) {
            rule.set_enabled(enabled);
            true
        } else {
            false
        }
    }

    /// 获取所有活跃规则（已启用），按 priority 升序排列。
    pub fn active_rules(&self) -> Vec<&MaskRule> {
        let mut active: Vec<&MaskRule> = self.rules.iter().filter(|r| r.is_enabled()).collect();
        active.sort_by_key(|r| r.priority());
        active
    }

    /// 返回注册中心所有规则的数量（含禁用的）。
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// 注册中心是否为空。
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 从 TOML 字符串解析自定义规则定义。
    ///
    /// TOML 格式：
    /// ```toml
    /// [[masking_rules]]
    /// name = "custom_id"
    /// pattern = "\\bCUSTOM-\\d{6}\\b"
    /// replacement = "***CUSTOM***"
    /// priority = 100
    /// enabled = true
    /// ```
    ///
    /// # Errors
    /// - TOML 解析失败返回 `Err(InklogError)`
    /// - 缺少 `name` 或 `pattern` 字段返回 `Err(InklogError)`
    /// - 无效正则在构建规则时返回 `Err(InklogError)`
    pub fn load_from_toml(toml_str: &str) -> Result<Vec<MaskRule>, InklogError> {
        let parsed: toml::Value = toml::from_str(toml_str)
            .map_err(|e| InklogError::ConfigError(format!("Failed to parse TOML: {}", e)))?;

        let rules_tables = parsed
            .get("masking_rules")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                InklogError::ConfigError("TOML missing '[[masking_rules]]' array".to_string())
            })?;

        let mut rules = Vec::new();
        for table in rules_tables {
            let name = table.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                InklogError::ConfigError("masking_rules entry missing 'name' field".to_string())
            })?;
            let pattern = table
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    InklogError::ConfigError(format!(
                        "masking_rules entry '{}' missing 'pattern' field",
                        name
                    ))
                })?;
            let replacement = table
                .get("replacement")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let priority = table
                .get("priority")
                .and_then(|v| v.as_integer())
                .unwrap_or(100) as i32;
            let enabled = table
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let rule = MaskRule::builder(name)
                .pattern(pattern)
                .replacement(replacement)
                .priority(priority)
                .enabled(enabled)
                .build()?;
            rules.push(rule);
        }

        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_builtins() {
        let registry = MaskRuleRegistry::with_builtins();
        assert!(registry.len() >= 21);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_register_and_remove() {
        let mut registry = MaskRuleRegistry::default();
        let rule = MaskRule::builder("custom")
            .pattern(r"\d+")
            .replacement("***")
            .build()
            .unwrap();
        assert!(registry.register(rule).is_ok());
        assert_eq!(registry.len(), 1);

        // Duplicate name should fail
        let dup = MaskRule::builder("custom")
            .pattern(r"\w+")
            .replacement("###")
            .build()
            .unwrap();
        assert!(registry.register(dup).is_err());

        // Remove
        let removed = registry.remove("custom");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);

        // Remove non-existent
        assert!(registry.remove("nonexistent").is_none());
    }

    #[test]
    fn test_set_enabled() {
        let mut registry = MaskRuleRegistry::with_builtins();
        let initial_active = registry.active_rules().len();

        // Disable email rule
        assert!(registry.set_enabled("email", false));
        assert_eq!(registry.active_rules().len(), initial_active - 1);

        // Re-enable
        assert!(registry.set_enabled("email", true));
        assert_eq!(registry.active_rules().len(), initial_active);

        // Non-existent rule
        assert!(!registry.set_enabled("nonexistent", false));
    }

    #[test]
    fn test_active_rules_sorted_by_priority() {
        let registry = MaskRuleRegistry::with_builtins();
        let active = registry.active_rules();
        for window in active.windows(2) {
            assert!(window[0].priority() <= window[1].priority());
        }
    }

    #[test]
    fn test_load_from_toml() {
        let toml_str = r#"
[[masking_rules]]
name = "custom_id"
pattern = "\\bCUSTOM-\\d{6}\\b"
replacement = "***CUSTOM***"
priority = 100
enabled = true

[[masking_rules]]
name = "another_rule"
pattern = "\\bTEST-\\w+\\b"
"#;
        let rules = MaskRuleRegistry::load_from_toml(toml_str).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name(), "custom_id");
        assert_eq!(rules[0].priority(), 100);
        assert!(rules[0].is_enabled());
        assert_eq!(rules[1].name(), "another_rule");
        // Default priority and enabled
        assert_eq!(rules[1].priority(), 100);
        assert!(rules[1].is_enabled());
    }

    #[test]
    fn test_load_from_toml_missing_name() {
        let toml_str = r#"
[[masking_rules]]
pattern = "\\d+"
"#;
        assert!(MaskRuleRegistry::load_from_toml(toml_str).is_err());
    }

    #[test]
    fn test_load_from_toml_invalid_regex() {
        let toml_str = r#"
[[masking_rules]]
name = "bad"
pattern = "[invalid"
"#;
        assert!(MaskRuleRegistry::load_from_toml(toml_str).is_err());
    }

    #[test]
    fn test_load_from_toml_missing_array() {
        let toml_str = "[other]\nkey = \"value\"\n";
        assert!(MaskRuleRegistry::load_from_toml(toml_str).is_err());
    }
}
