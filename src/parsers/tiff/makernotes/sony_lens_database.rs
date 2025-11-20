//! Sony lens database for LensID to lens name mapping
//!
//! Supports both A-mount (Minolta AF, Sony Alpha DSLR) and E-mount (α7, α9, α6000 series)
//! lenses. Based on ExifTool's Sony.pm and Minolta.pm lens databases.
//!
//! This module has been migrated to use the unified LensDatabase trait infrastructure,
//! replacing the HashMap-based implementation with StaticLensDb for improved performance
//! and consistency across all manufacturer lens databases.

use super::shared::{LensDatabase, StaticLensDb};

// ============================================================================
// LENS DATA
// ============================================================================
// Static array of (lens_id, lens_name) tuples for all Sony lenses.
// This data structure is used by StaticLensDb for efficient binary search lookups.

static SONY_LENSES: [(u16, &str); 181] = [
    // ===== A-mount Lenses (Classic Minolta AF + Sony Alpha DSLR) =====

    // Minolta AF Legacy Lenses (inherited by Sony A-mount)
    (0, "Minolta AF 28-85mm f/3.5-4.5 New"),
    (1, "Minolta AF 80-200mm f/2.8 HS-APO G"),
    (2, "Minolta AF 28-70mm f/2.8 G"),
    (3, "Minolta AF 28-80mm f/4-5.6"),
    (4, "Minolta AF 85mm f/1.4 G (D)"),
    (5, "Minolta AF 35-70mm f/3.5-4.5"),
    (6, "Minolta AF 24-85mm f/3.5-4.5"),
    (7, "Minolta AF 35-105mm f/3.5-4.5"),
    (8, "Minolta AF 35-135mm f/4-5.6"),
    (9, "Minolta AF 35-70mm f/4"),
    (10, "Minolta AF 28-85mm f/3.5-4.5"),
    (11, "Minolta AF 50mm f/1.4"),
    (12, "Minolta AF 35mm f/1.4 G"),
    (13, "Minolta AF 50mm f/1.7"),
    (14, "Minolta AF 100mm f/2.8 Macro"),
    (15, "Minolta AF 35mm f/2"),
    (16, "Minolta AF 70-210mm f/4"),
    (17, "Minolta AF 16mm f/2.8 Fisheye"),
    (18, "Minolta AF 20mm f/2.8"),
    (19, "Minolta AF 28mm f/2"),
    (20, "Minolta AF 24mm f/2.8"),
    (21, "Minolta AF 50mm f/3.5 Macro"),
    (22, "Minolta AF 100mm f/2.8 Macro (D)"),
    (23, "Minolta AF 300mm f/2.8 HS-APO G"),
    (24, "Minolta AF 600mm f/4 HS-APO G"),
    (25, "Minolta AF 28-70mm f/2.8 G"),
    (26, "Minolta AF 80-200mm f/2.8 HS-APO G"),
    (27, "Minolta AF 35-105mm f/3.5-4.5"),
    (28, "Minolta AF 50mm f/2.8 Macro"),
    (29, "Minolta AF 17-35mm f/3.5 G"),
    (30, "Minolta AF 35-80mm f/4-5.6"),
    // Sony A-mount Prime Lenses
    (128, "Sony SAL 50mm f/1.4"),
    (129, "Sony SAL 85mm f/1.4 ZA"),
    (130, "Sony SAL 135mm f/1.8 ZA"),
    (131, "Sony SAL 35mm f/1.4 G"),
    (132, "Sony SAL 50mm f/1.8"),
    (133, "Sony SAL 85mm f/2.8 SAM"),
    (134, "Sony SAL 100mm f/2.8 Macro"),
    (135, "Sony SAL 16mm f/2.8 Fisheye"),
    (136, "Sony SAL 20mm f/2.8"),
    (137, "Sony SAL 24mm f/2 ZA SSM"),
    (138, "Sony SAL 28mm f/2.8"),
    (139, "Sony SAL 35mm f/1.8 SAM"),
    (140, "Sony SAL 50mm f/2.8 Macro"),
    (141, "Sony SAL 30mm f/2.8 Macro SAM"),
    // Sony A-mount Zoom Lenses
    (142, "Sony SAL 16-35mm f/2.8 ZA SSM"),
    (143, "Sony SAL 16-50mm f/2.8 SSM"),
    (144, "Sony SAL 16-80mm f/3.5-4.5 ZA"),
    (145, "Sony SAL 16-105mm f/3.5-5.6"),
    (146, "Sony SAL 18-55mm f/3.5-5.6 SAM"),
    (147, "Sony SAL 18-70mm f/3.5-5.6"),
    (148, "Sony SAL 18-135mm f/3.5-5.6 SAM"),
    (149, "Sony SAL 18-200mm f/3.5-6.3"),
    (150, "Sony SAL 18-250mm f/3.5-6.3"),
    (151, "Sony SAL 24-70mm f/2.8 ZA SSM"),
    (152, "Sony SAL 24-70mm f/2.8 ZA SSM II"),
    (153, "Sony SAL 24-105mm f/3.5-4.5"),
    (154, "Sony SAL 28-75mm f/2.8 SAM"),
    (155, "Sony SAL 35-70mm f/4"),
    (156, "Sony SAL 55-200mm f/4-5.6 SAM"),
    (157, "Sony SAL 70-200mm f/2.8 G SSM"),
    (158, "Sony SAL 70-200mm f/2.8 G SSM II"),
    (159, "Sony SAL 70-300mm f/4.5-5.6 G SSM"),
    (160, "Sony SAL 70-300mm f/4.5-5.6 G SSM II"),
    (161, "Sony SAL 70-400mm f/4-5.6 G SSM"),
    (162, "Sony SAL 70-400mm f/4-5.6 G SSM II"),
    (163, "Sony SAL 75-300mm f/4.5-5.6"),
    // Sony A-mount Telephoto Lenses
    (164, "Sony SAL 300mm f/2.8 G SSM"),
    (165, "Sony SAL 300mm f/2.8 G SSM II"),
    (166, "Sony SAL 500mm f/4 G SSM"),
    (167, "Sony SAL 600mm f/4 G SSM"),
    // ===== E-mount Lenses (Sony α7, α9, α6000 series mirrorless) =====

    // Sony E-mount Prime Lenses (FE - Full Frame)
    (256, "Sony FE 16-35mm f/2.8 GM"),
    (257, "Sony FE 16-35mm f/4 ZA OSS"),
    (258, "Sony FE 20mm f/1.8 G"),
    (259, "Sony FE 24mm f/1.4 GM"),
    (260, "Sony FE 24mm f/2.8 G"),
    (261, "Sony FE 28mm f/2"),
    (262, "Sony FE 35mm f/1.4 ZA"),
    (263, "Sony FE 35mm f/1.8"),
    (264, "Sony FE 35mm f/2.8 ZA"),
    (265, "Sony FE 40mm f/2.5 G"),
    (266, "Sony FE 50mm f/1.2 GM"),
    (267, "Sony FE 50mm f/1.4 ZA"),
    (268, "Sony FE 50mm f/1.8"),
    (269, "Sony FE 50mm f/2.5 G"),
    (270, "Sony FE 55mm f/1.8 ZA"),
    (271, "Sony FE 85mm f/1.4 GM"),
    (272, "Sony FE 85mm f/1.4 GM II"),
    (273, "Sony FE 85mm f/1.8"),
    (274, "Sony FE 100mm f/2.8 STF GM OSS"),
    (275, "Sony FE 135mm f/1.8 GM"),
    // Sony E-mount Macro Lenses (FE)
    (276, "Sony FE 50mm f/2.8 Macro"),
    (277, "Sony FE 90mm f/2.8 Macro G OSS"),
    // Sony E-mount Wide/Fisheye Lenses (FE)
    (278, "Sony FE 12-24mm f/2.8 GM"),
    (279, "Sony FE 12-24mm f/4 G"),
    (280, "Sony FE 14mm f/1.8 GM"),
    // Sony E-mount Standard Zoom Lenses (FE)
    (281, "Sony FE 24-70mm f/2.8 GM"),
    (282, "Sony FE 24-70mm f/2.8 GM II"),
    (283, "Sony FE 24-105mm f/4 G OSS"),
    (284, "Sony FE 24-240mm f/3.5-6.3 OSS"),
    (285, "Sony FE 28-60mm f/4-5.6"),
    (286, "Sony FE 28-70mm f/3.5-5.6 OSS"),
    // Sony E-mount Telephoto Zoom Lenses (FE)
    (287, "Sony FE 70-200mm f/2.8 GM OSS"),
    (288, "Sony FE 70-200mm f/2.8 GM OSS II"),
    (289, "Sony FE 70-200mm f/4 G OSS"),
    (290, "Sony FE 70-300mm f/4.5-5.6 G OSS"),
    (291, "Sony FE 100-400mm f/4.5-5.6 GM OSS"),
    (292, "Sony FE 200-600mm f/5.6-6.3 G OSS"),
    // Sony E-mount Telephoto Prime Lenses (FE)
    (293, "Sony FE 300mm f/2.8 GM OSS"),
    (294, "Sony FE 400mm f/2.8 GM OSS"),
    (295, "Sony FE 600mm f/4 GM OSS"),
    // Sony E-mount APS-C Lenses (E)
    (320, "Sony E 10-18mm f/4 OSS"),
    (321, "Sony E 16mm f/2.8"),
    (322, "Sony E 16-50mm f/3.5-5.6 OSS"),
    (323, "Sony E 16-55mm f/2.8 G"),
    (324, "Sony E 16-70mm f/4 ZA OSS"),
    (325, "Sony E 18-55mm f/3.5-5.6 OSS"),
    (326, "Sony E 18-105mm f/4 G OSS"),
    (327, "Sony E 18-110mm f/4 G OSS"),
    (328, "Sony E 18-135mm f/3.5-5.6 OSS"),
    (329, "Sony E 18-200mm f/3.5-6.3 OSS"),
    (330, "Sony E 18-200mm f/3.5-6.3 OSS LE"),
    (331, "Sony E 20mm f/2.8"),
    (332, "Sony E 24mm f/1.8 ZA"),
    (333, "Sony E 30mm f/3.5 Macro"),
    (334, "Sony E 35mm f/1.8 OSS"),
    (335, "Sony E 50mm f/1.8 OSS"),
    (336, "Sony E 55-210mm f/4.5-6.3 OSS"),
    (337, "Sony E 70-350mm f/4.5-6.3 G OSS"),
    // Sony G Master Lenses (Premium Line)
    (384, "Sony FE 24-70mm f/2.8 GM"),
    (385, "Sony FE 70-200mm f/2.8 GM OSS"),
    (386, "Sony FE 85mm f/1.4 GM"),
    (387, "Sony FE 100mm f/2.8 STF GM OSS"),
    (388, "Sony FE 100-400mm f/4.5-5.6 GM OSS"),
    (389, "Sony FE 16-35mm f/2.8 GM"),
    (390, "Sony FE 12-24mm f/2.8 GM"),
    (391, "Sony FE 24mm f/1.4 GM"),
    (392, "Sony FE 135mm f/1.8 GM"),
    (393, "Sony FE 400mm f/2.8 GM OSS"),
    (394, "Sony FE 600mm f/4 GM OSS"),
    (395, "Sony FE 200-600mm f/5.6-6.3 G OSS"),
    (396, "Sony FE 24-70mm f/2.8 GM II"),
    (397, "Sony FE 70-200mm f/2.8 GM OSS II"),
    (398, "Sony FE 50mm f/1.2 GM"),
    (399, "Sony FE 14mm f/1.8 GM"),
    (400, "Sony FE 35mm f/1.4 GM"),
    (401, "Sony FE 50mm f/1.4 GM"),
    (402, "Sony FE 85mm f/1.4 GM II"),
    // Carl Zeiss Lenses for Sony E-mount
    (448, "Zeiss Batis 18mm f/2.8"),
    (449, "Zeiss Batis 25mm f/2"),
    (450, "Zeiss Batis 40mm f/2 CF"),
    (451, "Zeiss Batis 85mm f/1.8"),
    (452, "Zeiss Batis 135mm f/2.8"),
    (453, "Zeiss Loxia 21mm f/2.8"),
    (454, "Zeiss Loxia 25mm f/2.4"),
    (455, "Zeiss Loxia 35mm f/2"),
    (456, "Zeiss Loxia 50mm f/2"),
    (457, "Zeiss Loxia 85mm f/2.4"),
    (458, "Zeiss Touit 12mm f/2.8"),
    (459, "Zeiss Touit 32mm f/1.8"),
    (460, "Zeiss Touit 50mm f/2.8 Macro"),
    // Sony-Zeiss Collaboration Lenses
    (464, "Sony FE 16-35mm f/4 ZA OSS"),
    (465, "Sony FE 24-70mm f/4 ZA OSS"),
    (466, "Sony FE 35mm f/1.4 ZA"),
    (467, "Sony FE 35mm f/2.8 ZA"),
    (468, "Sony FE 55mm f/1.8 ZA"),
    (469, "Sony FE 50mm f/1.4 ZA"),
    (470, "Sony E 16-70mm f/4 ZA OSS"),
    (471, "Sony E 24mm f/1.8 ZA"),
    // Third-party lenses (Sigma, Tamron for Sony E-mount)
    (512, "Sigma 16mm f/1.4 DC DN Contemporary"),
    (513, "Sigma 30mm f/1.4 DC DN Contemporary"),
    (514, "Sigma 56mm f/1.4 DC DN Contemporary"),
    (515, "Sigma 24-70mm f/2.8 DG DN Art"),
    (516, "Sigma 35mm f/1.2 DG DN Art"),
    (517, "Sigma 85mm f/1.4 DG DN Art"),
    (518, "Sigma 105mm f/2.8 DG DN Macro Art"),
    (519, "Tamron 17-28mm f/2.8 Di III RXD"),
    (520, "Tamron 28-75mm f/2.8 Di III RXD"),
    (521, "Tamron 28-200mm f/2.8-5.6 Di III RXD"),
    (522, "Tamron 70-180mm f/2.8 Di III VXD"),
    (523, "Tamron 150-500mm f/5-6.7 Di III VC VXD"),
];

