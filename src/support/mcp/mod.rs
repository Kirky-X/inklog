// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP（Model Context Protocol）stdio 工具面（feature `mcp`）。
//!
//! 安全策略在工具内部强制执行（见 design.md）：Agent 参数不可越权。

#[cfg(feature = "mcp")]
pub mod protocol;
#[cfg(feature = "mcp")]
pub mod tools;
