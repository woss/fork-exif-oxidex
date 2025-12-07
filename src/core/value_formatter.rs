//! Value formatting to match ExifTool conventions
//!
//! This module provides formatting functions for various types of metadata values
//! to ensure they match ExifTool's output format exactly, including:
//! - File sizes (e.g., "2.1 kB" not "2 kB")
//! - EXIF dates (YYYY:MM:DD HH:MM:SS)
//! - IPTC dates (YYYYMMDD -> YYYY:MM:DD)
//! - IPTC times (HHMMSS±HHMM -> HH:MM:SS±HH:MM)
//! - Rational numbers with tag-specific formatting

/// Format file size like ExifTool (e.g., "2.1 kB" not "2 kB")
///
/// ExifTool uses decimal (base-10) units, not binary (base-2) units.
/// - 1 kB = 1000 bytes (not 1024)
/// - 1 MB = 1,000,000 bytes
/// - 1 GB = 1,000,000,000 bytes
///
/// Small files (< 1000 bytes) show exact byte count.
/// Larger files show one decimal place.
///
/// # Examples
///
/// ```
/// use oxidex::core::value_formatter::format_file_size;
///
/// assert_eq!(format_file_size(500), "500 bytes");
/// assert_eq!(format_file_size(2100), "2.1 kB");
/// assert_eq!(format_file_size(1_500_000), "1.5 MB");
/// assert_eq!(format_file_size(2_500_000_000), "2.5 GB");
/// ```
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1000 {
        format!("{} bytes", bytes)
    } else if bytes < 1_000_000 {
        format!("{:.1} kB", bytes as f64 / 1000.0)
    } else if bytes < 1_000_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    }
}

/// Format EXIF date/time to ExifTool format (YYYY:MM:DD HH:MM:SS)
///
/// ExifTool uses colons in dates, not dashes.
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Utc, TimeZone};
/// use oxidex::core::value_formatter::format_exif_datetime;
///
/// let dt = Utc.with_ymd_and_hms(2002, 6, 20, 2, 11, 11).unwrap();
/// assert_eq!(format_exif_datetime(&dt), "2002:06:20 02:11:11");
/// ```
pub fn format_exif_datetime(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y:%m:%d %H:%M:%S").to_string()
}

/// Format IPTC date from raw format (YYYYMMDD -> YYYY:MM:DD)
///
/// IPTC stores dates as 8-digit strings without separators.
/// ExifTool displays them with colon separators.
///
/// # Examples
///
/// ```
/// use oxidex::core::value_formatter::format_iptc_date;
///
/// assert_eq!(format_iptc_date("20020620"), "2002:06:20");
/// assert_eq!(format_iptc_date("19991231"), "1999:12:31");
/// assert_eq!(format_iptc_date("invalid"), "invalid"); // Preserves invalid input
/// ```
pub fn format_iptc_date(raw: &str) -> String {
    if raw.len() == 8 {
        format!("{}:{}:{}", &raw[0..4], &raw[4..6], &raw[6..8])
    } else {
        raw.to_string()
    }
}

/// Format IPTC time from raw format (HHMMSS±HHMM -> HH:MM:SS±HH:MM)
///
/// IPTC stores times as 6-digit strings (HHMMSS) optionally followed by
/// timezone offset (±HHMM). ExifTool displays them with colon separators.
///
/// # Examples
///
/// ```
/// use oxidex::core::value_formatter::format_iptc_time;
///
/// assert_eq!(format_iptc_time("021111+0100"), "02:11:11+01:00");
/// assert_eq!(format_iptc_time("143000-0500"), "14:30:00-05:00");
/// assert_eq!(format_iptc_time("120000"), "12:00:00"); // No timezone
/// assert_eq!(format_iptc_time("bad"), "bad"); // Preserves invalid input
/// ```
pub fn format_iptc_time(raw: &str) -> String {
    if raw.len() >= 6 {
        let base = format!("{}:{}:{}", &raw[0..2], &raw[2..4], &raw[4..6]);
        if raw.len() >= 11 {
            // Format: HHMMSS±HHMM -> HH:MM:SS±HH:MM
            // Extract timezone: ±HHMM at positions 6-11
            let tz_sign = &raw[6..7];
            let tz_hours = &raw[7..9];
            let tz_mins = &raw[9..11];
            format!("{}{}{}:{}", base, tz_sign, tz_hours, tz_mins)
        } else {
            base
        }
    } else {
        raw.to_string()
    }
}

