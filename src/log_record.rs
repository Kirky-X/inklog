use crate::masking::DataMasker;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{Event, Level};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, Value>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub thread_id: String,
}

impl Default for LogRecord {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: String::new(),
            message: String::new(),
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: String::new(),
        }
    }
}

impl LogRecord {
    pub fn reset(&mut self) {
        self.timestamp = Utc::now();
        self.level.clear();
        self.level.push_str("INFO");
        self.target.clear();
        self.message.clear();
        self.fields.clear();
        self.file = None;
        self.line = None;
        self.thread_id.clear();
    }
    pub fn new(level: Level, target: String, message: String) -> Self {
        Self {
            timestamp: Utc::now(),
            level: level.to_string(),
            target,
            message,
            fields: HashMap::new(),
            file: None,
            line: None,
            thread_id: format!("{:?}", std::thread::current().id()),
        }
    }

    pub fn from_event(event: &Event) -> Self {
        use crate::pool::{LOG_RECORD_POOL, STRING_POOL};

        let mut record = LOG_RECORD_POOL.get();
        record.reset();

        let mut fields = HashMap::with_capacity(4);
        let mut message = STRING_POOL.get();
        message.clear();

        let mut visitor = LogVisitor {
            fields: &mut fields,
            message: &mut message,
        };
        event.record(&mut visitor);

        let metadata = event.metadata();

        record.level.clear();
        record.level.push_str(metadata.level().as_str());
        record.target.clear();
        record.target.push_str(metadata.target());
        record.message = message;
        record.fields = fields;
        record.file = metadata.file().map(|s| s.to_string());
        record.line = metadata.line();
        record.thread_id = format!("{:?}", std::thread::current().id());

        record.mask_sensitive_fields();
        record
    }

    fn mask_sensitive_fields(&mut self) {
        let masker = DataMasker::new();
        self.message = masker.mask(&self.message);
        for (_, v) in self.fields.iter_mut() {
            masker.mask_value(v);
        }
        let sensitive_keys = ["password", "token", "secret", "key", "credential", "auth"];
        for (k, v) in self.fields.iter_mut() {
            for sensitive in sensitive_keys {
                if k.to_lowercase().contains(sensitive) {
                    *v = Value::String("***MASKED***".to_string());
                    break;
                }
            }
        }
    }
}

struct LogVisitor<'a> {
    fields: &'a mut HashMap<String, Value>,
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for LogVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        if name == "message" {
            *self.message = format!("{:?}", value);
        } else {
            self.fields
                .insert(name.to_string(), Value::String(format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let name = field.name();
        if name == "message" {
            *self.message = value.to_string();
        } else {
            self.fields
                .insert(name.to_string(), Value::String(value.to_string()));
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), Value::Number(n));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_sensitive_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "password".to_string(),
            Value::String("secret123".to_string()),
        );
        record
            .fields
            .insert("api_key".to_string(), Value::String("abcdef".to_string()));
        record
            .fields
            .insert("username".to_string(), Value::String("user".to_string()));

        record.mask_sensitive_fields();

        assert_eq!(
            record.fields.get("password").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
        assert_eq!(
            record.fields.get("api_key").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
        assert_eq!(
            record.fields.get("username").unwrap(),
            &Value::String("user".to_string())
        );
    }

    #[test]
    fn test_mask_email_in_message() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "Contact: user@example.com".to_string(),
        );
        record.mask_sensitive_fields();
        assert_eq!(record.message, "Contact: **@**.***");
    }

    #[test]
    fn test_mask_phone_in_message() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "Call: 13812345678".to_string(),
        );
        record.mask_sensitive_fields();
        assert_eq!(record.message, "Call: ***-****-****");
    }

    #[test]
    fn test_mask_id_card_in_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "id_card".to_string(),
            Value::String("110101199001011234".to_string()),
        );
        record.mask_sensitive_fields();
        assert_eq!(
            record.fields.get("id_card").unwrap(),
            &Value::String("**************1234".to_string())
        );
    }

    #[test]
    fn test_mask_bank_card_in_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "card_number".to_string(),
            Value::String("6222021234567890123".to_string()),
        );
        record.mask_sensitive_fields();
        assert_eq!(
            record.fields.get("card_number").unwrap(),
            &Value::String("****-****-****-0123".to_string())
        );
    }

    #[test]
    fn test_mask_nested_json_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "user_info".to_string(),
            Value::Object(serde_json::from_str(r#"{"email":"admin@test.com"}"#).unwrap()),
        );
        record.mask_sensitive_fields();
        let user_info = record.fields.get("user_info").unwrap();
        assert_eq!(user_info["email"], Value::String("**@**.***".to_string()));
    }

    #[test]
    fn test_mask_array_fields() {
        let mut record = LogRecord::new(Level::INFO, "test".to_string(), "message".to_string());
        record.fields.insert(
            "contacts".to_string(),
            Value::Array(vec![
                Value::String("test@email.com".to_string()),
                Value::String("13912345678".to_string()),
            ]),
        );
        record.mask_sensitive_fields();
        let contacts = record.fields.get("contacts").unwrap().as_array().unwrap();
        assert_eq!(contacts[0], Value::String("**@**.***".to_string()));
        assert_eq!(contacts[1], Value::String("***-****-****".to_string()));
    }

    #[test]
    fn test_combined_masking() {
        let mut record = LogRecord::new(
            Level::INFO,
            "test".to_string(),
            "User test@example.com called 13812345678".to_string(),
        );
        record.fields.insert(
            "id_card".to_string(),
            Value::String("110101199001011234".to_string()),
        );
        record.fields.insert(
            "password".to_string(),
            Value::String("mypass123".to_string()),
        );

        record.mask_sensitive_fields();

        assert_eq!(record.message, "User **@**.*** called ***-****-****");
        assert_eq!(
            record.fields.get("id_card").unwrap(),
            &Value::String("**************1234".to_string())
        );
        assert_eq!(
            record.fields.get("password").unwrap(),
            &Value::String("***MASKED***".to_string())
        );
    }
}
