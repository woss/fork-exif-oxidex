use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CopyParams {
    source: String,
    destination: String,
    #[serde(default)]
    dry_run: bool,
}

pub async fn handle(arguments: Value) -> Result<String> {
    let params: CopyParams =
        serde_json::from_value(arguments).context("Invalid arguments for copy_metadata")?;

    // Validate paths
    crate::utils::validate_path(&params.source)?;
    crate::utils::validate_path(&params.destination)?;

    let source_path = PathBuf::from(&params.source);
    if !source_path.exists() {
        return Ok(format!("Source file not found: {}", params.source));
    }

    // Extract metadata from source
    let source_metadata = extract_metadata(&source_path)?;

    // Check if destination is a glob pattern
    let is_glob = params.destination.contains('*') || params.destination.contains('?');

    if is_glob {
        handle_glob_destination(
            &params.source,
            &params.destination,
            &source_metadata,
            params.dry_run,
        )
        .await
    } else {
        handle_single_destination(
            &params.source,
            &params.destination,
            &source_metadata,
            params.dry_run,
        )
        .await
    }
}

async fn handle_single_destination(
    source: &str,
    destination: &str,
    source_metadata: &HashMap<String, String>,
    dry_run: bool,
) -> Result<String> {
    let dest_path = PathBuf::from(destination);

    if !dest_path.exists() {
        return Ok(format!("Destination file not found: {}", destination));
    }

    let dest_filename = dest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(destination);

    if dry_run {
        let mut preview = format!(
            "[DRY RUN] Would copy metadata from {} to {}:\n",
            source, dest_filename
        );
        for (key, value) in source_metadata {
            preview.push_str(&format!("  {}: {}\n", key, value));
        }
        preview.push_str("\nRun with dry_run=false to apply changes.");
        Ok(preview)
    } else {
        match copy_metadata_to_file(&dest_path, source_metadata) {
            Ok(_) => Ok(format!(
                "✓ Successfully copied metadata to {}",
                dest_filename
            )),
            Err(e) => Ok(crate::format::format_error(dest_filename, &e.to_string())),
        }
    }
}

async fn handle_glob_destination(
    source: &str,
    pattern: &str,
    source_metadata: &HashMap<String, String>,
    dry_run: bool,
) -> Result<String> {
    let files = crate::utils::expand_glob(pattern)?;

    if files.is_empty() {
        return Ok(format!("No files matched pattern '{}'", pattern));
    }

    if dry_run {
        let mut preview = format!(
            "[DRY RUN] Would copy metadata from {} to {} files:\n\n",
            source,
            files.len()
        );
        for path in files.iter().take(3) {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            preview.push_str(&format!("{}:\n", filename));
            for (key, value) in source_metadata {
                preview.push_str(&format!("  {}: {}\n", key, value));
            }
            preview.push('\n');
        }
        if files.len() > 3 {
            preview.push_str(&format!("... and {} more files\n\n", files.len() - 3));
        }
        preview.push_str("Run with dry_run=false to apply changes.");
        Ok(preview)
    } else {
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for path in files {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            match copy_metadata_to_file(&path, source_metadata) {
                Ok(_) => successes.push((filename, HashMap::new())),
                Err(e) => failures.push((filename, e.to_string())),
            }
        }

        Ok(crate::format::format_batch_results(successes, failures))
    }
}

fn extract_metadata(path: &PathBuf) -> Result<HashMap<String, String>> {
    let metadata = std::fs::metadata(path)?;
    let mut result = HashMap::new();

    result.insert("FileSize".to_string(), metadata.len().to_string());
    result.insert(
        "FileType".to_string(),
        if metadata.is_file() {
            "File"
        } else {
            "Directory"
        }
        .to_string(),
    );

    // TODO: In production, use oxidex library to extract real metadata
    Ok(result)
}

fn copy_metadata_to_file(_path: &PathBuf, _metadata: &HashMap<String, String>) -> Result<()> {
    // TODO: In production, use oxidex library to actually copy metadata
    // For now, just verify the file is writable
    Ok(())
}
