// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 处理管道：把记录进入发送阶段前的治理与改写阶段抽象为中间件链。
//!
//! 两轮结构（[`ProcessingPipeline`]）：
//! - 治理轮：限流/配额/采样类中间件，可 `Drop`（丢弃）或 `Reround`
//!   （终审放行，短路本轮剩余治理件——target 前缀配额组放行后不再进入
//!   全局限流的链上表达）；
//! - 改写轮：身份注入/脱敏类中间件，原位改写记录，恒不丢弃。
//!
//! 内置件由 `LoggerSubscriber` 按配置装配，与迁移前 on_event 的硬编码
//! 顺序逐点等价：target 配额 → 全局限流 → 身份注入 → 脱敏。发送阶段
//! 不属于管道（依赖通道与兜底缓冲，仍是 subscriber 职责）。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::LogRecord;
use crate::Metrics;
use crate::support::io::sink::middleware::{MiddlewareChain, MiddlewareVerdict, RecordMiddleware};
use crate::support::io::sink::sampling::SamplingPolicy;
use crate::support::processing::rate_limiter::RateLimiter;
use crate::support::processing::target_rate_limiter::{TargetQuotaVerdict, TargetRateLimiter};
use crate::validation::sanitize::LogSanitizer;

/// 限流压力下的内置兜底采样率：采样策略未命中时 ERROR/FATAL 保留 1/N。
pub const ERROR_SAMPLING_RATE: u64 = 100;

/// 关键级别判定（ERROR/FATAL）：压力兜底采样与发送侧兜底缓冲共用。
pub(crate) fn is_critical_level(level: &str) -> bool {
    level == "ERROR" || level == "FATAL"
}

/// 限流压力下的救援裁决，供两个限流中间件共享：采样策略规则优先
/// （[`SamplingPolicy::should_emit`] 命中即策略决策），无策略或无规则
/// 命中回退内置兜底——非关键级别丢弃，ERROR/FATAL 按 1-in-N 采样保留。
///
/// 丢弃计数与迁移前 on_event 压力路径逐点一致：采样淘汰计入
/// `sampled_out` + `logs_dropped`，非采样的压力丢弃仅计 `logs_dropped`。
///
/// 策略支持构建期任意顺序后绑定（`set_policy`）：装配顺序为先限流器
/// 后采样策略时，救援裁决同样可见。
pub struct StressRelief {
    metrics: Arc<Metrics>,
    error_sample_counter: AtomicU64,
    policy: parking_lot::RwLock<Option<Arc<SamplingPolicy>>>,
}

impl StressRelief {
    /// 以共享指标创建救援裁决（丢弃计数走同一 `Metrics` 实例）。
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self {
            metrics,
            error_sample_counter: AtomicU64::new(0),
            policy: parking_lot::RwLock::new(None),
        }
    }

    /// 绑定（或替换）采样策略。
    pub fn set_policy(&self, policy: Arc<SamplingPolicy>) {
        *self.policy.write() = Some(policy);
    }

    /// 压力救援裁决：`true` = 保留（继续后续阶段），`false` = 丢弃
    /// （丢弃计数已在内完成，调用方只负责短路）。
    pub fn admit(&self, record: &LogRecord) -> bool {
        let (keep, sampled_eviction) = match self
            .policy
            .read()
            .as_ref()
            .and_then(|policy| policy.should_emit(record))
        {
            Some(decision) => (decision, !decision),
            None if is_critical_level(&record.level) => {
                let count = self.error_sample_counter.fetch_add(1, Ordering::Relaxed);
                let keep = count.is_multiple_of(ERROR_SAMPLING_RATE);
                (keep, !keep)
            }
            None => (false, false),
        };
        if !keep {
            if sampled_eviction {
                self.metrics.inc_sampled_out();
            }
            self.metrics.inc_logs_dropped();
        }
        keep
    }
}

/// target 前缀配额组中间件：命中组由组桶独立裁决——放行即终审
/// （`Reround`，不再进入全局限流），组预算耗尽与全局限流拒绝共用同一
/// 关键级别救援语义；未命中 target 维持既有全局路径（`Continue`）。
pub struct TargetQuotaMiddleware {
    limiter: Arc<TargetRateLimiter>,
    stress: Arc<StressRelief>,
}

