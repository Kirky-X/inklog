// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Sink recovery control messages.

/// Messages used to control sink recovery and status queries.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum SinkControlMessage {
    RecoverSink(String), // sink name
    GetStatus,
}
