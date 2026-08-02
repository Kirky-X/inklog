// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Domain core module - core engine components.

pub mod container;
pub mod manager;
pub mod subscriber;

// Submodules extracted from manager.rs for maintainability
mod builder;
mod http_server;
mod recovery;
mod workers;

pub use builder::{LoggerBuilder, LoggerDependencies};
pub use container::{InklogContainer, InklogContainerBuilder};
pub use manager::LoggerManager;
pub use subscriber::LoggerSubscriber;
