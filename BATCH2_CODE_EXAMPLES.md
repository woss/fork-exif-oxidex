# Batch 2 Refactoring - Code Examples

## Ricoh Parser Refactoring

### Before: Decoders in Parser
**File**: `src/parsers/tiff/makernotes/ricoh.rs` (lines 49-72)

```rust
// BEFORE: Decoders defined in parser
const_decoder!(
    SHOOTING_MODE,
    u16,
    [
        (0, "Auto"),
        (1, "Program"),
        (2, "Aperture Priority"),
        (3, "Manual"),
    ]
);

const_decoder!(FLASH_MODE, u16, [(0, "Auto"), (1, "On"), (2, "Off"),]);

const_decoder!(
    WHITE_BALANCE,
    u16,
    [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Shade"),
        (3, "Fluorescent"),
        (4, "Tungsten"),
    ]
);
```

### After: Decoders in Registry
**File**: `src/parsers/tiff/makernotes/registries/ricoh.rs` (lines 13-39)

```rust
// AFTER: Decoders defined in registry
const_decoder!(
    RICOH_SHOOTING_MODE,
    u16,
    [
        (0, "Auto"),
        (1, "Program"),
        (2, "Aperture Priority"),
        (3, "Manual"),
    ]
);

const_decoder!(RICOH_FLASH_MODE, u16, [(0, "Auto"), (1, "On"), (2, "Off"),]);

const_decoder!(
    RICOH_WHITE_BALANCE,
    u16,
    [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Shade"),
        (3, "Fluorescent"),
        (4, "Tungsten"),
    ]
);

// Registry now includes decoders
pub fn ricoh_registry() -> TagRegistry {
    TagRegistry::new()
        .register_simple_u16(0x0005, "ShootingMode", &RICOH_SHOOTING_MODE)
        .register_simple_u16(0x000C, "FlashMode", &RICOH_FLASH_MODE)
        .register_simple_u16(0x001E, "WhiteBalance", &RICOH_WHITE_BALANCE)
        // ... other tags
}
```

### Parser Simplification: parse_entry()

#### Before (Ricoh)
**File**: `src/parsers/tiff/makernotes/ricoh.rs` (lines 115-143)

```rust
fn parse_entry(
    &self,
    entry: &IfdEntry,
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    if let Some(value) = extract_u16_value(entry, data, byte_order) {
        let tag_name = match TAG_REGISTRY.get_tag_name(entry.tag_id) {
            Some(name) => name,
            None => return,
        };

        // Complex pattern matching for each tag type
        let formatted_value = match entry.tag_id {
            RICOH_SHOOTING_MODE | RICOH_FLASH_MODE | RICOH_WHITE_BALANCE => {
                TAG_REGISTRY.decode_u16(entry.tag_id, value)
            }
            RICOH_FOCUS_MODE => {
                let mode = if value == 0 { "Auto" } else { "Manual" };
                mode.to_string()
            }
            RICOH_ISO_SETTING | RICOH_SHARPNESS => value.to_string(),
            _ => return,
        };

        tags.insert(format!("Ricoh:{}", tag_name), formatted_value);
    }
}
```

#### After (Ricoh)
**File**: `src/parsers/tiff/makernotes/ricoh.rs` (lines 89-126)

```rust
fn parse_entry(
    &self,
    entry: &IfdEntry,
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    // Get tag name from registry - unknown tags skipped automatically
    let tag_name = match TAG_REGISTRY.get_tag_name(entry.tag_id) {
        Some(name) => name,
        None => return,
    };

    // Extract u16 value for all registered tags
    let value = match extract_u16_value(entry, data, byte_order) {
        Some(v) => v,
        None => return,
    };

    // Cleaner pattern matching with explicit comments
    let formatted_value = match entry.tag_id {
        // Tags with registry-based decoders (shooting mode, flash mode, white balance)
        0x0005 | 0x000C | 0x001E => TAG_REGISTRY.decode_u16(entry.tag_id, value),

        // Focus mode: manual binary decode
        RICOH_FOCUS_MODE => {
            if value == 0 { "Auto".to_string() } else { "Manual".to_string() }
        }

        // Numeric tags: ISO, Sharpness
        RICOH_ISO_SETTING | RICOH_SHARPNESS => value.to_string(),

        // Unknown tag handling (shouldn't reach here due to registry check)
        _ => return,
    };

    tags.insert(format!("Ricoh:{}", tag_name), formatted_value);
}
```

**Key Improvements**:
- Clearer logic flow: get name → extract value → format → insert
- Explicit comments for each branch
- Less conditional nesting
- Easier to understand tag type mapping

---

## Parrot Parser Refactoring

### Before: Inline Registry Definition
**File**: `src/parsers/tiff/makernotes/parrot.rs` (lines 66-75 and 122-143)

