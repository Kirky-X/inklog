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

pub use masking::{DataMasker, DataMaskerBuilder, MaskMatch, MaskRule, MaskRuleBuilder};
#[cfg(feature = "fast-masking")]
pub use masking_ac::AcMasker;
pub use masking_registry::MaskRuleRegistry;
pub use object_pool::{
    ObjectPool, ObjectPoolConfig, get_log_record, get_string_buffer, put_log_record,
    put_string_buffer,
};
pub use pipeline::{
    GlobalRateLimitMiddleware, IdentityFieldsMiddleware, ProcessingPipeline, SanitizeMiddleware,
    StressRelief, TargetQuotaMiddleware,
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
