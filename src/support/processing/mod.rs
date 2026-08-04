// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Processing module - log processing utilities.

pub mod masking;
#[cfg(feature = "fast-masking")]
pub mod masking_ac;
pub mod masking_registry;
pub mod object_pool;
pub mod rate_limiter;
pub mod template;

pub use masking::{DataMasker, DataMaskerBuilder, MaskRule, MaskRuleBuilder};
#[cfg(feature = "fast-masking")]
pub use masking_ac::AcMasker;
pub use masking_registry::MaskRuleRegistry;
pub use object_pool::{
    ObjectPool, ObjectPoolConfig, get_log_record, get_string_buffer, put_log_record,
    put_string_buffer,
};
pub use rate_limiter::RateLimiter;
pub use template::LogTemplate;
pub use template::OutputFormat;