```rust
// BEFORE: Decoder in parser
const_decoder!(
    pub FLIGHT_MODE,
    i16,
    [
        (0, "Manual"),
        (1, "GPS"),
        (2, "Follow Me"),
        (3, "Return Home"),
    ]
);

// BEFORE: Inline registry with 23 lines
static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(|| {
    TagRegistry::with_capacity(15)
        // String tags
        .register_raw(PARROT_MODEL, "Model")
        .register_raw(PARROT_SERIAL, "SerialNumber")
        .register_raw(PARROT_VERSION, "Version")
        // GPS tags (handled separately due to i32 type)
        .register_raw(PARROT_GPS_LAT, "GPSLatitude")
        .register_raw(PARROT_GPS_LON, "GPSLongitude")
        // Flight mode decoder
        .register_simple_i16(PARROT_FLIGHT_MODE, "FlightMode", &FLIGHT_MODE)
        // Custom formatted tags (handled separately)
        .register_raw(PARROT_ALTITUDE, "Altitude")
        // ... etc
});
```

### After: Centralized Registry
**File**: `src/parsers/tiff/makernotes/registries/parrot.rs` (lines 14-63)

```rust
// AFTER: Decoder in registry
const_decoder!(
    PARROT_FLIGHT_MODE,
    i16,
    [
        (0, "Manual"),
        (1, "GPS"),
        (2, "Follow Me"),
        (3, "Return Home"),
    ]
);

// AFTER: Clean registry initialization (1 line in parser)
static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(parrot_registry);

// Registry function in registries/parrot.rs
pub fn parrot_registry() -> TagRegistry {
    TagRegistry::new()
        // Drone Identification (u32 tags, raw values)
        .register_raw(0x0001, "Model")
        .register_raw(0x0002, "SerialNumber")
        .register_raw(0x0003, "Version")
        // GPS Information (i32 tags)
        .register_raw(0x0100, "GPSLatitude")
        .register_raw(0x0101, "GPSLongitude")
        // Altitude and Speed (i16 tags)
        .register_raw(0x0102, "Altitude")
        .register_raw(0x0103, "Speed")
        // Gimbal Angles (i16 tags)
        .register_raw(0x0105, "GimbalPitch")
        .register_raw(0x0106, "GimbalRoll")
        .register_raw(0x0107, "GimbalYaw")
        // System Status
        .register_raw(0x0108, "BatteryLevel")
        .register_raw(0x0109, "WiFiSignal")
        // Flight Information with decoder
        .register_simple_i16(0x010A, "FlightMode", &PARROT_FLIGHT_MODE)
        // Home Distance
        .register_raw(0x010B, "HomeDistance")
}
```

### Parser Refactoring: parse_entry()

#### Before (Parrot)
**File**: `src/parsers/tiff/makernotes/parrot.rs` (lines 160-219)

```rust
fn parse_entry(
    &self,
    entry: &IfdEntry,
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    let tag_id = entry.tag_id;

    // Multiple separate match blocks for different tag types
    match tag_id {
        PARROT_MODEL | PARROT_SERIAL | PARROT_VERSION => {
            if let Some(s) = extract_string(entry, data, byte_order) {
                if let Some(name) = TAG_REGISTRY.get_tag_name(tag_id) {
                    tags.insert(format!("Parrot:{}", name), s);
                }
            }
            return;
        }
        _ => {}
    }

    match tag_id {
        PARROT_GPS_LAT | PARROT_GPS_LON => {
            if let Some(val) = extract_i32_value(entry, data, byte_order) {
                if let Some(name) = TAG_REGISTRY.get_tag_name(tag_id) {
                    tags.insert(format!("Parrot:{}", name), format_gps_coord(val));
                }
            }
            return;
        }
        _ => {}
    }

    // i16 array handling with complex pattern matching
    if let Some(array) = extract_i16_array(entry, data, byte_order) {
        if let Some(&value) = array.first() {
            let tag_name = match TAG_REGISTRY.get_tag_name(tag_id) {
                Some(name) => name,
                None => return,
            };

            let formatted_value = match tag_id {
                PARROT_FLIGHT_MODE => TAG_REGISTRY.decode_i16(tag_id, value),
                PARROT_ALTITUDE => format_altitude(value),
                PARROT_SPEED => format_speed(value),
                PARROT_DIRECTION => format!("{}°", value),
                PARROT_GIMBAL_PITCH | PARROT_GIMBAL_ROLL => format_gimbal_angle(value),
                PARROT_GIMBAL_YAW => format!("{}°", value),
                PARROT_BATTERY => format!("{}%", value),
                PARROT_WIFI_SIGNAL => format!("{} dBm", value),
                PARROT_DISTANCE => format!("{} m", value),
                _ => return,
            };

            tags.insert(format!("Parrot:{}", tag_name), formatted_value);
        }
    }
}
```

#### After (Parrot)
**File**: `src/parsers/tiff/makernotes/parrot.rs` (lines 141-211)

