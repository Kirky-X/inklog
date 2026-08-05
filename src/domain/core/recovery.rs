// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Sink recovery control messages.

/// Messages used to control sink recovery and status queries.
#[derive(Debug, Clone)]
pub(crate) enum SinkControlMessage {
    RecoverSink(String), // sink name
    /// Query sink status (used in tests; production code only sends `RecoverSink`).
    #[allow(dead_code)]
    GetStatus,
}
