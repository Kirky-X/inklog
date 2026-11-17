// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 安全支撑：KMS 密钥提供者端口（feature `kms`）与安全姿态评估（核心）。
//!
//! - [`assess_security_posture`]（核心，无 feature 门控）：对给定配置输出
//!   三维度（编译期 feature / 运行时配置 / 环境变量）安全姿态检查项；
//! - KMS 部分（feature `kms`）：[`KeyProvider`] 为加密链路（FileSink /
//!   DatabaseSink 的 AES-256-GCM 主密钥）提供可插拔取钥口：
//!   - [`EnvKeyProvider`]：环境变量（现有 `INKLOG_ENCRYPTION_KEY` 语义的显式化）；
//!   - [`ConfersKeyProvider`]：confers `AsyncKeyProvider` 适配器（async → async）；
//!   - [`vault_transit_provider`]：HashiCorp Vault transit MVP——经 confers
//!     `VaultTransitKeyProvider` 远程解包主密钥（`vault:v1:...` 信封），
//!     真实云联调由使用方配置，测试用本地 mock server。
//!
//! # Example
//! ```ignore
//! let provider = vault_transit_provider(&VaultTransitConfig {
//!     vault_addr: "https://vault.internal:8200".into(),
//!     transit_key: "inklog-master".into(),
//!     ciphertext: std::env::var("INKLOG_VAULT_CIPHERTEXT")?,
//!     token: std::env::var("VAULT_TOKEN").ok(),
//!     namespace: None,
//!     allow_http: false,
//! })?;
//! let key = provider.get_key().await?;
//! ```

#[cfg(feature = "kms")]
pub mod kms {
    //! KMS 密钥提供者（confers 生态适配）。独立内嵌模块 + `pub use *`：
    //! 姿态评估属核心代码不随 kms 门控，本模块路径保持
    //! `support::security::*` 不变。

    use std::sync::Arc;

    use async_trait::async_trait;
    use zeroize::Zeroizing;

    use crate::InklogError;

    /// 密钥提供者端口：返回 AES-256-GCM 主密钥（32 字节，drop 时清零）。
    #[async_trait]
    pub trait KeyProvider: Send + Sync {
        /// 取主密钥。
        async fn get_key(&self) -> Result<Zeroizing<[u8; 32]>, InklogError>;

        /// Provider 类型名（诊断）。
        fn provider_type(&self) -> &str;
    }

    /// 环境变量密钥（与 `get_encryption_key` 同一取钥语义：原始/Base64 32 字节，
    /// 或密码经 PBKDF2 派生）。
    pub struct EnvKeyProvider {
        /// 持有密钥的环境变量名（如 `INKLOG_ENCRYPTION_KEY`）
        pub env_var: String,
    }

    impl EnvKeyProvider {
        pub fn new(env_var: impl Into<String>) -> Self {
            Self {
                env_var: env_var.into(),
            }
        }
    }

    #[async_trait]
    impl KeyProvider for EnvKeyProvider {
        async fn get_key(&self) -> Result<Zeroizing<[u8; 32]>, InklogError> {
            let var = self.env_var.clone();
            // get_encryption_key 本就返回 Zeroizing<[u8; 32]>
            tokio::task::spawn_blocking(move || {
                crate::support::io::sink::encryption::get_encryption_key(&var)
            })
            .await
            .map_err(|e| InklogError::ConfigError(format!("key provider task panicked: {e}")))?
        }

        fn provider_type(&self) -> &str {
            "env"
        }
    }

    /// confers `AsyncKeyProvider` 适配器（端口反转：confers 密钥生态 → inklog）。
    pub struct ConfersKeyProvider {
        inner: Arc<dyn confers::interface::AsyncKeyProvider>,
    }

    impl ConfersKeyProvider {
        pub fn new(inner: Arc<dyn confers::interface::AsyncKeyProvider>) -> Self {
            Self { inner }
        }
    }

    #[async_trait]
    impl KeyProvider for ConfersKeyProvider {
        async fn get_key(&self) -> Result<Zeroizing<[u8; 32]>, InklogError> {
            let bytes = self.inner.get_key().await.map_err(|e| {
                InklogError::ConfigError(format!("confers key provider failed: {e}"))
            })?;
            let bytes: &[u8] = bytes.as_slice();
            let arr: [u8; 32] = bytes.try_into().map_err(|_| {
                InklogError::ConfigError(format!(
                    "confers key provider returned {} bytes; need 32",
                    bytes.len()
                ))
            })?;
            Ok(Zeroizing::new(arr))
        }

        fn provider_type(&self) -> &str {
            self.inner.provider_type()
        }
    }

