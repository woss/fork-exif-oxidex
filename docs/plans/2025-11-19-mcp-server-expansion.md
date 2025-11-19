# OxiDex MCP Server Expansion Implementation Plan

**Date:** 2025-11-19
**Status:** Planning
**Goal:** Expand MCP server with complete tag database integration and advanced features

## Overview

The current MCP server provides basic metadata extraction, writing, searching, analysis, and copying. This plan expands it with:

1. Complete tag database integration (browsing, descriptions, metadata)
2. Advanced search with tag name resolution
3. Tag validation and autocomplete
4. Batch operations enhancements
5. Format detection and capabilities
6. Performance monitoring

## Current State

**Implemented:**
- ✅ 5 core tools (extract, write, search, analyze, copy)
- ✅ Full OxiDex metadata extraction
- ✅ Glob pattern support
- ✅ Dry-run mode
- ✅ Human-readable output
- ✅ Path validation

**Missing:**
- ❌ Tag database browsing
- ❌ Tag descriptions and metadata
- ❌ Tag name validation/autocomplete
- ❌ Format detection tool
- ❌ Advanced filtering by tag groups
- ❌ Tag comparison between files
- ❌ Writable tag enumeration
- ❌ Tag value validation

## Implementation Tasks

### Phase 1: Tag Database Integration (High Priority)

#### Task 1.1: List All Tags Tool
**Description:** Add tool to browse all available tags in the OxiDex tag database

**Tool Schema:**
```json
{
  "name": "list_tags",
  "description": "List all available metadata tags, optionally filtered by group or format",
  "inputSchema": {
    "type": "object",
    "properties": {
      "group": {
        "type": "string",
        "description": "Filter by tag group (e.g., 'EXIF', 'XMP', 'IPTC', 'File')"
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
  }
}
```

**Implementation Steps:**
1. Create `oxidex-mcp/src/tools/list_tags.rs`
2. Use `oxidex::tag_db::list_all_tags()` or equivalent
3. Filter by group, format, writable status
4. Format output as categorized list with descriptions
5. Add tests for filtering logic

**Output Format:**
```
Found 150 EXIF tags:

Camera Settings:
  EXIF:Make - Camera manufacturer
  EXIF:Model - Camera model name
  EXIF:ISO - ISO speed rating
  EXIF:FNumber - Lens aperture (f-stop)
  ...

Date/Time:
  EXIF:DateTimeOriginal - Date/time when image was captured
  EXIF:CreateDate - File creation timestamp
  ...
```

#### Task 1.2: Tag Information Tool
**Description:** Get detailed information about a specific tag

**Tool Schema:**
```json
{
  "name": "get_tag_info",
  "description": "Get detailed information about a metadata tag including description, data type, and examples",
  "inputSchema": {
    "type": "object",
    "properties": {
      "tag": {
        "type": "string",
        "description": "Tag name (e.g., 'EXIF:Make', 'XMP:Creator')"
      }
    },
    "required": ["tag"]
  }
}
```

**Implementation Steps:**
1. Create `oxidex-mcp/src/tools/tag_info.rs`
2. Use `oxidex::tag_db::lookup_tag()` or tag registry
3. Return tag description, data type, writable status, format info
4. Include examples and valid values if available
5. Add tests with known tags

**Output Format:**
```
Tag: EXIF:ISO

Description: ISO speed rating of the camera sensor
Data Type: Integer
Writable: Yes
Format: JPEG, TIFF, RAW formats
Group: EXIF

Valid Range: 50 - 204800
Common Values: 100, 200, 400, 800, 1600, 3200

Example Usage:
  oxidex -EXIF:ISO=800 photo.jpg
```

#### Task 1.3: Tag Groups Tool
**Description:** List all tag groups with counts

**Tool Schema:**
```json
{
  "name": "list_tag_groups",
  "description": "List all metadata tag groups (EXIF, XMP, IPTC, etc.) with tag counts",
  "inputSchema": {
    "type": "object",
    "properties": {
      "format": {
        "type": "string",
        "description": "Filter by file format"
      }
    }
  }
}
```

