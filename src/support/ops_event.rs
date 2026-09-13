// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 内部审计/运维事件流（ops event log）。
//!
//! sink 恢复、降级切换、丢弃激增等运维事件以结构化 [`InklogOpsEvent`]
//! 表达，经 [`LoggerManager::publish_ops_event`](crate::LoggerManager::publish_ops_event)
//! 复用统一 sink 通道写入全部已启用 sink（file/db/自定义），供告警
//! 系统与事后审计消费（与 [`crate::support::audit_chain`] 的防篡改链互补：
//! 链防外部篡改，本事件流记内部运维）。

use chrono::Utc;
use serde_json::Value;

use crate::LogRecord;

/// 内部运维/审计事件。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InklogOpsEvent {
    /// 事件种类（如 `sink_recovered` / `sink_degraded` / `config_loaded` /
    /// `records_dropped` / `level_changed`）
    pub kind: String,
    /// 事件时间（UTC）
    pub timestamp: chrono::DateTime<Utc>,
    /// 关联 sink 名（可空）
    pub sink: Option<String>,
    /// 结构化细节（自由 JSON）
    pub detail: Value,
}

impl InklogOpsEvent {
    /// 以当前时间创建事件。
    pub fn now(kind: impl Into<String>, sink: Option<&str>, detail: Value) -> Self {
        Self {
            kind: kind.into(),
            timestamp: Utc::now(),
            sink: sink.map(str::to_string),
            detail,
        }
    }

    /// 转换为结构化日志记录：`target = "inklog::ops"`，message = 事件 JSON，
    /// 关键字段平铺进 `fields`；`sink_degraded`/`records_dropped` 类事件
    /// 映射为 WARN 级，其余 INFO。
    pub fn to_log_record(&self) -> LogRecord {
        let warn = matches!(
            self.kind.as_str(),
            "sink_degraded" | "records_dropped" | "sink_unavailable"
        );
        let mut record = LogRecord::new(
            if warn {
                tracing::Level::WARN
            } else {
                tracing::Level::INFO
            },
            "inklog::ops".to_string(),
            serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string()),
        );
        record
            .fields
            .insert("ops_kind".to_string(), Value::String(self.kind.clone()));
        if let Some(sink) = &self.sink {
            record
                .fields
                .insert("ops_sink".to_string(), Value::String(sink.clone()));
        }
        record
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ops_event_to_log_record_mapping() {
        let event = InklogOpsEvent::now(
            "sink_recovered",
            Some("database"),
            serde_json::json!({ "attempts": 2 }),
        );
        let record = event.to_log_record();
        assert_eq!(record.target, "inklog::ops");
        assert_eq!(record.level, "INFO");
        assert_eq!(record.fields.get("ops_kind").unwrap(), "sink_recovered");
        assert_eq!(record.fields.get("ops_sink").unwrap(), "database");
        let json: Value = serde_json::from_str(&record.message).unwrap();
        assert_eq!(json["detail"]["attempts"], 2);
    }

    #[test]
    fn test_ops_event_warn_level_for_degraded_kinds() {
        let event = InklogOpsEvent::now("sink_degraded", Some("file"), Value::Null);
        assert_eq!(event.to_log_record().level, "WARN");
        let event = InklogOpsEvent::now("config_loaded", None, Value::Null);
        assert_eq!(event.to_log_record().level, "INFO");
    }
}
