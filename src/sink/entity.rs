//! Database entity definitions for inklog database sink.
//!
//! This module provides entity definitions for database logging.

use sea_orm::entity::prelude::*;

/// The main log entity for database storage
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(column_type = "TimestampWithTimeZone", enum_name = "Timestamp")]
    pub timestamp: chrono::NaiveDateTime,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Option<String>,
    pub file: Option<String>,
    pub line: Option<i32>,
    pub thread_id: String,
    pub module_path: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Table name constant for use in queries
pub const TABLE_NAME: &str = "logs";
