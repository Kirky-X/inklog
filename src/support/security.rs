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
