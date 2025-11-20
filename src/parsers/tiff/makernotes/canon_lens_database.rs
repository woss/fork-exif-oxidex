//! Canon lens database using unified LensDatabase infrastructure
//!
//! Based on ExifTool's Canon.pm %canonLensTypes hash

use super::shared::{LensDatabase, StaticLensDb};

// Canon lens data as static array for zero-overhead lookup
static CANON_LENSES: [(u16, &str); 146] = [
    // Most common Canon EF lenses (sorted by ID)
    (1, "Canon EF 50mm f/1.8"),
    (2, "Canon EF 28mm f/2.8"),
    (3, "Canon EF 135mm f/2.8 Soft-Focus"),
    (4, "Canon EF 35-70mm f/3.5-4.5"),
    (5, "Canon EF 35-105mm f/3.5-4.5"),
    (6, "Canon EF 75-300mm f/4-5.6"),
    (7, "Canon EF 100-300mm f/5.6L"),
    (8, "Canon EF 100mm f/2.8 Macro"),
    (9, "Canon EF 35mm f/2"),
    (10, "Canon EF 15mm f/2.8 Fisheye"),
    (11, "Canon EF 50-200mm f/3.5-4.5L"),
    (13, "Canon EF 50mm f/1.4"),
    (14, "Canon EF 300mm f/2.8L"),
    (15, "Canon EF 50-200mm f/3.5-4.5"),
    (16, "Canon EF 35-135mm f/3.5-4.5"),
    (17, "Canon EF 35-70mm f/3.5-4.5A"),
    (18, "Canon EF 28-70mm f/3.5-4.5"),
    (20, "Canon EF 100-200mm f/4.5A"),
    (21, "Canon EF 35-135mm f/4-5.6 USM"),
    (22, "Canon EF 80-200mm f/2.8L"),
    (23, "Canon EF 35-105mm f/3.5-4.5 USM"),
    (24, "Canon EF 35-80mm f/4-5.6 Power Zoom"),
    (26, "Canon EF 100-300mm f/5.6L"),
    (27, "Canon EF 100mm f/2"),
    (
        28,
        "Canon EF 14mm f/2.8L or Sigma 14mm f/2.8 EX Aspherical HSM",
    ),
    (29, "Canon EF 200mm f/2.8L"),
    (30, "Canon EF 300mm f/2.8L"),
    (31, "Canon EF 400mm f/2.8L"),
    (32, "Canon EF 500mm f/4.5L"),
    (35, "Canon EF 135mm f/2L"),
    (36, "Canon EF 600mm f/4L"),
    (37, "Canon EF 24-85mm f/3.5-4.5 USM"),
    (38, "Canon EF 300mm f/4L"),
    (39, "Canon EF 400mm f/5.6L"),
    (40, "Canon EF 500mm f/4.5L USM"),
    (41, "Canon EF 100-400mm f/4.5-5.6L IS USM"),
    (42, "Canon EF 70-210mm f/3.5-4.5 USM"),
    (43, "Canon EF 80-200mm f/4.5-5.6 USM"),
    (44, "Canon EF 35-80mm f/4-5.6 USM"),
    (45, "Canon EF 50mm f/1.0L"),
    (48, "Canon EF 50mm f/1.8 II"),
    (49, "Canon EF 28-105mm f/3.5-4.5 USM"),
    (50, "Canon EF 17-40mm f/4L USM"),
    (51, "Canon EF 10-22mm f/3.5-4.5 USM"),
    (124, "Canon MP-E 65mm f/2.8 1-5x Macro Photo"),
    (125, "Canon TS-E 24mm f/3.5L"),
    (126, "Canon TS-E 45mm f/2.8"),
    (127, "Canon TS-E 90mm f/2.8"),
    (129, "Canon EF 300mm f/2.8L"),
    (130, "Canon EF 50mm f/1.0L"),
    (131, "Canon EF 28-80mm f/2.8-4L or Sigma 24-70mm f/2.8 EX"),
    (132, "Canon EF 1200mm f/5.6L"),
    (134, "Canon EF 600mm f/4L IS"),
    (135, "Canon EF 200mm f/1.8L"),
    (136, "Canon EF 300mm f/2.8L"),
    (137, "Canon EF 85mm f/1.2L or Sigma 15mm f/2.8 EX Fisheye"),
    (138, "Canon EF 28-80mm f/2.8-4L"),
    (139, "Canon EF 400mm f/2.8L"),
    (140, "Canon EF 500mm f/4L IS"),
    (
        141,
        "Canon EF 500mm f/4L IS or Sigma 17-35mm f/2.8-4 EX Aspherical",
    ),
    (142, "Canon EF 300mm f/2.8L IS"),
    (143, "Canon EF 500mm f/4L"),
    (149, "Canon EF 100mm f/2"),
    (
        150,
        "Canon EF 14mm f/2.8L or Sigma 20mm f/1.8 EX Aspherical",
    ),
    (151, "Canon EF 200mm f/2.8L"),
    (152, "Canon EF 300mm f/4L IS or Sigma 55-200mm f/4-5.6 DC"),
    (
        153,
        "Canon EF 35-350mm f/3.5-5.6L or Sigma 28-300mm f/3.5-6.3 Macro",
    ),
    (
        154,
        "Canon EF 20mm f/2.8 USM or Tamron AF 28-300mm f/3.5-6.3 XR Di VC",
    ),
    (155, "Canon EF 85mm f/1.8 USM or Sigma 30mm f/1.4 EX DC HSM"),
    (
        156,
        "Canon EF 28-105mm f/3.5-4.5 USM or Tamron AF 90mm f/2.8 Di Macro",
    ),
    (
        160,
        "Canon EF 20-35mm f/3.5-4.5 USM or Tamron AF 19-35mm f/3.5-4.5",
    ),
    (161, "Canon EF 28-70mm f/2.8L or Sigma 24-70mm f/2.8 EX"),
    (162, "Canon EF 200mm f/2.8L"),
    (163, "Canon EF 300mm f/4L"),
    (164, "Canon EF 400mm f/5.6L"),
    (165, "Canon EF 70-200mm f/2.8L"),
    (166, "Canon EF 70-200mm f/2.8L + 1.4x"),
    (167, "Canon EF 70-200mm f/2.8L + 2x"),
    (
        168,
        "Canon EF 28mm f/1.8 USM or Sigma 50-500mm f/4-6.3 APO HSM EX",
    ),
    (
        169,
        "Canon EF 17-35mm f/2.8L or Sigma 18-200mm f/3.5-6.3 DC OS",
    ),
    (170, "Canon EF 200mm f/2.8L II"),
    (171, "Canon EF 300mm f/4L"),
    (
        172,
        "Canon EF 400mm f/5.6L or Sigma 150-600mm f/5-6.3 DG OS HSM | S",
    ),
    (
        173,
        "Canon EF 180mm Macro f/3.5L or Sigma 180mm EX HSM Macro f/3.5",
    ),
    (174, "Canon EF 135mm f/2L or Sigma 28mm f/1.8 DG Macro EX"),
    (175, "Canon EF 400mm f/2.8L"),
    (176, "Canon EF 24-85mm f/3.5-4.5 USM"),
    (177, "Canon EF 300mm f/4L IS"),
    (178, "Canon EF 28-135mm f/3.5-5.6 IS"),
    (179, "Canon EF 24mm f/1.4L"),
    (180, "Canon EF 35mm f/1.4L or Sigma 50mm f/1.4 EX DG HSM"),
    (181, "Canon EF 100-400mm f/4.5-5.6L IS"),
    (182, "Canon EF 70-200mm f/4L"),
    (183, "Canon EF 70-200mm f/4L + 1.4x"),
    (184, "Canon EF 70-200mm f/4L + 2x"),
    (185, "Canon EF 70-200mm f/4L + 2.8x"),
    (186, "Canon EF 70-200mm f/2.8L IS"),
    (187, "Canon EF 70-200mm f/2.8L IS + 1.4x"),
    (188, "Canon EF 70-200mm f/2.8L IS + 2x"),
    (189, "Canon EF 70-200mm f/2.8L IS + 2.8x"),
    (190, "Canon EF 100mm f/2.8 Macro"),
    (191, "Canon EF 400mm f/4 DO IS"),
    (193, "Canon EF 35-80mm f/4-5.6 USM"),
    (194, "Canon EF 80-200mm f/4.5-5.6 USM"),
    (195, "Canon EF 35-105mm f/4.5-5.6 USM"),
    (196, "Canon EF 75-300mm f/4-5.6 IS USM"),
    (197, "Canon EF 75-300mm f/4-5.6 USM"),
    (198, "Canon EF 50mm f/1.4 USM"),
    (199, "Canon EF 28-80mm f/3.5-5.6 USM"),
    (200, "Canon EF 75-300mm f/4-5.6 USM"),
    (224, "Canon EF 70-200mm f/2.8L IS II"),
    (225, "Canon EF 70-200mm f/2.8L IS II + 1.4x"),
    (226, "Canon EF 70-200mm f/2.8L IS II + 2x"),
    (
        234,
        "Canon EF 200mm f/2L IS or Sigma 24-105mm f/4 DG OS HSM | A",
    ),
    (235, "Canon EF 800mm f/5.6L IS"),
    (236, "Canon EF 24mm f/1.4L II or Sigma 35mm f/1.4 DG HSM"),
    (237, "Canon EF 70-300mm f/4-5.6L IS USM"),
    (248, "Canon EF 16-35mm f/2.8L II"),
    (251, "Canon EF 300mm f/2.8L IS II"),
    (252, "Canon EF 400mm f/2.8L IS II"),
    (254, "Canon EF 500mm f/4L IS II or EF 24-105mm f/4L IS USM"),
    (255, "Canon EF 600mm f/4L IS II"),
    (368, "Canon EF 24-70mm f/2.8L II USM"),
    (488, "Canon EF 16-35mm f/4L IS USM"),
    (489, "Canon EF 24-105mm f/3.5-5.6 IS STM"),
    (4142, "Canon EF 24mm f/2.8 IS USM"),
    (4143, "Canon EF 28mm f/2.8 IS USM"),
    (4144, "Canon EF-S 24mm f/2.8 STM"),
    (4145, "Canon EF-M 28mm f/3.5 Macro IS STM"),
    (4146, "Canon EF 24-105mm f/4L IS II USM"),
    (4147, "Canon EF 16-35mm f/2.8L III USM"),
    (4150, "Canon EF 24-70mm f/2.8L III USM"),
    (4152, "Canon EF 100-400mm f/4.5-5.6L IS II USM"),
    (4156, "Canon EF 50mm f/1.8 STM"),
    (61182, "Canon RF 24-105mm f/4L IS USM"),
    (61183, "Canon RF 28-70mm f/2L USM"),
    (61184, "Canon RF 50mm f/1.2L USM"),
    (61185, "Canon RF 24-70mm f/2.8L IS USM"),
    (61186, "Canon RF 15-35mm f/2.8L IS USM"),
    (61187, "Canon RF 70-200mm f/2.8L IS USM"),
    (61188, "Canon RF 85mm f/1.2L USM"),
    (61189, "Canon RF 100-500mm f/4.5-7.1L IS USM"),
    (61190, "Canon RF 600mm f/11 IS STM"),
    (61191, "Canon RF 800mm f/11 IS STM"),
    (61192, "Canon RF 24-240mm f/4-6.3 IS USM"),
    (61193, "Canon RF 35mm f/1.8 IS STM Macro"),
];

