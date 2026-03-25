// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! I/O module - log output adapters and sink implementations.

pub mod log_adapter;
pub mod sink;

pub use log_adapter::{LogAdapter, LogLogger};
