// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Processing module - log processing utilities.

pub mod masking;
pub mod object_pool;
pub mod template;

pub use masking::DataMasker;
pub use object_pool::{
    get_log_record, get_string_buffer, put_log_record, put_string_buffer, ObjectPool,
    ObjectPoolBuilder, ObjectPoolConfig, PoolMetrics, LOG_RECORD_POOL, STRING_POOL,
};
pub use template::LogTemplate;
