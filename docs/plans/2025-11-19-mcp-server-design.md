# OxiDex MCP Server Design

**Date:** 2025-11-19
**Status:** Approved
**Purpose:** AI Assistant Integration for OxiDex metadata operations

## Overview

An MCP (Model Context Protocol) server that exposes OxiDex's metadata extraction and manipulation capabilities to AI assistants like Claude. This enables natural language interactions with file metadata, allowing users to ask questions like "What camera took this photo?" or "Add copyright to all my images."

## Use Case

**Primary:** AI Assistant Integration
Enable AI assistants to extract, analyze, search, and modify file metadata during conversations with users.

## Scope

### Operations Supported
- **Read**: Extract metadata from files
- **Write**: Modify metadata with dry-run preview support
- **Batch**: Process multiple files via glob patterns
- **Search**: Find files by metadata criteria
- **Analysis**: Generate statistical summaries

### File Access
- **Local files only**: No remote URLs or cloud storage
- **Glob pattern support**: `*.jpg`, `photos/**/*.raw`, etc.
- **No artificial limits**: Process any number/size of files

### Safety Features
- **Dry-run mode**: Preview changes before applying
- **Path validation**: Prevent directory traversal attacks
- **Graceful error handling**: Report issues clearly to AI assistant

## Architecture

### Core Components

1. **MCP Server Layer**
   - Implements MCP JSON-RPC protocol
   - Handles stdio communication with AI assistants
   - Registers and dispatches tool calls

2. **Tool Handlers**
   - Bridge between MCP and OxiDex library
   - Handle parameter validation and glob expansion
   - Format results as human-readable text

3. **OxiDex Integration**
   - Direct use of `oxidex::core::MetadataMap` API
   - Leverage existing parallel processing with rayon
   - Use existing format support (JPEG, PNG, RAW, PDF, MP4, etc.)

4. **Output Formatter**
   - Convert structured metadata to readable text
   - Format errors and warnings clearly
   - Present batch results with success/failure breakdown

### Technology Stack
- **Rust**: Native integration with OxiDex library
- **tokio**: Async runtime for concurrent requests
- **serde/serde_json**: JSON-RPC serialization
- **glob crate**: Pattern expansion
- **rayon**: Parallel file processing (from OxiDex)

### Communication Pattern
AI assistant ↔ stdio ↔ MCP Server ↔ OxiDex Library ↔ File System

## Tool Design

### 1. extract_metadata

**Purpose:** Extract metadata from files

**Parameters:**
- `path`: File path or glob pattern (e.g., `photo.jpg`, `*.jpg`, `photos/**/*.raw`)

**Returns:** Human-readable metadata listing

**Example:**
```
Found 3 files:

photo1.jpg:
  Make: Canon
  Model: EOS R5
  DateTimeOriginal: 2024-01-15 14:30:00
  GPS: 37.7749°N, 122.4194°W

photo2.jpg:
  Make: Sony
  Model: A7R IV
  DateTimeOriginal: 2024-01-20 09:15:00
```

### 2. write_metadata

**Purpose:** Write or update metadata tags

**Parameters:**
- `path`: File path or glob pattern
- `tags`: Key-value pairs to write (e.g., `{"Artist": "John Doe", "Copyright": "© 2024"}`)
- `dry_run`: Optional boolean (default: false)

**Returns:** Preview (dry-run) or confirmation of changes

**Example (dry-run):**
```
[DRY RUN] Would update 5 files:

photo1.jpg:
  Artist: (none) → "John Doe"
  Copyright: (none) → "© 2024 John Doe"

photo2.jpg:
  Artist: (none) → "John Doe"
  Copyright: (none) → "© 2024 John Doe"

...

Run with dry_run=false to apply changes.
```

### 3. search_metadata

**Purpose:** Find files matching metadata criteria

**Parameters:**
- `directory`: Directory to search (supports glob patterns)
- `filters`: Array of filter expressions
  - Format: `"TagName=Value"` (exact match)
  - Format: `"TagName>Value"` (greater than, for dates/numbers)
  - Format: `"TagName<Value"` (less than)
  - Format: `"TagName~Value"` (contains, for text)

**Returns:** List of matching files with relevant metadata