    /// Vault transit MVP 配置。
    #[derive(Debug, Clone, Default)]
    pub struct VaultTransitConfig {
        /// Vault 地址（`https://...`；mock 测试可 `http://127.0.0.1` + `allow_http`）
        pub vault_addr: String,
        /// transit 引擎中的密钥名
        pub transit_key: String,
        /// 被包裹的主密钥信封（`vault:v1:<base64>`）
        pub ciphertext: String,
        /// Vault token（None 时回退 `VAULT_TOKEN` 环境变量）
        pub token: Option<String>,
        /// Vault enterprise namespace
        pub namespace: Option<String>,
        /// 允许明文 HTTP（仅限 loopback mock 测试；生产必须 TLS）
        pub allow_http: bool,
    }

    /// 构建 Vault transit 密钥提供者（经 confers `VaultTransitKeyProvider`）。
    pub fn vault_transit_provider(
        cfg: &VaultTransitConfig,
    ) -> Result<ConfersKeyProvider, InklogError> {
        let mut builder = confers::secret::VaultTransitKeyProvider::builder()
            .vault_addr(cfg.vault_addr.clone())
            .transit_key(cfg.transit_key.clone())
            .ciphertext(cfg.ciphertext.clone())
            .allow_http(cfg.allow_http);
        if let Some(token) = &cfg.token {
            builder = builder.token(token.clone());
        }
        if let Some(ns) = &cfg.namespace {
            builder = builder.namespace(ns.clone());
        }
        let inner = builder.build().map_err(|e| {
            InklogError::ConfigError(format!("vault transit provider build failed: {e}"))
        })?;
        Ok(ConfersKeyProvider::new(Arc::new(inner)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// mock：confers AsyncKeyProvider 的测试替身（恒定密钥）。
        struct MockAsyncKeyProvider;

        #[async_trait::async_trait]
        impl confers::interface::AsyncKeyProvider for MockAsyncKeyProvider {
            async fn get_key(&self) -> confers::ConfigResult<confers::ZeroizingBytes> {
                Ok(confers::ZeroizingBytes::new(vec![0x42u8; 32]))
            }
            fn provider_type(&self) -> &'static str {
                "mock"
            }
        }

        #[tokio::test]
        async fn test_confers_adapter_returns_key() {
            let provider = ConfersKeyProvider::new(Arc::new(MockAsyncKeyProvider));
            let key = provider.get_key().await.unwrap();
            assert_eq!(key.as_slice(), &[0x42u8; 32]);
            assert_eq!(provider.provider_type(), "mock");
        }

        #[tokio::test]
        async fn test_confers_adapter_rejects_wrong_key_length() {
            struct ShortProvider;
            #[async_trait::async_trait]
            impl confers::interface::AsyncKeyProvider for ShortProvider {
                async fn get_key(&self) -> confers::ConfigResult<confers::ZeroizingBytes> {
                    Ok(confers::ZeroizingBytes::new(vec![1u8; 16]))
                }
                fn provider_type(&self) -> &'static str {
                    "mock-short"
                }
            }
            let provider = ConfersKeyProvider::new(Arc::new(ShortProvider));
            let err = provider.get_key().await.unwrap_err();
            assert!(
                err.to_string().contains("16 bytes"),
                "length mismatch must be diagnosed"
            );
        }

        /// Vault transit MVP：mock collector（本地 TCP server 回 transit 解包响应）
        /// 下经适配器取钥成功——与 confers 官方 e2e 同款响应协议。
        #[tokio::test]
        async fn test_vault_transit_mvp_against_mock_server() {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();

            use base64::Engine;
            let plaintext_b64 = base64::engine::general_purpose::STANDARD.encode([0x7bu8; 32]);
            let body = serde_json::json!({ "data": { "plaintext": plaintext_b64 } }).to_string();
            let server = tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let (mut stream, _) = listener.accept().await.expect("accept");
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).await.expect("write");
            });

            let ciphertext = format!(
                "vault:v1:{}",
                base64::engine::general_purpose::STANDARD.encode([0u8; 32])
            );
            let provider = vault_transit_provider(&VaultTransitConfig {
                vault_addr: format!("http://{addr}"),
                transit_key: "inklog-master".to_string(),
                ciphertext,
                token: Some("test-token".to_string()),
                namespace: None,
                allow_http: true,
            })
            .unwrap();

            assert_eq!(provider.provider_type(), "vault-transit");
            let key = provider.get_key().await.unwrap();
            server.abort();
            assert_eq!(
                key.as_slice(),
                &[0x7bu8; 32],
                "vault-transit MVP must unwrap the key"
            );
        }

        #[tokio::test]
        async fn test_vault_transit_rejects_plain_http_without_opt_in() {
            // 非回环明文 HTTP 在取钥时被 confers 校验拒绝（构造期不联网）
            let provider = vault_transit_provider(&VaultTransitConfig {
                vault_addr: "http://vault.internal:8200".to_string(),
                transit_key: "inklog-master".to_string(),
                ciphertext: "vault:v1:abc".to_string(),
                token: None,
                namespace: None,
                allow_http: false,
            })
            .expect("build succeeds (validation happens on use)");
            let err = match provider.get_key().await {
                Err(e) => e,
                Ok(_) => panic!("non-loopback plain HTTP must be rejected on use"),
            };
            assert!(
                err.to_string().contains("HTTPS"),
                "non-loopback plain HTTP must be rejected, got: {err}"
            );
        }
    }
}

