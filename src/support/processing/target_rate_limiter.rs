// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 按 target 前缀分组的配额限流（R7：per-target 分级限流）。
//!
//! 与全局 [`RateLimiter`](crate::support::processing::RateLimiter) 的关系：
//! 命中前缀规则的 target 由其配额组桶独立裁决（通过后不再进入全局限流），
//! 未命中规则的 target 维持既有全局路径——未配置规则时整体零介入。组预算
//! 耗尽的拒绝在订阅器侧与全局限流拒绝共用同一关键级别救援（ERROR/FATAL
//! 1-in-N 采样保留），两条限流来源语义一致。
//! 查找为无分配的线性扫描（规则按前缀长度降序，最长前缀优先），目标规模
//! 典型 <8 条，量级在 100ns 以内（benches/inklog_bench.rs 的
//! `target_rate_limiter_lookup` 组持续锁定该基线）。

use std::collections::HashMap;

use crate::InklogError;
use crate::support::processing::rate_limiter::RateLimiter;

/// 前缀配额组裁决结果：组预算放行 / 组预算耗尽 / 无规则管辖。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetQuotaVerdict {
    /// 命中规则且组桶放行（不再进入全局限流）
    Pass,
    /// 命中规则且组桶预算耗尽（记录应丢弃）
    Drop,
    /// 无规则命中（走既有全局限流路径）
    Ungoverned,
}

/// target 前缀配额组限流器：规则 = 前缀 → 该组每秒令牌数（组内共享预算）。
pub struct TargetRateLimiter {
    /// (小写前缀, 组桶)；按前缀长度降序排列，首个命中生效
    rules: Vec<(String, RateLimiter)>,
}

impl std::fmt::Debug for TargetRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 组桶内部状态（锁 + 余额）无诊断价值，只列出规则前缀
        let prefixes: Vec<&str> = self.rules.iter().map(|(p, _)| p.as_str()).collect();
        f.debug_struct("TargetRateLimiter")
            .field("rules", &prefixes)
            .finish()
    }
}

impl TargetRateLimiter {
    /// 从「前缀 → 每秒令牌数」规则表构建。
    ///
    /// # Errors
    ///
    /// 前缀为空或速率为 0 返回 [`InklogError::ConfigError`]（与采样配置
    /// 一致的硬拒绝语义：配额直接决定压力下的日志保留）。
    pub fn from_rules(rules: HashMap<String, u64>) -> Result<Self, InklogError> {
        let mut sorted: Vec<(String, RateLimiter)> = Vec::with_capacity(rules.len());
        for (prefix, rate) in &rules {
            if prefix.is_empty() {
                return Err(InklogError::ConfigError(
                    "rate_limit.rules key must not be empty".to_string(),
                ));
            }
            if *rate == 0 {
                return Err(InklogError::ConfigError(format!(
                    "rate_limit.rules[{prefix:?}] rate must be >= 1, got 0"
                )));
            }
            // ASCII 小写存储，与 rule_index 的 eq_ignore_ascii_case 同语义
            sorted.push((prefix.to_ascii_lowercase(), RateLimiter::new(*rate)));
        }
        sorted.sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));
        Ok(Self { rules: sorted })
    }

    /// 纯查找：target 是否命中某前缀规则（大小写不敏感，无分配）。
    pub fn governs(&self, target: &str) -> bool {
        self.rule_index(target).is_some()
    }

    /// 组裁决：命中规则的 target 消费其组桶（最长前缀优先），未命中返回
    /// [`TargetQuotaVerdict::Ungoverned`]。
    pub fn evaluate(&self, target: &str) -> TargetQuotaVerdict {
        match self.rule_index(target) {
            Some(idx) if self.rules[idx].1.try_acquire() => TargetQuotaVerdict::Pass,
            Some(_) => TargetQuotaVerdict::Drop,
            None => TargetQuotaVerdict::Ungoverned,
        }
    }

    /// 最长前缀命中：无分配的 ASCII 大小写不敏感前缀比较。
    fn rule_index(&self, target: &str) -> Option<usize> {
        let bytes = target.as_bytes();
        self.rules.iter().position(|(prefix, _)| {
            let p = prefix.as_bytes();
            bytes.len() >= p.len() && bytes[..p.len()].eq_ignore_ascii_case(p)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter_from(pairs: &[(&str, u64)]) -> TargetRateLimiter {
        TargetRateLimiter::from_rules(pairs.iter().map(|(p, r)| (p.to_string(), *r)).collect())
            .unwrap()
    }

    #[test]
    fn test_from_rules_rejects_empty_prefix_and_zero_rate() {
        let mut rules = HashMap::new();
        rules.insert(String::new(), 10);
        let err = TargetRateLimiter::from_rules(rules).unwrap_err();
        assert!(err.to_string().contains("empty"), "unexpected: {err}");

        let mut rules = HashMap::new();
        rules.insert("app".to_string(), 0);
        let err = TargetRateLimiter::from_rules(rules).unwrap_err();
        assert!(err.to_string().contains(">= 1"), "unexpected: {err}");
    }

    #[test]
    fn test_longest_prefix_wins() {
        let lim = limiter_from(&[
            ("app", 1000),
            ("app::audit", 1000),
            ("app::audit::core", 1000),
        ]);
        assert!(lim.governs("app::audit::core::handler"));
        // 前缀匹配按字节 starts_with（与 SamplingPolicy per_target_prefix 同语义）：
        // "application" 以 "app" 开头，同属 app 组——配额组语义与采样前缀一致
        assert!(lim.governs("application"));
        assert!(lim.governs("app::net"));
        assert!(!lim.governs("other::mod"));
    }

    #[test]
    fn test_prefix_match_is_case_insensitive() {
        let lim = limiter_from(&[("app::audit", 1000)]);
        assert!(lim.governs("App::Audit::Core"));
        assert!(lim.governs("APP::AUDIT"));
    }

    #[test]
    fn test_ungoverned_targets_pass_without_consuming() {
        let lim = limiter_from(&[("app::noise", 1)]);
        for _ in 0..100 {
            assert_eq!(
                lim.evaluate("other::clean"),
                TargetQuotaVerdict::Ungoverned,
                "无规则 target 必须不受配额管辖"
            );
        }
        // 管辖组的预算不被未管辖 target 消耗
        assert_eq!(lim.evaluate("app::noise"), TargetQuotaVerdict::Pass);
    }

    #[test]
    fn test_quota_group_exhausts_rejects_same_instant() {
        // rate=1：组桶启动 1 令牌；同刻第二次消费必然拒绝（补充速率 1/s，
        // refill 属性由底层 RateLimiter 自测覆盖）
        let lim = limiter_from(&[("app::noise", 1)]);
        assert_eq!(lim.evaluate("app::noise"), TargetQuotaVerdict::Pass);
        assert_eq!(
            lim.evaluate("app::noise"),
            TargetQuotaVerdict::Drop,
            "组预算耗尽必须拒绝同组后续记录"
        );
    }

    #[test]
    fn test_higher_rate_group_allows_burst() {
        let lim = limiter_from(&[("app::burst", 16)]);
        for _ in 0..16 {
            assert_eq!(lim.evaluate("app::burst"), TargetQuotaVerdict::Pass);
        }
        assert_eq!(lim.evaluate("app::burst"), TargetQuotaVerdict::Drop);
    }
}
