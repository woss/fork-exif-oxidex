//! ExifTool tag extractor - Extract tags by running exiftool -json on fixtures

use crate::models::TagInfo;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::process::Command;
use walkdir::WalkDir;

/// Extract tags from ExifTool by running exiftool CLI
pub struct ExifToolExtractor {
    exiftool_path: String,
    cache: HashMap<String, Vec<TagInfo>>,
}

impl ExifToolExtractor {
    /// Create a new ExifTool extractor
    pub fn new(exiftool_path: String) -> Self {
        Self {
            exiftool_path,
            cache: HashMap::new(),
        }
    }

    /// Extract tags from all fixtures of a specific format
    ///
    /// # Arguments
    /// * `format` - Format name (e.g., "JPEG", "PNG")
    ///
    /// # Returns
    /// Vector of TagInfo representing all unique tags found via ExifTool
    pub async fn extract_format_tags(
        &mut self,
        format: &str,
        fixture_path: &Path,
    ) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached) = self.cache.get(format) {
            return Ok(cached.clone());
        }

        let format_path = fixture_path.join(format.to_lowercase());
        if !format_path.exists() {
            return Ok(Vec::new());
        }

        // Find all fixture files for this format
        let files: Vec<PathBuf> = WalkDir::new(&format_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        if files.is_empty() {
            return Ok(Vec::new());
        }

        // Extract tags from each file
        let mut all_tags: HashMap<String, (TagInfo, usize)> = HashMap::new();

        for file_path in &files {
            match self.run_exiftool_on_file(file_path) {
                Ok(file_tags) => {
                    for tag_info in file_tags {
                        all_tags
                            .entry(format!("{}:{}", tag_info.family, tag_info.name))
                            .and_modify(|(_info, count)| *count += 1)
                            .or_insert((tag_info.clone(), 1));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to extract tags from {}: {}", file_path.display(), e);
                    // Continue processing other files
                }
            }
        }

        // Convert to final format
        let mut result: Vec<TagInfo> = all_tags
            .into_values()
            .map(|(tag_info, _count)| tag_info)
            .collect();

        // Sort by key for consistency
        result.sort_by(|a, b| a.key().cmp(&b.key()));

        // Cache the result
        self.cache.insert(format.to_string(), result.clone());

        Ok(result)
    }

    /// Run exiftool on a file and parse JSON output
    fn run_exiftool_on_file(&self, file_path: &Path) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
        let output = Command::new(&self.exiftool_path)
            .arg("-json")
            .arg(file_path)
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "ExifTool failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        let json: serde_json::Value = serde_json::from_str(&stdout)?;
        let tags = self.parse_exiftool_json(&json);

        Ok(tags)
    }

    /// Parse ExifTool JSON output into TagInfo
    fn parse_exiftool_json(&self, json: &serde_json::Value) -> Vec<TagInfo> {
        let mut tags = Vec::new();

        // ExifTool returns an array of objects, one per file
        if let Some(array) = json.as_array() {
            if let Some(file_data) = array.first() {
                if let Some(obj) = file_data.as_object() {
                    for (key, value) in obj.iter() {
                        let (family, name) = self.parse_tag_name(key);
                        if family != "UNKNOWN" {
                            let value_str = match value {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::Bool(b) => b.to_string(),
                                serde_json::Value::Array(_) => value.to_string(),
                                serde_json::Value::Object(_) => value.to_string(),
                                serde_json::Value::Null => "null".to_string(),
                            };
                            let tag_info = TagInfo::new(name, family, value_str);
                            tags.push(tag_info);
                        }
                    }
                }
            }
        }

        tags
    }

    /// Parse tag name to extract family and tag name
    /// "EXIF:Make" → ("EXIF", "Make")
    /// "ExifTool:Version" → ("ExifTool", "Version")
    fn parse_tag_name(&self, exiftool_name: &str) -> (String, String) {
        if let Some(colon_pos) = exiftool_name.find(':') {
            let (family, name) = exiftool_name.split_at(colon_pos);
            (family.to_string(), name[1..].to_string()) // Skip the ':'
        } else {
            ("UNKNOWN".to_string(), exiftool_name.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exiftool_extractor_creation() {
        let extractor = ExifToolExtractor::new("exiftool".to_string());
        assert_eq!(extractor.exiftool_path, "exiftool");
    }

    #[test]
    fn test_parse_tag_name_with_colon() {
        let extractor = ExifToolExtractor::new("exiftool".to_string());
        let (family, name) = extractor.parse_tag_name("EXIF:Make");
        assert_eq!(family, "EXIF");
        assert_eq!(name, "Make");
    }

    #[test]
    fn test_parse_tag_name_without_colon() {
        let extractor = ExifToolExtractor::new("exiftool".to_string());
        let (family, name) = extractor.parse_tag_name("SourceFile");
        assert_eq!(family, "UNKNOWN");
        assert_eq!(name, "SourceFile");
    }

    #[test]
    fn test_parse_tag_name_xmp() {
        let extractor = ExifToolExtractor::new("exiftool".to_string());
        let (family, name) = extractor.parse_tag_name("XMP:Creator");
        assert_eq!(family, "XMP");
        assert_eq!(name, "Creator");
    }

    #[test]
    fn test_parse_exiftool_json_empty() {
        let extractor = ExifToolExtractor::new("exiftool".to_string());
        let json = serde_json::json!([]);
        let tags = extractor.parse_exiftool_json(&json);
        assert_eq!(tags.len(), 0);
    }

    #[test]
    fn test_parse_exiftool_json_with_data() {
        let extractor = ExifToolExtractor::new("exiftool".to_string());
        let json = serde_json::json!([{
            "EXIF:Make": "Canon",
            "EXIF:Model": "Canon EOS 5D",
            "XMP:Creator": "John Doe"
        }]);
        let tags = extractor.parse_exiftool_json(&json);
        assert_eq!(tags.len(), 3);
        assert!(tags.iter().any(|t| t.name == "Make" && t.family == "EXIF"));
        assert!(tags.iter().any(|t| t.name == "Creator" && t.family == "XMP"));
    }
}