// 路径兼容：kms 内嵌模块项按既有 `support::security::*` 路径导出
#[cfg(feature = "kms")]
pub use kms::*;

// ── 安全姿态评估（doctor --security 的库侧核心）─────────────────────
//
// 三维度（编译期 feature / 运行时配置 / 环境变量）安全姿态的**只读**体检：
// 不改运行时行为、不连接外部服务、不在证据里输出密钥明文。评估时点为
// 调用瞬间（key 在构造后被清除之类的窗口差不覆盖）。

/// 单项安全姿态检查结果。
///
/// 字段全部为编译期常量或 i18n key——机器可读、语言无关，人类可读文案
/// 由 CLI 层经 `tr(detail_key, detail_args)` 渲染。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PostureCheck {
    /// 稳定检查项 ID（如 "audit-chain"），机器可读
    pub id: &'static str,
    /// 能力名 i18n key（如 "posture-audit-chain"）
    pub capability: &'static str,
    /// 检查结论
    pub status: PostureStatus,
    /// 证据描述 i18n key（参数不内嵌密钥明文）
    pub detail_key: &'static str,
    /// 证据参数（序列化为字符串 map；不含密钥值）
    #[serde(serialize_with = "serialize_msg_args_as_map")]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "std::collections::BTreeMap<String, String>")
    )]
    pub detail_args: crate::i18n::MsgArgs,
    /// 未达标时的修复提示 i18n key；达标为 None
    pub remedy_key: Option<&'static str>,
}

/// 检查结论四档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PostureStatus {
    /// 能力已启用且前置条件满足
    Ok,
    /// 已启用但存在弱化条件（如 insecure-tls）
    Warn,
    /// 用户声称需要但前置缺失（如 audit_chain_enabled 而无 key）
    Critical,
    /// 编译期未包含（feature 未启用），非错误
    Unavailable,
}

/// 汇总报告。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PostureReport {
    /// 检查项列表（固定顺序：pii-masking / file-encryption / audit-chain /
    /// secret-scan / net-sink-tls / journal-crypto / db-backend / kms）
    pub checks: Vec<PostureCheck>,
}

impl PostureReport {
    /// Critical 检查项数。
    pub fn critical_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == PostureStatus::Critical)
            .count()
    }

    /// Warn 检查项数。
    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == PostureStatus::Warn)
            .count()
    }

    /// 总体判定：任一 Critical → false（Warn 不影响，单独计数上报）。
    pub fn is_sound(&self) -> bool {
        self.critical_count() == 0
    }
}

/// `detail_args` 序列化为字符串 map（键序稳定，BTreeMap 保 JSON 确定性）。
fn serialize_msg_args_as_map<S>(args: &crate::i18n::MsgArgs, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap as _;
    let map = args.to_map();
    let mut out = s.serialize_map(Some(map.len()))?;
    for (k, v) in &map {
        out.serialize_entry(k, v)?;
    }
    out.end()
}

/// 评估入口：纯只读，不产生副作用、不读 env 以外的外部资源。
///
/// 评估时点为调用瞬间：编译期 feature 快照（`cfg!` 常量展开，运行时零
/// 开销）+ 当前配置 + 当前环境变量。密钥明文不进入任何 `detail_args`。
pub fn assess_security_posture(config: &crate::domain::config::InklogConfig) -> PostureReport {
    PostureReport {
        checks: vec![
            check_pii_masking(config),
            check_file_encryption(config),
            check_audit_chain(config),
            check_secret_scan(),
            check_net_sink_tls(),
            check_journal_crypto(config),
            check_db_backend(),
            check_kms(),
        ],
    }
}

/// secret-scan：编译期 feature 快照（`cfg!` 常量展开，运行时零开销）。
fn check_secret_scan() -> PostureCheck {
    compile_time_check(
        "secret-scan",
        "posture-secret-scan",
        cfg!(feature = "secret-scan"),
        "posture-secret-scan-detail-on",
        "posture-secret-scan-detail-off",
        Some("posture-secret-scan-remedy"),
    )
}

/// net-sink-tls：`net-sink` 编译态 + `insecure-tls` 弱化告警。
///
/// 未编译 net-sink Unavailable / TLS 校验正常 Ok / insecure-tls Warn。
fn check_net_sink_tls() -> PostureCheck {
    let (status, detail_key, remedy) = if cfg!(feature = "net-sink") {
        if cfg!(feature = "insecure-tls") {
            (
                PostureStatus::Warn,
                "posture-net-sink-tls-detail-insecure",
                Some("posture-net-sink-tls-remedy"),
            )
        } else {
            (PostureStatus::Ok, "posture-net-sink-tls-detail-on", None)
        }
    } else {
        (
            PostureStatus::Unavailable,
            "posture-net-sink-tls-detail-off",
            None,
        )
    };
    PostureCheck {
        id: "net-sink-tls",
        capability: "posture-net-sink-tls",
        status,
        detail_key,
        detail_args: crate::i18n::MsgArgs::new(),
        remedy_key: remedy,
    }
}