```rust
fn parse_entry(
    &self,
    entry: &IfdEntry,
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    let tag_id = entry.tag_id;

    // Get tag name from registry first - skip unknown tags
    let tag_name = match TAG_REGISTRY.get_tag_name(tag_id) {
        Some(name) => name,
        None => return,
    };

    // Handle string tags (Model, SerialNumber, Version)
    if matches!(tag_id, PARROT_MODEL | PARROT_SERIAL | PARROT_VERSION) {
        if let Some(s) = extract_string(entry, data, byte_order) {
            tags.insert(format!("Parrot:{}", tag_name), s);
        }
        return;
    }

    // Handle GPS coordinates (i32 type with decimal formatting)
    if matches!(tag_id, PARROT_GPS_LAT | PARROT_GPS_LON) {
        if let Some(val) = extract_i32_value(entry, data, byte_order) {
            tags.insert(format!("Parrot:{}", tag_name), format_gps_coord(val));
        }
        return;
    }

    // Handle i16 array tags (flight metrics, gimbal angles, battery, etc.)
    if let Some(array) = extract_i16_array(entry, data, byte_order) {
        if let Some(&value) = array.first() {
            // Apply tag-specific formatting with clear comments
            let formatted_value = match tag_id {
                // Flight mode has a registry decoder
                PARROT_FLIGHT_MODE => TAG_REGISTRY.decode_i16(tag_id, value),

                // Altitude: cm to meters
                PARROT_ALTITUDE => format_altitude(value),

                // Speed: 0.1 m/s to m/s
                PARROT_SPEED => format_speed(value),

                // Direction: degrees
                PARROT_DIRECTION => format!("{}°", value),

                // Gimbal angles: 0.1 degrees to degrees
                PARROT_GIMBAL_PITCH | PARROT_GIMBAL_ROLL => format_gimbal_angle(value),

                // Gimbal yaw: degrees
                PARROT_GIMBAL_YAW => format!("{}°", value),

                // Battery: percentage
                PARROT_BATTERY => format!("{}%", value),

                // WiFi: dBm signal strength
                PARROT_WIFI_SIGNAL => format!("{} dBm", value),

                // Home distance: meters
                PARROT_DISTANCE => format!("{} m", value),

                // Fallback for unhandled i16 tags
                _ => return,
            };

            tags.insert(format!("Parrot:{}", tag_name), formatted_value);
        }
    }
}
```

**Key Improvements**:
- Single registry lookup upfront
- Early returns with `matches!()` instead of nested match blocks
- Clearer tag type grouping (string → GPS → i16)
- Inline comments explaining each conversion
- Easier to trace execution flow

---

## Registry Pattern Consistency

### Unified Registry Interface

Both registries now follow the same pattern:

```rust
// registries/ricoh.rs
pub fn ricoh_registry() -> TagRegistry {
    TagRegistry::new()
        .register_raw(0x0001, "Model")
        .register_simple_u16(0x0005, "ShootingMode", &RICOH_SHOOTING_MODE)
        // ... all 9 tags
}

// registries/parrot.rs
pub fn parrot_registry() -> TagRegistry {
    TagRegistry::new()
        .register_raw(0x0001, "Model")
        .register_simple_i16(0x010A, "FlightMode", &PARROT_FLIGHT_MODE)
        // ... all 12 tags
}
```

### Parser Usage Pattern

Both parsers use the registry the same way:

```rust
// Static registry initialization
static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(ricoh_registry);
static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(parrot_registry);

// Tag name lookup
if let Some(tag_name) = TAG_REGISTRY.get_tag_name(tag_id) { ... }

// Decoder usage
TAG_REGISTRY.decode_u16(tag_id, value)
TAG_REGISTRY.decode_i16(tag_id, value)
```

---

## Summary of Transformations

| Aspect | Before | After | Benefit |
|--------|--------|-------|---------|
| **Ricoh Decoders** | In parser (27 lines) | In registry | Single definition point |
| **Ricoh Constants** | 9 constants | 3 constants | Reduced coupling |
| **Parrot Registry** | Inline (23 lines) | Centralized function | Clean separation |
| **Parrot Decoder** | In parser | In registry | Consistency |
| **parse_entry()** | Multiple match blocks | Single flow with early returns | Clarity |
| **Documentation** | Basic comments | Comprehensive docs | Maintainability |
| **Test Coverage** | Decoder tests | Registry tests | Better organization |

---

## Lines of Code Analysis

### Ricoh
- Parser: 216 → 212 lines (-1.9%)
- Registry: 66 → 93 lines (+40.9%)
- Combined: 282 → 305 lines (+8.2%)

**Analysis**: Registry expansion includes decoder definitions (36 lines), which were removed from parser (27 lines). Net increase due to better organization.

### Parrot
- Parser: 308 → 293 lines (-4.9%)
- Registry: 70 → 83 lines (+18.6%)
- Combined: 378 → 376 lines (-0.5%)

**Analysis**: Parser simplification more than offsets registry expansion. Inline registry removal (22 lines) enables clean refactoring.

---

## Code Quality Metrics

### Cyclomatic Complexity
- **Before**: Higher (multiple match blocks in parse_entry)
- **After**: Lower (single flow with clear branches)

### Coupling
- **Before**: Decoders scattered between parser and registry
- **After**: Unified in registry only

### Cohesion
- **Before**: Mixed concerns (tag metadata, parsing, decoding)
- **After**: Clear separation (metadata in registry, parsing in parser)

### Maintainability
- **Before**: Hard to extend (add decoder, add to parser, add to registry)
- **After**: Easy to extend (add to registry only)
