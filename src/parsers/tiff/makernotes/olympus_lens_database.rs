//! Olympus lens database using unified LensDatabase infrastructure
//!
//! Based on ExifTool's Olympus.pm lens database, covering both:
//! - Four Thirds (4/3): Zuiko Digital lenses for E-series DSLRs
//! - Micro Four Thirds (M.Zuiko): Current mirrorless system (OM-D, PEN)
//! - Olympus Pro series and Premium primes
//!
//! Olympus uses both legacy lens IDs and newer hex-based IDs for M.Zuiko lenses

use super::shared::{LensDatabase, StaticLensDb};

// Olympus lens data as static array for zero-overhead lookup
static OLYMPUS_LENSES: [(u16, &str); 95] = [
    (0, "None"),
    (1, "Olympus Zuiko Digital ED 50mm f/2.0 Macro"),
    (2, "Olympus Zuiko Digital 40-150mm f/3.5-4.5"),
    (3, "Olympus Zuiko Digital ED 300mm f/2.8"),
    (4, "Olympus Zuiko Digital 14-54mm f/2.8-3.5"),
    (5, "Olympus Zuiko Digital ED 50-200mm f/2.8-3.5"),
    (6, "Olympus Zuiko Digital ED 7-14mm f/4.0"),
    (7, "Olympus Zuiko Digital 11-22mm f/2.8-3.5"),
    (8, "Olympus Zuiko Digital ED 50-200mm f/2.8-3.5 SWD"),
    (9, "Olympus Zuiko Digital ED 12-60mm f/2.8-4.0 SWD"),
    (10, "Olympus Zuiko Digital ED 14-35mm f/2.0 SWD"),
    (11, "Olympus Zuiko Digital 25mm f/2.8"),
    (12, "Olympus Zuiko Digital ED 9-18mm f/4.0-5.6"),
    (13, "Olympus Zuiko Digital 14-45mm f/3.5-5.6"),
    (14, "Olympus Zuiko Digital 35mm f/3.5 Macro"),
    (15, "Olympus Zuiko Digital ED 14-42mm f/3.5-5.6"),
    (16, "Olympus Zuiko Digital ED 40-150mm f/4.0-5.6"),
    (17, "Olympus Zuiko Digital ED 70-300mm f/4.0-5.6"),
    (18, "Olympus Zuiko Digital ED 18-180mm f/3.5-6.3"),
    (19, "Olympus Zuiko Digital ED 100-400mm f/5.0-6.3"),
    (32, "M.Zuiko Digital ED 14-42mm f/3.5-5.6 L"),
    (33, "M.Zuiko Digital 17mm f/2.8 Pancake"),
    (34, "M.Zuiko Digital ED 14-150mm f/4.0-5.6"),
    (35, "M.Zuiko Digital ED 9-18mm f/4.0-5.6"),
    (36, "M.Zuiko Digital ED 14-42mm f/3.5-5.6"),
    (37, "M.Zuiko Digital ED 40-150mm f/4.0-5.6"),
    (38, "M.Zuiko Digital ED 75-300mm f/4.8-6.7"),
    (39, "M.Zuiko Digital 14-42mm f/3.5-5.6 II"),
    (40, "M.Zuiko Digital ED 12-50mm f/3.5-6.3 EZ"),
    (41, "M.Zuiko Digital 45mm f/1.8"),
    (42, "M.Zuiko Digital ED 60mm f/2.8 Macro"),
    (43, "M.Zuiko Digital ED 14-42mm f/3.5-5.6 IIR"),
    (44, "M.Zuiko Digital ED 40-150mm f/4.0-5.6 R"),
    (45, "M.Zuiko Digital ED 75mm f/1.8"),
    (46, "M.Zuiko Digital 17mm f/1.8"),
    (47, "M.Zuiko Digital 25mm f/1.8"),
    (48, "M.Zuiko Digital ED 12-40mm f/2.8 PRO"),
    (49, "M.Zuiko Digital ED 40-150mm f/2.8 PRO"),
    (50, "M.Zuiko Digital ED 14-42mm f/3.5-5.6 EZ"),
    (51, "M.Zuiko Digital ED 7-14mm f/2.8 PRO"),
    (52, "M.Zuiko Digital ED 300mm f/4.0 IS PRO"),
    (53, "M.Zuiko Digital ED 8mm f/1.8 Fisheye PRO"),
    (54, "M.Zuiko Digital ED 12-50mm f/3.5-6.3 II EZ"),
    (55, "M.Zuiko Digital ED 40-150mm f/4.0-5.6 R II"),
    (56, "M.Zuiko Digital ED 14-150mm f/4.0-5.6 II"),
    (57, "M.Zuiko Digital ED 12-200mm f/3.5-6.3"),
    (58, "M.Zuiko Digital ED 75-300mm f/4.8-6.7 II"),
    (64, "M.Zuiko Digital ED 12-100mm f/4.0 IS PRO"),
    (65, "M.Zuiko Digital ED 25mm f/1.2 PRO"),
    (66, "M.Zuiko Digital ED 17mm f/1.2 PRO"),
    (67, "M.Zuiko Digital ED 45mm f/1.2 PRO"),
    (68, "M.Zuiko Digital ED 100-400mm f/5.0-6.3 IS"),
    (69, "M.Zuiko Digital ED 8-25mm f/4.0 PRO"),
    (70, "M.Zuiko Digital ED 150-400mm f/4.5 TC1.25x IS PRO"),
    (71, "M.Zuiko Digital ED 12-45mm f/4.0 PRO"),
    (72, "M.Zuiko Digital ED 20mm f/1.4 PRO"),
    (73, "M.Zuiko Digital ED 40-150mm f/4.0 PRO"),
    (80, "M.Zuiko Digital ED 30mm f/3.5 Macro"),
    (81, "M.Zuiko Digital 9mm f/8.0 Fisheye Body Cap Lens"),
    (82, "M.Zuiko Digital 15mm f/8.0 Body Cap Lens"),
    (83, "M.Zuiko Digital ED 12mm f/2.0"),
    (84, "M.Zuiko Digital ED 30mm f/3.5 Macro ED"),
    (0x0101, "M.Zuiko Digital 14-42mm f/3.5-5.6 II R"),
    (0x0201, "M.Zuiko Digital ED 12-50mm f/3.5-6.3 EZ"),
    (0x0202, "M.Zuiko Digital ED 40-150mm f/4.0-5.6 R"),
    (0x0203, "M.Zuiko Digital ED 75mm f/1.8"),
    (0x0204, "M.Zuiko Digital 17mm f/1.8"),
    (0x0205, "M.Zuiko Digital 25mm f/1.8"),
    (0x0206, "M.Zuiko Digital ED 12-40mm f/2.8 PRO"),
    (0x0207, "M.Zuiko Digital ED 40-150mm f/2.8 PRO"),
    (0x0208, "M.Zuiko Digital ED 14-42mm f/3.5-5.6 EZ"),
    (0x0209, "M.Zuiko Digital ED 7-14mm f/2.8 PRO"),
    (0x020A, "M.Zuiko Digital ED 300mm f/4.0 IS PRO"),
    (0x020B, "M.Zuiko Digital ED 8mm f/1.8 Fisheye PRO"),
    (0x020C, "M.Zuiko Digital ED 12-50mm f/3.5-6.3 II EZ"),
    (0x020D, "M.Zuiko Digital ED 14-150mm f/4.0-5.6 II"),
    (0x020E, "M.Zuiko Digital ED 12-200mm f/3.5-6.3"),
    (0x020F, "M.Zuiko Digital ED 75-300mm f/4.8-6.7 II"),
    (0x0210, "M.Zuiko Digital ED 12-100mm f/4.0 IS PRO"),
    (0x0211, "M.Zuiko Digital ED 30mm f/3.5 Macro"),
    (0x0212, "M.Zuiko Digital ED 25mm f/1.2 PRO"),
    (0x0213, "M.Zuiko Digital ED 17mm f/1.2 PRO"),
    (0x0214, "M.Zuiko Digital ED 45mm f/1.2 PRO"),
    (0x0215, "M.Zuiko Digital ED 100-400mm f/5.0-6.3 IS"),
    (0x0216, "M.Zuiko Digital ED 8-25mm f/4.0 PRO"),
    (0x0217, "M.Zuiko Digital ED 150-400mm f/4.5 TC1.25x IS PRO"),
    (0x0218, "M.Zuiko Digital ED 12-45mm f/4.0 PRO"),
    (0x0219, "M.Zuiko Digital ED 20mm f/1.4 PRO"),
    (0x021A, "M.Zuiko Digital ED 40-150mm f/4.0 PRO"),
    (0x021B, "M.Zuiko Digital ED 90mm f/3.5 Macro IS PRO"),
    (0x0301, "Sigma 30mm f/2.8 DN Art"),
    (0x0302, "Sigma 19mm f/2.8 DN Art"),
    (0x0303, "Sigma 60mm f/2.8 DN Art"),
    (0x0304, "Panasonic Lumix G 20mm f/1.7 ASPH"),
    (0x0305, "Panasonic Leica DG Summilux 15mm f/1.7 ASPH"),
];

