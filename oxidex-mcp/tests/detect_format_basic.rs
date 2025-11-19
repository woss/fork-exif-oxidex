// Basic integration test for detect_format tool
use serde_json::json;

#[tokio::test]
async fn test_detect_format_basic() {
    let args = json!({
        "path": "tests/fixtures/sample.jpg"
    });

    // Test that the tool can be called and returns a result
    let result = oxidex_mcp::tools::detect_format::handle(args).await;

    // Should succeed
    assert!(result.is_ok(), "detect_format should succeed: {:?}", result);

    let output = result.unwrap();
    println!("===== OUTPUT =====");
    println!("{}", output);
    println!("==================");

    // Basic checks
    assert!(output.contains("sample.jpg"), "Output should contain filename");
    assert!(output.contains("Format:"), "Output should show format");
    assert!(output.contains("MIME Type:"), "Output should show MIME type");
}
