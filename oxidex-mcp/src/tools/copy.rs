use anyhow::Result;
use serde_json::Value;

pub async fn handle(_arguments: Value) -> Result<String> {
    Ok("copy_metadata not implemented yet".to_string())
}