**Output Format:**
```
Metadata Tag Groups:

EXIF (450 tags)
  - Standard camera and image metadata
  - Supported formats: JPEG, TIFF, RAW, DNG

XMP (380 tags)
  - Adobe Extensible Metadata Platform
  - Supported formats: JPEG, PNG, PDF, TIFF

IPTC (120 tags)
  - International Press Telecommunications Council
  - Supported formats: JPEG, TIFF

File (25 tags)
  - File system metadata
  - Supported formats: All

...
```

### Phase 2: Advanced Search & Filtering (Medium Priority)

#### Task 2.1: Enhanced Search with Tag Resolution
**Description:** Improve search_metadata to resolve partial tag names

**Changes:**
1. Accept partial tag names (e.g., "Make" instead of "EXIF:Make")
2. Suggest similar tags if exact match not found
3. Support tag aliases
4. Case-insensitive matching

**Implementation:**
- Modify `oxidex-mcp/src/tools/search.rs`
- Add tag name resolution before filtering
- Use fuzzy matching for suggestions

**Example:**
```
User searches: "Make=Canon"
System resolves: "EXIF:Make=Canon" or "XMP:Make=Canon"
If ambiguous: "Did you mean: EXIF:Make, XMP:Make, QuickTime:Make?"
```

#### Task 2.2: Compare Tags Between Files
**Description:** New tool to compare metadata between two or more files

**Tool Schema:**
```json
{
  "name": "compare_metadata",
  "description": "Compare metadata between two or more files, showing differences",
  "inputSchema": {
    "type": "object",
    "properties": {
      "files": {
        "type": "array",
        "description": "List of file paths to compare",
        "items": { "type": "string" }
      },
      "tags": {
        "type": "array",
        "description": "Specific tags to compare (optional, compares all if omitted)",
        "items": { "type": "string" }
      },
      "show_common": {
        "type": "boolean",
        "description": "Show tags with same values (default: false, only shows differences)"
      }
    },
    "required": ["files"]
  }
}
```

**Output Format:**
```
Comparing 3 files:

Differences:
  EXIF:Make
    photo1.jpg: Canon
    photo2.jpg: Sony
    photo3.jpg: Nikon

  EXIF:ISO
    photo1.jpg: 400
    photo2.jpg: 800
    photo3.jpg: 1600

Common Values (10 tags):
  EXIF:Orientation: Horizontal (normal)
  File:FileType: JPEG
  ...
```

### Phase 3: Format Detection & Capabilities (Medium Priority)

#### Task 3.1: Detect File Format Tool
**Description:** Detect file format and show supported operations

**Tool Schema:**
```json
{
  "name": "detect_format",
  "description": "Detect file format and show supported metadata operations",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path or glob pattern"
      }
    },
    "required": ["path"]
  }
}
```

**Implementation:**
1. Create `oxidex-mcp/src/tools/detect_format.rs`
2. Use `oxidex::parsers::format_detector::detect_format()`
3. Return format, MIME type, supported tag groups
4. Indicate read/write capabilities

**Output Format:**
```
photo.jpg:
  Format: JPEG
  MIME Type: image/jpeg

  Supported Metadata:
    ✓ EXIF (read/write)
    ✓ XMP (read/write)
    ✓ IPTC (read/write)
    ✓ JFIF (read only)
    ✓ ICC Profile (read only)

  Supported Operations:
    ✓ Extract metadata
    ✓ Write metadata
    ✓ Copy metadata
    ✓ Search metadata
```

### Phase 4: Validation & Error Prevention (Low Priority)

#### Task 4.1: Validate Tag Values
**Description:** Add validation before writing tags

**Changes:**
1. Validate tag exists before write
2. Check data type compatibility
3. Validate value format (dates, GPS, etc.)
4. Check if tag is writable for file format

**Implementation:**
- Modify `oxidex-mcp/src/tools/write.rs`
- Add validation layer using tag database
- Return helpful error messages with suggestions

#### Task 4.2: List Writable Tags for Format
**Description:** Show which tags can be written to a file

**Tool Schema:**
```json
{
  "name": "list_writable_tags",
  "description": "List tags that can be written to a specific file format",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path to check format"
      },
      "group": {
        "type": "string",
        "description": "Filter by tag group"
      }
    },
    "required": ["path"]
  }
}
```

### Phase 5: Batch Operations Enhancements (Low Priority)

