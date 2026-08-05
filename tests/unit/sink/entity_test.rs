// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 数据库实体单元测试
//!
//! 自 0.2.0 起，entity 模块仅导出 `TABLE_NAME` 常量（ORM 已替换为 dbnexus 原生 SQL）。

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
mod entity_tests {
    use inklog::sink::entity::TABLE_NAME;

    #[test]
    fn test_table_name_constant() {
        assert_eq!(TABLE_NAME, "logs");
    }

    #[test]
    fn test_table_name_is_non_empty() {
        assert!(!TABLE_NAME.is_empty());
    }

    #[test]
    fn test_table_name_is_valid_identifier() {
        // 表名必须是合法 SQL 标识：仅含字母数字和下划线，且以字母或下划线开头
        assert!(
            TABLE_NAME
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "TABLE_NAME should only contain alphanumeric chars and underscores"
        );
        assert!(
            TABLE_NAME
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic() || c == '_')
                .unwrap_or(false),
            "TABLE_NAME should start with a letter or underscore"
        );
    }
}
