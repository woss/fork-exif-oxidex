use serde_json::json;
use std::io::Cursor;

#[test]
fn test_parse_jsonrpc_request() {
    let input = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "extract_metadata",
            "arguments": {
                "path": "test.jpg"
            }
        }
    });

    let request: oxidex_mcp::server::JsonRpcRequest = serde_json::from_value(input).unwrap();

    assert_eq!(request.id, 1);
    assert_eq!(request.method, "tools/call");
}

#[tokio::test]
async fn test_server_handles_request() {
    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let cursor = Cursor::new(input.as_bytes());

    let result = oxidex_mcp::server::handle_single_request(cursor).await;
    assert!(result.is_ok());
}
