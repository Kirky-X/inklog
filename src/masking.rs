use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct DataMasker {
    rules: Vec<MaskRule>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MaskRule {
    name: String,
    pattern: Regex,
    replacement: String,
    replace_count: usize,
}

impl DataMasker {
    pub fn new() -> Self {
        let rules = vec![
            MaskRule::new_email_rule(),
            MaskRule::new_phone_rule(),
            MaskRule::new_id_card_rule(),
            MaskRule::new_bank_card_rule(),
        ];

        Self { rules }
    }

    pub fn mask(&self, text: &str) -> String {
        let mut result = text.to_string();
        for rule in &self.rules {
            result = rule.apply(&result);
        }
        result
    }

    pub fn mask_value(&self, value: &mut Value) {
        match value {
            Value::String(s) => {
                *s = self.mask(s);
            }
            Value::Array(arr) => {
                for item in arr {
                    self.mask_value(item);
                }
            }
            Value::Object(map) => {
                for (_, v) in map {
                    self.mask_value(v);
                }
            }
            _ => {}
        }
    }

    pub fn mask_hashmap(&self, map: &mut HashMap<String, Value>) {
        for (_, v) in map.iter_mut() {
            self.mask_value(v);
        }
    }
}

impl MaskRule {
    fn new_email_rule() -> Self {
        Self {
            name: "email".to_string(),
            pattern: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+").unwrap(),
            replacement: "**@**.***".to_string(),
            replace_count: 1,
        }
    }

    fn new_phone_rule() -> Self {
        Self {
            name: "phone".to_string(),
            pattern: Regex::new(r"\b1[3-9]\d{9}\b").unwrap(),
            replacement: "***-****-****".to_string(),
            replace_count: 1,
        }
    }

    fn new_id_card_rule() -> Self {
        Self {
            name: "id_card".to_string(),
            pattern: Regex::new(r"\b\d{14}(\d{4})\b").unwrap(),
            replacement: "**************$1".to_string(),
            replace_count: 1,
        }
    }

    fn new_bank_card_rule() -> Self {
        Self {
            name: "bank_card".to_string(),
            pattern: Regex::new(r"\b\d+(\d{4})\b").unwrap(),
            replacement: "****-****-****-$1".to_string(),
            replace_count: 1,
        }
    }

    fn apply(&self, text: &str) -> String {
        self.pattern
            .replace(text, self.replacement.as_str())
            .to_string()
    }
}

pub fn mask_email(email: &str) -> String {
    let pattern = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+").unwrap();
    pattern.replace(email, "**@**.***").to_string()
}

pub fn mask_phone(phone: &str) -> String {
    let pattern = Regex::new(r"\b1[3-9]\d{9}\b").unwrap();
    pattern.replace(phone, "***-****-****").to_string()
}

pub fn mask_id_card(id_card: &str) -> String {
    let pattern = Regex::new(r"\b\d{14}(\d{3}\w)\b").unwrap();
    pattern.replace(id_card, "**************$1").to_string()
}

pub fn mask_bank_card(bank_card: &str) -> String {
    let pattern = Regex::new(r"\b\d+(\d{4})\b").unwrap();
    pattern.replace(bank_card, "****-****-****-$1").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email() {
        let test_cases = vec![
            ("test@example.com", "**@**.***"),
            ("user.name@company.co.uk", "**@**.***"),
            ("admin@localhost", "**@**.***"),
        ];

        for (input, expected) in test_cases {
            let result = mask_email(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_phone() {
        let test_cases = vec![
            ("13812345678", "***-****-****"),
            ("15987654321", "***-****-****"),
            ("Contact: 18655556666 now", "Contact: ***-****-**** now"),
        ];

        for (input, expected) in test_cases {
            let result = mask_phone(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_id_card() {
        let test_cases = vec![
            ("110101199001011234", "**************1234"),
            ("31011519880530218X", "**************218X"),
        ];

        for (input, expected) in test_cases {
            let result = mask_id_card(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_mask_bank_card() {
        let test_cases = vec![
            ("6222021234567890123", "****-****-****-0123"),
            ("4567890123456789", "****-****-****-6789"),
        ];

        for (input, expected) in test_cases {
            let result = mask_bank_card(input);
            assert_eq!(result, expected, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_data_masker() {
        let masker = DataMasker::new();

        let test_email = "user@example.com";
        assert_eq!(masker.mask(test_email), "**@**.***");

        let test_phone = "13912345678";
        assert_eq!(masker.mask(test_phone), "***-****-****");

        let mixed = "Contact user at test@example.com, phone: 13812345678";
        let result = masker.mask(mixed);
        assert!(!result.contains("test@example.com"));
        assert!(!result.contains("13812345678"));
    }

    #[test]
    fn test_mask_value() {
        let masker = DataMasker::new();

        let mut value = serde_json::json!({
            "email": "user@example.com",
            "phone": "13712345678",
            "name": "John"
        });

        masker.mask_value(&mut value);

        assert_eq!(value["email"], "**@**.***");
        assert_eq!(value["phone"], "***-****-****");
        assert_eq!(value["name"], "John");
    }

    #[test]
    fn test_mask_nested_value() {
        let masker = DataMasker::new();

        let mut value = serde_json::json!({
            "user": {
                "email": "admin@company.org",
                "contacts": ["test@email.com", "13811112222"]
            }
        });

        masker.mask_value(&mut value);

        let user = &value["user"];
        assert_eq!(user["email"], "**@**.***");

        let contacts = user["contacts"].as_array().unwrap();
        assert_eq!(contacts[0], "**@**.***");
        assert_eq!(contacts[1], "***-****-****");
    }
}
