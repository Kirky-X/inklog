// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Database sink configuration types.

use serde::{Deserialize, Serialize};

// DatabaseDriver - Supported database drivers

/// Supported database drivers for the database sink.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    #[serde(rename = "postgres")]
    PostgreSQL,
    #[serde(rename = "mysql")]
    MySQL,
    #[serde(rename = "sqlite")]
    #[default]
    SQLite,
    #[serde(rename = "duckdb")]
    DuckDB,
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
        } else if s.eq_ignore_ascii_case("duckdb") {
            Ok(DatabaseDriver::DuckDB)
        } else {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("driver", s);
            args.set("valid", "postgres, mysql, sqlite, duckdb");
            Err(crate::i18n::tr_args("config-unknown_db_driver", args))
        }
    }
}

impl std::fmt::Display for DatabaseDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseDriver::PostgreSQL => write!(f, "postgres"),
            DatabaseDriver::MySQL => write!(f, "mysql"),
            DatabaseDriver::SQLite => write!(f, "sqlite"),
            DatabaseDriver::DuckDB => write!(f, "duckdb"),
        }
    }
}

// PartitionStrategy - Database table partitioning strategy

/// Database table partitioning strategy for log storage optimization.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
            let mut args = crate::i18n::MsgArgs::new();
            args.set("strategy", s);
            Err(crate::i18n::tr_args(
                "config-unknown_partition_strategy",
                args,
            ))
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

// ParquetConfig - Parquet export configuration

/// Parquet export configuration for database sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

// ArchiveFormat - Database archive export format

/// Supported archive formats for database log export.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
        // Use eq_ignore_ascii_case to avoid heap allocation from to_lowercase
        if s.eq_ignore_ascii_case("json") {
            Ok(ArchiveFormat::Json)
        } else if s.eq_ignore_ascii_case("parquet") {
            Ok(ArchiveFormat::Parquet)
        } else if s.eq_ignore_ascii_case("csv") {
            Ok(ArchiveFormat::Csv)
        } else {
            let mut args = crate::i18n::MsgArgs::new();
            args.set("format", s);
            args.set("valid", "json, parquet, csv");
            Err(crate::i18n::tr_args("config-unknown_archive_format", args))
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

// DatabaseSinkConfig - Database sink settings

/// Database sink configuration for persistent log storage.
///
/// `Debug` 与 `Serialize` 均遮蔽 DSN 中的口令：配置对象常被整体打印
/// （`tracing::info!("{:?}", config)`）或整体回显（管理接口 JSON）——
/// 派生实现会把内嵌凭据（`postgres://user:password@host/db`）一并带出。
///
/// 序列化产物是**脱敏视图**，不是可回载的配置——序列化→反序列化往返会丢
/// 口令。需要保真往返（备份、迁移、跨环境搬运）用
/// [`to_lossless_json`](Self::to_lossless_json) /
/// [`to_lossless_toml`](Self::to_lossless_toml)。
///
/// 口令位是脱敏占位符（`***`）的 DSN 无法认证，[`validate`](Self::validate)
/// 显式拒绝。
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DatabaseSinkConfig {
    #[serde(default = "default_db_sink_name")]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub driver: DatabaseDriver,
    #[serde(default = "default_db_url", serialize_with = "serialize_redacted_dsn")]
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
    /// Path to the RBAC permissions configuration file.
    /// When `None`, permission checking is disabled.
    #[serde(default)]
    pub permissions_path: Option<String>,
    /// Admin role name used for DDL and write operations.
    #[serde(default = "default_admin_role")]
    pub admin_role: String,
    /// Enable the secret value-pattern scan (feature `secret-scan`) before
    /// records are persisted: bare API keys (`sk-` families, AWS AKIA, PEM
    /// blocks, …) are attributed and masked by the outbound gate even without
    /// a sensitive field name.
    ///
    /// Requires the `secret-scan` feature — without it the flag is ignored
    /// (a warning is emitted during validation).
    ///
    /// # Default
    ///
    /// `false` - keeps the pre-existing persisting behavior byte-identical.
    #[serde(default)]
    pub secret_scan_enabled: bool,
}

/// `Debug` 手工实现：url 只回显遮蔽后的 DSN。
///
/// 派生 `Debug` 会原样打印 `url`——配置对象整体打印（`{:?}`）时把数据库
/// 口令写进日志。字段清单与结构体保持同步（新增字段须同步登记）。
impl std::fmt::Debug for DatabaseSinkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseSinkConfig")
            .field("name", &self.name)
            .field("enabled", &self.enabled)
            .field("driver", &self.driver)
            .field("url", &redact_dsn(&self.url))
            .field("pool_size", &self.pool_size)
            .field("batch_size", &self.batch_size)
            .field("flush_interval_ms", &self.flush_interval_ms)
            .field("partition", &self.partition)
            .field("table_name", &self.table_name)
            .field("archive_format", &self.archive_format)
            .field("parquet_config", &self.parquet_config)
            .field("permissions_path", &self.permissions_path)
            .field("admin_role", &self.admin_role)
            .field("secret_scan_enabled", &self.secret_scan_enabled)
            .finish()
    }
}

