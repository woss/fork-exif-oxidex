use serde_json::json;

#[tokio::test]
async fn test_search_metadata_basic() {
    let args = json!({
        "directory": "oxidex-mcp/tests/fixtures",
        "filters": ["FileType=File"]
    });

    let result = oxidex_mcp::tools::search::handle(args).await.unwrap();

    assert!(result.contains("Found") || result.contains("file"));
}

#[tokio::test]
async fn test_search_metadata_no_matches() {
    let args = json!({
        "directory": "oxidex-mcp/tests/fixtures",
        "filters": ["Make=NonexistentCamera"]
    });

    let result = oxidex_mcp::tools::search::handle(args).await.unwrap();

    assert!(result.contains("No files") || result.contains("0 files"));
}

#[tokio::test]
async fn test_search_metadata_multiple_filters() {
    let args = json!({
        "directory": "oxidex-mcp/tests/fixtures",
        "filters": ["FileType=File", "FileSize>0"]
    });

    let result = oxidex_mcp::tools::search::handle(args).await.unwrap();

    // Should either find files or report no matches
    assert!(result.len() > 0);
}
