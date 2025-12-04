//! GPS Movement and Tracking Tag Integration Tests
//!
//! Tests for forensic geolocation analysis tags including:
//! - GPSSpeed/GPSSpeedRef - Vehicle/object speed tracking
//! - GPSTrack/GPSTrackRef - Direction of movement analysis
//! - GPSImgDirection/GPSImgDirectionRef - Camera orientation tracking
//! - GPSDestBearing - Navigation bearing analysis
//! - GPSDestDistance - Distance calculations
//! - GPSHPositioningError - GPS accuracy assessment

#[cfg(test)]
mod tests {
    use oxidex::core::operations;
    use std::collections::HashMap;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a minimal TIFF file with GPS IFD and movement tags
    fn create_test_tiff_with_gps_movement() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();

        // TIFF header (little-endian, magic 42, IFD offset at 8)
        file.write_all(&[0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00])
            .unwrap();

        // IFD0: 1 entry pointing to GPS IFD
        file.write_all(&[0x01, 0x00]).unwrap(); // 1 entry

        // GPS IFD Pointer (tag 0x8825)
        file.write_all(&[0x25, 0x88]).unwrap(); // Tag 0x8825
        file.write_all(&[0x04, 0x00]).unwrap(); // Type: LONG
        file.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap(); // Count: 1
        file.write_all(&[0x1A, 0x00, 0x00, 0x00]).unwrap(); // Offset to GPS IFD

        // Next IFD offset (0 = none)
        file.write_all(&[0x00, 0x00, 0x00, 0x00]).unwrap();

        // GPS IFD at offset 0x1A (26)
        // We'll write 9 entries for GPS movement tags
        file.write_all(&[0x09, 0x00]).unwrap(); // 9 entries

        // Entry 1: GPSVersionID (0x0000)
        file.write_all(&[0x00, 0x00]).unwrap(); // Tag
        file.write_all(&[0x01, 0x00]).unwrap(); // Type: BYTE
        file.write_all(&[0x04, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[0x02, 0x02, 0x00, 0x00]).unwrap(); // Value: 2.2.0.0

        // Entry 2: GPSSpeedRef (0x000C)
        file.write_all(&[0x0C, 0x00]).unwrap(); // Tag
        file.write_all(&[0x02, 0x00]).unwrap(); // Type: ASCII
        file.write_all(&[0x02, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[b'K', 0x00, 0x00, 0x00]).unwrap(); // Value: "K" (km/h)

        // Entry 3: GPSSpeed (0x000D) - 55.5 km/h = 555/10
        file.write_all(&[0x0D, 0x00]).unwrap(); // Tag
        file.write_all(&[0x05, 0x00]).unwrap(); // Type: RATIONAL
        file.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[0xB8, 0x00, 0x00, 0x00]).unwrap(); // Offset to rational

        // Entry 4: GPSTrackRef (0x000E)
        file.write_all(&[0x0E, 0x00]).unwrap(); // Tag
        file.write_all(&[0x02, 0x00]).unwrap(); // Type: ASCII
        file.write_all(&[0x02, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[b'T', 0x00, 0x00, 0x00]).unwrap(); // Value: "T" (true north)

        // Entry 5: GPSTrack (0x000F) - 275.5 degrees = 2755/10
        file.write_all(&[0x0F, 0x00]).unwrap(); // Tag
        file.write_all(&[0x05, 0x00]).unwrap(); // Type: RATIONAL
        file.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[0xC0, 0x00, 0x00, 0x00]).unwrap(); // Offset to rational

        // Entry 6: GPSImgDirectionRef (0x0010)
        file.write_all(&[0x10, 0x00]).unwrap(); // Tag
        file.write_all(&[0x02, 0x00]).unwrap(); // Type: ASCII
        file.write_all(&[0x02, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[b'M', 0x00, 0x00, 0x00]).unwrap(); // Value: "M" (magnetic)

        // Entry 7: GPSImgDirection (0x0011) - 90.25 degrees = 9025/100
        file.write_all(&[0x11, 0x00]).unwrap(); // Tag
        file.write_all(&[0x05, 0x00]).unwrap(); // Type: RATIONAL
        file.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[0xC8, 0x00, 0x00, 0x00]).unwrap(); // Offset to rational

        // Entry 8: GPSDestBearing (0x0018) - 45.25 degrees = 4525/100
        file.write_all(&[0x18, 0x00]).unwrap(); // Tag
        file.write_all(&[0x05, 0x00]).unwrap(); // Type: RATIONAL
        file.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[0xD0, 0x00, 0x00, 0x00]).unwrap(); // Offset to rational

