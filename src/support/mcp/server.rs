// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP stdio 服务器（feature `mcp`）：按行 JSON-RPC 帧循环。

use crate::domain::config::mcp::McpConfig;
use crate::support::mcp::protocol::{
    JsonRpcRequest, PARSE_ERROR, dispatch, error_response, ok_response, parse_frame,
};
use crate::support::mcp::tools::{AuditVerifyTool, McpTool, QueryTool, StatsTool};
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 组装当前策略启用的工具清单（tools/list 返回；allow 白名单过滤）。
fn enabled_tools(cfg: &McpConfig) -> Vec<Box<dyn McpTool>> {
    let all: Vec<Box<dyn McpTool>> = vec![
        Box::new(QueryTool::new(cfg.clone())),
        Box::new(AuditVerifyTool::new(cfg.clone())),
        Box::new(StatsTool::new(cfg.clone())),
    ];
    if cfg.allow.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|tool| cfg.allow.iter().any(|name| name == tool.name()))
        .collect()
}

/// 处理一帧请求，返回响应 JSON 行；EOF（None）返回空串表示循环结束。
///
/// 拆成独立函数便于测试：T007 Red 测试直接喂 initialize + tools/call 帧序列。
pub fn process_line(line: &str, cfg: &McpConfig) -> Option<String> {
    let request = match parse_frame(line) {
        Ok(Some(req)) => req,
        Ok(None) => return None,
        Err(e) => {
            // 坏帧：id 未知，用 null 占位（RequestId 不支持 null，这里手工构造响应）
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": { "code": PARSE_ERROR, "message": format!("parse error: {e}") },
            });
            return Some(resp.to_string());
        }
    };
    let response = handle_request(&request, cfg);
    Some(response.to_string())
}

fn handle_request(request: &JsonRpcRequest, cfg: &McpConfig) -> serde_json::Value {
    let tools = enabled_tools(cfg);
    let tool_list: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name(),
                "description": format!("{} tool", tool.name()),
                "inputSchema": tool.schema(),
            })
        })
        .collect();

    match dispatch(request, &tool_list) {
        Ok(result) => {
            // tools/call 骨架由 protocol 返回空 content；此处接真实工具实现
            if request.method == "tools/call" {
                let name = request
                    .params
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let args = request
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                if let Some(tool) = tools.iter().find(|t| t.name() == name) {
                    let tool_result = tool.call(args);
                    serde_json::to_value(ok_response(
                        request.id.clone(),
                        serde_json::json!({
                            "content": tool_result.content,
                            "isError": tool_result.is_error,
                        }),
                    ))
                    .unwrap_or_else(|_| serde_json::json!({}))
                } else {
                    serde_json::to_value(error_response(
                        request.id.clone(),
                        -32602,
                        format!("unknown tool: {name}"),
                    ))
                    .unwrap_or_else(|_| serde_json::json!({}))
                }
            } else {
                serde_json::to_value(ok_response(request.id.clone(), result))
                    .unwrap_or_else(|_| serde_json::json!({}))
            }
        }
        Err(err) => serde_json::to_value(error_response(request.id.clone(), err.code, err.message))
            .unwrap_or_else(|_| serde_json::json!({})),
    }
}

/// 运行 stdio 服务器：逐行读 stdin，回写 JSON-RPC 响应行；EOF 退出码 0。
pub async fn run_stdio_server(cfg: McpConfig) -> Result<i32> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = reader.next_line().await? {
        if let Some(response) = process_line(&line, &cfg) {
            stdout.write_all(response.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_then_tools_call_two_responses() {
        let cfg = McpConfig::default();
        let init = process_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            &cfg,
        )
        .expect("initialize must produce a response");
        let init_json: serde_json::Value = serde_json::from_str(&init).unwrap();
        assert!(init_json["result"]["capabilities"].get("tools").is_some());

        let list = process_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            &cfg,
        )
        .expect("tools/list must produce a response");
        let list_json: serde_json::Value = serde_json::from_str(&list).unwrap();
        let names: Vec<&str> = list_json["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"inklog_query"));
        assert!(names.contains(&"inklog_audit_verify"));
        assert!(names.contains(&"inklog_stats"));
    }

    #[test]
    fn test_tools_call_dispatches_to_stats() {
        let cfg = McpConfig::default();
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inklog_stats","arguments":{}}}"#;
        let resp = process_line(line, &cfg).expect("tools/call must produce a response");
        let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(json["result"]["isError"], false);
        let text = json["result"]["content"][0]["text"].as_str().unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(text).is_ok());
    }

    #[test]
    fn test_bad_frame_continues_and_empty_line_ignored() {
        let cfg = McpConfig::default();
        let bad = process_line("{bad", &cfg).expect("bad frame must produce error response");
        let bad_json: serde_json::Value = serde_json::from_str(&bad).unwrap();
        assert_eq!(bad_json["error"]["code"], PARSE_ERROR);
        assert!(
            process_line("", &cfg).is_none(),
            "empty line must be ignored"
        );
        let ping = process_line(
            r#"{"jsonrpc":"2.0","id":4,"method":"ping","params":{}}"#,
            &cfg,
        )
        .expect("valid frame after bad must still work");
        assert!(serde_json::from_str::<serde_json::Value>(&ping).is_ok());
    }
}
