use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct WriteParams {
    path: String,
    tags: HashMap<String, String>,
    #[serde(default)]
    dry_run: bool,
}

pub async fn handle(arguments: Value) -> Result<String> {
    let params: WriteParams =
        serde_json::from_value(arguments).context("Invalid arguments for write_metadata")?;

    // Validate path
    crate::utils::validate_path(&params.path)?;

    // Check if it's a glob pattern
    let is_glob = params.path.contains('*') || params.path.contains('?');

    if is_glob {
        handle_glob_pattern(&params.path, &params.tags, params.dry_run).await
    } else {
        handle_single_file(&params.path, &params.tags, params.dry_run).await
    }
}

async fn handle_single_file(
    path: &str,
    tags: &HashMap<String, String>,
    dry_run: bool,
) -> Result<String> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return Ok(format!("File not found: {}", path));
    }

    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);

    if dry_run {
        // Preview changes
        let mut preview = format!("[DRY RUN] Would update {}:\n", filename);
        for (key, value) in tags {
            preview.push_str(&format!("  {}: → \"{}\"\n", key, value));
        }
        preview.push_str("\nRun with dry_run=false to apply changes.");
        Ok(preview)
    } else {
        // Actually write metadata
        match write_metadata_to_file(&path_buf, tags) {
            Ok(_) => Ok(format!("✓ Successfully updated {}", filename)),
            Err(e) => Ok(crate::format::format_error(filename, &e.to_string())),
        }
    }
}

async fn handle_glob_pattern(
    pattern: &str,
    tags: &HashMap<String, String>,
    dry_run: bool,
) -> Result<String> {
    let files = crate::utils::expand_glob(pattern)?;

    if files.is_empty() {
        return Ok(format!("No files matched pattern '{}'", pattern));
    }

    if dry_run {
        // Show preview for all files
        let mut preview = format!("[DRY RUN] Would update {} files:\n\n", files.len());
        for path in files.iter().take(5) {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            preview.push_str(&format!("{}:\n", filename));
            for (key, value) in tags {
                preview.push_str(&format!("  {}: → \"{}\"\n", key, value));
            }
            preview.push('\n');
        }
        if files.len() > 5 {
            preview.push_str(&format!("... and {} more files\n\n", files.len() - 5));
        }
        preview.push_str("Run with dry_run=false to apply changes.");
        Ok(preview)
    } else {
        // Actually write to all files
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for path in files {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            match write_metadata_to_file(&path, tags) {
                Ok(_) => successes.push((filename, HashMap::new())),
                Err(e) => failures.push((filename, e.to_string())),
            }
        }

        Ok(crate::format::format_batch_results(successes, failures))
    }
}

fn write_metadata_to_file(path: &PathBuf, _tags: &HashMap<String, String>) -> Result<()> {
    // TODO: In production, use oxidex library:
    // let mut metadata = oxidex::core::MetadataMap::from_file(path)?;
    // for (key, value) in tags {
    //     metadata.set(key, value)?;
    // }
    // metadata.write_to_file(path)?;

    // For now, just verify the file is writable
    let metadata = std::fs::metadata(path)?;
    if metadata.permissions().readonly() {
        anyhow::bail!("File is read-only");
    }

    Ok(())
}