**Example:**
```
Found 12 files matching 'Make=Canon AND DateTimeOriginal>2024-01-01':

photo1.jpg
  Make: Canon
  Model: EOS R5
  DateTimeOriginal: 2024-01-15 14:30:00

photo2.jpg
  Make: Canon
  Model: EOS R6
  DateTimeOriginal: 2024-01-20 09:15:00

...
```

### 4. analyze_metadata

**Purpose:** Generate statistical summary of metadata across files

**Parameters:**
- `path`: File path or glob pattern

**Returns:** Statistical breakdown

**Example:**
```
Analyzed 50 files:

Cameras Used:
  Canon (30 files)
    - EOS R5: 20 files
    - EOS R6: 10 files
  Sony (20 files)
    - A7R IV: 15 files
    - A7 III: 5 files

Date Range: 2024-01-01 to 2024-03-15

GPS Coverage: 45/50 files (90%)

File Formats:
  JPEG: 30 files
  Canon RAW (CR2/CR3): 15 files
  Sony RAW (ARW): 5 files
```

### 5. copy_metadata

**Purpose:** Copy metadata from one file to another

**Parameters:**
- `source`: Source file path
- `destination`: Destination file path or glob pattern
- `dry_run`: Optional boolean (default: false)

**Returns:** Preview or confirmation

**Example:**
```
Copying metadata from source.jpg to 3 files:

target1.jpg:
  Will copy: Make, Model, DateTimeOriginal, Artist, Copyright

target2.jpg:
  Will copy: Make, Model, DateTimeOriginal, Artist, Copyright

...
```

## Data Flow

### Request Processing Flow

1. **AI assistant sends tool call** → JSON-RPC request via stdio
2. **MCP server receives & validates** → Parse parameters, validate paths
3. **Glob expansion** → Convert patterns to file list (e.g., `*.jpg` → `[photo1.jpg, photo2.jpg]`)
4. **Path validation** → Check for directory traversal, verify access
5. **OxiDex library calls** → For each file:
   - Read: `MetadataMap::from_file(path)`
   - Write: `metadata.set(key, value)` + `metadata.write_to_file(path)` (if not dry-run)
6. **Parallel processing** → Use rayon for batch operations
7. **Result formatting** → Convert to human-readable text
8. **Response** → Send to AI assistant via JSON-RPC

### Example: extract_metadata Flow

```
User: "What camera took these photos?"
AI: calls extract_metadata("photos/*.jpg")

MCP Server:
  1. Receives: {"method": "extract_metadata", "params": {"path": "photos/*.jpg"}}
  2. Expands glob: ["photos/img1.jpg", "photos/img2.jpg", "photos/img3.jpg"]
  3. Validates paths: ✓ All within working directory
  4. Parallel map: MetadataMap::from_file() for each file
  5. Formats output:
     "Found 3 files:

      img1.jpg:
        Make: Canon
        Model: EOS R5

      img2.jpg:
        Make: Canon
        Model: EOS R5

      img3.jpg:
        Make: Sony
        Model: A7R IV"
  6. Returns formatted text to AI

AI: "These photos were taken with two cameras: a Canon EOS R5 (2 photos) and a Sony A7R IV (1 photo)."
```

## Error Handling

### Principles
- **Graceful degradation**: Continue processing on partial failures
- **Clear messaging**: Human-readable errors for AI to communicate
- **No silent failures**: Always report what went wrong

### Error Scenarios

#### 1. File Not Found
```
Pattern: photos/*.jpg
Error: No files matched pattern 'photos/*.jpg' in current directory
```

#### 2. Unsupported Format
```
File: document.xyz
Error: Unsupported format (OxiDex supports JPEG, PNG, TIFF, RAW, PDF, MP4, MP3, FLAC, and 300+ other formats)

Batch behavior: Skip and continue with other files
```

#### 3. Corrupted/Unreadable Metadata
```
File: photo.jpg
Error: Could not parse EXIF data (file may be corrupted)
Extracted: Basic file info (size, modified date) only

Batch behavior: Report partial data, continue processing
```

#### 4. Permission Denied
```
File: /root/photo.jpg
Error: Permission denied (check file permissions)

Batch behavior: Skip and continue
```

#### 5. Batch Partial Failures
```
Processed 8/10 files successfully:

✓ photo1.jpg
✓ photo2.jpg
✓ photo3.jpg
✓ photo4.jpg
✓ photo5.jpg
✓ photo6.jpg
✓ photo7.jpg
✓ photo8.jpg
✗ photo9.jpg: Permission denied
✗ photo10.jpg: Corrupted EXIF data
```

