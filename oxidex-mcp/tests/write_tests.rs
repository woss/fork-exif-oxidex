use serde_json::json;
use std::fs;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_write_metadata_dry_run() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();

    let args = json!({
        "path": path,
        "tags": {
            "Artist": "Test Artist",
            "Copyright": "© 2024"
        },
        "dry_run": true
    });

    let original_content = fs::read(path).unwrap();
    let result = oxidex_mcp::tools::write::handle(args).await.unwrap();
    let after_content = fs::read(path).unwrap();

    // File should not be modified
    assert_eq!(original_content, after_content);

    // Result should indicate dry-run
    assert!(result.contains("[DRY RUN]") || result.contains("Would update"));
}

#[tokio::test]
async fn test_write_metadata_actual_write() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();

    let args = json!({
        "path": path,
        "tags": {
            "Artist": "Test Artist"
        },
        "dry_run": false
    });

    let result = oxidex_mcp::tools::write::handle(args).await.unwrap();

    // Should indicate success
    assert!(result.contains("Successfully") || result.contains("updated"));
}
