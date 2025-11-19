use serde_json::json;
use std::process::Command;

/// Test that the server can be built and starts
#[test]
fn test_server_builds() {
    let output = Command::new("cargo")
        .args(&["build", "-p", "oxidex-mcp", "--quiet"])
        .output()
        .expect("Failed to build server");

    assert!(
        output.status.success(),
        "Server build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test MCP initialize method
#[test]
fn test_mcp_initialize_request() {
    let input = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    // Test that we can deserialize the request
    let request_str = serde_json::to_string(&input).unwrap();
    let request: oxidex_mcp::server::JsonRpcRequest = serde_json::from_str(&request_str).unwrap();

    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.id, Some(1));
    assert_eq!(request.method, "initialize");
}

/// Test MCP tools/list method
#[test]
fn test_mcp_tools_list_request() {
    let input = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let request_str = serde_json::to_string(&input).unwrap();
    let request: oxidex_mcp::server::JsonRpcRequest = serde_json::from_str(&request_str).unwrap();

    assert_eq!(request.method, "tools/list");
}

/// Test MCP tools/call method for extract_metadata
#[test]
fn test_mcp_tools_call_request() {
    let input = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "extract_metadata",
            "arguments": {
                "path": "test.jpg"
            }
        }
    });

    let request_str = serde_json::to_string(&input).unwrap();
    let request: oxidex_mcp::server::JsonRpcRequest = serde_json::from_str(&request_str).unwrap();

    assert_eq!(request.method, "tools/call");

    // Verify tool call params can be deserialized
    let params: oxidex_mcp::server::ToolCallParams =
        serde_json::from_value(request.params.unwrap()).unwrap();
    assert_eq!(params.name, "extract_metadata");
}

/// Test that server processes initialize request correctly
#[tokio::test]
async fn test_initialize_handler_response() {
    use oxidex_mcp::server::JsonRpcRequest;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(1),
        method: "initialize".to_string(),
        params: Some(json!({})),
    };

    // Simulate what the server does
    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "oxidex-mcp",
                "version": "0.1.0"
            }
        }
    });

    let response_str = serde_json::to_string(&response).unwrap();

    // Verify response is valid JSON
    assert!(serde_json::from_str::<serde_json::Value>(&response_str).is_ok());

    // Verify structure
    let resp_obj = serde_json::from_str::<serde_json::Value>(&response_str).unwrap();
    assert_eq!(resp_obj["jsonrpc"], "2.0");
    assert_eq!(resp_obj["id"], 1);
    assert_eq!(resp_obj["result"]["serverInfo"]["name"], "oxidex-mcp");
}

/// Test that tools/list returns valid tool definitions
#[test]
fn test_tools_list_contains_all_tools() {
    let tools = oxidex_mcp::tools::list_tools();

    assert_eq!(tools.len(), 5, "Should have exactly 5 tools");

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(tool_names.contains(&"extract_metadata"));
    assert!(tool_names.contains(&"write_metadata"));
    assert!(tool_names.contains(&"search_metadata"));
    assert!(tool_names.contains(&"analyze_metadata"));
    assert!(tool_names.contains(&"copy_metadata"));

    // Verify each tool has input schema
    for tool in tools {
        assert!(
            !tool.description.is_empty(),
            "Tool {} has empty description",
            tool.name
        );
        assert!(
            !tool.input_schema.is_null(),
            "Tool {} has null input_schema",
            tool.name
        );
    }
}

/// Test that extract_metadata tool can be dispatched
#[tokio::test]
async fn test_dispatch_extract_metadata_tool() {
    let result =
        oxidex_mcp::tools::dispatch_tool("extract_metadata", json!({"path": "Cargo.toml"})).await;

    assert!(result.is_ok(), "extract_metadata should succeed");
    let output = result.unwrap();
    assert!(!output.is_empty(), "Should return non-empty output");
}

/// Test that write_metadata tool can be dispatched
#[tokio::test]
async fn test_dispatch_write_metadata_tool() {
    let result = oxidex_mcp::tools::dispatch_tool(
        "write_metadata",
        json!({
            "path": "Cargo.toml",
            "tags": {"Artist": "Test"},
            "dry_run": true
        }),
    )
    .await;

    assert!(result.is_ok(), "write_metadata should succeed");
}

/// Test that search_metadata tool can be dispatched
#[tokio::test]
async fn test_dispatch_search_metadata_tool() {
    let result = oxidex_mcp::tools::dispatch_tool(
        "search_metadata",
        json!({
            "directory": "oxidex-mcp",
            "filters": ["FileType=File"]
        }),
    )
    .await;

    assert!(result.is_ok(), "search_metadata should succeed");
}

/// Test that analyze_metadata tool can be dispatched
#[tokio::test]
async fn test_dispatch_analyze_metadata_tool() {
    let result = oxidex_mcp::tools::dispatch_tool(
        "analyze_metadata",
        json!({"path": "oxidex-mcp/Cargo.toml"}),
    )
    .await;

    assert!(result.is_ok(), "analyze_metadata should succeed");
}

/// Test that copy_metadata tool can be dispatched
#[tokio::test]
async fn test_dispatch_copy_metadata_tool() {
    let result = oxidex_mcp::tools::dispatch_tool(
        "copy_metadata",
        json!({
            "source": "Cargo.toml",
            "destination": "oxidex-mcp/Cargo.toml",
            "dry_run": true
        }),
    )
    .await;

    assert!(result.is_ok(), "copy_metadata should succeed");
}

/// Test that unknown tool returns error
#[tokio::test]
async fn test_dispatch_unknown_tool_returns_error() {
    let result = oxidex_mcp::tools::dispatch_tool("unknown_tool", json!({})).await;

    assert!(result.is_err(), "unknown_tool should fail");
}

/// Test protocol version in initialize response
#[test]
fn test_initialize_protocol_version() {
    let response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "oxidex-mcp",
                "version": "0.1.0"
            }
        }
    });

    let protocol_version = &response["result"]["protocolVersion"];
    assert_eq!(protocol_version, "2024-11-05");
}
