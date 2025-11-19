use oxidex_mcp::tools;
use serde_json::json;

#[tokio::test]
async fn test_detect_format_single_jpeg() {
    let args = json!({
        "path": "tests/fixtures/sample.jpg"
    });

    let result = tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should succeed: {:?}", result);

    let output = result.unwrap();
    assert!(output.contains("sample.jpg"), "Output should contain filename");
    assert!(output.contains("Format: JPEG"), "Output should identify JPEG format");
    assert!(output.contains("MIME Type: image/jpeg"), "Output should show MIME type");
    assert!(output.contains("Supported Metadata:"), "Output should list metadata groups");
    assert!(output.contains("EXIF"), "JPEG should support EXIF metadata");
    assert!(output.contains("XMP"), "JPEG should support XMP metadata");
}

#[tokio::test]
async fn test_detect_format_glob_pattern() {
    let args = json!({
        "path": "tests/fixtures/*.jpg"
    });

    let result = tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should succeed with glob: {:?}", result);

    let output = result.unwrap();
    assert!(output.contains("Detected formats for"), "Output should show multiple files");
    assert!(output.contains("Format: JPEG"), "Output should identify JPEG format");
}

#[tokio::test]
async fn test_detect_format_nonexistent_file() {
    let args = json!({
        "path": "nonexistent_file.jpg"
    });

    let result = tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should handle missing file gracefully");

    let output = result.unwrap();
    assert!(output.contains("File not found"), "Output should indicate file not found");
}

#[tokio::test]
async fn test_detect_format_empty_glob() {
    let args = json!({
        "path": "tests/fixtures/*.xyz"
    });

    let result = tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should handle empty glob gracefully");

    let output = result.unwrap();
    assert!(output.contains("No files matched pattern"), "Output should indicate no matches");
}

#[tokio::test]
async fn test_detect_format_directory_traversal_blocked() {
    let args = json!({
        "path": "../../../etc/passwd"
    });

    let result = tools::detect_format::handle(args).await;
    assert!(result.is_err(), "detect_format should reject directory traversal");

    let error = result.unwrap_err();
    assert!(error.to_string().contains("directory traversal"),
        "Error should mention directory traversal");
}
