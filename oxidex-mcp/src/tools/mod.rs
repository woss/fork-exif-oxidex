//! MCP Tool Handlers

pub mod analyze;
pub mod copy;
pub mod detect_format;
pub mod extract;
pub mod list_tags;
pub mod search;
pub mod tag_groups;
pub mod tag_info;
pub mod write;

use anyhow::Result;
use serde_json::Value;

/// Tool information for MCP tools/list
pub fn list_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "extract_metadata".to_string(),
            description: "Extract metadata from files (supports glob patterns like *.jpg)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path or glob pattern"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolInfo {
            name: "write_metadata".to_string(),
            description: "Write or update metadata tags (with dry-run support)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path or glob pattern"
                    },
                    "tags": {
                        "type": "object",
                        "description": "Key-value pairs to write"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Preview changes without applying (default: false)"
                    }
                },
                "required": ["path", "tags"]
            }),
        },
        ToolInfo {
            name: "search_metadata".to_string(),
            description: "Search for files by metadata criteria".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "Directory to search"
                    },
                    "filters": {
                        "type": "array",
                        "description": "Filter expressions (e.g., ['Make=Canon', 'DateTimeOriginal>2024-01-01'])"
                    }
                },
                "required": ["directory", "filters"]
            }),
        },
        ToolInfo {
            name: "analyze_metadata".to_string(),
            description: "Generate statistical summary of metadata".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path or glob pattern"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolInfo {
            name: "copy_metadata".to_string(),
            description: "Copy metadata from source to destination file(s)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Source file path"
                    },
                    "destination": {
                        "type": "string",
                        "description": "Destination file path or glob pattern"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Preview changes without applying"
                    }
                },
                "required": ["source", "destination"]
            }),
        },
        ToolInfo {
            name: "detect_format".to_string(),
            description: "Detect file format and show supported metadata operations".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path or glob pattern"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolInfo {
            name: "list_tags".to_string(),
            description: "List all available metadata tags, optionally filtered by group, format, or search term".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "group": {
                        "type": "string",
                        "description": "Filter by tag group (e.g., 'EXIF', 'XMP', 'IPTC', 'GPS')"
                    },
                    "format": {
                        "type": "string",
                        "description": "Filter by file format (e.g., 'JPEG', 'PNG', 'PDF')"
                    },
                    "writable": {
                        "type": "boolean",
                        "description": "Only show writable tags"
                    },
                    "search": {
                        "type": "string",
                        "description": "Search tags by name or description"
                    }
                }
            }),
        },
        ToolInfo {
            name: "get_tag_info".to_string(),
            description: "Get detailed information about a specific metadata tag including description, data type, and examples".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tag": {
                        "type": "string",
                        "description": "Tag name (e.g., 'EXIF:Make', 'XMP:Creator')"
                    }
                },
                "required": ["tag"]
            }),
        },
        ToolInfo {
            name: "list_tag_groups".to_string(),
            description: "List all metadata tag groups (EXIF, XMP, IPTC, etc.) with tag counts and format support".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "description": "Filter by file format"
                    }
                }
            }),
        },
    ]
}

#[derive(Debug, serde::Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Dispatch tool call to appropriate handler
pub async fn dispatch_tool(name: &str, arguments: Value) -> Result<String> {
    match name {
        "extract_metadata" => extract::handle(arguments).await,
        "write_metadata" => write::handle(arguments).await,
        "search_metadata" => search::handle(arguments).await,
        "analyze_metadata" => analyze::handle(arguments).await,
        "copy_metadata" => copy::handle(arguments).await,
        "detect_format" => detect_format::handle(arguments).await,
        "list_tags" => list_tags::handle(arguments).await,
        "get_tag_info" => tag_info::handle(arguments).await,
        "list_tag_groups" => tag_groups::handle(arguments).await,
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}
