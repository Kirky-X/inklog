// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
