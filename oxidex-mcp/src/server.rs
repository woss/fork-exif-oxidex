use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::BufRead;
use tokio::io::{AsyncBufReadExt, BufReader};

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
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

        let id = request
            .id
            .ok_or_else(|| anyhow::anyhow!("Missing id field"))?;

        // For now, just echo back a success response
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
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

        // Handle notifications (no id field, no response needed)
        if request.id.is_none() {
            tracing::debug!("Received notification: {}", request.method);
            // Notifications don't require a response
            continue;
        }

        let id = request.id.unwrap();

        let response_json = match request.method.as_str() {
            "initialize" => serde_json::to_string(&handle_initialize(id))?,
            "tools/list" => serde_json::to_string(&handle_tools_list(id))?,
            "tools/call" => serde_json::to_string(&handle_tool_call(id, request.params).await?)?,
            _ => serde_json::to_string(&create_error_response(id, -32601, "Method not found"))?,
        };

        println!("{}", response_json);
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
            },
            "instructions": "Use this server to extract, analyze, search, and manage EXIF and metadata from image files. Supports glob patterns for batch operations."
        }),
    }
}

fn handle_tools_list(id: u64) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::json!({
            "tools": crate::tools::list_tools()
        }),
    }
}

async fn handle_tool_call(id: u64, params: Option<Value>) -> Result<JsonRpcResponse> {
    let params: ToolCallParams =
        serde_json::from_value(params.ok_or_else(|| anyhow::anyhow!("Missing params"))?)?;

    let result = crate::tools::dispatch_tool(&params.name, params.arguments).await?;

    Ok(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::json!({
            "content": [{
                "type": "text",
                "text": result
            }]
        }),
    })
}

fn create_error_response(id: u64, code: i32, message: &str) -> JsonRpcError {
    JsonRpcError {
        jsonrpc: "2.0".to_string(),
        id,
        error: ErrorObject {
            code,
            message: message.to_string(),
            data: None,
        },
    }
}
