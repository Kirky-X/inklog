// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Processing module - log processing utilities.

pub mod masking;
#[cfg(feature = "fast-masking")]
pub mod masking_ac;
pub mod masking_registry;
pub mod object_pool;
pub mod pipeline;
pub mod rate_limiter;
#[cfg(feature = "secret-scan")]
pub mod secret_entropy;
#[cfg(feature = "secret-scan")]
pub mod secret_patterns;
#[cfg(feature = "secret-scan")]
pub mod secret_scan;
pub mod target_rate_limiter;
pub mod template;
pub(crate) mod token_bucket;

pub use masking::secret_scan_masker;
pub use masking::{DataMasker, DataMaskerBuilder, MaskMatch, MaskRule, MaskRuleBuilder};
#[cfg(feature = "fast-masking")]
pub use masking_ac::AcMasker;
pub use masking_registry::MaskRuleRegistry;
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
#[cfg(feature = "secret-scan")]
pub use secret_entropy::EntropyScanner;
#[cfg(feature = "secret-scan")]
pub use secret_patterns::{SecretMatch, SecretPattern, SecretPatternRegistry};
#[cfg(feature = "secret-scan")]
pub use secret_scan::SecretScanGate;
pub use target_rate_limiter::{TargetQuotaVerdict, TargetRateLimiter};
pub use template::LogTemplate;
pub use template::OutputFormat;