fn default_db_sink_name() -> String {
    "default".to_string()
}

/// 遮蔽后的凭据占位符。
const REDACTED_CREDENTIAL: &str = "***";

/// 凭据字段名（大小写不敏感）：出现在 query、fragment 或 keyword/value
/// 形态中时，其取值按凭据遮蔽。
const CREDENTIAL_KEYS: &[&str] = &[
    "password",
    "pwd",
    "pass",
    "token",
    "secret",
    "api_key",
    "apikey",
    "auth",
    "key",
    "sig",
    "signature",
];

/// 脱敏占位符：凭据位取这些值说明配置里存的是脱敏视图，不是真口令。
const REDACTED_CREDENTIAL_MARKERS: &[&str] = &[REDACTED_CREDENTIAL, "REDACTED", "xxxxx"];

/// 口令位已是脱敏占位符时的错误文案（sink 校验与配置校验共用）。
pub(crate) const REDACTED_CREDENTIAL_ERROR: &str =
    "database_sink.url password is a redaction placeholder; the real password is missing";

/// 遮蔽 DSN 中的口令与凭据取值。
///
/// 覆盖形态：
/// - URL userinfo：`postgres://user:secret@host/db` → `postgres://user:***@host/db`
/// - URL query / fragment 的凭据参数：`?password=secret` → `?password=***`
/// - keyword/value：`host=db password=secret` → `host=db password=***`
///
/// 形态不可判定（既非 URL 也非 keyword/value）却疑似带凭据时整体遮蔽——
/// 宁可少信息，也不回显疑似口令的原文。
pub(crate) fn redact_dsn(dsn: &str) -> String {
    let spans = credential_spans(dsn);
    if spans.is_empty() {
        return dsn.to_string();
    }
    let mut out = String::with_capacity(dsn.len());
    let mut cursor = 0usize;
    for (start, end) in spans {
        out.push_str(&dsn[cursor..start]);
        out.push_str(REDACTED_CREDENTIAL);
        cursor = end;
    }
    out.push_str(&dsn[cursor..]);
    out
}

/// DSN 中的凭据取值是否已是脱敏占位符。
///
/// 序列化视图（`Serialize`）把口令遮蔽为 `***`：把它当配置回载会拿占位符去
/// 认证，必然失败——配置校验须显式拒绝。
pub(crate) fn dsn_has_redacted_credential(dsn: &str) -> bool {
    credential_spans(dsn).iter().any(|(start, end)| {
        let value = dsn[*start..*end].trim();
        REDACTED_CREDENTIAL_MARKERS
            .iter()
            .any(|marker| value.eq_ignore_ascii_case(marker))
    })
}

