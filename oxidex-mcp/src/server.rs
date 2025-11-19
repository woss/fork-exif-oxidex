use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::BufRead;
use tokio::io::{AsyncBufReadExt, BufReader};

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response (success)
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Value,
}

/// JSON-RPC 2.0 Error Response
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: u64,
    pub error: ErrorObject,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// MCP Tools/Call Parameters
#[derive(Debug, Deserialize, Serialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}

/// MCP Tool Response Content
#[derive(Debug, Deserialize, Serialize)]
pub struct ToolResponse {
    pub content: Vec<ContentItem>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text { text: String },
}

/// Handle a single JSON-RPC request from stdin
pub async fn handle_single_request<R: BufRead>(reader: R) -> Result<JsonRpcResponse> {
    let mut lines = reader.lines();
    if let Some(line) = lines.next() {
        let line = line?;
        let request: JsonRpcRequest = serde_json::from_str(&line)?;

        // For now, just echo back a success response
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: serde_json::json!({"status": "ok"}),
        })
    } else {
        anyhow::bail!("No input received")
    }
}

/// Run the MCP server (reads from stdin, writes to stdout)
pub async fn run_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let request: JsonRpcRequest = serde_json::from_str(&line)?;

        let response = match request.method.as_str() {
            "initialize" => handle_initialize(request.id),
            "tools/list" => handle_tools_list(request.id),
            "tools/call" => handle_tool_call(request.id, request.params).await?,
            _ => create_error_response(request.id, -32601, "Method not found"),
        };

        println!("{}", serde_json::to_string(&response)?);
    }

    Ok(())
}

fn handle_initialize(id: u64) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "oxidex-mcp",
                "version": "0.1.0"
            }
        }),
    }
}

fn handle_tools_list(id: u64) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::json!({
            "tools": []
        }),
    }
}

async fn handle_tool_call(id: u64, params: Option<Value>) -> Result<JsonRpcResponse> {
    let params: ToolCallParams = serde_json::from_value(
        params.ok_or_else(|| anyhow::anyhow!("Missing params"))?
    )?;

    // Tool dispatch will be added later
    Ok(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::json!({
            "content": [{
                "type": "text",
                "text": "Tool not implemented yet"
            }]
        }),
    })
}

fn create_error_response(id: u64, code: i32, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::json!({
            "error": {
                "code": code,
                "message": message
            }
        }),
    }
}
