// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Domain core module - core engine components.

pub mod container;
pub mod manager;
pub mod subscriber;

pub use container::{InklogContainer, InklogContainerBuilder};
pub use manager::{LoggerBuilder, LoggerDependencies, LoggerManager};