/// DSN 中需要遮蔽的凭据取值区间（相对 DSN 起点的字节偏移，升序且不重叠）。
fn credential_spans(dsn: &str) -> Vec<(usize, usize)> {
    if let Some(scheme_end) = dsn.find("://") {
        return url_credential_spans(dsn, scheme_end + 3);
    }
    if is_keyword_value_dsn(dsn) {
        return keyword_credential_spans(dsn);
    }
    if has_undetermined_credentials(dsn) {
        // 形态不可判定：整体遮蔽
        return vec![(0, dsn.len())];
    }
    Vec::new()
}

/// URL 形态的凭据区间：userinfo 口令 + query / fragment 的凭据参数。
fn url_credential_spans(dsn: &str, body_start: usize) -> Vec<(usize, usize)> {
    // authority 结束于首个 '/'、'?' 或 '#'
    let authority_end = dsn[body_start..]
        .find(['/', '?', '#'])
        .map(|i| body_start + i)
        .unwrap_or(dsn.len());
    let authority = &dsn[body_start..authority_end];
    let tail = &dsn[authority_end..];

    let mut spans = Vec::new();
    match authority.rfind('@') {
        Some(at) => {
            if let Some(password_start) = userinfo_password_start(&authority[..at]) {
                spans.push((body_start + password_start, body_start + at));
            }
        }
        None if looks_like_userinfo(authority) => {
            // authority 形如 `user:pass` 却没有 '@'：口令里的 '/' 未编码，
            // userinfo 被截断——口令边界不可判定，整体遮蔽。
            return vec![(body_start, dsn.len())];
        }
        None => {}
    }
    spans.extend(param_credential_spans(tail, authority_end));
    spans
}

/// userinfo 中口令的起始偏移（相对原始 userinfo）。
///
/// 先把 userinfo 整体按百分号解码，再按解码后的首个 ':' 切分——口令是
/// ':' 之后的全部内容。返回口令在**原始** userinfo 中的起点，百分号编码
/// 形态的口令（`pa%2Fss`）整段被遮蔽，不残留原文。
fn userinfo_password_start(userinfo: &str) -> Option<usize> {
    let bytes = userinfo.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b':' {
            return Some(i + 1);
        }
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(high) = (bytes[i + 1] as char).to_digit(16)
            && let Some(low) = (bytes[i + 2] as char).to_digit(16)
        {
            if high * 16 + low == u32::from(b':') {
                // 转义序列解码后是 ':'：口令起点在该转义序列之后
                return Some(i + 3);
            }
            // 其它转义序列解码后不是 ':'，跳过整段
            i += 3;
            continue;
        }
        i += 1;
    }
    None
}

