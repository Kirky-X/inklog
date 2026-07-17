// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 数据库实体单元测试

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
mod entity_tests {
    use inklog::chrono::{DateTime, Utc};
    use inklog::sink::entity::{ActiveModel, Column, Entity, Model, TABLE_NAME};
    use sea_orm::entity::prelude::*;

    // ========================================================================
    // Table Name and Column Tests
    // ========================================================================

    #[test]
    fn test_table_name_constant() {
        assert_eq!(TABLE_NAME, "logs");
    }

    #[test]
    fn test_entity_table_name() {
        assert_eq!(Entity.table_name(), "logs");
    }

    #[test]
    fn test_column_enum_has_all_expected_variants() {
        // Verify all expected column variants exist
        let _ = Column::Id;
        let _ = Column::Timestamp;
        let _ = Column::Level;
        let _ = Column::Target;
        let _ = Column::Message;
        let _ = Column::Fields;
        let _ = Column::File;
        let _ = Column::Line;
        let _ = Column::ThreadId;
        let _ = Column::ModulePath;
        let _ = Column::Metadata;
    }

    // ========================================================================
    // Model Serialization Tests
    // ========================================================================

    #[test]
    fn test_model_json_serialization_includes_required_fields() {
        let model = Model {
            id: 42,
            timestamp: DateTime::<Utc>::from_timestamp(1700000000, 0)
                .unwrap()
                .naive_utc(),
            level: "INFO".to_string(),
            target: "test_target".to_string(),
            message: "Test message".to_string(),
            fields: None,
            file: None,
            line: None,
            thread_id: "thread_1".to_string(),
            module_path: None,
            metadata: None,
        };

        let json = serde_json::to_string(&model).expect("Should serialize to JSON");
        let parsed: serde_json::Value = json.parse().expect("Should parse as JSON");

        assert!(parsed.get("id").is_some(), "JSON should contain id");
        assert!(
            parsed.get("timestamp").is_some(),
            "JSON should contain timestamp"
        );
        assert!(parsed.get("level").is_some(), "JSON should contain level");
        assert!(parsed.get("target").is_some(), "JSON should contain target");
        assert!(
            parsed.get("message").is_some(),
            "JSON should contain message"
        );
        assert!(
            parsed.get("thread_id").is_some(),
            "JSON should contain thread_id"
        );
    }

    #[test]
    fn test_model_json_serialization_optional_fields_null_when_none() {
        let model = Model {
            id: 1,
            timestamp: DateTime::<Utc>::from_timestamp(0, 0).unwrap().naive_utc(),
            level: "WARN".to_string(),
            target: "opt_test".to_string(),
            message: "Optional fields test".to_string(),
            fields: None,
            file: None,
            line: None,
            thread_id: "main".to_string(),
            module_path: None,
            metadata: None,
        };

        let json = serde_json::to_string(&model).expect("Should serialize");
        let parsed: serde_json::Value = json.parse().expect("Should parse as JSON");

        // Optional fields should be present as null when None
        assert!(
            parsed.get("fields").unwrap().is_null(),
            "fields should be null when None"
        );
        assert!(
            parsed.get("file").unwrap().is_null(),
            "file should be null when None"
        );
        assert!(
            parsed.get("line").unwrap().is_null(),
            "line should be null when None"
        );
        assert!(
            parsed.get("module_path").unwrap().is_null(),
            "module_path should be null when None"
        );
        assert!(
            parsed.get("metadata").unwrap().is_null(),
            "metadata should be null when None"
        );
    }

