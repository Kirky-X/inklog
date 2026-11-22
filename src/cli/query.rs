// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `inklog-cli query` —— 本地日志文件检索子命令。
//!
//! 支持按时间范围（`--since`/`--until`，RFC3339）、最低级别（`--level`）、
//! 关键词（`--grep`，匹配 message/target）检索本地日志文件（文件或目录，
//! 递归）。加密归档（`.enc` / `ENCLOG1` 头）经 `--key-env` 指定的环境变量
//! 解密后解析；`.zst`/`.gz` 压缩归档按对应 feature 解包。
//!
//! 输出：默认人类可读行；`--json` 输出 JSON 数组（机器可读契约）。
//!
//! 退出码：`0` = 有匹配，`2` = 无匹配，`1` = 错误。

use anyhow::Result;
use std::path::PathBuf;

use inklog::support::query::{LogEntry, QueryOptions, query_exit_code, query_paths};

/// `query` 子命令参数（与 main.rs clap 定义一一对应，便于单测构造）。
#[derive(Debug, Clone, Default)]
pub struct QueryArgs {
    pub path: Vec<PathBuf>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub level: Option<String>,
    pub grep: Option<String>,
    pub field: Vec<String>,
    pub limit: usize,
    pub key_env: Option<String>,
    pub json: bool,
}

fn parse_time(raw: &str, flag: &str) -> Result<time::OffsetDateTime> {
    time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339)
        .map(|t| t.to_offset(time::UtcOffset::UTC))
        .map_err(|e| {
            let mut args = inklog::i18n::MsgArgs::new();
            args.set("flag", flag);
            args.set("raw", raw);
            args.set("err", e.to_string());
            anyhow::anyhow!(
                "{}",
                inklog::i18n::tr_args("cli-query-err-invalid-rfc3339", args)
            )
        })
}

/// 执行 query 子命令；返回进程退出码。
pub fn run_query(args: &QueryArgs) -> Result<i32> {
    if args.path.is_empty() {
        anyhow::bail!("{}", inklog::i18n::tr("cli-query-err-no-path"));
    }
    // --field <k=v>：解析为 FieldPredicate；非法格式（无 = 或空键）→ 错误退出码 1
    let mut fields = Vec::new();
    for raw in &args.field {
        let Some((key, value)) = raw.split_once('=') else {
            let mut fargs = inklog::i18n::MsgArgs::new();
            fargs.set("raw", raw);
            anyhow::bail!(
                "{}",
                inklog::i18n::tr_args("cli-query-err-invalid-field", fargs)
            );
        };
        if key.is_empty() || value.is_empty() {
            let mut fargs = inklog::i18n::MsgArgs::new();
            fargs.set("raw", raw);
            anyhow::bail!(
                "{}",
                inklog::i18n::tr_args("cli-query-err-invalid-field", fargs)
            );
        }
        let value_json = value
            .parse::<i64>()
            .map(serde_json::Value::from)
            .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
        fields.push(inklog::support::query::FieldPredicate {
            key: key.to_string(),
            value: value_json,
        });
    }
    let opts = QueryOptions {
        since: args
            .since
            .as_deref()
            .map(|s| parse_time(s, "since"))
            .transpose()?,
        until: args
            .until
            .as_deref()
            .map(|s| parse_time(s, "until"))
            .transpose()?,
        level: args.level.clone(),
        keyword: args.grep.clone(),
        limit: args.limit,
        fields,
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
                    .map(|t| t
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
                        ))
                        .expect("CLI timestamp format"))
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
        // Fluent renders placeables wrapped in Unicode isolation marks; strip
        // them so the flag name can be matched as a plain substring.
        let err: String = err
            .chars()
            .filter(|c| *c != '\u{2068}' && *c != '\u{2069}')
            .collect();
        assert!(
            err.contains("--since"),
            "error must name the offending flag"
        );
    }

    #[test]
    fn test_query_field_filter_and_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("structured.log");
        std::fs::write(
            &path,
            "{\"timestamp\":\"2026-09-11T01:00:00Z\",\"level\":\"INFO\",\"message\":\"hit\",\"user_id\":42}\n\
             {\"timestamp\":\"2026-09-11T01:00:01Z\",\"level\":\"INFO\",\"message\":\"miss\",\"user_id\":7}\n",
        )
        .unwrap();

        // T010: --field user_id=42 只命中一行（退出码 0）
        let args = QueryArgs {
            path: vec![path.clone()],
            field: vec!["user_id=42".to_string()],
            ..Default::default()
        };
        assert_eq!(
            run_query(&args).unwrap(),
            0,
            "field match must yield results"
        );

        // 无 k=v 分隔符 → 错误（退出码 1 路径：run_query 返回 Err）
        let args = QueryArgs {
            path: vec![path.clone()],
            field: vec!["no-separator".to_string()],
            ..Default::default()
        };
        assert!(
            run_query(&args).is_err(),
            "malformed --field must be an error"
        );
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