        // Entry 9: GPSHPositioningError (0x001F) - 8.5 m = 85/10
        file.write_all(&[0x1F, 0x00]).unwrap(); // Tag
        file.write_all(&[0x05, 0x00]).unwrap(); // Type: RATIONAL
        file.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap(); // Count
        file.write_all(&[0xD8, 0x00, 0x00, 0x00]).unwrap(); // Offset to rational

        // Next IFD offset
        file.write_all(&[0x00, 0x00, 0x00, 0x00]).unwrap();

        // RATIONAL values at offsets
        // 0xB8: GPSSpeed = 555/10
        file.write_all(&[0x2B, 0x02, 0x00, 0x00]).unwrap(); // 555
        file.write_all(&[0x0A, 0x00, 0x00, 0x00]).unwrap(); // 10

        // 0xC0: GPSTrack = 2755/10
        file.write_all(&[0xC3, 0x0A, 0x00, 0x00]).unwrap(); // 2755
        file.write_all(&[0x0A, 0x00, 0x00, 0x00]).unwrap(); // 10

        // 0xC8: GPSImgDirection = 9025/100
        file.write_all(&[0x41, 0x23, 0x00, 0x00]).unwrap(); // 9025
        file.write_all(&[0x64, 0x00, 0x00, 0x00]).unwrap(); // 100

        // 0xD0: GPSDestBearing = 4525/100
        file.write_all(&[0xAD, 0x11, 0x00, 0x00]).unwrap(); // 4525
        file.write_all(&[0x64, 0x00, 0x00, 0x00]).unwrap(); // 100

        // 0xD8: GPSHPositioningError = 85/10
        file.write_all(&[0x55, 0x00, 0x00, 0x00]).unwrap(); // 85
        file.write_all(&[0x0A, 0x00, 0x00, 0x00]).unwrap(); // 10