// Unified lens database using StaticLensDb for zero-overhead lookups
static CANON_LENS_DB: StaticLensDb = StaticLensDb::new(&CANON_LENSES);

/// Looks up a lens name from a Canon lens ID
///
/// # Arguments
/// * `lens_id` - The lens ID from CameraInfo or LensInfo arrays
///
/// # Returns
/// * `Some(String)` - The lens model name if found
/// * `None` - If lens ID is not in database
///
/// # Note
/// This function maintains backward compatibility with the old HashMap-based implementation.
/// New code should prefer using `get_lens_database()` and the `LensDatabase` trait directly.
pub fn lookup_lens_name(lens_id: u16) -> Option<String> {
    CANON_LENS_DB.lookup(lens_id).map(|s| s.to_string())
}

/// Get Canon lens database for direct use with LensDatabase trait
///
/// Returns a reference to the static lens database that implements the LensDatabase trait.
/// This is the preferred way to access the lens database in new code.
///
/// # Example
/// ```ignore
/// use oxidex::parsers::tiff::makernotes::canon_lens_database::get_lens_database;
/// use oxidex::parsers::tiff::makernotes::shared::LensDatabase;
///
/// let db = get_lens_database();
/// if let Some(name) = db.lookup(368) {
///     println!("Lens: {}", name);
/// }
/// ```
pub fn get_lens_database() -> &'static impl LensDatabase {
    &CANON_LENS_DB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_common_lens() {
        // Canon EF 50mm f/1.8 STM
        let result = lookup_lens_name(4156);
        assert_eq!(result, Some("Canon EF 50mm f/1.8 STM".to_string()));
    }

    #[test]
    fn test_lookup_l_series_lens() {
        // Canon EF 24-70mm f/2.8L II USM
        let result = lookup_lens_name(368);
        assert_eq!(result, Some("Canon EF 24-70mm f/2.8L II USM".to_string()));
    }

    #[test]
    fn test_lookup_rf_lens() {
        // Canon RF 24-105mm f/4L IS USM
        let result = lookup_lens_name(61182);
        assert_eq!(result, Some("Canon RF 24-105mm f/4L IS USM".to_string()));
    }

    #[test]
    fn test_lookup_unknown_lens() {
        let result = lookup_lens_name(65000);
        assert_eq!(result, None);
    }

    #[test]
    fn test_database_size() {
        // Should have 100+ lens entries
        assert!(
            CANON_LENSES.len() >= 100,
            "Expected at least 100 lens entries, found {}",
            CANON_LENSES.len()
        );
    }

    #[test]
    fn test_lens_database_trait() {
        // Test that the LensDatabase trait works correctly
        let db = get_lens_database();
        assert_eq!(db.lookup(368), Some("Canon EF 24-70mm f/2.8L II USM"));
        assert_eq!(db.lookup(65000), None);
    }
}
