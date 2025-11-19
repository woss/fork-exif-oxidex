# OxiDex MCP Server

MCP (Model Context Protocol) server for OxiDex metadata operations, enabling AI assistants to extract, search, analyze, and modify file metadata.

## Features

- **extract_metadata**: Extract metadata from files (supports glob patterns)
- **write_metadata**: Write or update metadata tags (with dry-run support)
- **search_metadata**: Search files by metadata criteria
- **analyze_metadata**: Generate statistical summaries
- **copy_metadata**: Copy metadata between files

## Installation

### Build from source

```bash
cargo build --release -p oxidex-mcp
```

The binary will be located at `./target/release/oxidex-mcp`.

## Configuration

Add to your MCP client configuration (e.g., Claude Desktop, Cline, etc.):

```json
{
  "mcpServers": {
    "oxidex": {
      "command": "/path/to/oxidex-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

For Claude Desktop on macOS, edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "oxidex": {
      "command": "/Users/your-username/path/to/oxidex-mcp"
    }
  }
}
```

## Usage Examples

### Extract Metadata

Extract metadata from a single file:

```json
{
  "name": "extract_metadata",
  "arguments": {
    "path": "photo.jpg"
  }
}
```

Extract from multiple files using glob patterns:

```json
{
  "name": "extract_metadata",
  "arguments": {
    "path": "photos/*.jpg"
  }
}
```

### Write Metadata (with Dry-Run)

Preview changes before applying:

```json
{
  "name": "write_metadata",
  "arguments": {
    "path": "*.jpg",
    "tags": {
      "Artist": "John Doe",
      "Copyright": "© 2024"
    },
    "dry_run": true
  }
}
```

Apply changes:

```json
{
  "name": "write_metadata",
  "arguments": {
    "path": "photo.jpg",
    "tags": {
      "Artist": "John Doe"
    },
    "dry_run": false
  }
}
```

### Search by Metadata

Find files matching metadata criteria:

```json
{
  "name": "search_metadata",
  "arguments": {
    "directory": "photos",
    "filters": ["Make=Canon", "DateTimeOriginal>2024-01-01"]
  }
}
```

Filter operators:
- `=` Equals (e.g., `Make=Canon`)
- `>` Greater than (e.g., `FileSize>1000000`)
- `<` Less than (e.g., `FileSize<1000000`)
- `~` Contains (e.g., `Model~R5`)

### Analyze Metadata

Generate statistics about metadata:

```json
{
  "name": "analyze_metadata",
  "arguments": {
    "path": "photos/*.jpg"
  }
}
```

### Copy Metadata

Copy metadata from source to destination:

```json
{
  "name": "copy_metadata",
  "arguments": {
    "source": "template.jpg",
    "destination": "photo.jpg",
    "dry_run": true
  }
}
```

Copy to multiple files:

```json
{
  "name": "copy_metadata",
  "arguments": {
    "source": "template.jpg",
    "destination": "photos/*.jpg",
    "dry_run": false
  }
}
```

## Protocol

The server implements the Model Context Protocol (MCP) 2024-11-05 specification:

- **initialize**: Handshake and capability negotiation
- **tools/list**: List available tools and their parameters
- **tools/call**: Execute a tool with arguments

All communication is JSON-RPC 2.0 over stdin/stdout.

## Development

### Run tests

```bash
cargo test -p oxidex-mcp
```

### Run specific test suite

```bash
cargo test -p oxidex-mcp --test integration_tests
```

### Run with debug logging

```bash
RUST_LOG=debug cargo run -p oxidex-mcp
```

### Test the server manually

```bash
# Test initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | cargo run -p oxidex-mcp --quiet

# Test tools/list
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | cargo run -p oxidex-mcp --quiet

# Test extract_metadata
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"extract_metadata","arguments":{"path":"Cargo.toml"}}}' | cargo run -p oxidex-mcp --quiet
```

## Architecture

### Request Flow

1. Client sends JSON-RPC request via stdin
2. Server parses request using `serde_json`
3. Request routed to appropriate handler:
   - `initialize` → server capabilities
   - `tools/list` → available tools with schemas
   - `tools/call` → tool dispatcher
4. Tool handler processes arguments (glob expansion, validation, etc.)
5. Tool executes using OxiDex library or file system operations
6. Results formatted as human-readable text
7. Response sent to stdout as JSON-RPC response

### Tool Implementation

Each tool in `oxidex-mcp/src/tools/` follows this pattern:

1. Parse arguments from JSON
2. Validate input paths (security checks)
3. Expand glob patterns if needed
4. Process files (extract, write, search, etc.)
5. Format results as text
6. Return as JSON-RPC response

### Security

- Path validation prevents directory traversal (`..` sequences)
- Glob patterns validated and constrained
- Dry-run support for destructive operations
- No arbitrary command execution

## Limitations

Currently, the implementation uses file system metadata as a placeholder. For full OxiDex library integration:

- Connect to `oxidex::core::MetadataMap::from_file()` for reading actual metadata
- Implement metadata writing through OxiDex write APIs
- Add support for format-specific metadata handlers (EXIF, IPTC, XMP, etc.)

## License

GPL-3.0 (same as OxiDex)

## See Also

- [OxiDex](../) - Main metadata extraction library
- [MCP Specification](https://spec.modelcontextprotocol.io/)
