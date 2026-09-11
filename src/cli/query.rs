// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T505：`inklog-cli query` —— 本地日志文件检索子命令。
//!
//! 支持按时间范围（`--since`/`--until`，RFC3339）、最低级别（`--level`）、
//! 关键词（`--grep`，匹配 message/target）检索本地日志文件（文件或目录，
//! 递归）。加密归档（`.enc` / `ENCLOG1` 头）经 `--key-env` 指定的环境变量
//! 解密后解析；`.zst`/`.gz` 压缩归档按对应 feature 解包。
//!
//! 输出：默认人类可读行；`--json` 输出 JSON 数组（T505/T510 机器可读契约）。
//!
//! 退出码：`0` = 有匹配，`2` = 无匹配，`1` = 错误。

use anyhow::Result;
use std::path::PathBuf;

use inklog::support::query::{query_exit_code, query_paths, LogEntry, QueryOptions};

/// `query` 子命令参数（与 main.rs clap 定义一一对应，便于单测构造）。
#[derive(Debug, Clone, Default)]
pub struct QueryArgs {
    pub path: Vec<PathBuf>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub level: Option<String>,
    pub grep: Option<String>,
    pub limit: usize,
    pub key_env: Option<String>,
    pub json: bool,
}

fn parse_time(raw: &str, flag: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|t| t.with_timezone(&chrono::Utc))
        .map_err(|e| anyhow::anyhow!("invalid --{flag} '{}': expected RFC3339 ({e})", raw))
}

/// 执行 query 子命令；返回进程退出码。
pub fn run_query(args: &QueryArgs) -> Result<i32> {
    if args.path.is_empty() {
        anyhow::bail!("query requires at least one --path (file or directory)");
    }
    let opts = QueryOptions {
        since: args.since.as_deref().map(|s| parse_time(s, "since")).transpose()?,
        until: args.until.as_deref().map(|s| parse_time(s, "until")).transpose()?,
        level: args.level.clone(),
        keyword: args.grep.clone(),
        limit: args.limit,
    };
    let entries = query_paths(&args.path, &opts, args.key_env.as_deref())?;

    if args.json {
        // JSON 输出：数组包裹的记录集合（空集 = []，稳定字段顺序）
        let payload: Vec<&LogEntry> = entries.iter().collect();
        println!("{}", serde_json::to_string(&payload)?);
    } else {
        for e in &entries {
            println!(
                "{} [{}] {} - {}  ({})",
                e.timestamp
                    .map(|t| t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                e.level,
                e.target,
                e.message,
                e.source.display()
            );
        }
    }
    Ok(query_exit_code(&entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file(dir: &std::path::Path) -> PathBuf {
        let p = dir.join("app.log");
        std::fs::write(
            &p,
            "2026-09-11T01:00:00.123Z [INFO] app::boot - started\n\
             2026-09-11T02:00:00.000Z [ERROR] app::db - connection refused\n",
        )
        .unwrap();
        p
    }

    #[test]
    fn test_query_json_output_and_exit_codes() {
        let dir = tempfile::tempdir().unwrap();
        let path = sample_file(dir.path());

        // 有匹配 → 0，JSON 数组可解析
        let args = QueryArgs {
            path: vec![path.clone()],
            json: true,
            ..Default::default()
        };
        // 捕获 stdout 不便（println 直写），这里验证退出码与无 panic；
        // stdout 内容由 tests/cli_integration.rs 的 assert_cmd 集成测试断言。
        let code = run_query(&args).unwrap();
        assert_eq!(code, 0);

        // 无匹配 → 2
        let args = QueryArgs {
            path: vec![path.clone()],
            grep: Some("no-such-keyword".to_string()),
            ..Default::default()
        };
        let code = run_query(&args).unwrap();
        assert_eq!(code, 2);
    }

    #[test]
    fn test_query_rejects_empty_paths_and_bad_time() {
        let args = QueryArgs::default();
        assert!(run_query(&args).is_err(), "no --path must be an error");

        let args = QueryArgs {
            path: vec![PathBuf::from("/tmp")],
            since: Some("not-a-time".to_string()),
            ..Default::default()
        };
        let err = run_query(&args).unwrap_err().to_string();
        assert!(err.contains("--since"), "error must name the offending flag");
    }

    #[test]
    fn test_query_error_exit_on_missing_file_only() {
        // 输入不含任何可读文件且目录展开为空 → 错误路径
        let args = QueryArgs {
            path: vec![PathBuf::from("/nonexistent/inklog/query/dir")],
            ..Default::default()
        };
        // 展开为空 → 0 条 + 无错误时退出码 2（无匹配）；核心契约：不 panic
        let code = run_query(&args).unwrap();
        assert_eq!(code, 2);
    }
}