/// journal-crypto：fallback journal 加密（`JournalCryptoConfig` 同款配置字段）。
///
/// journal 关闭 Ok（无落盘） / 开启且加密 Ok（key 缺失在构造期已显性失败，
/// 非本检查职责）/ 开启但明文 Warn。
fn check_journal_crypto(config: &crate::domain::config::InklogConfig) -> PostureCheck {
    const DEFAULT_JOURNAL_KEY_ENV: &str = "INKLOG_JOURNAL_KEY";

    let (status, detail_key, remedy) = match (
        config.global.fallback_journal,
        config.global.fallback_journal_encrypt,
    ) {
        (false, _) => (PostureStatus::Ok, "posture-journal-crypto-detail-off", None),
        (true, true) => (PostureStatus::Ok, "posture-journal-crypto-detail-on", None),
        (true, false) => (
            PostureStatus::Warn,
            "posture-journal-crypto-detail-plaintext",
            Some("posture-journal-crypto-remedy"),
        ),
    };
    let mut args = crate::i18n::MsgArgs::new();
    args.set(
        "env",
        config
            .global
            .fallback_journal_key_env
            .as_deref()
            .unwrap_or(DEFAULT_JOURNAL_KEY_ENV),
    );
    PostureCheck {
        id: "journal-crypto",
        capability: "posture-journal-crypto",
        status,
        detail_key,
        detail_args: args,
        remedy_key: remedy,
    }
}

/// db-backend：四个互斥后端 feature 的 `cfg!` 快照（多选在 cargo 层已
/// 禁止，此处仅陈述快照）：编译 0 个 Unavailable / 恰 1 个 Ok。
fn check_db_backend() -> PostureCheck {
    let backends: Vec<&'static str> = [
        ("sqlite", cfg!(feature = "sqlite")),
        ("postgres", cfg!(feature = "postgres")),
        ("mysql", cfg!(feature = "mysql")),
        ("duckdb", cfg!(feature = "duckdb")),
    ]
    .into_iter()
    .filter(|(_, on)| *on)
    .map(|(name, _)| name)
    .collect();

    let mut args = crate::i18n::MsgArgs::new();
    args.set("names", backends.join("/"));
    let (status, detail_key) = if backends.is_empty() {
        (PostureStatus::Unavailable, "posture-db-backend-detail-off")
    } else {
        (PostureStatus::Ok, "posture-db-backend-detail-on")
    };
    PostureCheck {
        id: "db-backend",
        capability: "posture-db-backend",
        status,
        detail_key,
        detail_args: args,
        remedy_key: None,
    }
}

/// kms：`kms` feature 编译态快照。
fn check_kms() -> PostureCheck {
    compile_time_check(
        "kms",
        "posture-kms",
        cfg!(feature = "kms"),
        "posture-kms-detail-on",
        "posture-kms-detail-off",
        Some("posture-kms-remedy"),
    )
}

/// 纯编译期能力检查的公共形态（feature 开 Ok / 关 Unavailable）。
fn compile_time_check(
    id: &'static str,
    capability: &'static str,
    enabled: bool,
    detail_on: &'static str,
    detail_off: &'static str,
    remedy_off: Option<&'static str>,
) -> PostureCheck {
    PostureCheck {
        id,
        capability,
        status: if enabled {
            PostureStatus::Ok
        } else {
            PostureStatus::Unavailable
        },
        detail_key: if enabled { detail_on } else { detail_off },
        detail_args: crate::i18n::MsgArgs::new(),
        remedy_key: if enabled { None } else { remedy_off },
    }
}