impl TargetQuotaMiddleware {
    pub fn new(limiter: Arc<TargetRateLimiter>, stress: Arc<StressRelief>) -> Self {
        Self { limiter, stress }
    }
}

impl RecordMiddleware for TargetQuotaMiddleware {
    fn name(&self) -> &str {
        "target-quota"
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        match self.limiter.evaluate(&record.target) {
            TargetQuotaVerdict::Pass => MiddlewareVerdict::Reround,
            TargetQuotaVerdict::Drop => {
                if self.stress.admit(record) {
                    MiddlewareVerdict::Reround
                } else {
                    MiddlewareVerdict::Drop
                }
            }
            TargetQuotaVerdict::Ungoverned => MiddlewareVerdict::Continue,
        }
    }
}

/// 全局限流中间件：未拒绝继续（`Continue`），拒绝时走压力救援——
/// 保留的记录继续后续治理件与改写轮，淘汰的丢弃（计数在救援内完成）。
pub struct GlobalRateLimitMiddleware {
    limiter: Arc<RateLimiter>,
    stress: Arc<StressRelief>,
}

impl GlobalRateLimitMiddleware {
    pub fn new(limiter: Arc<RateLimiter>, stress: Arc<StressRelief>) -> Self {
        Self { limiter, stress }
    }
}

impl RecordMiddleware for GlobalRateLimitMiddleware {
    fn name(&self) -> &str {
        "global-rate-limit"
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        if self.limiter.try_acquire() || self.stress.admit(record) {
            MiddlewareVerdict::Continue
        } else {
            MiddlewareVerdict::Drop
        }
    }
}

/// 服务身份静态字段注入中间件：or-insert 语义，事件显式同名字段优先。
pub struct IdentityFieldsMiddleware {
    fields: Arc<BTreeMap<String, serde_json::Value>>,
}

impl IdentityFieldsMiddleware {
    pub fn new(fields: Arc<BTreeMap<String, serde_json::Value>>) -> Self {
        Self { fields }
    }
}

impl RecordMiddleware for IdentityFieldsMiddleware {
    fn name(&self) -> &str {
        "identity-fields"
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        for (key, value) in self.fields.iter() {
            record
                .fields
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
        MiddlewareVerdict::Continue
    }
}

/// 脱敏中间件：message 与全部字段值递归脱敏（防日志注入，CWE-117）。
pub struct SanitizeMiddleware {
    sanitizer: Arc<LogSanitizer>,
}

impl SanitizeMiddleware {
    pub fn new(sanitizer: Arc<LogSanitizer>) -> Self {
        Self { sanitizer }
    }
}

impl RecordMiddleware for SanitizeMiddleware {
    fn name(&self) -> &str {
        "sanitize"
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        record.message = self.sanitizer.sanitize(&record.message);
        for value in record.fields.values_mut() {
            Self::sanitize_field_value(&self.sanitizer, value);
        }
        MiddlewareVerdict::Continue
    }
}

impl SanitizeMiddleware {
    /// 递归脱敏字段值：字符串值一律 sanitize；Object 按键递归（敏感键的
    /// 字符串值同样被脱敏）；Array 逐元素递归。递归深度上限 16 层，超限
    /// 子树整体替换为截断标记（防深嵌套栈溢出）。
    fn sanitize_field_value(sanitizer: &LogSanitizer, value: &mut serde_json::Value) {
        Self::sanitize_field_value_depth(sanitizer, value, 0, MAX_SANITIZE_DEPTH);
    }

