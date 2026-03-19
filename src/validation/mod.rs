// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Input validation module.
//!
//! This module provides validation for user inputs to prevent security vulnerabilities.

pub mod path;
pub mod sanitize;

pub use path::{PathValidator, PathValidatorConfig, ValidationResult};
pub use sanitize::{EscapeMode, LogSanitizer, SanitizerConfig};
