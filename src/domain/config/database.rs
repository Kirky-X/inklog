// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Database sink configuration types.

use serde::{Deserialize, Serialize};

// ============================================================================
// DatabaseDriver - Supported database drivers
// ============================================================================

/// Supported database drivers for the database sink.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[serde(rename = "postgres")]
    #[default]
    PostgreSQL,
    #[serde(rename = "mysql")]
    MySQL,
    #[serde(rename = "sqlite")]
    SQLite,
}

impl std::str::FromStr for DatabaseDriver {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(DatabaseDriver::PostgreSQL),
            "mysql" => Ok(DatabaseDriver::MySQL),
            "sqlite" | "sqlite3" => Ok(DatabaseDriver::SQLite),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for DatabaseDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseDriver::PostgreSQL => write!(f, "postgres"),
            DatabaseDriver::MySQL => write!(f, "mysql"),
            DatabaseDriver::SQLite => write!(f, "sqlite"),
        }
    }
}

// ============================================================================
// PartitionStrategy - Database table partitioning strategy
// ============================================================================

/// Database table partitioning strategy for log storage optimization.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PartitionStrategy {
    #[serde(rename = "monthly")]
    #[default]
    Monthly,
    #[serde(rename = "yearly")]
    Yearly,
}

impl std::str::FromStr for PartitionStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "monthly" | "month" => Ok(PartitionStrategy::Monthly),
            "yearly" | "year" => Ok(PartitionStrategy::Yearly),
            _ => Err(format!("Unknown partition strategy: {}", s)),
        }
    }
}

impl std::fmt::Display for PartitionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartitionStrategy::Monthly => write!(f, "monthly"),
            PartitionStrategy::Yearly => write!(f, "yearly"),
        }
    }
}

// ============================================================================
// ParquetConfig - Parquet export configuration
// ============================================================================

/// Parquet export configuration for database sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetConfig {
    #[serde(default = "default_parquet_compression_level")]
    pub compression_level: i32,
    #[serde(default = "default_parquet_encoding")]
    pub encoding: String,
    #[serde(default = "default_parquet_max_row_group_size")]
    pub max_row_group_size: usize,
    #[serde(default = "default_parquet_max_page_size")]
    pub max_page_size: usize,
    #[serde(default)]
    pub include_fields: Vec<String>,
}

fn default_parquet_compression_level() -> i32 {
    3
}
fn default_parquet_encoding() -> String {
    "PLAIN".to_string()
}
fn default_parquet_max_row_group_size() -> usize {
    10000
}
fn default_parquet_max_page_size() -> usize {
    1048576
}

impl Default for ParquetConfig {
    fn default() -> Self {
        Self {
            compression_level: default_parquet_compression_level(),
            encoding: default_parquet_encoding(),
            max_row_group_size: default_parquet_max_row_group_size(),
            max_page_size: default_parquet_max_page_size(),
            include_fields: Vec::new(),
        }
    }
}

// ============================================================================
// DatabaseSinkConfig - Database sink settings
// ============================================================================

/// Database sink configuration for persistent log storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSinkConfig {
    #[serde(default = "default_db_sink_name")]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub driver: DatabaseDriver,
    #[serde(default = "default_db_url")]
    pub url: String,
    #[serde(default = "default_db_pool_size")]
    pub pool_size: u32,
    #[serde(default = "default_db_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_db_flush_interval_ms")]
    pub flush_interval_ms: u64,
    #[serde(default)]
    pub partition: PartitionStrategy,
    #[serde(default = "default_db_table_name")]
    pub table_name: String,
    #[serde(default = "default_db_archive_format")]
    pub archive_format: String,
    #[serde(default)]
    pub parquet_config: ParquetConfig,
}

fn default_db_sink_name() -> String {
    "default".to_string()
}
fn default_db_url() -> String {
    "sqlite::memory:".to_string()
}
fn default_db_pool_size() -> u32 {
    10
}
fn default_db_batch_size() -> usize {
    100
}
fn default_db_flush_interval_ms() -> u64 {
    500
}
fn default_db_table_name() -> String {
    "logs".to_string()
}
fn default_db_archive_format() -> String {
    "json".to_string()
}

impl Default for DatabaseSinkConfig {
    fn default() -> Self {
        Self {
            name: default_db_sink_name(),
            enabled: false,
            driver: DatabaseDriver::default(),
            url: default_db_url(),
            pool_size: default_db_pool_size(),
            batch_size: default_db_batch_size(),
            flush_interval_ms: default_db_flush_interval_ms(),
            partition: PartitionStrategy::default(),
            table_name: default_db_table_name(),
            archive_format: default_db_archive_format(),
            parquet_config: ParquetConfig::default(),
        }
    }
}
