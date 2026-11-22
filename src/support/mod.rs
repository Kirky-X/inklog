// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Support layer module - functional support layer.

pub mod audit_chain;
pub mod fallback_journal;
pub mod io;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod observability;
pub mod ops_event;
pub mod processing;
pub mod query;
// security：安全姿态评估属核心代码（不随 feature 门控）；KMS 密钥提供者
// 部分在文件内挂 feature = "kms" 门控（kms 内嵌模块）
pub mod security;
#[cfg(test)]
pub mod test_support;
pub(crate) mod time_format;