#### Task 5.1: Batch Rename Tool
**Description:** Rename files based on metadata patterns

**Tool Schema:**
```json
{
  "name": "rename_files",
  "description": "Rename files using metadata tag patterns",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path or glob pattern"
      },
      "pattern": {
        "type": "string",
        "description": "Rename pattern using %TAG% placeholders (e.g., '%DateTimeOriginal%_%Make%_%Model%')"
      },
      "dry_run": {
        "type": "boolean",
        "description": "Preview changes without renaming"
      }
    },
    "required": ["path", "pattern"]
  }
}
```

#### Task 5.2: Template-Based Metadata Writing
**Description:** Apply metadata templates to multiple files

**Tool Schema:**
```json
{
  "name": "apply_template",
  "description": "Apply a metadata template to multiple files",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path or glob pattern"
      },
      "template": {
        "type": "string",
        "description": "Template name or JSON template"
      },
      "dry_run": {
        "type": "boolean"
      }
    },
    "required": ["path", "template"]
  }
}
```

### Phase 6: Advanced Features (Future)

#### Task 6.1: Metadata Export/Import
- Export metadata to JSON/CSV/XML
- Import metadata from external files
- Sidecar file support (.xmp, .json)

#### Task 6.2: Geolocation Features
- Reverse geocoding (coordinates → location names)
- Distance calculations between photos
- Map visualization data export

#### Task 6.3: Date/Time Operations
- Date shifting (already in CLI, add to MCP)
- Timezone conversion
- Date range filtering

## Testing Strategy

### Unit Tests
- Test each new tool handler
- Test tag database queries
- Test validation logic
- Test format detection

### Integration Tests
- Test tool registration
- Test complete workflows
- Test with real files containing metadata
- Test error handling

### Manual Testing with Claude Desktop
- Test each tool through Claude conversation
- Verify human-readable output
- Test error messages are helpful
- Test with various file formats

## Documentation Updates

### Update `docs/book/src/mcp_integration.md`
- Document all new tools
- Add usage examples for each
- Update feature matrix
- Add troubleshooting for new tools

### Update Implementation Plan
- Mark completed tasks
- Update status
- Document any deviations

### Update README
- Add new tool descriptions
- Update capabilities list
- Add new examples

## Success Criteria

**Phase 1 (Tag Database):**
- [ ] Users can browse all available tags
- [ ] Users can get detailed tag information
- [ ] Users can filter tags by group/format
- [ ] Tag search helps with partial names

**Phase 2 (Advanced Search):**
- [ ] Search resolves partial tag names
- [ ] Users can compare files easily
- [ ] Fuzzy matching suggests similar tags

**Phase 3 (Format Detection):**
- [ ] Users can query format capabilities
- [ ] Clear indication of read/write support
- [ ] MIME type detection works

**Phase 4 (Validation):**
- [ ] Write operations validate tags
- [ ] Helpful errors for invalid tags
- [ ] Type checking prevents errors

**Phase 5 (Batch Operations):**
- [ ] Rename based on metadata works
- [ ] Templates simplify bulk updates
- [ ] All batch ops support dry-run

## Risk Assessment

**High Risk:**
- Tag database API may not be fully exposed
- Performance with large tag lists
- Breaking changes to existing tools

**Mitigation:**
- Check tag_db module API before starting
- Implement pagination for large lists
- Add new tools without modifying existing ones

**Low Risk:**
- Documentation updates
- Test coverage
- Claude Desktop integration

## Timeline Estimate

**Phase 1:** 2-3 days (high priority, foundational)
**Phase 2:** 2-3 days (builds on Phase 1)
**Phase 3:** 1-2 days (independent, can parallelize)
**Phase 4:** 1-2 days (enhancement of existing)
**Phase 5:** 2-3 days (new complex features)

**Total:** ~10-15 days for complete implementation

**Note:** Can implement phases independently, prioritize based on user feedback.

## Next Steps

1. Review plan with stakeholders
2. Verify tag database API availability
3. Create feature branch: `feature/mcp-server-expansion`
4. Start with Phase 1, Task 1.1 (list_tags)
5. Implement in TDD approach with tests first
6. Update documentation as features are added
7. Test each phase in Claude Desktop before moving to next