#### 6. Invalid Filter Syntax (search_metadata)
```
Filter: "Make=="Canon"
Error: Invalid filter syntax. Use format: TagName=Value, TagName>Value, TagName<Value, or TagName~Value
```

#### 7. Write Conflicts
```
File: readonly.jpg
Error: Cannot write to read-only file (check file permissions)

Dry-run behavior: Show in preview with warning flag
```

### Error Response Format

All errors returned as human-readable text, not technical stack traces:
```
❌ Failed to process request:
  Reason: No files matched pattern '*.raw'
  Suggestion: Check the pattern and current directory
```

## Testing Strategy

### Unit Tests
- Tool handler functions (parameter validation, error cases)
- Glob pattern expansion logic
- Output formatting functions
- Path validation (security checks)
- Error message generation

### Integration Tests
- Full MCP protocol flow (JSON-RPC over stdio)
- Real OxiDex library calls with test fixtures
- All 5 tools with various file types
- Batch operations with multiple files
- Dry-run vs actual write behavior
- Error handling end-to-end

### Test Fixtures
Located in `oxidex-mcp/tests/fixtures/`:
- `sample.jpg` - JPEG with EXIF (Make: Canon, Model: EOS R5)
- `sample.png` - PNG with metadata
- `sample.cr2` - Canon RAW file
- `sample.pdf` - PDF with document metadata
- `sample.mp4` - Video with QuickTime atoms
- `corrupted.jpg` - Intentionally corrupted EXIF
- `readonly.jpg` - File with read-only permissions

### Property-Based Tests
Using `proptest`:
- Glob patterns always return valid paths
- Write operations never corrupt files (verify readability after write)
- Dry-run never modifies files
- Parallel processing produces same results as sequential

### Example Test
```rust
#[tokio::test]
async fn test_extract_metadata_with_glob() {
    let result = handle_extract_metadata(ExtractParams {
        path: "tests/fixtures/*.jpg".into(),
    }).await.unwrap();

    assert!(result.contains("sample.jpg"));
    assert!(result.contains("Make:"));
    assert!(result.contains("Canon"));
}

#[tokio::test]
async fn test_write_dry_run_does_not_modify() {
    let test_file = "tests/fixtures/sample.jpg";
    let original_hash = file_hash(test_file);

    handle_write_metadata(WriteParams {
        path: test_file.into(),
        tags: hashmap!{"Artist" => "Test"},
        dry_run: Some(true),
    }).await.unwrap();

    let after_hash = file_hash(test_file);
    assert_eq!(original_hash, after_hash, "Dry-run modified file!");
}
```

## Project Structure

```
oxidex/
├── Cargo.toml                    # Add oxidex-mcp to workspace.members
├── oxidex-mcp/                   # New workspace member
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs              # Entry point, stdio listener
│   │   ├── server.rs            # MCP protocol handler (JSON-RPC)
│   │   ├── tools/
│   │   │   ├── mod.rs           # Tool registration
│   │   │   ├── extract.rs       # extract_metadata implementation
│   │   │   ├── write.rs         # write_metadata implementation
│   │   │   ├── search.rs        # search_metadata implementation
│   │   │   ├── analyze.rs       # analyze_metadata implementation
│   │   │   └── copy.rs          # copy_metadata implementation
│   │   ├── format.rs            # Output formatting utilities
│   │   └── utils.rs             # Glob expansion, path validation
│   └── tests/
│       ├── integration_tests.rs
│       └── fixtures/            # Sample test files
│           ├── sample.jpg
│           ├── sample.png
│           ├── sample.cr2
│           ├── sample.pdf
│           └── sample.mp4
├── src/                         # Existing OxiDex library
├── oxidex-tags/                 # Existing tag database
└── ...
```

### Dependencies (oxidex-mcp/Cargo.toml)

```toml
[package]
name = "oxidex-mcp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "oxidex-mcp"
path = "src/main.rs"

[dependencies]
# OxiDex library
oxidex = { path = ".." }

# Async runtime
tokio = { version = "1", features = ["full"] }

# JSON-RPC / MCP protocol
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Glob pattern matching
glob = "0.3"

# Error handling
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
# Testing
tempfile = "3"
proptest = "1"
```

## Deployment & Usage