/// audit-chain：`audit_chain_enabled` + `INKLOG_AUDIT_KEY` 非空——与
/// `FileSink::new`（file.rs）的判定同判：开链而 key 缺失时 sink 侧是
/// warn 后静默禁用链，本检查把它显性化为 Critical。
///
/// 双满足 Ok / 开链无 key Critical / 未开链 Warn / 无 enabled file sink Ok。
fn check_audit_chain(config: &crate::domain::config::InklogConfig) -> PostureCheck {
    const AUDIT_KEY_ENV: &str = "INKLOG_AUDIT_KEY";

    let Some(file) = config.file_sink.as_ref().filter(|f| f.enabled) else {
        return PostureCheck {
            id: "audit-chain",
            capability: "posture-audit-chain",
            status: PostureStatus::Ok,
            detail_key: "posture-audit-chain-detail-no-sink",
            detail_args: crate::i18n::MsgArgs::new(),
            remedy_key: None,
        };
    };

    // 与 file.rs 同判：仅「变量存在且非空」视为 key 可得（空串按缺失处理）
    let key_available = std::env::var(AUDIT_KEY_ENV).is_ok_and(|k| !k.is_empty());
    let (status, detail_key, remedy) = match (file.audit_chain_enabled, key_available) {
        (true, true) => (PostureStatus::Ok, "posture-audit-chain-detail-ok", None),
        (true, false) => (
            PostureStatus::Critical,
            "posture-audit-chain-detail-key-missing",
            Some("posture-audit-chain-remedy"),
        ),
        (false, _) => (
            PostureStatus::Warn,
            "posture-audit-chain-detail-off",
            Some("posture-audit-chain-remedy"),
        ),
    };
    let mut args = crate::i18n::MsgArgs::new();
    args.set("env", AUDIT_KEY_ENV);
    PostureCheck {
        id: "audit-chain",
        capability: "posture-audit-chain",
        status,
        detail_key,
        detail_args: args,
        remedy_key: remedy,
    }
}

/// pii-masking：遍历 console/file sink（仅 enabled 的 sink 计入——未启用
/// 的 sink 不产出，掩码与否无意义）。
///
/// 全启用 Ok / 部分启用 Warn / 全未启用 Critical；无 enabled sink 时 Ok
/// （无产出即无 PII 落地面）。
fn check_pii_masking(config: &crate::domain::config::InklogConfig) -> PostureCheck {
    let mut sinks: Vec<(&str, bool)> = Vec::new();
    if let Some(console) = config.console_sink.as_ref().filter(|c| c.enabled) {
        sinks.push(("console", console.masking_enabled));
    }
    if let Some(file) = config.file_sink.as_ref().filter(|f| f.enabled) {
        sinks.push(("file", file.masking_enabled));
    }

    let (status, detail_key, remedy) = match sinks.len() {
        0 => (
            PostureStatus::Ok,
            "posture-pii-masking-detail-no-sinks",
            None,
        ),
        _ if sinks.iter().all(|(_, m)| *m) => {
            (PostureStatus::Ok, "posture-pii-masking-detail-all", None)
        }
        _ if sinks.iter().any(|(_, m)| *m) => (
            PostureStatus::Warn,
            "posture-pii-masking-detail-partial",
            Some("posture-pii-masking-remedy"),
        ),
        _ => (
            PostureStatus::Critical,
            "posture-pii-masking-detail-none",
            Some("posture-pii-masking-remedy"),
        ),
    };
    let mut args = crate::i18n::MsgArgs::new();
    args.set("count", sinks.len());
    args.set(
        "names",
        sinks
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join("/"),
    );
    args.set(
        "unmasked",
        sinks
            .iter()
            .filter(|(_, m)| !*m)
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join("/"),
    );
    PostureCheck {
        id: "pii-masking",
        capability: "posture-pii-masking",
        status,
        detail_key,
        detail_args: args,
        remedy_key: remedy,
    }
}

/// file-encryption：`FileSinkConfig.encrypt` + key 来源可解析（env 优先、
/// 其次 key 文件，与 `FileSink` 实际取钥同范式：`resolve_key_material`）。
///
/// 启用且 key 可得 Ok / 启用无 key Critical / 未启用 Warn / 未配置 file
/// sink Ok（无文件产出）。
fn check_file_encryption(config: &crate::domain::config::InklogConfig) -> PostureCheck {
    use crate::support::io::sink::encryption::resolve_key_material;

    const DEFAULT_KEY_ENV: &str = "LOG_ENCRYPTION_KEY";

    let Some(file) = config.file_sink.as_ref().filter(|f| f.enabled) else {
        return PostureCheck {
            id: "file-encryption",
            capability: "posture-file-encryption",
            status: PostureStatus::Ok,
            detail_key: "posture-file-encryption-detail-no-sink",
            detail_args: crate::i18n::MsgArgs::new(),
            remedy_key: None,
        };
    };

    let mut args = crate::i18n::MsgArgs::new();
    let key_env = file
        .encryption_key_env
        .as_deref()
        .unwrap_or(DEFAULT_KEY_ENV);
    args.set("env", key_env);
    let (status, detail_key, remedy) = if !file.encrypt {
        (
            PostureStatus::Warn,
            "posture-file-encryption-detail-off",
            Some("posture-file-encryption-remedy"),
        )
    } else {
        match resolve_key_material(
            key_env,
            file.encryption_key_file
                .as_deref()
                .map(std::path::Path::new),
        ) {
            Ok(_) => (PostureStatus::Ok, "posture-file-encryption-detail-on", None),
            // key 缺失/无效与 sink 构造失败同判（file.rs 宁缺毋滥语义）
            Err(_) => (
                PostureStatus::Critical,
                "posture-file-encryption-detail-key-missing",
                Some("posture-file-encryption-remedy"),
            ),
        }
    };
    PostureCheck {
        id: "file-encryption",
        capability: "posture-file-encryption",
        status,
        detail_key,
        detail_args: args,
        remedy_key: remedy,
    }
}

