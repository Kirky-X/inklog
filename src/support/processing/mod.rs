// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Processing module - log processing utilities.

pub mod masking;
pub mod object_pool;
pub mod template;

pub use masking::DataMasker;
pub use object_pool::{
    ObjectPool, ObjectPoolConfig, get_log_record, get_string_buffer, put_log_record,
    put_string_buffer,
};
pub use template::LogTemplate;
