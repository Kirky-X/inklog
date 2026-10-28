// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Processing module - log processing utilities.
//!
//! 掩码引擎与 secret-scan 门族已下沉 trait-kit（`trait-kit/masking` feature，
//! 单一来源供 inklog/limiteron 共用）；本模块 re-export 保持既有公开路径
//! 与 feature 门控不变。

pub mod object_pool;
pub mod pipeline;
pub mod rate_limiter;
pub mod target_rate_limiter;
pub mod template;
pub(crate) mod token_bucket;

#[cfg(feature = "cache")]
pub use object_pool::ObjectPool;
pub use object_pool::{
    ObjectPoolConfig, get_log_record, get_string_buffer, put_log_record, put_string_buffer,
};
pub use pipeline::{GlobalRateLimitMiddleware, TargetQuotaMiddleware};
pub use pipeline::{
    IdentityFieldsMiddleware, ProcessingPipeline, SanitizeMiddleware, StressRelief,
};
pub use rate_limiter::RateLimiter;
pub use target_rate_limiter::{TargetQuotaVerdict, TargetRateLimiter};
pub use template::LogTemplate;
pub use template::OutputFormat;
pub use trait_kit::masking;
#[cfg(feature = "fast-masking")]
pub use trait_kit::masking::AcMasker;
#[cfg(feature = "secret-scan")]
pub use trait_kit::masking::EntropyScanner;
pub use trait_kit::masking::MaskRuleRegistry;
#[cfg(feature = "secret-scan")]
pub use trait_kit::masking::SecretScanGate;
#[cfg(feature = "secret-scan")]
pub use trait_kit::masking::secret_entropy;
#[cfg(feature = "secret-scan")]
pub use trait_kit::masking::secret_patterns;
#[cfg(feature = "secret-scan")]
pub use trait_kit::masking::secret_scan;
pub use trait_kit::masking::{DataMasker, DataMaskerBuilder, secret_scan_masker};
pub use trait_kit::masking::{MaskMatch, MaskRule, MaskRuleBuilder};
#[cfg(feature = "secret-scan")]
pub use trait_kit::masking::{SecretMatch, SecretPattern, SecretPatternRegistry};
