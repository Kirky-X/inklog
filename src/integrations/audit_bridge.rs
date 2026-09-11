// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T503：dbnexus `AuditStorage` 端口的 inklog 适配器（feature `dbnexus-audit`）。
//!
//! 实现下层 dbnexus 定义的 [`AuditStorage`] 端口：把 dbnexus 审计事件转换为
//! 结构化 [`LogRecord`]（`target = "audit::<entity_type>"`、`message` = 事件
//! JSON、关键字段入 `fields`），经 inklog `Database`（与 DB sink 落库同一条
//! 管道）写入，使 dbnexus 审计事件出现在 inklog DB sink 写入流中。
//!
//! 写入桥为单向（write-only）：查询/清理请消费方直接检索 inklog 日志表，
//! 或改用 dbnexus 自带的 `DbAuditStorage`。

use std::sync::Arc;

use async_trait::async_trait;
use dbnexus::{AuditEvent, AuditQueryFilters, AuditStorage, AuditSeverity};

use crate::integrations::Database;
use crate::{InklogError, LogRecord};

/// dbnexus 审计事件 → inklog 结构化日志 落库桥。
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
/// use inklog::integrations::InklogAuditStorage;
///
/// let storage = InklogAuditStorage::new(database_adapter);
/// dbnexus_audit_logger.set_storage(Arc::new(storage));
/// ```
pub struct InklogAuditStorage {
    db: Arc<dyn Database>,
}

impl InklogAuditStorage {
    /// 以注入的 inklog `Database` 实现（如 `DbNexusAdapter`）创建审计桥。
    pub fn new(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    /// 审计事件严重级别 → inklog 日志级别字符串。
    fn severity_to_level(severity: &AuditSeverity) -> &'static str {
        match severity {
            AuditSeverity::Info | AuditSeverity::Low => "INFO",
            AuditSeverity::Medium => "WARN",
            AuditSeverity::High => "ERROR",
            AuditSeverity::Critical => "FATAL",
        }
    }

    /// AuditEvent → LogRecord 映射：message 携带完整事件 JSON，关键字段平铺
    /// 进 `fields` 便于数据库侧按列检索。
    pub fn to_log_record(event: &AuditEvent) -> LogRecord {
        let mut record = LogRecord::new(
            tracing::Level::INFO,
            format!("audit::{}", event.entity_type),
            event.to_json().unwrap_or_else(|_| "{}".to_string()),
        );
        record.level = Self::severity_to_level(&event.severity).to_string();
        record.timestamp = event.timestamp;
        record.fields.insert(
            "operation".to_string(),
            serde_json::to_value(&event.operation).unwrap_or(serde_json::Value::Null),
        );
        record.fields.insert(
            "user_id".to_string(),
            serde_json::Value::String(event.user_id.clone()),
        );
        record.fields.insert(
            "entity_id".to_string(),
            serde_json::Value::String(event.entity_id.clone()),
        );
        record
            .fields
            .insert("result".to_string(), serde_json::to_value(&event.result).unwrap_or(serde_json::Value::Null));
        record.fields.insert(
            "request_id".to_string(),
            serde_json::Value::String(event.request_id.clone()),
        );
        if let Some(error) = &event.error_message {
            record.fields.insert(
                "error_message".to_string(),
                serde_json::Value::String(error.clone()),
            );
        }
        record
    }

    /// 查询/清理不可用的统一错误（写入桥语义）。
    fn write_only_error(op: &str) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(InklogError::ConfigError(format!(
            "InklogAuditStorage is a write-only bridge; '{op}' is not supported. \
             Query the inklog log table directly or use dbnexus DbAuditStorage."
        )))
    }
}

#[async_trait]
impl AuditStorage for InklogAuditStorage {
    /// 审计事件 → 结构化日志 → inklog `Database::insert_batch` 落库。
    async fn store(
        &self,
        event: &AuditEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let record = Self::to_log_record(event);
        self.db.insert_batch(std::slice::from_ref(&record)).await?;
        Ok(())
    }

