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
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("postgres") || s.eq_ignore_ascii_case("postgresql") {
            Ok(DatabaseDriver::PostgreSQL)
        } else if s.eq_ignore_ascii_case("mysql") {
            Ok(DatabaseDriver::MySQL)
        } else if s.eq_ignore_ascii_case("sqlite") || s.eq_ignore_ascii_case("sqlite3") {
            Ok(DatabaseDriver::SQLite)
        } else {
            Err(format!(
                "Unknown database driver '{}'. Valid drivers: postgres, mysql, sqlite",
                s
            ))
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
        if s.eq_ignore_ascii_case("monthly") || s.eq_ignore_ascii_case("month") {
            Ok(PartitionStrategy::Monthly)
        } else if s.eq_ignore_ascii_case("yearly") || s.eq_ignore_ascii_case("year") {
            Ok(PartitionStrategy::Yearly)
        } else {
            Err(format!("Unknown partition strategy: {}", s))
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
#[serde(deny_unknown_fields)]
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
// ArchiveFormat - Database archive export format
// ============================================================================

/// Supported archive formats for database log export.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveFormat {
    #[default]
    Json,
    Parquet,
    Csv,
}

impl std::str::FromStr for ArchiveFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Use eq_ignore_ascii_case to avoid heap allocation from to_lowercase()
        if s.eq_ignore_ascii_case("json") {
            Ok(ArchiveFormat::Json)
        } else if s.eq_ignore_ascii_case("parquet") {
            Ok(ArchiveFormat::Parquet)
        } else if s.eq_ignore_ascii_case("csv") {
            Ok(ArchiveFormat::Csv)
        } else {
            Err(format!(
                "Unknown archive format: '{}'. Valid: json, parquet, csv",
                s
            ))
        }
    }
}

impl std::fmt::Display for ArchiveFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveFormat::Json => write!(f, "json"),
            ArchiveFormat::Parquet => write!(f, "parquet"),
            ArchiveFormat::Csv => write!(f, "csv"),
        }
    }
}

// ============================================================================
// DatabaseSinkConfig - Database sink settings
// ============================================================================

/// Database sink configuration for persistent log storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    pub archive_format: ArchiveFormat,
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
// ArchiveFormat default is Json via #[default] derive

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
            archive_format: ArchiveFormat::default(),
            parquet_config: ParquetConfig::default(),
        }
    }
}

impl DatabaseSinkConfig {
    /// Validate and adjust the configuration.
    ///
    /// - When using SQLite, `pool_size` is overridden to 1 since SQLite
    ///   only supports a single writer connection.
    /// - Rejects zero `batch_size` and `flush_interval_ms` to prevent
    ///   busy-loops or panics at runtime.
    /// - Validates `compression_level` is within the valid zstd range (1–22).
    pub fn validate(&mut self) {
        if self.driver == DatabaseDriver::SQLite && self.pool_size != 1 {
            self.pool_size = 1;
        }
        if self.batch_size == 0 {
            tracing::warn!("database_sink.batch_size is 0, resetting to default 100");
            self.batch_size = 100;
        }
        if self.flush_interval_ms == 0 {
            tracing::warn!("database_sink.flush_interval_ms is 0, resetting to default 500");
            self.flush_interval_ms = 500;
        }
        // Clamp Parquet compression level to valid zstd range 1–22
        if !(1..=22).contains(&self.parquet_config.compression_level) {
            tracing::warn!(
                level = self.parquet_config.compression_level,
                "parquet_config.compression_level out of range 1-22, clamping to 3"
            );
            self.parquet_config.compression_level = 3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_pool_size_defaults_to_one() {
        let mut config = DatabaseSinkConfig::default();
        config.driver = DatabaseDriver::SQLite;
        config.pool_size = 10;
        config.validate();
        assert_eq!(config.pool_size, 1, "SQLite pool_size must be 1");
    }

    #[test]
    fn test_non_sqlite_pool_size_unchanged() {
        let mut config = DatabaseSinkConfig::default();
        config.driver = DatabaseDriver::PostgreSQL;
        config.pool_size = 10;
        config.validate();
        assert_eq!(
            config.pool_size, 10,
            "non-SQLite pool_size should be unchanged"
        );
    }
}