// Static lens database instance
static OLYMPUS_LENS_DB: StaticLensDb = StaticLensDb::new(&OLYMPUS_LENSES);

/// Looks up a lens name from an Olympus lens ID
///
/// # Arguments
/// * `lens_id` - The lens ID from LensType tag (can be decimal or hex format)
///
/// # Returns
/// * `Some(String)` - The lens model name if found
/// * `None` - If lens ID is not in database
///
/// # Note
/// This function maintains backward compatibility with the old HashMap-based implementation.
/// New code should prefer using `get_lens_database()` and the `LensDatabase` trait directly.
pub fn lookup_lens_name(lens_id: u16) -> Option<String> {
    OLYMPUS_LENS_DB.lookup(lens_id).map(|s| s.to_string())
}

/// Get Olympus lens database for direct use with LensDatabase trait
///
/// Returns a reference to the static lens database that implements the LensDatabase trait.
/// This is the preferred way to access the lens database in new code.
pub fn get_lens_database() -> &'static impl LensDatabase {
    &OLYMPUS_LENS_DB
}

/// Converts hex string lens ID to numeric value for lookup
///
/// Olympus sometimes encodes lens IDs as hex strings (e.g., "0x0201")
pub fn parse_hex_lens_id(hex_str: &str) -> Option<u16> {
    if hex_str.starts_with("0x") || hex_str.starts_with("0X") {
        u16::from_str_radix(&hex_str[2..], 16).ok()
    } else {
        hex_str.parse::<u16>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_four_thirds_lens() {
        // Zuiko Digital ED 12-60mm f/2.8-4.0 SWD
        let result = lookup_lens_name(9);
        assert_eq!(
            result,
            Some("Olympus Zuiko Digital ED 12-60mm f/2.8-4.0 SWD".to_string())
        );
    }

    #[test]
    fn test_lookup_mzuiko_standard_lens() {
        // M.Zuiko Digital 45mm f/1.8
        let result = lookup_lens_name(41);
        assert_eq!(result, Some("M.Zuiko Digital 45mm f/1.8".to_string()));
    }

    #[test]
    fn test_lookup_mzuiko_pro_lens() {
        // M.Zuiko Digital ED 12-40mm f/2.8 PRO
        let result = lookup_lens_name(48);
        assert_eq!(
            result,
            Some("M.Zuiko Digital ED 12-40mm f/2.8 PRO".to_string())
        );
    }

    #[test]
    fn test_lookup_hex_encoded_lens() {
        // M.Zuiko Digital ED 12-100mm f/4.0 IS PRO (hex ID)
        let result = lookup_lens_name(0x0210);
        assert_eq!(
            result,
            Some("M.Zuiko Digital ED 12-100mm f/4.0 IS PRO".to_string())
        );
    }

    #[test]
    fn test_lookup_premium_prime() {
        // M.Zuiko Digital ED 25mm f/1.2 PRO
        let result = lookup_lens_name(65);
        assert_eq!(
            result,
            Some("M.Zuiko Digital ED 25mm f/1.2 PRO".to_string())
        );
    }

    #[test]
    fn test_lookup_unknown_lens() {
        let result = lookup_lens_name(9999);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_hex_lens_id() {
        assert_eq!(parse_hex_lens_id("0x0210"), Some(0x0210));
        assert_eq!(parse_hex_lens_id("0X0210"), Some(0x0210));
        assert_eq!(parse_hex_lens_id("48"), Some(48));
        assert_eq!(parse_hex_lens_id("invalid"), None);
    }

    #[test]
    fn test_database_size() {
        // Should have 90+ lens entries
        assert!(
            OLYMPUS_LENSES.len() >= 90,
            "Expected at least 90 lens entries, found {}",
            OLYMPUS_LENSES.len()
        );
    }

    #[test]
    fn test_lookup_fisheye() {
        // M.Zuiko Digital ED 8mm f/1.8 Fisheye PRO
        let result = lookup_lens_name(53);
        assert_eq!(
            result,
            Some("M.Zuiko Digital ED 8mm f/1.8 Fisheye PRO".to_string())
        );
    }

    #[test]
    fn test_lookup_macro() {
        // M.Zuiko Digital ED 60mm f/2.8 Macro
        let result = lookup_lens_name(42);
        assert_eq!(
            result,
            Some("M.Zuiko Digital ED 60mm f/2.8 Macro".to_string())
        );
    }

    #[test]
    fn test_lens_database_trait() {
        // Test that the LensDatabase trait works correctly
        let db = get_lens_database();
        assert_eq!(
            db.lookup(9),
            Some("Olympus Zuiko Digital ED 12-60mm f/2.8-4.0 SWD")
        );
        assert_eq!(db.lookup(9999), None);
    }
}
