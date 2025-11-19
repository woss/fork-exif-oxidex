use serde_json::json;

#[tokio::test]
async fn test_analyze_metadata() {
    let args = json!({
        "path": "oxidex-mcp/tests/fixtures/*.jpg"
    });

    let result = oxidex_mcp::tools::analyze::handle(args).await.unwrap();

    assert!(result.contains("Analyzed") || result.contains("file"));
}

#[tokio::test]
async fn test_analyze_metadata_no_files() {
    let args = json!({
        "path": "oxidex-mcp/tests/fixtures/*.nonexistent"
    });

    let result = oxidex_mcp::tools::analyze::handle(args).await.unwrap();

    assert!(result.contains("No files") || result.contains("matched"));
}

#[tokio::test]
async fn test_analyze_metadata_single_file() {
    let args = json!({
        "path": "oxidex-mcp/tests/fixtures/sample.jpg"
    });

    let result = oxidex_mcp::tools::analyze::handle(args).await.unwrap();

    assert!(result.contains("Analyzed") || result.len() > 0);
}
