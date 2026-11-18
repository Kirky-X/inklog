// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! MCP JSON-RPC 2.0 帧层（feature `mcp`）。
//!
//! stdio 传输按行分帧（newline-delimited JSON）。单帧解析失败回标准
//! JSON-RPC 错误响应（parse error -32700），循环继续——一行坏帧不杀服务器。

use serde::{Deserialize, Serialize};

/// JSON-RPC 请求 id：字符串或数值。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum RequestId {
    /// 字符串 id
    Str(String),
    /// 数值 id
    Num(i64),
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => Ok(RequestId::Str(s)),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(RequestId::Num)
                .ok_or_else(|| serde::de::Error::custom("numeric id out of i64 range")),
            _ => Err(serde::de::Error::custom("id must be string or number")),
        }
    }
}

/// JSON-RPC 2.0 请求信封。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    /// 协议版本（必须为 "2.0"）
    pub jsonrpc: String,
    /// 请求 id
    pub id: RequestId,
    /// 方法名
    pub method: String,
    /// 参数（对象或省略）
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 错误体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// 错误码
    pub code: i32,
    /// 简述
    pub message: String,
}

/// JSON-RPC 2.0 响应信封。成功与错误二选一。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// 协议版本
    pub jsonrpc: String,
    /// 对应请求 id
    pub id: RequestId,
    /// 成功结果（错误时为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 错误体（成功时为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// 标准错误码：方法不存在。
pub const METHOD_NOT_FOUND: i32 = -32601;
/// 标准错误码：解析错误。
pub const PARSE_ERROR: i32 = -32700;
/// 标准错误码：无效请求。
pub const INVALID_REQUEST: i32 = -32600;

/// 解析单行 JSON-RPC 帧。
///
/// 返回 `Ok(None)` 表示空行（忽略）；`Err` 表示坏帧，调用方应回 parse error
/// 响应并继续循环。
pub fn parse_frame(line: &str) -> Result<Option<JsonRpcRequest>, serde_json::Error> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str::<JsonRpcRequest>(line).map(Some)
}

/// 构造成功响应。
pub fn ok_response(id: RequestId, result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

/// 构造错误响应。
pub fn error_response(id: RequestId, code: i32, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
        }),
    }
}

/// 方法分发骨架：返回该方法的 result 值（T005/T006 填实现），未知方法回 -32601。
///
/// `tools` 参数为 tools/list 应返回的工具清单（骨架阶段可为空集）。
pub fn dispatch(
    request: &JsonRpcRequest,
    tools: &[serde_json::Value],
) -> Result<serde_json::Value, JsonRpcError> {
    if request.jsonrpc != "2.0" {
        return Err(JsonRpcError {
            code: INVALID_REQUEST,
            message: "jsonrpc must be \"2.0\"".to_string(),
        });
    }
    match request.method.as_str() {
        "initialize" => Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "inklog", "version": env!("CARGO_PKG_VERSION") },
        })),
        "tools/list" => Ok(serde_json::json!({ "tools": tools })),
        "tools/call" => Ok(serde_json::json!({ "content": [], "isError": false })),
        "ping" => Ok(serde_json::json!({})),
        other => Err(JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: format!("method not found: {other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response_has_capabilities_tools() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Num(1),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        };
        let result = dispatch(&req, &[]).expect("initialize must succeed");
        assert!(result["capabilities"].get("tools").is_some());
    }

    #[test]
    fn test_bad_frame_does_not_poison_next_parse() {
        assert!(parse_frame("{-not-json").is_err(), "bad frame must error");
        let good = parse_frame(r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#)
            .expect("good frame must parse")
            .expect("non-empty");
        assert_eq!(good.method, "ping");
        assert_eq!(good.id, RequestId::Num(2));
    }

    #[test]
    fn test_tools_list_returns_empty_skeleton() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Str("a".to_string()),
            method: "tools/list".to_string(),
            params: serde_json::Value::Null,
        };
        let result = dispatch(&req, &[]).expect("tools/list must succeed");
        assert_eq!(result["tools"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn test_unknown_method_returns_minus_32601() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Num(3),
            method: "resources/list".to_string(),
            params: serde_json::Value::Null,
        };
        let err = dispatch(&req, &[]).expect_err("unknown method must error");
        assert_eq!(err.code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_error_response_serializes_jsonrpc_id_and_error() {
        let resp = error_response(RequestId::Str("x".to_string()), METHOD_NOT_FOUND, "no");
        let value = serde_json::to_value(&resp).expect("serialize");
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], "x");
        assert_eq!(value["error"]["code"], METHOD_NOT_FOUND);
        assert!(value.get("result").is_none());
    }
}
