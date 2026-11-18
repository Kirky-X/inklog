// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `inklog-cli mcp` 子命令（feature `mcp`）：加载策略配置并启动 stdio 服务器。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// 启动 MCP stdio 服务器。
pub(crate) fn run_mcp(config_path: Option<PathBuf>) -> Result<i32> {
    let cfg = match config_path {
        Some(path) => load_config(&path)?,
        None => inklog::InklogConfig::default(),
    };
    cfg.mcp.validate().map_err(|e| anyhow::anyhow!("{}", e))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(inklog::run_stdio_server(cfg.mcp))
}

fn load_config(path: &Path) -> Result<inklog::InklogConfig> {
    let content = std::fs::read_to_string(path).with_context(|| {
        inklog::i18n::tr_args("cli-err-read-config", {
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("path", path.display().to_string());
            args
        })
    })?;
    content.parse().map_err(|e| anyhow::anyhow!("{}", e))
}
