// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Sink recovery control messages.

/// Messages used to control sink recovery.
#[derive(Debug, Clone)]
pub(crate) enum SinkControlMessage {
    RecoverSink(String), // sink name
}