/// authority 是否形如 `user:pass`（冒号后不是端口号）——userinfo 形态。
fn looks_like_userinfo(authority: &str) -> bool {
    // IPv6 字面量 `[::1]:5432`：方括号内的冒号不是 userinfo 分隔符
    let host_part = match authority.strip_prefix('[') {
        Some(rest) => match rest.find(']') {
            Some(i) => &rest[i + 1..],
            None => authority,
        },
        None => authority,
    };
    match host_part.rfind(':') {
        Some(i) => {
            let suffix = &host_part[i + 1..];
            !suffix.is_empty() && !suffix.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

/// query / fragment 里的凭据参数取值区间。
fn param_credential_spans(tail: &str, tail_start: usize) -> Vec<(usize, usize)> {
    let (before_fragment, fragment) = match tail.find('#') {
        Some(i) => (&tail[..i], Some(&tail[i + 1..])),
        None => (tail, None),
    };
    let mut spans = Vec::new();
    if let Some(q) = before_fragment.find('?') {
        spans.extend(kv_credential_spans(
            &before_fragment[q + 1..],
            tail_start + q + 1,
        ));
    }
    if let Some(fragment) = fragment {
        spans.extend(kv_credential_spans(
            fragment,
            tail_start + (tail.len() - fragment.len()),
        ));
    }
    spans
}

/// `k=v&k2=v2` 形态里凭据参数的取值区间。
fn kv_credential_spans(params: &str, base: usize) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for param in params.split('&') {
        if let Some(eq) = param.find('=')
            && is_credential_key(&param[..eq])
        {
            spans.push((base + offset + eq + 1, base + offset + param.len()));
        }
        offset += param.len() + 1;
    }
    spans
}

/// keyword/value 形态里凭据参数的取值区间。
fn keyword_credential_spans(dsn: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for (start, token) in whitespace_tokens(dsn) {
        if let Some(eq) = token.find('=')
            && is_credential_key(&token[..eq])
        {
            spans.push((start + eq + 1, start + token.len()));
        }
    }
    spans
}

/// 空白分隔的词元及其起始偏移。
fn whitespace_tokens(text: &str) -> Vec<(usize, &str)> {
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        tokens.push((start, &text[start..i]));
    }
    tokens
}

/// 是否为 keyword/value 形态：所有词元均为 `key=value`（key 非空）。
fn is_keyword_value_dsn(dsn: &str) -> bool {
    let mut any = false;
    for (_, token) in whitespace_tokens(dsn) {
        match token.split_once('=') {
            Some((key, _)) if !key.is_empty() && !key.contains([':', '@', '/']) => any = true,
            _ => return false,
        }
    }
    any
}

/// 形态不可判定时是否疑似含凭据：`:@` 组合，或出现凭据字段名。
fn has_undetermined_credentials(dsn: &str) -> bool {
    if dsn.contains(':') && dsn.contains('@') {
        return true;
    }
    let lowered = dsn.to_ascii_lowercase();
    CREDENTIAL_KEYS.iter().any(|key| {
        lowered.match_indices(key).any(|(idx, _)| {
            let before = idx == 0
                || (!lowered.as_bytes()[idx - 1].is_ascii_alphanumeric()
                    && lowered.as_bytes()[idx - 1] != b'_');
            let after = &lowered[idx + key.len()..];
            let after = after
                .as_bytes()
                .first()
                .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_');
            before && after
        })
    })
}

/// 字段名是否为凭据字段（大小写不敏感）。
fn is_credential_key(key: &str) -> bool {
    let key = key.trim();
    CREDENTIAL_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key))
}

/// `Serialize` 的 url 字段：遮蔽口令后输出。
fn serialize_redacted_dsn<S: serde::Serializer>(
    url: &str,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&redact_dsn(url))
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
fn default_admin_role() -> String {
    "admin".to_string()
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
            permissions_path: None,
            admin_role: default_admin_role(),
            secret_scan_enabled: false,
        }
    }
}