        file.flush().unwrap();
        file
    }

    #[test]
    fn test_gps_speed_and_ref_extraction() {
        let temp_file = create_test_tiff_with_gps_movement();
        let path = temp_file.path().to_str().unwrap();

        let mut tags = HashMap::new();
        operations::read_metadata(path, &mut tags).unwrap();

        // Verify GPSSpeedRef
        assert_eq!(
            tags.get("GPS:GPSSpeedRef").map(|s| s.as_str()),
            Some("K"),
            "GPSSpeedRef should be 'K' for km/h"
        );

        // Verify GPSSpeed - should be formatted as 55.50
        assert_eq!(
            tags.get("GPS:GPSSpeed").map(|s| s.as_str()),
            Some("55.50"),
            "GPSSpeed should be formatted as decimal"
        );
    }

    #[test]
    fn test_gps_track_and_ref_extraction() {
        let temp_file = create_test_tiff_with_gps_movement();
        let path = temp_file.path().to_str().unwrap();

        let mut tags = HashMap::new();
        operations::read_metadata(path, &mut tags).unwrap();

        // Verify GPSTrackRef
        assert_eq!(
            tags.get("GPS:GPSTrackRef").map(|s| s.as_str()),
            Some("T"),
            "GPSTrackRef should be 'T' for true north"
        );

        // Verify GPSTrack - should be formatted as 275.50
        assert_eq!(
            tags.get("GPS:GPSTrack").map(|s| s.as_str()),
            Some("275.50"),
            "GPSTrack should be formatted as decimal degrees"
        );
    }

    #[test]
    fn test_gps_img_direction_and_ref_extraction() {
        let temp_file = create_test_tiff_with_gps_movement();
        let path = temp_file.path().to_str().unwrap();

        let mut tags = HashMap::new();
        operations::read_metadata(path, &mut tags).unwrap();

        // Verify GPSImgDirectionRef
        assert_eq!(
            tags.get("GPS:GPSImgDirectionRef").map(|s| s.as_str()),
            Some("M"),
            "GPSImgDirectionRef should be 'M' for magnetic north"
        );

        // Verify GPSImgDirection - should be formatted as 90.25
        assert_eq!(
            tags.get("GPS:GPSImgDirection").map(|s| s.as_str()),
            Some("90.25"),
            "GPSImgDirection should be formatted as decimal degrees"
        );
    }

    #[test]
    fn test_gps_dest_bearing_extraction() {
        let temp_file = create_test_tiff_with_gps_movement();
        let path = temp_file.path().to_str().unwrap();

        let mut tags = HashMap::new();
        operations::read_metadata(path, &mut tags).unwrap();

        // Verify GPSDestBearing - should be formatted as 45.25
        assert_eq!(
            tags.get("GPS:GPSDestBearing").map(|s| s.as_str()),
            Some("45.25"),
            "GPSDestBearing should be formatted as decimal degrees"
        );
    }

    #[test]
    fn test_gps_h_positioning_error_extraction() {
        let temp_file = create_test_tiff_with_gps_movement();
        let path = temp_file.path().to_str().unwrap();

        let mut tags = HashMap::new();
        operations::read_metadata(path, &mut tags).unwrap();

        // Verify GPSHPositioningError - should be formatted as "8.50 m"
        assert_eq!(
            tags.get("GPS:GPSHPositioningError").map(|s| s.as_str()),
            Some("8.50 m"),
            "GPSHPositioningError should be formatted with meters unit"
        );
    }

    #[test]
    fn test_gps_movement_forensic_scenario() {
        // Complete forensic scenario: Vehicle moving at 55.5 km/h, heading 275.5 degrees
        let temp_file = create_test_tiff_with_gps_movement();
        let path = temp_file.path().to_str().unwrap();

        let mut tags = HashMap::new();
        operations::read_metadata(path, &mut tags).unwrap();

        // Verify all movement tags are present and correctly formatted
        let movement_tags = vec![
            ("GPS:GPSSpeedRef", "K"),
            ("GPS:GPSSpeed", "55.50"),
            ("GPS:GPSTrackRef", "T"),
            ("GPS:GPSTrack", "275.50"),
            ("GPS:GPSImgDirectionRef", "M"),
            ("GPS:GPSImgDirection", "90.25"),
            ("GPS:GPSDestBearing", "45.25"),
            ("GPS:GPSHPositioningError", "8.50 m"),
        ];

        for (tag_name, expected_value) in movement_tags {
            assert_eq!(
                tags.get(tag_name).map(|s| s.as_str()),
                Some(expected_value),
                "Tag {} should have value {}",
                tag_name,
                expected_value
            );
        }

        // Forensic analysis assertions
        // 1. Speed is in km/h (K)
        assert_eq!(tags.get("GPS:GPSSpeedRef").unwrap(), "K");

        // 2. Speed is a reasonable vehicle speed (55.5 km/h)
        let speed: f64 = tags
            .get("GPS:GPSSpeed")
            .unwrap()
            .parse()
            .expect("Speed should be parseable as float");
        assert!(speed > 0.0 && speed < 200.0, "Speed should be reasonable");

        // 3. Track is in valid degree range (0-360)
        let track: f64 = tags
            .get("GPS:GPSTrack")
            .unwrap()
            .parse()
            .expect("Track should be parseable as float");
        assert!(
            track >= 0.0 && track < 360.0,
            "Track should be in valid degree range"
        );

        // 4. GPS positioning error indicates fix quality
        let error_str = tags.get("GPS:GPSHPositioningError").unwrap();
        assert!(
            error_str.ends_with(" m"),
            "Positioning error should have meter unit"
        );
        let error: f64 = error_str
            .trim_end_matches(" m")
            .parse()
            .expect("Error should be parseable as float");
        assert!(error > 0.0, "Positioning error should be positive");
    }

    #[test]
    fn test_gps_movement_tag_registry_lookup() {
        // Verify all GPS movement tags are registered in the tag database
        use oxidex::tag_db::get_tag_info;

        let movement_tags = vec![
            "GPS:GPSSpeedRef",
            "GPS:GPSSpeed",
            "GPS:GPSTrackRef",
            "GPS:GPSTrack",
            "GPS:GPSImgDirectionRef",
            "GPS:GPSImgDirection",
            "GPS:GPSDestBearing",
            "GPS:GPSDestDistance",
            "GPS:GPSHPositioningError",
        ];

        for tag_name in movement_tags {
            let tag_info = get_tag_info(tag_name);
            assert!(
                tag_info.is_some(),
                "Tag {} should be registered in tag database",
                tag_name
            );

            let info = tag_info.unwrap();
            assert!(
                info.tag_name.contains("GPS:"),
                "Tag should be in GPS family"
            );
        }
    }
}
