use serde_json::json;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_copy_metadata_dry_run() {
    let source = NamedTempFile::new().unwrap();
    let dest = NamedTempFile::new().unwrap();

    let args = json!({
        "source": source.path().to_str().unwrap(),
        "destination": dest.path().to_str().unwrap(),
        "dry_run": true
    });

    let result = oxidex_mcp::tools::copy::handle(args).await.unwrap();

    assert!(result.contains("[DRY RUN]") || result.contains("Would copy"));
}

#[tokio::test]
async fn test_copy_metadata_actual_copy() {
    let source = NamedTempFile::new().unwrap();
    let dest = NamedTempFile::new().unwrap();

    let args = json!({
        "source": source.path().to_str().unwrap(),
        "destination": dest.path().to_str().unwrap(),
        "dry_run": false
    });

    let result = oxidex_mcp::tools::copy::handle(args).await.unwrap();

    assert!(result.contains("Successfully") || result.contains("copied"));
}

#[tokio::test]
async fn test_copy_metadata_source_not_found() {
    let dest = NamedTempFile::new().unwrap();

    let args = json!({
        "source": "/nonexistent/path/file.jpg",
        "destination": dest.path().to_str().unwrap(),
        "dry_run": false
    });

    let result = oxidex_mcp::tools::copy::handle(args).await.unwrap();

    assert!(result.contains("Source file not found") || result.contains("not found"));
}

#[tokio::test]
async fn test_copy_metadata_dest_not_found() {
    let source = NamedTempFile::new().unwrap();

    let args = json!({
        "source": source.path().to_str().unwrap(),
        "destination": "/nonexistent/path/file.jpg",
        "dry_run": false
    });

    let result = oxidex_mcp::tools::copy::handle(args).await.unwrap();

    assert!(result.contains("Destination file not found") || result.contains("not found"));
}