    #[test]
    fn test_model_json_serialization_with_populated_optional_fields() {
        let model = Model {
            id: 99,
            timestamp: DateTime::<Utc>::from_timestamp(1700000000, 0)
                .unwrap()
                .naive_utc(),
            level: "ERROR".to_string(),
            target: "full_test".to_string(),
            message: "Full model test".to_string(),
            fields: Some(r#"{"key":"value"}"#.to_string()),
            file: Some("main.rs".to_string()),
            line: Some(42),
            thread_id: "worker-1".to_string(),
            module_path: Some("inklog::test".to_string()),
            metadata: Some(r#"{"env":"test"}"#.to_string()),
        };

        let json = serde_json::to_string(&model).expect("Should serialize");
        let parsed: serde_json::Value = json.parse().expect("Should parse");

        assert_eq!(parsed["fields"].as_str().unwrap(), r#"{"key":"value"}"#);
        assert_eq!(parsed["file"].as_str().unwrap(), "main.rs");
        assert_eq!(parsed["line"].as_i64().unwrap(), 42);
        assert_eq!(parsed["module_path"].as_str().unwrap(), "inklog::test");
        assert_eq!(parsed["metadata"].as_str().unwrap(), r#"{"env":"test"}"#);
    }

    #[test]
    fn test_model_json_deserialization() {
        let json = r#"{
            "id": 123,
            "timestamp": "2024-01-01T00:00:00",
            "level": "INFO",
            "target": "deser_test",
            "message": "Deserialized message",
            "fields": null,
            "file": null,
            "line": null,
            "thread_id": "test_thread",
            "module_path": null,
            "metadata": null
        }"#;

        let model: Model = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(model.id, 123);
        assert_eq!(model.level, "INFO");
        assert_eq!(model.target, "deser_test");
        assert_eq!(model.message, "Deserialized message");
        assert_eq!(model.thread_id, "test_thread");
        assert!(model.fields.is_none());
        assert!(model.file.is_none());
    }

    #[test]
    fn test_model_roundtrip_serialization_deserialization() {
        let original = Model {
            id: 555,
            timestamp: DateTime::<Utc>::from_timestamp(1704067200, 0)
                .unwrap()
                .naive_utc(),
            level: "DEBUG".to_string(),
            target: "roundtrip".to_string(),
            message: "Roundtrip test message".to_string(),
            fields: Some("{}".to_string()),
            file: Some("test.rs".to_string()),
            line: Some(100),
            thread_id: "rt-thread".to_string(),
            module_path: Some("inklog::roundtrip".to_string()),
            metadata: Some(r#"{"test":true}"#.to_string()),
        };

        let json = serde_json::to_string(&original).expect("Serialize");
        let deserialized: Model = serde_json::from_str(&json).expect("Deserialize");

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.level, deserialized.level);
        assert_eq!(original.target, deserialized.target);
        assert_eq!(original.message, deserialized.message);
        assert_eq!(original.thread_id, deserialized.thread_id);
    }

    // ========================================================================
    // ActiveModel Tests
    // ========================================================================

    #[test]
    fn test_active_model_from_model() {
        let model = Model {
            id: 1,
            timestamp: DateTime::<Utc>::from_timestamp(0, 0).unwrap().naive_utc(),
            level: "INFO".to_string(),
            target: "active_test".to_string(),
            message: "Active model test".to_string(),
            fields: None,
            file: Some("mod.rs".to_string()),
            line: Some(10),
            thread_id: "test".to_string(),
            module_path: None,
            metadata: None,
        };

        let active: ActiveModel = model.clone().into();

        // Verify field values were transferred via into_value()
        let id_val: Option<sea_orm::Value> = active.id.clone().into_value();
        let level_val: Option<sea_orm::Value> = active.level.clone().into_value();
        let message_val: Option<sea_orm::Value> = active.message.clone().into_value();

        assert!(id_val.is_some(), "id should be set from model");
        assert!(level_val.is_some(), "level should be set from model");
        assert!(message_val.is_some(), "message should be set from model");
    }

    #[test]
    fn test_active_model_set_column() {
        let mut active: ActiveModel = ActiveModel::new();

        // Set required fields using set()
        active.set(Column::Level, "ERROR".into());
        active.set(Column::Target, "set_test".into());
        active.set(Column::Message, "Set column test".into());
        active.set(Column::ThreadId, "main".into());

        assert_eq!(
            active.get(Column::Level).unwrap(),
            sea_orm::Value::String(Some("ERROR".to_string()))
        );
        assert_eq!(
            active.get(Column::Target).unwrap(),
            sea_orm::Value::String(Some("set_test".to_string()))
        );
        assert_eq!(
            active.get(Column::Message).unwrap(),
            sea_orm::Value::String(Some("Set column test".to_string()))
        );
    }

    #[test]
    fn test_active_model_new() {
        let active = ActiveModel::new();
        // New ActiveModel: id field should not be set
        assert!(!active.id.is_set());
    }

    // ========================================================================
    // Thread Safety Tests (Compile-Time)
    // ========================================================================

    #[test]
    fn test_model_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Model>();
    }

    #[test]
    fn test_model_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Model>();
    }

    #[test]
    fn test_active_model_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ActiveModel>();
    }

    #[test]
    fn test_active_model_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<ActiveModel>();
    }

    #[test]
    fn test_column_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Column>();
    }

    #[test]
    fn test_column_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Column>();
    }

    #[test]
    fn test_entity_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Entity>();
    }

    #[test]
    fn test_entity_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Entity>();
    }

    // ========================================================================
    // Model Clone and Debug
    // ========================================================================

    #[test]
    fn test_model_clone() {
        let model = Model {
            id: 1,
            timestamp: DateTime::<Utc>::from_timestamp(0, 0).unwrap().naive_utc(),
            level: "INFO".to_string(),
            target: "clone".to_string(),
            message: "Clone test".to_string(),
            fields: None,
            file: None,
            line: None,
            thread_id: "main".to_string(),
            module_path: None,
            metadata: None,
        };
        let cloned = model.clone();
        assert_eq!(model.id, cloned.id);
        assert_eq!(model.message, cloned.message);
    }

    #[test]
    fn test_model_debug() {
        let model = Model {
            id: 1,
            timestamp: DateTime::<Utc>::from_timestamp(0, 0).unwrap().naive_utc(),
            level: "INFO".to_string(),
            target: "debug".to_string(),
            message: "Debug test".to_string(),
            fields: None,
            file: None,
            line: None,
            thread_id: "main".to_string(),
            module_path: None,
            metadata: None,
        };
        let debug_str = format!("{:?}", model);
        assert!(!debug_str.is_empty());
        // Debug should not panic and should produce some output
    }
}