    fn sanitize_field_value_depth(
        sanitizer: &LogSanitizer,
        value: &mut serde_json::Value,
        depth: usize,
        max_depth: usize,
    ) {
        if depth >= max_depth {
            *value = serde_json::Value::String("***TRUNCATED***".to_string());
            return;
        }
        match value {
            serde_json::Value::String(s) => *s = sanitizer.sanitize(s),
            serde_json::Value::Array(items) => {
                for item in items.iter_mut() {
                    Self::sanitize_field_value_depth(sanitizer, item, depth + 1, max_depth);
                }
            }
            serde_json::Value::Object(map) => {
                for (nested_key, nested_value) in map.iter_mut() {
                    if LogRecord::is_sensitive_key(nested_key)
                        && let serde_json::Value::String(s) = nested_value
                    {
                        *s = sanitizer.sanitize(s);
                    } else {
                        Self::sanitize_field_value_depth(
                            sanitizer,
                            nested_value,
                            depth + 1,
                            max_depth,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

const MAX_SANITIZE_DEPTH: usize = 16;

/// 处理管道：治理轮 → 改写轮。`process` 返回 `false` 表示治理轮裁决
/// 丢弃（丢弃计数由治理中间件内部完成，调用方只负责短路发送）。
///
/// 装配约定：内置限流件经 [`ProcessingPipeline::register_governance_front`]
/// 头插保持 `target 配额 → 全局限流` 的固定裁决顺序；用户中间件经
/// [`ProcessingPipeline::register_governance`] 尾插，链上位次在内置治理
/// 之后、改写轮之前（可裁决丢弃，也可原位改写，改写结果仍会被脱敏）。
/// 位次不等于恒执行：治理链任一环节 `Drop`/`Reround` 即短路剩余治理件，
/// 配额管辖 target 放行（`Reround`）时用户件被跳过。
#[derive(Clone, Default)]
pub struct ProcessingPipeline {
    governance: MiddlewareChain,
    rewrite: MiddlewareChain,
}

impl ProcessingPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// 治理环头部插入（内置限流类专用：固定配额先于全局限流）。
    pub fn register_governance_front(&mut self, middleware: Arc<dyn RecordMiddleware>) {
        self.governance.push_front(middleware);
    }

    /// 治理环尾部追加（`with_middleware` 装配入口）。尾插只定位次：链上
    /// 任一环节 `Drop`/`Reround` 即短路剩余治理件，本件不保证对每条记录
    /// 执行（配额管辖 target 放行时被 `Reround` 跳过）。
    pub fn register_governance(&mut self, middleware: Arc<dyn RecordMiddleware>) {
        self.governance.append(middleware);
    }

    /// 改写环尾部追加。
    ///
    /// 陷阱：改写环契约恒不丢弃，返回 `Drop` 的改写件不生效——`process`
    /// 忽略改写环裁决（debug 构建下断言失败），记录仍进入发送阶段；需要
    /// 丢弃语义请注册为治理件（[`ProcessingPipeline::register_governance`]）。
    /// `Reround` 同理会短路改写环剩余改写件。
    pub fn register_rewrite(&mut self, middleware: Arc<dyn RecordMiddleware>) {
        self.rewrite.append(middleware);
    }

    /// 改写环头部插入：固定内置改写件次序用（身份注入恒先于脱敏，
    /// 与 builder 接线顺序无关）。
    pub fn register_rewrite_front(&mut self, middleware: Arc<dyn RecordMiddleware>) {
        self.rewrite.push_front(middleware);
    }

    /// 治理环中间件数（诊断/测试）。
    pub fn governance_len(&self) -> usize {
        self.governance.len()
    }

    /// 改写环中间件数（诊断/测试）。
    pub fn rewrite_len(&self) -> usize {
        self.rewrite.len()
    }

    /// 处理一条记录：治理轮（`Drop`/`Reround` 短路）→ 改写轮。返回
    /// `false` 表示记录被治理轮丢弃，不应进入发送阶段。
    ///
    /// 改写环按契约恒不丢弃：其裁决被忽略（debug 构建下断言失败），
    /// 记录恒进入发送阶段。
    pub fn process(&self, record: &mut LogRecord) -> bool {
        if !self.governance.apply(record) {
            return false;
        }
        let rewritten = self.rewrite.apply(record);
        debug_assert!(
            rewritten,
            "rewrite middleware returned Drop: the rewrite round is \
             non-dropping by contract, register a governance middleware instead"
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tracing::Level;

    fn record(level: Level, target: &str, message: &str) -> LogRecord {
        LogRecord::new(level, target.to_string(), message.to_string())
    }

    fn metrics() -> Arc<Metrics> {
        Arc::new(Metrics::new())
    }

    fn quota_limiter(rules: &[(&str, u64)]) -> Arc<TargetRateLimiter> {
        Arc::new(
            TargetRateLimiter::from_rules(
                rules
                    .iter()
                    .map(|(p, r)| (p.to_string(), *r))
                    .collect::<HashMap<_, _>>(),
            )
            .unwrap(),
        )
    }

    /// 记录治理轮执行顺序的探针中间件。
    struct ProbeMiddleware {
        name: &'static str,
        log: Arc<parking_lot::Mutex<Vec<&'static str>>>,
        verdict: MiddlewareVerdict,
    }

    impl RecordMiddleware for ProbeMiddleware {
        fn name(&self) -> &str {
            self.name
        }
        fn process(&self, _record: &mut LogRecord) -> MiddlewareVerdict {
            self.log.lock().push(self.name);
            self.verdict
        }
    }

    // =========================================================================
    // 内置治理件：与迁移前 on_event 限流路径逐点等价
    // =========================================================================

    #[test]
    fn test_target_quota_pass_rerounds_and_ungoverned_continues() {
        let mw = TargetQuotaMiddleware::new(
            quota_limiter(&[("app", 16)]),
            Arc::new(StressRelief::new(metrics())),
        );
        // 组桶放行 → Reround（不再进入全局限流）
        let mut governed = record(Level::INFO, "app::audit", "m");
        assert_eq!(mw.process(&mut governed), MiddlewareVerdict::Reround);
        // 无规则命中 → Continue（交给后续治理件）
        let mut ungoverned = record(Level::INFO, "other::x", "m");
        assert_eq!(mw.process(&mut ungoverned), MiddlewareVerdict::Continue);
    }

    #[test]
    fn test_target_quota_drop_non_critical_drops_with_metrics() {
        // 组配额 1 令牌：首条放行耗尽组预算，次条进入压力路径；无策略 →
        // 内置兜底：INFO 丢弃，计 dropped 不计 sampled_out
        let m = metrics();
        let mw = TargetQuotaMiddleware::new(
            quota_limiter(&[("app", 1)]),
            Arc::new(StressRelief::new(m.clone())),
        );
        let mut first = record(Level::INFO, "app::core", "first");
        assert_eq!(mw.process(&mut first), MiddlewareVerdict::Reround);
        let mut second = record(Level::INFO, "app::core", "second");
        assert_eq!(mw.process(&mut second), MiddlewareVerdict::Drop);
        assert_eq!(m.logs_dropped(), 1);
        assert_eq!(
            m.sampled_out(),
            0,
            "non-critical stress rejection is a rate-limit drop, not sampling eviction"
        );
    }

    #[test]
    fn test_target_quota_drop_critical_kept_by_builtin_sampling() {
        // 组预算耗尽后的 ERROR：兜底 1-in-100 首个计数（0）放行 → Reround
        // （不进全局限流），且不产生丢弃计数
        let m = metrics();
        let mw = TargetQuotaMiddleware::new(
            quota_limiter(&[("app", 1)]),
            Arc::new(StressRelief::new(m.clone())),
        );
        let mut exhaust = record(Level::INFO, "app::core", "exhaust");
        assert_eq!(mw.process(&mut exhaust), MiddlewareVerdict::Reround);
        let mut critical = record(Level::ERROR, "app::core", "critical");
        assert_eq!(mw.process(&mut critical), MiddlewareVerdict::Reround);
        assert_eq!(m.logs_dropped(), 0);
        assert_eq!(m.sampled_out(), 0);
    }

    #[test]
    fn test_global_rate_limit_rejects_with_relief_semantics() {
        let m = metrics();
        let mw = GlobalRateLimitMiddleware::new(
            Arc::new(RateLimiter::new(0)),
            Arc::new(StressRelief::new(m.clone())),
        );
        // 非关键：恒拒（0 令牌）→ Drop + dropped 计数
        let mut info = record(Level::INFO, "t", "m");
        assert_eq!(mw.process(&mut info), MiddlewareVerdict::Drop);
        assert_eq!(m.logs_dropped(), 1);
        // ERROR：兜底采样保留 → Continue（继续后续治理件与改写轮）
        let mut err = record(Level::ERROR, "t", "m");
        assert_eq!(mw.process(&mut err), MiddlewareVerdict::Continue);
    }

    #[test]
    fn test_stress_relief_policy_eviction_counts_sampled_out() {
        let m = metrics();
        let stress = Arc::new(StressRelief::new(m.clone()));
        let mut per_level = HashMap::new();
        per_level.insert("info".to_string(), 2u64);
        stress.set_policy(Arc::new(
            SamplingPolicy::from_config(&crate::domain::config::sampling::SamplingConfig {
                per_level,
                per_target_prefix: HashMap::new(),
            })
            .unwrap(),
        ));
        // per-level 1-in-2：第 1 条（counter 0）放行，第 2 条淘汰计入 sampled_out
        let first = record(Level::INFO, "t", "first");
        let second = record(Level::INFO, "t", "second");
        assert!(stress.admit(&first));
        assert!(!stress.admit(&second));
        assert_eq!(
            m.sampled_out(),
            1,
            "policy eviction must count as sampled_out"
        );
        assert_eq!(m.logs_dropped(), 1);
    }

    // =========================================================================
    // 内置改写件
    // =========================================================================

    #[test]
    fn test_identity_fields_or_insert_event_field_wins() {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("service_name".to_string(), serde_json::json!("orders"));
        let mw = IdentityFieldsMiddleware::new(Arc::new(fields));
        let mut r = record(Level::INFO, "t", "m");
        r.fields
            .insert("service_name".to_string(), serde_json::json!("explicit"));
        assert_eq!(mw.process(&mut r), MiddlewareVerdict::Continue);
        assert_eq!(
            r.fields.get("service_name").unwrap(),
            &serde_json::json!("explicit")
        );
        r.fields.remove("service_name");
        mw.process(&mut r);
        assert_eq!(
            r.fields.get("service_name").unwrap(),
            &serde_json::json!("orders")
        );
    }

    #[test]
    fn test_sanitize_middleware_message_and_nested_fields() {
        let mw = SanitizeMiddleware::new(Arc::new(LogSanitizer::new()));
        let mut r = record(Level::INFO, "t", "line1\nline2");
        r.fields.insert(
            "payload".to_string(),
            serde_json::json!({ "password": "a\nb", "note": "c" }),
        );
        assert_eq!(mw.process(&mut r), MiddlewareVerdict::Continue);
        assert!(
            !r.message.contains('\n'),
            "message newlines must be escaped"
        );
        let payload = r.fields.get("payload").unwrap();
        let password = payload.get("password").unwrap().as_str().unwrap();
        assert!(
            !password.contains('\n'),
            "sensitive nested value must be sanitized"
        );
    }

    #[test]
    fn test_sanitize_middleware_recurses_into_nested_objects_and_arrays() {
        // 嵌套对象/数组中的敏感键字符串值被脱敏；普通键同样被递归处理
        let mw = SanitizeMiddleware::new(Arc::new(LogSanitizer::new()));
        let mut r = record(Level::INFO, "test::sanitize", "nested sanitize");
        let mut nested = serde_json::Map::new();
        nested.insert("password".to_string(), serde_json::json!("line1\nline2"));
        nested.insert("note".to_string(), serde_json::json!("a\nb"));
        r.fields
            .insert("config".to_string(), serde_json::Value::Object(nested));
        let mut item = serde_json::Map::new();
        item.insert("api_token".to_string(), serde_json::json!("tok1\ntok2"));
        r.fields.insert(
            "items".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::Object(item)]),
        );

        assert_eq!(mw.process(&mut r), MiddlewareVerdict::Continue);

        let config = r.fields.get("config").unwrap();
        if let serde_json::Value::Object(map) = config {
            if let serde_json::Value::String(s) = map.get("password").unwrap() {
                assert!(
                    !s.contains('\n') && s.contains("\\n"),
                    "nested sensitive key 'password' must be sanitized, got: {s:?}"
                );
            } else {
                panic!("password value should remain a string");
            }
            if let serde_json::Value::String(s) = map.get("note").unwrap() {
                assert!(
                    !s.contains('\n'),
                    "nested plain string must also be sanitized, got: {s:?}"
                );
            }
        } else {
            panic!("config field should remain an object");
        }

        let items = r.fields.get("items").unwrap();
        if let serde_json::Value::Array(arr) = items {
            if let serde_json::Value::Object(map) = &arr[0] {
                if let serde_json::Value::String(s) = map.get("api_token").unwrap() {
                    assert!(
                        !s.contains('\n') && s.contains("\\n"),
                        "sensitive key inside array must be sanitized, got: {s:?}"
                    );
                } else {
                    panic!("api_token value should remain a string");
                }
            } else {
                panic!("array element should remain an object");
            }
        } else {
            panic!("items field should remain an array");
        }
    }

    #[test]
    fn test_sanitize_middleware_leaves_non_string_values_untouched() {
        let mw = SanitizeMiddleware::new(Arc::new(LogSanitizer::new()));
        let mut r = record(Level::INFO, "test::sanitize", "non-string values");
        r.fields.insert("count".to_string(), serde_json::json!(42));

        assert_eq!(mw.process(&mut r), MiddlewareVerdict::Continue);
        assert_eq!(
            r.fields.get("count").unwrap(),
            &serde_json::json!(42),
            "non-string values must not be modified"
        );
    }

    // =========================================================================
    // 管道两轮结构与装配
    // =========================================================================

    #[test]
    fn test_pipeline_empty_is_passthrough() {
        let pipeline = ProcessingPipeline::new();
        let mut r = record(Level::INFO, "t", "m");
        assert!(pipeline.process(&mut r));
        assert_eq!(pipeline.governance_len(), 0);
        assert_eq!(pipeline.rewrite_len(), 0);
    }

    #[test]
    fn test_pipeline_governance_drop_skips_rewrite() {
        let log = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let mut pipeline = ProcessingPipeline::new();
        pipeline.register_governance(Arc::new(ProbeMiddleware {
            name: "drop",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Drop,
        }));
        pipeline.register_rewrite(Arc::new(IdentityFieldsMiddleware::new(Arc::new(
            std::collections::BTreeMap::new(),
        ))));

        let mut r = record(Level::INFO, "t", "m");
        assert!(!pipeline.process(&mut r), "governance drop must discard");
        assert_eq!(*log.lock(), vec!["drop"]);
        assert_eq!(pipeline.rewrite_len(), 1);
    }

    #[test]
    fn test_pipeline_reround_short_circuits_governance_but_runs_rewrite() {
        let log = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let mut pipeline = ProcessingPipeline::new();
        pipeline.register_governance(Arc::new(ProbeMiddleware {
            name: "reround",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Reround,
        }));
        pipeline.register_governance(Arc::new(ProbeMiddleware {
            name: "skipped",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Drop,
        }));
        pipeline.register_rewrite(Arc::new(IdentityFieldsMiddleware::new(Arc::new(
            std::collections::BTreeMap::new(),
        ))));

        let mut r = record(Level::INFO, "t", "m");
        assert!(pipeline.process(&mut r), "reround must emit");
        assert_eq!(
            *log.lock(),
            vec!["reround"],
            "reround must skip remaining governance middlewares"
        );
    }

    #[test]
    fn test_pipeline_governance_order_is_insertion_with_front_registration() {
        let log = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let mut pipeline = ProcessingPipeline::new();
        // 全局限流类先注册、配额类头插：环内恒为 [target-quota, global]
        pipeline.register_governance(Arc::new(ProbeMiddleware {
            name: "global",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Continue,
        }));
        pipeline.register_governance_front(Arc::new(ProbeMiddleware {
            name: "quota",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Continue,
        }));
        // 用户中间件（with_middleware 装配入口）尾插：恒在内置治理之后
        pipeline.register_governance(Arc::new(ProbeMiddleware {
            name: "user",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Continue,
        }));

        let mut r = record(Level::INFO, "t", "m");
        assert!(pipeline.process(&mut r));
        assert_eq!(*log.lock(), vec!["quota", "global", "user"]);
        assert_eq!(pipeline.governance_len(), 3);
    }

    #[test]
    fn test_pipeline_quota_reround_bypasses_user_governance_middleware() {
        // 配额件（头插）放行即 Reround 终审：短路其后的用户件——「用户件
        // 恒在内置限流件之后执行」不成立，钉住该旁路行为（配额管辖 target
        // 的放行记录不经过用户治理件），防止未来无意变更
        let log = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let mut pipeline = ProcessingPipeline::new();
        pipeline.register_governance_front(Arc::new(ProbeMiddleware {
            name: "quota",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Reround,
        }));
        pipeline.register_governance(Arc::new(ProbeMiddleware {
            name: "user",
            log: Arc::clone(&log),
            verdict: MiddlewareVerdict::Continue,
        }));

        let mut r = record(Level::INFO, "t", "m");
        assert!(pipeline.process(&mut r), "reround must emit");
        assert_eq!(
            *log.lock(),
            vec!["quota"],
            "quota Reround must bypass the user governance middleware after it"
        );
    }
}