    /// 写入桥不支持查询（MVP）：返回带指引的错误。
    async fn query(
        &self,
        _filters: &AuditQueryFilters,
    ) -> Result<Vec<AuditEvent>, Box<dyn std::error::Error + Send + Sync>> {
        Err(Self::write_only_error("query"))
    }

    /// 写入桥不支持清理（MVP）：返回带指引的错误。
    async fn cleanup(
        &self,
        _before: &chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Err(Self::write_only_error("cleanup"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::MockDatabaseAdapter;
    use dbnexus::{AuditOperation, AuditStatus};

    fn sample_event() -> AuditEvent {
        AuditEvent::update(
            "users",
            "42",
            "admin",
            Some(r#"{"a":1}"#.to_string()),
            Some(r#"{"a":2}"#.to_string()),
        )
        .with_user("root", "10.0.0.1")
        .with_result(AuditStatus::Failure)
        .with_severity(AuditSeverity::High)
        .with_request_id("req-1")
    }

    fn sample_event_with_error() -> AuditEvent {
        let mut e = sample_event();
        e.error_message = Some("boom".to_string());
        e
    }

    #[test]
    fn test_to_log_record_mapping() {
        let event = sample_event_with_error();
        let record = InklogAuditStorage::to_log_record(&event);
        // severity High → ERROR；target = audit::<entity_type>
        assert_eq!(record.level, "ERROR");
        assert_eq!(record.target, "audit::users");
        // message 为事件 JSON（可解析且含 entity_id）
        let json: serde_json::Value = serde_json::from_str(&record.message).unwrap();
        assert_eq!(json["entity_id"], serde_json::json!("42"));
        // 关键字段平铺
        assert_eq!(record.fields.get("user_id").unwrap(), &serde_json::json!("admin"));
        assert_eq!(record.fields.get("entity_id").unwrap(), &serde_json::json!("42"));
        assert_eq!(
            record.fields.get("operation").unwrap(),
            &serde_json::json!("Update")
        );
        assert_eq!(
            record.fields.get("error_message").unwrap(),
            &serde_json::json!("boom")
        );
        // 事件时间戳透传
        assert_eq!(record.timestamp, event.timestamp);
    }

    #[test]
    fn test_severity_to_level_mapping() {
        assert_eq!(
            InklogAuditStorage::severity_to_level(&AuditSeverity::Info),
            "INFO"
        );
        assert_eq!(
            InklogAuditStorage::severity_to_level(&AuditSeverity::Medium),
            "WARN"
        );
        assert_eq!(
            InklogAuditStorage::severity_to_level(&AuditSeverity::Critical),
            "FATAL"
        );
    }

    #[tokio::test]
    async fn test_store_lands_audit_event_in_inklog_db_stream() {
        let mock = Arc::new(MockDatabaseAdapter::new());
        let storage = InklogAuditStorage::new(mock.clone() as Arc<dyn Database>);

        storage.store(&sample_event()).await.expect("store must succeed");

        let records = mock.get_records();
        assert_eq!(records.len(), 1, "audit event must appear in the inklog write stream");
        assert_eq!(records[0].target, "audit::users");
        assert!(records[0].message.contains("entity_id"));
    }

    #[tokio::test]
    async fn test_query_and_cleanup_are_write_only() {
        let storage = InklogAuditStorage::new(Arc::new(MockDatabaseAdapter::new()));
        let err = storage
            .query(&AuditQueryFilters::default())
            .await
            .expect_err("query must be unsupported");
        assert!(err.to_string().contains("write-only"));
        let err = storage
            .cleanup(&chrono::Utc::now())
            .await
            .expect_err("cleanup must be unsupported");
        assert!(err.to_string().contains("write-only"));
    }

    // AuditOperation 序列化（Other(String) 形态兜底）
    #[test]
    fn test_operation_serialization_covers_custom_variant() {
        let mut event = sample_event();
        event.operation = AuditOperation::Other("purge".to_string());
        event.error_message = Some("boom".to_string());
        let record = InklogAuditStorage::to_log_record(&event);
        // 枚举带 payload 的变体按 serde 外部标签序列化为 {"Other": "..."}
        assert_eq!(
            record.fields.get("operation").unwrap(),
            &serde_json::json!({"Other": "purge"})
        );
    }
}