// 进程级敏感字段注册（source-level-sensitive-fields Stage 2）

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

/// 进程级敏感字段名全局槽（与 [`register_ops_channel`] 同风格：显式
/// 注册函数 + 全局槽）。`RwLock` 允许初始化期多次 mark 合并去重；
/// `sensitive_fields()` 返回冻结快照，热路径只读。
static SENSITIVE_FIELDS: OnceLock<parking_lot::RwLock<HashSet<String>>> = OnceLock::new();

/// 初始化期声明进程级敏感字段名；幂等，重复声明合并去重（`HashSet` 语义）。
///
/// `LoggerSubscriber::new` 默认读取该注册集合预装配源头改写中间件；
/// 库内各集成适配器（confers_audit/oxcache_audit 等）可在各自初始化期
/// mark，宿主无需手工接线。声明后已构建的 subscriber 不受影响（构建期
/// 冻结快照），请先 mark 后建 subscriber。
pub fn mark_sensitive_fields(fields: impl IntoIterator<Item = impl Into<String>>) {
    let registry = SENSITIVE_FIELDS.get_or_init(|| parking_lot::RwLock::new(HashSet::new()));
    registry.write().extend(fields.into_iter().map(Into::into));
}

/// 当前注册集合的冻结快照（Arc 共享，无注册时空集合）。
pub fn sensitive_fields() -> Arc<HashSet<String>> {
    match SENSITIVE_FIELDS.get() {
        Some(registry) => Arc::new(registry.read().clone()),
        None => Arc::new(HashSet::new()),
    }
}

/// 全局注册是进程级单例：需要清空注册做隔离的测试经本 mod 操作，
/// 生产构建下整体不编译（对齐 ops_event::test_support 范式）。
#[cfg(test)]
pub(crate) mod sensitive_test_support {
    use super::SENSITIVE_FIELDS;

    /// 清空注册（测试隔离用）。
    pub(crate) fn reset_sensitive_fields_for_tests() {
        if let Some(registry) = SENSITIVE_FIELDS.get() {
            registry.write().clear();
        }
    }
}

#[cfg(test)]
mod posture_assess_tests {
    use super::*;
    use crate::domain::config::ConsoleSinkConfig;
    use crate::domain::config::FileSinkConfig;
    use crate::domain::config::InklogConfig;
    use serial_test::serial;