impl DatabaseSinkConfig {
    /// 导出**不脱敏**的完整配置（JSON 文本）。
    ///
    /// [`Serialize`] 输出的是脱敏视图（DSN 口令被遮蔽为 `***`）——
    /// 序列化→反序列化往返会丢口令。需要保真往返（配置备份、迁移、
    /// 跨环境搬运）时用本方法：口令原样出现在导出结果里，导出文件须按
    /// 凭据同等对待。
    ///
    /// # Errors
    /// 序列化失败（结构含 `serde_json::Value` 之外的不可序列化成员）时
    /// 返回 `serde_json::Error`。
    pub fn to_lossless_json(&self) -> Result<String, serde_json::Error> {
        let mut value = serde_json::to_value(self)
            .unwrap_or_else(|_| panic!("{}", crate::i18n::tr("config-serialize_invariant")));
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "url".to_string(),
                serde_json::Value::String(self.url.clone()),
            );
        }
        serde_json::to_string(&value)
    }

    /// 导出**不脱敏**的完整配置（TOML 文本）。
    ///
    /// 语义与 [`Self::to_lossless_json`] 相同：DSN 口令原样保留。
    ///
    /// # Errors
    /// TOML 序列化失败（成员含 TOML 无法表达的类型）时返回
    /// `toml::ser::Error`。
    pub fn to_lossless_toml(&self) -> Result<String, toml::ser::Error> {
        let mut value = toml::Value::try_from(self)?;
        if let Some(table) = value.as_table_mut() {
            table.insert("url".to_string(), toml::Value::String(self.url.clone()));
        }
        toml::to_string(&value)
    }

    /// Validate and adjust the configuration.
    ///
    /// - When using SQLite, `pool_size` is overridden to 1 since SQLite
    ///   only supports a single writer connection.
    /// - Rejects zero `pool_size` for non-SQLite drivers, which would leave
    ///   the sink without any connection.
    /// - Rejects zero `batch_size` and `flush_interval_ms` to prevent
    ///   busy-loops or panics at runtime.
    /// - Validates `compression_level` is within the valid zstd range (1–22).
    /// - Rejects a DSN whose password is a redaction placeholder: such a
    ///   DSN lost its password in a serializing round trip and cannot
    ///   authenticate.
    pub fn validate(&mut self) -> Result<(), String> {
        #[cfg(not(feature = "secret-scan"))]
        if self.secret_scan_enabled {
            tracing::warn!(
                "{}",
                crate::i18n::tr("log-database_secret_scan_feature_missing")
            );
        }
        if dsn_has_redacted_credential(&self.url) {
            return Err(REDACTED_CREDENTIAL_ERROR.to_string());
        }
        if self.driver == DatabaseDriver::SQLite {
            if self.pool_size != 1 {
                self.pool_size = 1;
            }
        } else if self.pool_size == 0 {
            return Err(format!(
                "pool_size must be at least 1 for the {} driver, got 0",
                self.driver
            ));
        }
        if self.batch_size == 0 {
            tracing::warn!("{}", crate::i18n::tr("warn-db_batch_size_zero"));
            self.batch_size = 100;
        }
        if self.flush_interval_ms == 0 {
            tracing::warn!("{}", crate::i18n::tr("warn-db_flush_interval_zero"));
            self.flush_interval_ms = 500;
        }
        // Clamp Parquet compression level to valid zstd range 1–22
        if !(1..=22).contains(&self.parquet_config.compression_level) {
            tracing::warn!("{}", crate::i18n::tr("warn-db_compression_level_clamp"));
            self.parquet_config.compression_level = 3;
        }
        Ok(())
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
        config.validate().unwrap();
        assert_eq!(config.pool_size, 1, "SQLite pool_size must be 1");
    }

    #[test]
    fn test_non_sqlite_pool_size_unchanged() {
        let mut config = DatabaseSinkConfig::default();
        config.driver = DatabaseDriver::PostgreSQL;
        config.pool_size = 10;
        config.validate().unwrap();
        assert_eq!(
            config.pool_size, 10,
            "non-SQLite pool_size should be unchanged"
        );
    }

    #[test]
    fn test_non_sqlite_pool_size_zero_rejected() {
        for driver in [
            DatabaseDriver::PostgreSQL,
            DatabaseDriver::MySQL,
            DatabaseDriver::DuckDB,
        ] {
            let mut config = DatabaseSinkConfig::default();
            config.driver = driver.clone();
            config.pool_size = 0;
            let err = config.validate().unwrap_err();
            assert!(
                err.contains("pool_size"),
                "driver {driver}: unexpected error: {err}"
            );
        }
    }

    #[test]
    fn test_sqlite_pool_size_zero_still_clamped() {
        let mut config = DatabaseSinkConfig::default();
        config.driver = DatabaseDriver::SQLite;
        config.pool_size = 0;
        config.validate().unwrap();
        assert_eq!(config.pool_size, 1);
    }

    #[test]
    fn test_database_driver_from_str() {
        assert_eq!(
            "postgres".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::PostgreSQL
        );
        assert_eq!(
            "PostgreSQL".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::PostgreSQL
        );
        assert_eq!(
            "postgresql".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::PostgreSQL
        );
        assert_eq!(
            "mysql".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::MySQL
        );
        assert_eq!(
            "MYSQL".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::MySQL
        );
        assert_eq!(
            "sqlite".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::SQLite
        );
        assert_eq!(
            "sqlite3".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::SQLite
        );
        assert_eq!(
            "duckdb".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::DuckDB
        );
        assert_eq!(
            "DuckDB".parse::<DatabaseDriver>().unwrap(),
            DatabaseDriver::DuckDB
        );
        assert!("oracle".parse::<DatabaseDriver>().is_err());
    }

    #[test]
    fn test_database_driver_display() {
        assert_eq!(DatabaseDriver::PostgreSQL.to_string(), "postgres");
        assert_eq!(DatabaseDriver::MySQL.to_string(), "mysql");
        assert_eq!(DatabaseDriver::SQLite.to_string(), "sqlite");
        assert_eq!(DatabaseDriver::DuckDB.to_string(), "duckdb");
    }

    #[test]
    fn test_partition_strategy_from_str() {
        assert_eq!(
            "monthly".parse::<PartitionStrategy>().unwrap(),
            PartitionStrategy::Monthly
        );
        assert_eq!(
            "month".parse::<PartitionStrategy>().unwrap(),
            PartitionStrategy::Monthly
        );
        assert_eq!(
            "yearly".parse::<PartitionStrategy>().unwrap(),
            PartitionStrategy::Yearly
        );
        assert_eq!(
            "year".parse::<PartitionStrategy>().unwrap(),
            PartitionStrategy::Yearly
        );
        assert!("weekly".parse::<PartitionStrategy>().is_err());
    }

    #[test]
    fn test_partition_strategy_display() {
        assert_eq!(PartitionStrategy::Monthly.to_string(), "monthly");
        assert_eq!(PartitionStrategy::Yearly.to_string(), "yearly");
    }

    #[test]
    fn test_archive_format_from_str() {
        assert_eq!(
            "json".parse::<ArchiveFormat>().unwrap(),
            ArchiveFormat::Json
        );
        assert_eq!(
            "JSON".parse::<ArchiveFormat>().unwrap(),
            ArchiveFormat::Json
        );
        assert_eq!(
            "parquet".parse::<ArchiveFormat>().unwrap(),
            ArchiveFormat::Parquet
        );
        assert_eq!("csv".parse::<ArchiveFormat>().unwrap(), ArchiveFormat::Csv);
        assert!("xml".parse::<ArchiveFormat>().is_err());
    }

    #[test]
    fn test_archive_format_display() {
        assert_eq!(ArchiveFormat::Json.to_string(), "json");
        assert_eq!(ArchiveFormat::Parquet.to_string(), "parquet");
        assert_eq!(ArchiveFormat::Csv.to_string(), "csv");
    }

    #[test]
    fn test_validate_batch_size_zero() {
        let mut config = DatabaseSinkConfig::default();
        config.batch_size = 0;
        config.validate().unwrap();
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_validate_flush_interval_zero() {
        let mut config = DatabaseSinkConfig::default();
        config.flush_interval_ms = 0;
        config.validate().unwrap();
        assert_eq!(config.flush_interval_ms, 500);
    }

    #[test]
    fn test_validate_compression_level_clamp() {
        let mut config = DatabaseSinkConfig::default();
        config.parquet_config.compression_level = 99;
        config.validate().unwrap();
        assert_eq!(config.parquet_config.compression_level, 3);
    }

    #[test]
    fn test_validate_compression_level_zero() {
        let mut config = DatabaseSinkConfig::default();
        config.parquet_config.compression_level = 0;
        config.validate().unwrap();
        assert_eq!(config.parquet_config.compression_level, 3);
    }

    // DatabaseSinkConfig 序列化/反序列化含新字段测试

    #[test]
    fn test_config_default_has_new_fields() {
        let config = DatabaseSinkConfig::default();
        assert_eq!(config.admin_role, "admin");
        assert!(config.permissions_path.is_none());
    }

    #[test]
    fn test_config_serialize_with_new_fields() {
        let mut config = DatabaseSinkConfig::default();
        config.permissions_path = Some("/etc/inklog/perms.yaml".to_string());
        config.admin_role = "superadmin".to_string();

        let toml_str = toml::to_string(&config).expect("serialize should succeed");
        assert!(toml_str.contains("permissions_path"));
        assert!(toml_str.contains("superadmin"));
    }

    /// DSN 口令不得随配置对象的 Debug / Serialize 外泄。
    #[test]
    fn test_config_debug_and_serialize_redact_dsn_password() {
        let mut config = DatabaseSinkConfig::default();
        config.url = "postgres://appuser:s3cr3t@db.internal:5432/logs".to_string();

        let debug = format!("{config:?}");
        assert!(
            !debug.contains("s3cr3t"),
            "Debug must not expose the DSN password: {debug}"
        );
        assert!(
            debug.contains("postgres://appuser:***@db.internal:5432/logs"),
            "Debug must keep the DSN shape: {debug}"
        );

        let json = serde_json::to_string(&config).expect("serialize should succeed");
        assert!(
            !json.contains("s3cr3t"),
            "Serialize must not expose the DSN password: {json}"
        );

        // 反序列化路径不受影响：配置加载（TOML/env）仍能原样读出口令，
        // 序列化形态本身即脱敏视图（回读到的是遮蔽后的 DSN）
        let parsed: DatabaseSinkConfig =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(parsed.url, "postgres://appuser:***@db.internal:5432/logs");
    }

    /// 脱敏视图不可回载（往返丢口令）；保真导出必须能回载。
    #[test]
    fn test_lossless_export_preserves_dsn_password() {
        let mut config = DatabaseSinkConfig::default();
        config.url = "postgres://appuser:s3cr3t@db.internal:5432/logs".to_string();

        let json = config.to_lossless_json().expect("lossless json export");
        assert!(
            json.contains("s3cr3t"),
            "lossless export must keep the password: {json}"
        );
        let parsed: DatabaseSinkConfig = serde_json::from_str(&json).expect("roundtrip");
        assert_eq!(parsed.url, config.url, "lossless json must roundtrip");

        let toml_text = config.to_lossless_toml().expect("lossless toml export");
        assert!(
            toml_text.contains("s3cr3t"),
            "lossless export must keep the password: {toml_text}"
        );
        let parsed: DatabaseSinkConfig = toml::from_str(&toml_text).expect("roundtrip");
        assert_eq!(parsed.url, config.url, "lossless toml must roundtrip");
    }

    /// 非 URL 形态的 DSN（sqlite::memory:）与无 userinfo 的 DSN 原样保留。
    #[test]
    fn test_redact_dsn_keeps_non_url_forms() {
        assert_eq!(redact_dsn("sqlite::memory:"), "sqlite::memory:");
        assert_eq!(redact_dsn("postgres://host/logs"), "postgres://host/logs");
        assert_eq!(
            redact_dsn("mysql://user@host/logs"),
            "mysql://user@host/logs"
        );
    }

    /// query 段里的凭据参数（`?password=`）必须遮蔽——sqlx 按 URL 解析
    /// query 参数，该形态是有效 DSN。
    #[test]
    fn test_redact_dsn_masks_query_password() {
        let redacted = redact_dsn("postgres://host/logs?password=s3cr3t");
        assert!(
            !redacted.contains("s3cr3t"),
            "query password must be masked: {redacted}"
        );
        assert!(
            redacted.contains("password=***"),
            "query password placeholder expected: {redacted}"
        );
    }

    /// keyword/value 形态（`host=db password=secret`）的口令必须遮蔽。
    #[test]
    fn test_redact_dsn_masks_keyword_value_password() {
        let redacted = redact_dsn("host=db user=app password=s3cr3t");
        assert!(
            !redacted.contains("s3cr3t"),
            "keyword/value password must be masked: {redacted}"
        );
        assert!(
            redacted.contains("password=***"),
            "keyword/value password placeholder expected: {redacted}"
        );
    }

    /// 口令含未编码的 '/' 时 userinfo 边界不可判定：整体遮蔽而非原样回显。
    #[test]
    fn test_redact_dsn_masks_password_containing_slash() {
        let redacted = redact_dsn("postgres://appuser:pa/ss@db.internal:5432/logs");
        assert!(
            !redacted.contains("pa/ss"),
            "ambiguous password must not be echoed: {redacted}"
        );
    }

    /// query 段含 '@' 时不得吞掉 host 与 query。
    #[test]
    fn test_redact_dsn_keeps_query_with_at_sign() {
        let redacted = redact_dsn("postgres://user:pw@host/logs?email=a@b");
        assert!(
            !redacted.contains(":pw@"),
            "password must be masked: {redacted}"
        );
        assert!(
            redacted.contains("@host/logs?email=a@b"),
            "host and query must survive redaction: {redacted}"
        );
    }

    /// fragment 段里的凭据参数同样必须遮蔽。
    #[test]
    fn test_redact_dsn_masks_fragment_password() {
        let redacted = redact_dsn("postgres://host/logs#password=s3cr3t");
        assert!(
            !redacted.contains("s3cr3t"),
            "fragment password must be masked: {redacted}"
        );
    }

    /// 百分号编码形态的口令（`pa%2Fss`）不得原样回显。
    #[test]
    fn test_redact_dsn_masks_percent_encoded_password() {
        let redacted = redact_dsn("postgres://appuser:pa%2Fss@db.internal:5432/logs");
        assert!(
            !redacted.contains("pa%2Fss"),
            "percent-encoded password must be masked: {redacted}"
        );
    }

    /// 口令位是脱敏占位符的 DSN 必须被校验拒绝——认证必然失败。
    #[test]
    fn test_validate_rejects_redacted_password() {
        let mut config = DatabaseSinkConfig::default();
        config.url = "postgres://appuser:***@db.internal:5432/logs".to_string();
        let err = config
            .validate()
            .expect_err("redacted password must be rejected");
        assert!(
            err.contains("redaction placeholder"),
            "unexpected error: {err}"
        );
    }

    /// 正常口令的 DSN 不受占位符校验影响。
    #[test]
    fn test_validate_accepts_real_password() {
        let mut config = DatabaseSinkConfig::default();
        config.url = "postgres://appuser:s3cr3t@db.internal:5432/logs".to_string();
        config.validate().expect("real password must be accepted");
    }

    #[test]
    fn test_config_deserialize_default_values() {
        // TOML without new fields should use defaults
        let toml_str = r#"
            name = "test"
            enabled = true
            url = "sqlite::memory:"
        "#;
        let config: DatabaseSinkConfig =
            toml::from_str(toml_str).expect("deserialize should succeed");
        assert_eq!(config.admin_role, "admin");
        assert!(config.permissions_path.is_none());
    }

    #[test]
    fn test_config_deserialize_custom_values() {
        let toml_str = r#"
            name = "test"
            enabled = true
            url = "sqlite::memory:"
            permissions_path = "/etc/perms.yaml"
            admin_role = "dba"
        "#;
        let config: DatabaseSinkConfig =
            toml::from_str(toml_str).expect("deserialize should succeed");
        assert_eq!(config.admin_role, "dba");
        assert_eq!(config.permissions_path, Some("/etc/perms.yaml".to_string()));
    }
}
