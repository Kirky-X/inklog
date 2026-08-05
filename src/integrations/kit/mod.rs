// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! trait-kit 0.4 `AsyncKit` integration for inklog.
//!
//! Provides:
//! - [`InklogModule`]: DI module wrapping inklog's `Database` abstraction
//! - [`InklogBuildObserver`]: build pipeline observability via `BuildObserver`
//! - [`create_inklog_scope`]: per-request scoped dependency isolation

pub mod module;
pub mod observer;
pub mod scope;

pub use module::InklogModule;
pub use observer::InklogBuildObserver;
pub use scope::{create_inklog_scope, populate_inklog_scope};