    fn find<'a>(report: &'a PostureReport, id: &str) -> &'a PostureCheck {
        report
            .checks
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("check {id} must exist"))
    }

    #[test]
    fn test_pii_masking_all_enabled_is_ok() {
        let mut config = InklogConfig::default();
        config.console_sink = Some(ConsoleSinkConfig {
            enabled: true,
            masking_enabled: true,
            ..Default::default()
        });
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            masking_enabled: true,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "pii-masking");
        assert_eq!(check.status, PostureStatus::Ok);
    }

    #[test]
    fn test_pii_masking_partial_is_warn() {
        let mut config = InklogConfig::default();
        config.console_sink = Some(ConsoleSinkConfig {
            enabled: true,
            masking_enabled: true,
            ..Default::default()
        });
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            masking_enabled: false,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "pii-masking");
        assert_eq!(check.status, PostureStatus::Warn);
        assert_eq!(check.remedy_key, Some("posture-pii-masking-remedy"));
    }

    #[test]
    fn test_pii_masking_none_enabled_is_critical() {
        let mut config = InklogConfig::default();
        config.console_sink = Some(ConsoleSinkConfig {
            enabled: true,
            masking_enabled: false,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "pii-masking");
        assert_eq!(check.status, PostureStatus::Critical);
    }

    #[test]
    fn test_pii_masking_disabled_sinks_excluded() {
        // disabled 的 file sink 不计入：console 掩码开启即 Ok
        let mut config = InklogConfig::default();
        config.console_sink = Some(ConsoleSinkConfig {
            enabled: true,
            masking_enabled: true,
            ..Default::default()
        });
        config.file_sink = Some(FileSinkConfig {
            enabled: false,
            masking_enabled: false,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "pii-masking");
        assert_eq!(check.status, PostureStatus::Ok);
    }

    #[test]
    #[serial]
    fn test_file_encryption_enabled_with_resolvable_key_is_ok() {
        unsafe {
            std::env::set_var(
                "INKLOG_POSTURE_TEST_KEY",
                "a-strong-posture-test-passphrase",
            );
        }
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            encrypt: true,
            encryption_key_env: Some("INKLOG_POSTURE_TEST_KEY".to_string()),
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "file-encryption");
        assert_eq!(check.status, PostureStatus::Ok);
        unsafe {
            std::env::remove_var("INKLOG_POSTURE_TEST_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_file_encryption_enabled_without_key_is_critical() {
        unsafe {
            std::env::remove_var("INKLOG_POSTURE_TEST_KEY");
        }
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            encrypt: true,
            encryption_key_env: Some("INKLOG_POSTURE_TEST_KEY".to_string()),
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "file-encryption");
        assert_eq!(check.status, PostureStatus::Critical);
        assert_eq!(check.remedy_key, Some("posture-file-encryption-remedy"));
    }

    #[test]
    fn test_file_encryption_disabled_is_warn() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            encrypt: false,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "file-encryption");
        assert_eq!(check.status, PostureStatus::Warn);
    }

    #[test]
    fn test_file_encryption_no_file_sink_is_ok() {
        let mut config = InklogConfig::default();
        config.file_sink = None;
        let report = assess_security_posture(&config);
        let check = find(&report, "file-encryption");
        assert_eq!(check.status, PostureStatus::Ok);
    }

    #[test]
    #[serial]
    fn test_audit_chain_enabled_without_key_is_critical() {
        unsafe {
            std::env::remove_var("INKLOG_AUDIT_KEY");
        }
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            audit_chain_enabled: true,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "audit-chain");
        assert_eq!(check.status, PostureStatus::Critical);
        assert_eq!(check.remedy_key, Some("posture-audit-chain-remedy"));
    }

    #[test]
    #[serial]
    fn test_audit_chain_enabled_with_key_is_ok_and_leaks_no_key_value() {
        unsafe {
            std::env::set_var("INKLOG_AUDIT_KEY", "posture-test-audit-secret");
        }
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            audit_chain_enabled: true,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "audit-chain");
        assert_eq!(check.status, PostureStatus::Ok);
        // 证据不得携带密钥明文：只允许出现变量名
        let json = serde_json::to_value(check).unwrap();
        let rendered = json.to_string();
        assert!(
            !rendered.contains("posture-test-audit-secret"),
            "detail must not embed the key value"
        );
        assert!(rendered.contains("INKLOG_AUDIT_KEY"));
        unsafe {
            std::env::remove_var("INKLOG_AUDIT_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_audit_chain_empty_key_counts_as_missing() {
        unsafe {
            std::env::set_var("INKLOG_AUDIT_KEY", "");
        }
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            audit_chain_enabled: true,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "audit-chain");
        assert_eq!(check.status, PostureStatus::Critical);
        unsafe {
            std::env::remove_var("INKLOG_AUDIT_KEY");
        }
    }

    #[test]
    fn test_audit_chain_disabled_is_warn() {
        let mut config = InklogConfig::default();
        config.file_sink = Some(FileSinkConfig {
            enabled: true,
            audit_chain_enabled: false,
            ..Default::default()
        });
        let report = assess_security_posture(&config);
        let check = find(&report, "audit-chain");
        assert_eq!(check.status, PostureStatus::Warn);
    }

    #[test]
    fn test_audit_chain_no_file_sink_is_ok() {
        let mut config = InklogConfig::default();
        config.file_sink = None;
        let report = assess_security_posture(&config);
        let check = find(&report, "audit-chain");
        assert_eq!(check.status, PostureStatus::Ok);
    }

    /// 编译期能力检查项：状态必须与 `cfg!` 编译快照一致。默认 feature
    /// 组合下四项全 Unavailable；CI 测试口径（`--features "sqlite ..."`）
    /// 下 db-backend 为 Ok——断言跟随快照而非硬编码，任意 feature 组合
    /// 下均成立。
    #[test]
    fn test_compile_time_checks_default_features() {
        let config = InklogConfig::default();
        let report = assess_security_posture(&config);
        let db_compiled = cfg!(any(
            feature = "sqlite",
            feature = "postgres",
            feature = "mysql",
            feature = "duckdb"
        ));
        assert_eq!(
            find(&report, "db-backend").status,
            if db_compiled {
                PostureStatus::Ok
            } else {
                PostureStatus::Unavailable
            },
            "db-backend status must match the compiled backend snapshot"
        );
        assert_eq!(
            find(&report, "secret-scan").status,
            if cfg!(feature = "secret-scan") {
                PostureStatus::Ok
            } else {
                PostureStatus::Unavailable
            },
            "secret-scan status must match the compiled feature snapshot"
        );
        assert_eq!(
            find(&report, "kms").status,
            if cfg!(feature = "kms") {
                PostureStatus::Ok
            } else {
                PostureStatus::Unavailable
            }
        );
        assert_eq!(
            find(&report, "net-sink-tls").status,
            if cfg!(feature = "net-sink") {
                if cfg!(feature = "insecure-tls") {
                    PostureStatus::Warn
                } else {
                    PostureStatus::Ok
                }
            } else {
                PostureStatus::Unavailable
            }
        );
        // 检查项全集与固定顺序
        let ids: Vec<&str> = report.checks.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![
                "pii-masking",
                "file-encryption",
                "audit-chain",
                "secret-scan",
                "net-sink-tls",
                "journal-crypto",
                "db-backend",
                "kms",
            ]
        );
    }

    #[test]
    #[serial]
    fn test_journal_crypto_states_and_no_key_leak() {
        unsafe {
            std::env::set_var("INKLOG_POSTURE_JOURNAL_KEY", "journal-key-secret-value");
        }
        // journal 关闭 → Ok
        let mut config = InklogConfig::default();
        config.global.fallback_journal = false;
        config.global.fallback_journal_encrypt = true;
        let report = assess_security_posture(&config);
        assert_eq!(find(&report, "journal-crypto").status, PostureStatus::Ok);

        // journal 开启且加密 → Ok；证据只带变量名，不带密钥值
        let mut config = InklogConfig::default();
        config.global.fallback_journal = true;
        config.global.fallback_journal_encrypt = true;
        config.global.fallback_journal_key_env = Some("INKLOG_POSTURE_JOURNAL_KEY".to_string());
        let report = assess_security_posture(&config);
        let check = find(&report, "journal-crypto");
        assert_eq!(check.status, PostureStatus::Ok);
        let rendered = serde_json::to_value(check).unwrap().to_string();
        assert!(
            !rendered.contains("journal-key-secret-value"),
            "journal detail must not embed the key value"
        );
        assert!(rendered.contains("INKLOG_POSTURE_JOURNAL_KEY"));

        // journal 开启但明文 → Warn
        let mut config = InklogConfig::default();
        config.global.fallback_journal = true;
        config.global.fallback_journal_encrypt = false;
        let report = assess_security_posture(&config);
        let check = find(&report, "journal-crypto");
        assert_eq!(check.status, PostureStatus::Warn);
        assert_eq!(check.remedy_key, Some("posture-journal-crypto-remedy"));
        unsafe {
            std::env::remove_var("INKLOG_POSTURE_JOURNAL_KEY");
        }
    }
}