### Building

```bash
# Build release binary
cargo build --release -p oxidex-mcp

# Binary location
target/release/oxidex-mcp

# Install to system
cargo install --path oxidex-mcp
```

### MCP Client Configuration

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "oxidex": {
      "command": "/usr/local/bin/oxidex-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Other MCP Clients:** Similar configuration pointing to the `oxidex-mcp` binary

### Usage Examples

#### Example 1: Extracting Metadata
```
User: "What camera took this photo?"

AI Assistant:
  → Calls: extract_metadata("photo.jpg")
  ← Receives: "photo.jpg:\n  Make: Canon\n  Model: EOS R5\n  DateTimeOriginal: 2024-01-15 14:30:00"
  → Responds: "This photo was taken with a Canon EOS R5 on January 15, 2024 at 2:30 PM."
```

#### Example 2: Batch Copyright Update
```
User: "Add my copyright to all JPEGs in this folder"

AI Assistant:
  → Calls: write_metadata("*.jpg", {"Artist": "John Doe", "Copyright": "© 2024 John Doe"}, dry_run=true)
  ← Receives: "[DRY RUN] Would update 15 files:\n  photo1.jpg: Artist → 'John Doe'\n  ..."
  → Responds: "I can update 15 JPEG files with your copyright. Here's what would change: [shows preview]. Should I proceed?"

User: "Yes, go ahead"

AI Assistant:
  → Calls: write_metadata("*.jpg", {"Artist": "John Doe", "Copyright": "© 2024 John Doe"})
  ← Receives: "✓ Successfully updated 15 files"
  → Responds: "Done! I've added your copyright information to 15 files."
```

#### Example 3: Finding Photos by Camera
```
User: "Find all photos I took with my Canon camera last month"

AI Assistant:
  → Calls: search_metadata(".", ["Make=Canon", "DateTimeOriginal>2024-10-01", "DateTimeOriginal<2024-11-01"])
  ← Receives: "Found 23 files matching criteria:\n  photo1.jpg (Canon EOS R5, 2024-10-05)\n  ..."
  → Responds: "I found 23 photos taken with your Canon camera in October 2024. Here they are: ..."
```

### Configuration

**Environment Variables:**
- `RUST_LOG`: Logging level (debug, info, warn, error)
- Working directory: Defaults to where MCP client starts the server

**No configuration file needed** - server is stateless

## Implementation Notes

### MCP Protocol Implementation

The server implements MCP's JSON-RPC 2.0 specification:

```rust
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "extract_metadata",
    "arguments": {
      "path": "*.jpg"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Found 3 files:\n\nphoto1.jpg:\n  Make: Canon\n  ..."
      }
    ]
  }
}
```

### Security Considerations

1. **Path Validation**: Block `..` segments to prevent directory traversal
2. **File System Limits**: No artificial limits, but OS limits apply
3. **Read-Only by Default**: Write operations require explicit parameters
4. **Local Files Only**: No network access reduces attack surface
5. **No Code Execution**: Pure data processing, no eval or shell commands

### Performance Characteristics

- **Parallel Processing**: Batch operations use rayon for multi-core utilization
- **Memory Efficient**: Stream large files, don't load entirely into memory
- **Fast Startup**: Minimal initialization, responds to first request quickly
- **No State**: Stateless server, no caching between requests

## Success Criteria

1. ✅ All 5 tools implemented and working
2. ✅ Dry-run mode prevents accidental writes
3. ✅ Glob patterns work correctly (`*.jpg`, `**/*.raw`)
4. ✅ Human-readable output format
5. ✅ Graceful error handling with clear messages
6. ✅ MCP protocol compliance (JSON-RPC 2.0)
7. ✅ Comprehensive test coverage (unit + integration)
8. ✅ Works with Claude Desktop
9. ✅ Documentation for users

## Future Enhancements (Out of Scope)

- Remote file support (HTTP/HTTPS URLs)
- Cloud storage integration (S3, GCS)
- Streaming progress updates for long operations
- Caching/indexing for faster repeated queries
- Watch mode for monitoring file changes
- Undo/redo support for write operations
- Advanced search query language
- Metadata templates/presets

---

**Next Steps:**
1. Set up git worktree for isolated development
2. Create detailed implementation plan
3. Implement MCP server skeleton
4. Implement each tool handler
5. Add tests
6. Documentation and examples
