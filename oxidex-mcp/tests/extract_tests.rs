use serde_json::json;

#[tokio::test]
async fn test_extract_metadata_single_file() {
    let args = json!({
        "path": "tests/fixtures/sample.jpg"
    });

    let result = oxidex_mcp::tools::extract::handle(args).await.unwrap();

    assert!(result.contains("sample.jpg"));
    // Result should contain some metadata
    assert!(result.len() > 30); // At least filename + some metadata
    assert!(result.contains("FileSize"));
    assert!(result.contains("FileType"));
}

#[tokio::test]
async fn test_extract_metadata_glob_pattern() {
    let args = json!({
        "path": "tests/fixtures/*.jpg"
    });

    let result = oxidex_mcp::tools::extract::handle(args).await.unwrap();

    assert!(result.contains("Found"));
    assert!(result.contains("file"));
}

#[tokio::test]
async fn test_extract_metadata_no_files_found() {
    let args = json!({
        "path": "tests/fixtures/nonexistent/*.xyz"
    });

    let result = oxidex_mcp::tools::extract::handle(args).await.unwrap();

    assert!(result.contains("No files matched"));
}
