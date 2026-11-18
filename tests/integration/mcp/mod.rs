// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP 端到端用例（feature `mcp`）：生成日志 → spawn mcp 子命令 →
//! initialize / tools-list / tools-call(inklog_query + --field) 全链路断言。

use assert_cmd::Command;
use std::fs;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use tempfile::TempDir;

#[test]
fn mcp_stdio_full_chain() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("app.log");
    fs::write(
        &log,
        "{\"timestamp\":\"2026-09-11T01:00:00Z\",\"level\":\"INFO\",\"message\":\"hit\",\"user_id\":42}\n\
         {\"timestamp\":\"2026-09-11T01:00:01Z\",\"level\":\"INFO\",\"message\":\"miss\",\"user_id\":7}\n",
    )
    .unwrap();

    // 策略配置：允许查询该目录
    let cfg = dir.path().join("mcp.toml");
    fs::write(
        &cfg,
        format!("[mcp]\nallowed_roots = [\"{}\"]\n", dir.path().display()),
    )
    .unwrap();

    // 通过 assert_cmd 找到二进制路径，再用 std::process 写多行 stdin
    let bin = assert_cmd::cargo::cargo_bin("inklog-cli");
    let mut child = StdCommand::new(bin)
        .arg("mcp")
        .arg("--config")
        .arg(&cfg)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mcp server");

    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{{}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"inklog_query","arguments":{{"paths":["{}"],"field":"user_id=42"}}}}}}"#,
            log.display()
        )
        .unwrap();
    }
    let output = child
        .wait_with_output()
        .expect("mcp server must exit on EOF");
    assert!(output.status.success(), "mcp exit code must be 0");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let responses: Vec<serde_json::Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).expect("each line must be JSON-RPC response"))
        .collect();
    assert_eq!(
        responses.len(),
        3,
        "initialize/tools-list/tools-call → 3 frames"
    );

    // initialize: capabilities.tools 存在
    assert!(
        responses[0]["result"]["capabilities"]
            .get("tools")
            .is_some()
    );
    // tools/list: 三工具齐全
    let names: Vec<&str> = responses[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"inklog_query"));
    assert!(names.contains(&"inklog_audit_verify"));
    assert!(names.contains(&"inklog_stats"));
    // tools-call: 字段过滤只返回命中行（user_id=42）
    assert_eq!(responses[2]["result"]["isError"], false);
    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "field filter must leave one matching entry");
    assert!(lines[0].contains("hit"));
}

// 确保 assert_cmd 在依赖清单中被引用（避免 dead_code 提示）
#[allow(dead_code)]
fn _assert_cmd_marker() {
    let _ = Command::cargo_bin("inklog-cli");
}