#[cfg(test)]
mod posture_type_tests {
    use super::*;

    fn check(id: &'static str, status: PostureStatus) -> PostureCheck {
        PostureCheck {
            id,
            capability: "posture-test",
            status,
            detail_key: "posture-test-detail",
            detail_args: crate::i18n::MsgArgs::new(),
            remedy_key: None,
        }
    }

    #[test]
    fn test_empty_report_is_sound() {
        let report = PostureReport { checks: vec![] };
        assert!(report.is_sound(), "no checks means nothing critical");
        assert_eq!(report.critical_count(), 0);
        assert_eq!(report.warn_count(), 0);
    }

    #[test]
    fn test_single_critical_breaks_soundness_and_counts() {
        let report = PostureReport {
            checks: vec![check("a", PostureStatus::Critical)],
        };
        assert!(!report.is_sound(), "one critical must fail soundness");
        assert_eq!(report.critical_count(), 1);
        assert_eq!(report.warn_count(), 0);
    }

    #[test]
    fn test_warn_counts_without_breaking_soundness() {
        let report = PostureReport {
            checks: vec![
                check("a", PostureStatus::Warn),
                check("b", PostureStatus::Warn),
                check("c", PostureStatus::Ok),
                check("d", PostureStatus::Unavailable),
            ],
        };
        assert!(report.is_sound(), "warn alone must keep soundness");
        assert_eq!(report.warn_count(), 2);
        assert_eq!(report.critical_count(), 0);
    }

    #[test]
    fn test_status_serializes_lowercase() {
        let report = PostureReport {
            checks: vec![check("a", PostureStatus::Critical)],
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["checks"][0]["status"], "critical");
        // detail_args 序列化为 map
        assert!(json["checks"][0]["detail_args"].is_object());
    }
}

#[cfg(test)]
mod sensitive_registry_tests {
    use super::sensitive_test_support::reset_sensitive_fields_for_tests;
    use super::{mark_sensitive_fields, sensitive_fields};
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_mark_sensitive_fields_merges_and_dedupes() {
        reset_sensitive_fields_for_tests();
        // 未注册 → 空快照
        assert!(sensitive_fields().is_empty());
        // mark 两次同名字段：合并去重，不重复计数
        mark_sensitive_fields(["session_token", "ssn"]);
        mark_sensitive_fields(["session_token"]);
        let snapshot = sensitive_fields();
        assert_eq!(snapshot.len(), 2, "duplicate marks must dedupe");
        assert!(snapshot.contains("session_token"));
        assert!(snapshot.contains("ssn"));
        reset_sensitive_fields_for_tests();
    }

    #[test]
    #[serial]
    fn test_reset_clears_registry_for_isolation() {
        reset_sensitive_fields_for_tests();
        mark_sensitive_fields(["isolation_probe_field"]);
        assert_eq!(sensitive_fields().len(), 1);
        reset_sensitive_fields_for_tests();
        assert!(
            sensitive_fields().is_empty(),
            "reset must clear the registry"
        );
    }
}
