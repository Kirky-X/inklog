// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Domain core module - core engine components.

pub mod container;
pub mod manager;
pub mod subscriber;

pub use container::{InklogContainer, InklogContainerBuilder};
pub use manager::{LoggerBuilder, LoggerDependencies, LoggerManager};