/// Format rational number as ExifTool does
///
/// Different tags have different formatting conventions:
/// - ExposureTime: Display as fraction (1/125) or decimal for >= 1 second
/// - FNumber: Display as decimal with one place (f/2.8)
/// - Other rationals: Display as fraction (num/denom)
///
/// # Examples
///
/// ```
/// use oxidex::core::value_formatter::format_rational;
///
/// // Exposure time as fraction
/// assert_eq!(format_rational(1, 125, "ExposureTime"), "1/125");
///
/// // Exposure time >= 1 second
/// assert_eq!(format_rational(2, 1, "ExposureTime"), "2.0");
///
/// // F-number as decimal
/// assert_eq!(format_rational(28, 10, "FNumber"), "2.8");
///
/// // Unknown tag as fraction
/// assert_eq!(format_rational(3, 2, "SomeTag"), "3/2");
///
/// // Division by zero
/// assert_eq!(format_rational(1, 0, "AnyTag"), "undef");
/// ```
pub fn format_rational(num: i32, denom: i32, tag_name: &str) -> String {
    if denom == 0 {
        return "undef".to_string();
    }

    // Some tags have special formatting
    match tag_name {
        "ExposureTime" => {
            let val = num as f64 / denom as f64;
            if val >= 1.0 {
                // Show as decimal for exposure >= 1 second
                format!("{:.1}", val)
            } else if num == 1 {
                // Show as simple fraction for 1/x
                format!("1/{}", denom)
            } else {
                // Show as approximate fraction
                format!("1/{:.0}", 1.0 / val)
            }
        }
        "FNumber" => {
            // F-number shown as decimal
            format!("{:.1}", num as f64 / denom as f64)
        }
        _ => {
            // Default: show as fraction
            format!("{}/{}", num, denom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_size_formatting() {
        // Bytes (< 1000)
        assert_eq!(format_file_size(0), "0 bytes");
        assert_eq!(format_file_size(1), "1 bytes");
        assert_eq!(format_file_size(500), "500 bytes");
        assert_eq!(format_file_size(999), "999 bytes");

        // Kilobytes (1000 - 999,999)
        assert_eq!(format_file_size(1000), "1.0 kB");
        assert_eq!(format_file_size(1500), "1.5 kB");
        assert_eq!(format_file_size(2100), "2.1 kB");
        assert_eq!(format_file_size(10_000), "10.0 kB");
        assert_eq!(format_file_size(999_999), "1000.0 kB");

        // Megabytes (1,000,000 - 999,999,999)
        assert_eq!(format_file_size(1_000_000), "1.0 MB");
        assert_eq!(format_file_size(1_500_000), "1.5 MB");
        assert_eq!(format_file_size(10_000_000), "10.0 MB");

        // Gigabytes (>= 1,000,000,000)
        assert_eq!(format_file_size(1_000_000_000), "1.0 GB");
        assert_eq!(format_file_size(2_500_000_000), "2.5 GB");
    }

    #[test]
    fn test_iptc_date_formatting() {
        // Valid dates
        assert_eq!(format_iptc_date("20020620"), "2002:06:20");
        assert_eq!(format_iptc_date("19991231"), "1999:12:31");
        assert_eq!(format_iptc_date("20250101"), "2025:01:01");

        // Invalid dates (preserved as-is)
        assert_eq!(format_iptc_date("2002620"), "2002620");
        assert_eq!(format_iptc_date("200206200"), "200206200");
        assert_eq!(format_iptc_date("invalid"), "invalid");
        assert_eq!(format_iptc_date(""), "");
    }

    #[test]
    fn test_iptc_time_formatting() {
        // With timezone
        assert_eq!(format_iptc_time("021111+0100"), "02:11:11+01:00");
        assert_eq!(format_iptc_time("143000-0500"), "14:30:00-05:00");
        assert_eq!(format_iptc_time("235959+0000"), "23:59:59+00:00");

        // Without timezone
        assert_eq!(format_iptc_time("120000"), "12:00:00");
        assert_eq!(format_iptc_time("000000"), "00:00:00");

        // Invalid times (preserved as-is)
        assert_eq!(format_iptc_time("12345"), "12345");
        assert_eq!(format_iptc_time("bad"), "bad");
        assert_eq!(format_iptc_time(""), "");
    }

    #[test]
    fn test_rational_formatting() {
        // ExposureTime - fractions
        assert_eq!(format_rational(1, 125, "ExposureTime"), "1/125");
        assert_eq!(format_rational(1, 1000, "ExposureTime"), "1/1000");

        // ExposureTime - >= 1 second
        assert_eq!(format_rational(2, 1, "ExposureTime"), "2.0");
        assert_eq!(format_rational(5, 2, "ExposureTime"), "2.5");

        // FNumber
        assert_eq!(format_rational(28, 10, "FNumber"), "2.8");
        assert_eq!(format_rational(56, 10, "FNumber"), "5.6");
        assert_eq!(format_rational(8, 1, "FNumber"), "8.0");

        // Other tags (default to fraction)
        assert_eq!(format_rational(3, 2, "SomeTag"), "3/2");
        assert_eq!(format_rational(100, 1, "OtherTag"), "100/1");

        // Division by zero
        assert_eq!(format_rational(1, 0, "ExposureTime"), "undef");
        assert_eq!(format_rational(1, 0, "FNumber"), "undef");
        assert_eq!(format_rational(1, 0, "AnyTag"), "undef");
    }

    #[test]
    fn test_exif_datetime_formatting() {
        use chrono::{TimeZone, Utc};

        let dt = Utc.with_ymd_and_hms(2002, 6, 20, 2, 11, 11).unwrap();
        assert_eq!(format_exif_datetime(&dt), "2002:06:20 02:11:11");

        let dt2 = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();
        assert_eq!(format_exif_datetime(&dt2), "2025:12:31 23:59:59");

        let dt3 = Utc.with_ymd_and_hms(1999, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(format_exif_datetime(&dt3), "1999:01:01 00:00:00");
    }
}
