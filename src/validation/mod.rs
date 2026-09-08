// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Input validation module.
//!
//! This module provides validation for user inputs to prevent security vulnerabilities.

pub mod path;
pub mod sanitize;

pub use path::{
    PathValidator, PathValidatorConfig, ValidationResult, create_validated_file,
    open_validated_file,
};
pub use sanitize::{EscapeMode, LogSanitizer, SanitizerConfig};
