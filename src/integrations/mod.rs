// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Integrations module - external service integrations.

#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub mod dbnexus_adapter;
pub mod infra;
#[cfg(all(
    feature = "kit",
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub mod kit;