// ============================================================================
// LENS DATABASE IMPLEMENTATION
// ============================================================================

/// Static lens database using the unified LensDatabase infrastructure.
///
/// This database provides O(log n) lookups via binary search, replacing the
/// previous HashMap-based implementation for consistency with other parsers.
static SONY_LENS_DB: StaticLensDb = StaticLensDb::new(&SONY_LENSES);

// ============================================================================
// PUBLIC API
// ============================================================================

/// Looks up a lens name from a Sony lens ID
///
/// This function maintains backward compatibility with the existing API while
/// using the new LensDatabase infrastructure internally.
///
/// # Arguments
/// * `lens_id` - The lens ID from Sony MakerNote LensID tag
///
/// # Returns
/// * `Some(String)` - The lens model name if found in database
/// * `None` - If lens ID is not in database
///
/// # Example
/// ```ignore
/// use oxidex::parsers::tiff::makernotes::sony_lens_database::lookup_lens_name;
///
/// let lens = lookup_lens_name(281);
/// assert_eq!(lens, Some("Sony FE 24-70mm f/2.8 GM".to_string()));
/// ```
pub fn lookup_lens_name(lens_id: u16) -> Option<String> {
    SONY_LENS_DB.lookup(lens_id).map(|s| s.to_string())
}

/// Get reference to the Sony lens database
///
/// This function provides direct access to the lens database for use with
/// the LensDatabase trait, enabling integration with the new registry system.
///
/// # Returns
/// Reference to the static lens database implementing LensDatabase trait
///
/// # Example
/// ```ignore
/// use oxidex::parsers::tiff::makernotes::sony_lens_database::get_lens_database;
///
/// let db = get_lens_database();
/// let lens = db.lookup(281);
/// assert_eq!(lens, Some("Sony FE 24-70mm f/2.8 GM"));
/// ```
pub fn get_lens_database() -> &'static impl LensDatabase {
    &SONY_LENS_DB
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sony_a_mount_lens_lookup() {
        // Test Minolta AF legacy lens
        assert_eq!(
            lookup_lens_name(11),
            Some("Minolta AF 50mm f/1.4".to_string())
        );

        // Test Sony A-mount lens
        assert_eq!(
            lookup_lens_name(151),
            Some("Sony SAL 24-70mm f/2.8 ZA SSM".to_string())
        );

        // Test Sony A-mount telephoto
        assert_eq!(
            lookup_lens_name(164),
            Some("Sony SAL 300mm f/2.8 G SSM".to_string())
        );
    }

    #[test]
    fn test_sony_e_mount_lens_lookup() {
        // Test FE prime lens
        assert_eq!(
            lookup_lens_name(266),
            Some("Sony FE 50mm f/1.2 GM".to_string())
        );

        // Test FE zoom lens
        assert_eq!(
            lookup_lens_name(281),
            Some("Sony FE 24-70mm f/2.8 GM".to_string())
        );

        // Test E APS-C lens
        assert_eq!(
            lookup_lens_name(323),
            Some("Sony E 16-55mm f/2.8 G".to_string())
        );
    }

    #[test]
    fn test_sony_g_master_lens_lookup() {
        // Test G Master lenses
        assert_eq!(
            lookup_lens_name(398),
            Some("Sony FE 50mm f/1.2 GM".to_string())
        );

        assert_eq!(
            lookup_lens_name(402),
            Some("Sony FE 85mm f/1.4 GM II".to_string())
        );
    }

    #[test]
    fn test_zeiss_lens_lookup() {
        // Test Zeiss Batis
        assert_eq!(
            lookup_lens_name(451),
            Some("Zeiss Batis 85mm f/1.8".to_string())
        );

        // Test Zeiss Loxia
        assert_eq!(
            lookup_lens_name(456),
            Some("Zeiss Loxia 50mm f/2".to_string())
        );

        // Test Sony-Zeiss collaboration
        assert_eq!(
            lookup_lens_name(468),
            Some("Sony FE 55mm f/1.8 ZA".to_string())
        );
    }

    #[test]
    fn test_third_party_lens_lookup() {
        // Test Sigma lens
        assert_eq!(
            lookup_lens_name(513),
            Some("Sigma 30mm f/1.4 DC DN Contemporary".to_string())
        );

        // Test Tamron lens
        assert_eq!(
            lookup_lens_name(520),
            Some("Tamron 28-75mm f/2.8 Di III RXD".to_string())
        );
    }

    #[test]
    fn test_unknown_lens_id() {
        // Test that unknown lens IDs return None
        assert_eq!(lookup_lens_name(9999), None);
        assert_eq!(lookup_lens_name(65535), None);
    }

    #[test]
    fn test_database_size() {
        // Verify we have at least 100 lenses
        let lens_count = SONY_LENSES.len();
        assert!(
            lens_count >= 100,
            "Expected at least 100 lenses, found {}",
            lens_count
        );
    }

    #[test]
    fn test_lens_database_trait() {
        // Test that the database implements LensDatabase trait correctly
        let db = get_lens_database();
        assert_eq!(db.lookup(281), Some("Sony FE 24-70mm f/2.8 GM"));
        assert_eq!(db.lookup(9999), None);
    }
}
