# MCP Server Integration

> **Status:** In Development - [Pull Request #10](https://github.com/swack-tools/oxidex/pull/10) 🔄

## Overview

The OxiDex MCP (Model Context Protocol) server enables AI assistants like Claude, Cline, and others to interact with OxiDex through natural language conversations. Instead of memorizing command-line options, users can simply ask questions and give instructions in plain English.

## What is MCP?

The Model Context Protocol is an open standard that allows AI assistants to access external tools and data sources. By implementing an MCP server, OxiDex becomes directly accessible to any MCP-compatible AI assistant.

## Features

### 5 Natural Language Tools

1. **extract_metadata** - Extract metadata from files
   - Supports glob patterns (`*.jpg`, `photos/**/*.raw`)
   - Works with single files or batches
   - Returns human-readable output

2. **write_metadata** - Write or update metadata
   - Dry-run mode to preview changes
   - Batch operations with glob patterns
   - Safe by default (requires confirmation)

3. **search_metadata** - Find files by metadata
   - Filter by any metadata field
   - Support for comparisons (`=`, `>`, `<`, `~`)
   - Returns matching files with relevant metadata

4. **analyze_metadata** - Statistical analysis
   - Aggregate metadata across multiple files
   - Find most common values
   - Calculate file size statistics

5. **copy_metadata** - Copy metadata between files
   - Dry-run preview mode
   - Batch copying with glob patterns
   - Preserves metadata structure

## Installation

### Prerequisites

- Rust 1.75+ (for building from source)
- Or download pre-built binary from releases

### Building the MCP Server

```bash
# Clone the repository
git clone https://github.com/swack-tools/oxidex.git
cd oxidex

# Build the MCP server
cargo build --release -p oxidex-mcp

# Binary will be at:
./target/release/oxidex-mcp
```

### Installation

```bash
# Install to system path
cargo install --path oxidex-mcp

# Or copy binary to a permanent location
sudo cp target/release/oxidex-mcp /usr/local/bin/
```

## Configuration

### Claude Desktop

Add to your Claude Desktop configuration file:

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

**Linux:** `~/.config/Claude/claude_desktop_config.json`

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

### Cline (VS Code Extension)

Add to your Cline MCP settings:

```json
{
  "mcpServers": {
    "oxidex": {
      "command": "/usr/local/bin/oxidex-mcp",
      "args": []
    }
  }
}
```

### Other MCP Clients

The server works with any MCP-compatible client. Refer to your client's documentation for configuration instructions.

## Usage Examples

### Extracting Metadata

**Question:** *"What camera took this photo?"*

```
User: What camera took this photo?
Assistant: [calls extract_metadata("photo.jpg")]
Assistant: This photo was taken with a Canon EOS R5 on January 15, 2024 at 2:30 PM.
```

**Question:** *"Show me metadata for all JPEGs in this folder"*

```
User: Show me metadata for all JPEGs
Assistant: [calls extract_metadata("*.jpg")]
Assistant: Found 15 JPEG files. Here's the metadata:

photo1.jpg:
  Make: Canon
  Model: EOS R5
  DateTimeOriginal: 2024-01-15 14:30:00

photo2.jpg:
  Make: Sony
  Model: A7R IV
  DateTimeOriginal: 2024-01-20 09:15:00
...
```

### Writing Metadata

**Request:** *"Add my copyright to all JPEGs in this folder"*

```
User: Add my copyright to all JPEGs in this folder
Assistant: I can update 15 JPEG files with your copyright. Let me show you what would change first.
Assistant: [calls write_metadata("*.jpg", {"Copyright": "© 2024 John Doe"}, dry_run=true)]
Assistant: This would update 15 files:
  photo1.jpg: Copyright → "© 2024 John Doe"
  photo2.jpg: Copyright → "© 2024 John Doe"
  ...
Should I proceed?

User: Yes
Assistant: [calls write_metadata("*.jpg", {"Copyright": "© 2024 John Doe"})]
Assistant: Done! Updated copyright info in 15 files.
```

### Searching Files

**Query:** *"Find all photos I took with my Canon camera last month"*

```
User: Find all photos from my Canon camera in October 2024
Assistant: [calls search_metadata(".", ["Make=Canon", "DateTimeOriginal>2024-10-01", "DateTimeOriginal<2024-11-01"])]
Assistant: I found 23 photos taken with your Canon camera in October 2024:

photo1.jpg (Canon EOS R5, 2024-10-05)
photo2.jpg (Canon EOS R5, 2024-10-08)
photo3.jpg (Canon EOS R6, 2024-10-12)
...
```

### Analyzing Metadata

**Request:** *"Show me a summary of all cameras used in these photos"*

```
User: Show me which cameras were used in these photos
Assistant: [calls analyze_metadata("*.jpg")]
Assistant: Analyzed 50 photos:

Cameras Used:
  Canon (30 photos)
    - EOS R5: 20 photos
    - EOS R6: 10 photos
  Sony (20 photos)
    - A7R IV: 15 photos
    - A7 III: 5 photos

Date Range: 2024-01-01 to 2024-03-15
GPS Tagged: 45/50 photos (90%)
```

### Copying Metadata

**Request:** *"Copy metadata from this photo to all others in the folder"*

```
User: Copy the metadata from reference.jpg to all other JPEGs
Assistant: [calls copy_metadata("reference.jpg", "*.jpg", dry_run=true)]
Assistant: This would copy metadata from reference.jpg to 14 other files. The following fields would be copied:
  - Artist
  - Copyright
  - GPS coordinates
  - Camera settings

Should I proceed?

User: Yes
Assistant: [calls copy_metadata("reference.jpg", "*.jpg")]
Assistant: Done! Copied metadata to 14 files successfully.
```

## Technical Details

### Architecture

- **Protocol:** JSON-RPC 2.0 over stdio
- **Runtime:** Tokio async runtime
- **Security:** Path validation to prevent directory traversal
- **Output:** Human-readable text optimized for conversation
- **Error Handling:** Graceful degradation with clear error messages

### Security Features

- Path validation blocks `..` directory traversal
- All operations validate file existence
- Write operations support dry-run preview
- No execution of arbitrary code
- Local file access only (no network operations)

### Performance

- Async I/O for concurrent operations
- Batch operations use parallel processing
- Minimal overhead over direct OxiDex library usage
- Fast startup time (<100ms)

## Troubleshooting

### Server Not Starting

**Problem:** MCP client shows "Server failed to start"

**Solutions:**
1. Check binary path is correct in configuration
2. Verify binary has execute permissions: `chmod +x /path/to/oxidex-mcp`
3. Test manually: `echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | /path/to/oxidex-mcp`

### Tools Not Listed

**Problem:** AI assistant doesn't see OxiDex tools

**Solutions:**
1. Restart your MCP client (Claude Desktop, Cline, etc.)
2. Check MCP configuration file syntax is valid JSON
3. Enable logging: Add `"RUST_LOG": "debug"` to env config
4. Check logs for errors

### Permission Denied Errors

**Problem:** "Permission denied" when accessing files

**Solutions:**
1. Ensure the MCP server process has read permissions for files
2. Check file ownership and permissions
3. For write operations, ensure write permissions

### Debug Mode

Enable detailed logging:

```json
{
  "mcpServers": {
    "oxidex": {
      "command": "/usr/local/bin/oxidex-mcp",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  }
}
```

Logs will show detailed information about requests, responses, and errors.

## Development Status

**Current Status:** In development (Pull Request #10)

**Completed:**
- ✅ MCP protocol implementation
- ✅ All 5 tools functional
- ✅ Comprehensive test suite (35 tests)
- ✅ Documentation and examples
- ✅ Security hardening

**In Progress:**
- 🔄 Code review and testing
- 🔄 Integration with full OxiDex library

**Planned:**
- 📋 Release as part of OxiDex v1.2.0
- 📋 Add to official MCP server registry
- 📋 Pre-built binaries in releases
- 📋 Homebrew formula update

## Resources

- **Source Code:** `oxidex-mcp/` workspace member
- **Design Document:** `docs/plans/2025-11-19-mcp-server-design.md`
- **Implementation Plan:** `docs/plans/2025-11-19-mcp-server-implementation.md`
- **Pull Request:** [#10](https://github.com/swack-tools/oxidex/pull/10)

## Contributing

The MCP server is open source and contributions are welcome! See the main project [contributing guide](https://github.com/swack-tools/oxidex/blob/main/CONTRIBUTING.md) for details.

Areas where contributions would be valuable:
- Additional tool implementations
- Enhanced metadata analysis features
- Performance optimizations
- Documentation improvements
- Testing with different MCP clients

## License

The MCP server is part of OxiDex and is licensed under GPL-3.0, the same as the main project.
