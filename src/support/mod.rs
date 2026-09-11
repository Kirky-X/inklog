// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Support layer module - functional support layer.

pub mod audit_chain;
pub mod io;
pub mod observability;
pub mod ops_event;
pub mod query;
pub mod processing;
#[cfg(feature = "kms")]
pub mod security;
