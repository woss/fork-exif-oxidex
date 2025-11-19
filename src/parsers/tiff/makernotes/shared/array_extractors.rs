use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};

/// Extract i16 array from IFD entry
///
/// Handles both inline arrays (≤2 values fitting in 4-byte value_offset)
/// and offset-based arrays (>2 values stored elsewhere in data).
///
/// Used by: Canon CameraSettings, Nikon ShotInfo, Sony CameraSettings
///
/// # Parameters
/// - `entry`: The IFD entry containing the array data
/// - `data`: The complete data buffer for offset-based reads
/// - `byte_order`: Byte order for parsing (little or big endian)
///
/// # Returns
/// Optional vector of i16 values, or None if the data is invalid
pub fn extract_i16_array(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<Vec<i16>> {
    // Validate field type is SHORT (3)
    if entry.field_type != 3 {
        return None;
    }

    if entry.value_count == 0 {
        return None;
    }

    let count = entry.value_count as usize;
    let bytes_needed = count * 2; // 2 bytes per i16

    // Inline: ≤2 shorts fit in 4-byte value_offset field
    if bytes_needed <= 4 {
        let mut result = Vec::with_capacity(count);
        // Respect byte order when converting value_offset to bytes
        let bytes = match byte_order {
            ByteOrder::LittleEndian => entry.value_offset.to_le_bytes(),
            ByteOrder::BigEndian => entry.value_offset.to_be_bytes(),
        };

        for i in 0..count {
            let offset = i * 2;
            let value = match byte_order {
                ByteOrder::LittleEndian => i16::from_le_bytes([bytes[offset], bytes[offset + 1]]),
                ByteOrder::BigEndian => i16::from_be_bytes([bytes[offset], bytes[offset + 1]]),
            };
            result.push(value);
        }

        return Some(result);
    }

    // Offset-based: read from data at specified offset
    let offset = entry.value_offset as usize;

    // Bounds check
    if offset + bytes_needed > data.len() {
        return None;
    }

    let mut result = Vec::with_capacity(count);
    let array_data = &data[offset..offset + bytes_needed];

    for i in 0..count {
        let byte_offset = i * 2;
        let value = match byte_order {
            ByteOrder::LittleEndian => {
                i16::from_le_bytes([array_data[byte_offset], array_data[byte_offset + 1]])
            }
            ByteOrder::BigEndian => {
                i16::from_be_bytes([array_data[byte_offset], array_data[byte_offset + 1]])
            }
        };
        result.push(value);
    }

    Some(result)
}

/// Extract u16 array from IFD entry
///
/// Used by: Nikon LensData, Sony AFInfo, Fuji FaceDetection
pub fn extract_u16_array(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<Vec<u16>> {
    if entry.value_count == 0 {
        return None;
    }

    let offset = entry.value_offset as usize;
    if offset + (entry.value_count as usize * 2) > data.len() {
        return None;
    }

    let mut array = Vec::with_capacity(entry.value_count as usize);
    for i in 0..entry.value_count {
        let pos = offset + (i as usize * 2);
        let value = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([data[pos], data[pos + 1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([data[pos], data[pos + 1]]),
        };
        array.push(value);
    }

    Some(array)
}

/// Extract u32 array from IFD entry
///
/// Used by: Canon FileInfo, Nikon ShutterData, Pentax CameraInfo
pub fn extract_u32_array(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<Vec<u32>> {
    if entry.value_count == 0 {
        return None;
    }

    let offset = entry.value_offset as usize;
    if offset + (entry.value_count as usize * 4) > data.len() {
        return None;
    }

    let mut array = Vec::with_capacity(entry.value_count as usize);
    for i in 0..entry.value_count {
        let pos = offset + (i as usize * 4);
        let value = match byte_order {
            ByteOrder::LittleEndian => {
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
            }
            ByteOrder::BigEndian => {
                u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
            }
        };
        array.push(value);
    }

    Some(array)
}

/// Extract i32 array from IFD entry
///
/// Used by: Olympus CameraSettings, Panasonic WBInfo
pub fn extract_i32_array(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<Vec<i32>> {
    if entry.value_count == 0 {
        return None;
    }

    let offset = entry.value_offset as usize;
    if offset + (entry.value_count as usize * 4) > data.len() {
        return None;
    }

    let mut array = Vec::with_capacity(entry.value_count as usize);
    for i in 0..entry.value_count {
        let pos = offset + (i as usize * 4);
        let value = match byte_order {
            ByteOrder::LittleEndian => {
                i32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
            }
            ByteOrder::BigEndian => {
                i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
            }
        };
        array.push(value);
    }

    Some(array)
}

/// Extract single i16 value from IFD entry
///
/// For SHORT type with count=1, value is stored inline in value_offset field.
/// Used by: Most single-value enum tags
///
/// # Parameters
/// - `entry`: IFD entry containing the value
/// - `_data`: Data buffer (unused for inline values)
/// - `byte_order`: Byte order for parsing
///
/// # Returns
/// Single i16 value, or None if count != 1
pub fn extract_i16_value(entry: &IfdEntry, _data: &[u8], byte_order: ByteOrder) -> Option<i16> {
    if entry.value_count != 1 {
        return None;
    }

    // For SHORT type (count=1), value is inline in value_offset field
    let value = match byte_order {
        ByteOrder::LittleEndian => (entry.value_offset & 0xFFFF) as i16,
        ByteOrder::BigEndian => ((entry.value_offset >> 16) & 0xFFFF) as i16,
    };

    Some(value)
}

/// Extract single u32 value from IFD entry
///
/// For LONG type with count=1, value is stored directly in value_offset.
/// Used by: Timestamps, file sizes, offsets
///
/// # Parameters
/// - `entry`: IFD entry containing the value
/// - `_data`: Data buffer (unused for inline values)
/// - `_byte_order`: Byte order (unused, u32 already parsed)
///
/// # Returns
/// Single u32 value, or None if count != 1
pub fn extract_u32_value(entry: &IfdEntry, _data: &[u8], _byte_order: ByteOrder) -> Option<u32> {
    if entry.value_count != 1 {
        return None;
    }

    Some(entry.value_offset)
}

/// Extract single i32 value from IFD entry
///
/// For SLONG type with count=1, value is stored directly in value_offset.
/// Used by: GPS coordinates, signed offsets
///
/// # Parameters
/// - `entry`: IFD entry containing the value
/// - `data`: Data buffer for offset-based reads
/// - `byte_order`: Byte order for parsing
///
/// # Returns
/// Single i32 value, or None if count != 1
pub fn extract_i32_value(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<i32> {
    if entry.value_count != 1 {
        return None;
    }

    // For SLONG with count=1, check if inline or offset-based
    if entry.field_type == 9 {
        // SLONG type - value is inline in value_offset
        Some(entry.value_offset as i32)
    } else {
        // Offset-based - read from data buffer
        let offset = entry.value_offset as usize;
        if offset + 4 > data.len() {
            return None;
        }
        let value = match byte_order {
            ByteOrder::LittleEndian => i32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]),
            ByteOrder::BigEndian => i32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]),
        };
        Some(value)
    }
}

/// Extract ASCII string from IFD entry
///
/// Handles both inline strings (count ≤ 4) and offset-based strings.
/// Used by: Make, Model, Software, Copyright tags
///
/// # Parameters
/// - `entry`: IFD entry containing the string
/// - `data`: Data buffer for offset-based reads
/// - `byte_order`: Byte order for inline string parsing
///
/// # Returns
/// Extracted string (trimmed of null bytes), or None if invalid/empty
pub fn extract_string(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<String> {
    if entry.value_count == 0 {
        return None;
    }

    let value_bytes = if entry.value_count <= 4 {
        // Inline string (stored in value_offset field)
        let mut bytes = Vec::new();
        for i in 0..entry.value_count as usize {
            let byte = match byte_order {
                ByteOrder::LittleEndian => ((entry.value_offset >> (i * 8)) & 0xFF) as u8,
                ByteOrder::BigEndian => ((entry.value_offset >> (24 - i * 8)) & 0xFF) as u8,
            };
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        bytes
    } else {
        // External string (offset points to data)
        let offset = entry.value_offset as usize;
        if offset >= data.len() {
            return None;
        }
        let end = std::cmp::min(offset + entry.value_count as usize, data.len());
        data[offset..end].to_vec()
    };

    if value_bytes.is_empty() {
        return None;
    }

    let string = String::from_utf8_lossy(&value_bytes)
        .trim_end_matches('\0')
        .to_string();

    if string.is_empty() {
        None
    } else {
        Some(string)
    }
}

/// Extract rational (u32/u32) array from IFD entry
///
/// Used by: GPS coordinates, exposure times, focal lengths
pub fn extract_rational_array(
    entry: &IfdEntry,
    data: &[u8],
    byte_order: ByteOrder,
) -> Option<Vec<(u32, u32)>> {
    if entry.value_count == 0 {
        return None;
    }

    let offset = entry.value_offset as usize;
    if offset + (entry.value_count as usize * 8) > data.len() {
        return None;
    }

    let mut array = Vec::with_capacity(entry.value_count as usize);
    for i in 0..entry.value_count {
        let pos = offset + (i as usize * 8);
        let (numerator, denominator) = match byte_order {
            ByteOrder::LittleEndian => {
                let num =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                let den = u32::from_le_bytes([
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                (num, den)
            }
            ByteOrder::BigEndian => {
                let num =
                    u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                let den = u32::from_be_bytes([
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                (num, den)
            }
        };
        array.push((numerator, denominator));
    }

    Some(array)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_i16_array_inline() {
        // Test inline array (count * 2 <= 4 bytes)
        let entry = IfdEntry {
            tag_id: 0x0001,
            field_type: 3, // SHORT
            value_count: 2,
            value_offset: 0x0064_0032, // Two shorts: 50, 100 (little-endian)
        };

        let result = extract_i16_array(&entry, &[], ByteOrder::LittleEndian);
        assert_eq!(result, Some(vec![50, 100]));
    }

    #[test]
    fn test_extract_i16_array_inline_big_endian() {
        // Test inline array with BigEndian byte order
        let entry = IfdEntry {
            tag_id: 0x0001,
            field_type: 3, // SHORT
            value_count: 2,
            value_offset: 0x0064_0032, // Two shorts: [100, 50] in big-endian
        };

        let result = extract_i16_array(&entry, &[], ByteOrder::BigEndian);
        assert_eq!(result, Some(vec![100, 50]));
    }

    #[test]
    fn test_extract_i16_array_big_endian() {
        let data = vec![0x00, 0x0A, 0x00, 0x14, 0xFF, 0xF6]; // [10, 20, -10]
        let entry = IfdEntry {
            tag_id: 0x0001,
            field_type: 3,
            value_count: 3,
            value_offset: 0,
        };

        let result = extract_i16_array(&entry, &data, ByteOrder::BigEndian);
        assert_eq!(result, Some(vec![10, 20, -10]));
    }

    #[test]
    fn test_extract_i16_array_little_endian() {
        let data = vec![0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00]; // [1, 2, 3, 4]
        let entry = IfdEntry {
            tag_id: 0x0001,
            field_type: 3, // SHORT
            value_count: 4,
            value_offset: 0,
        };

        let result = extract_i16_array(&entry, &data, ByteOrder::LittleEndian);
        assert_eq!(result, Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_extract_u32_array_little_endian() {
        let data = vec![0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00]; // [1, 2]
        let entry = IfdEntry {
            tag_id: 0x0001,
            field_type: 4,
            value_count: 2,
            value_offset: 0,
        };

        let result = extract_u32_array(&entry, &data, ByteOrder::LittleEndian);
        assert_eq!(result, Some(vec![1, 2]));
    }
}
