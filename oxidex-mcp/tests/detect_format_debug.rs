use oxidex_mcp::tools;
use serde_json::json;

#[tokio::test]
async fn debug_detect_format_single_pdf() {
    let args = json!({
        "path": "tests/fixtures/pdf/sample.pdf"
    });

    let result = tools::detect_format::handle(args).await;
    match result {
        Ok(output) => {
            println!("===== OUTPUT START =====");
            println!("{}", output);
            println!("===== OUTPUT END =====");
        }
        Err(e) => {
            println!("===== ERROR =====");
            println!("{:?}", e);
        }
    }
}

#[tokio::test]
async fn debug_detect_format_single_png() {
    let args = json!({
        "path": "tests/fixtures/png/simple/synthetic_text_001.png"
    });

    let result = tools::detect_format::handle(args).await;
    match result {
        Ok(output) => {
            println!("===== OUTPUT START =====");
            println!("{}", output);
            println!("===== OUTPUT END =====");
        }
        Err(e) => {
            println!("===== ERROR =====");
            println!("{:?}", e);
        }
    }
}
