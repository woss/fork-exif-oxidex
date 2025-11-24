//! Binary data readers for ICC profile parsing
//!
//! This module provides low-level functions for reading binary data
//! from ICC profile byte arrays with proper endianness handling.

use crate::error::{ExifToolError, Result};

/// Reads a 4-byte big-endian unsigned integer
pub fn read_u32_be(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Reads a 2-byte big-endian unsigned integer
pub fn read_u16_be(data: &[u8], offset: usize) -> Result<u16> {
    if offset + 2 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

/// Reads an 8-byte big-endian unsigned integer
pub fn read_u64_be(data: &[u8], offset: usize) -> Result<u64> {
    if offset + 8 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    Ok(u64::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]))
}

/// Reads a 4-byte signature as a trimmed ASCII string
pub fn read_signature(data: &[u8], offset: usize) -> Result<String> {
    if offset + 4 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    let bytes = &data[offset..offset + 4];
    Ok(String::from_utf8_lossy(bytes).to_string())
}

/// Reads a signed 15.16 fixed-point number and converts to f64
pub fn read_s15fixed16(data: &[u8], offset: usize) -> Result<f64> {
    let value = read_u32_be(data, offset)? as i32;
    let integer_part = (value >> 16) as i16 as f64;
    let fractional_part = (value & 0xFFFF) as f64 / 65536.0;
    Ok(integer_part + fractional_part)
}

/// Reads an unsigned 16.16 fixed-point number and converts to f64
pub fn read_u16fixed16(data: &[u8], offset: usize) -> Result<f64> {
    let value = read_u32_be(data, offset)?;
    let integer_part = (value >> 16) as f64;
    let fractional_part = (value & 0xFFFF) as f64 / 65536.0;
    Ok(integer_part + fractional_part)
}

/// Finds a byte sequence in a larger byte slice
pub fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
