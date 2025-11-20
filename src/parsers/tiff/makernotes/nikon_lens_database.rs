//! Nikon lens database for LensID to lens name mapping
//!
//! Based on ExifTool's Nikon.pm lens database, covering both F-mount
//! and Z-mount (mirrorless) lenses.
//!
//! This module has been migrated to use the unified LensDatabase infrastructure,
//! providing a consistent interface across all manufacturer lens databases.

use super::shared::{LensDatabase, StaticLensDb};

// ============================================================================
// LENS DATA
// ============================================================================

/// Nikon lens database entries mapping lens ID to lens name
/// Covers classic F-mount, AF, AF-S, and Z-mount lenses
static NIKON_LENSES: [(u16, &str); 139] = [
    // Classic Nikon F-mount lenses (Manual Focus era)
    (1, "Nikkor 50mm f/1.4"),
    (2, "Nikkor 35mm f/2.8"),
    (3, "Nikkor 135mm f/2.8"),
    (4, "Nikkor 50mm f/1.8"),
    (5, "Nikkor 28mm f/2.8"),
    (6, "Nikkor 24mm f/2.8"),
    (7, "Nikkor 180mm f/2.8 ED"),
    (8, "Nikkor 200mm f/4"),
    (9, "Nikkor 300mm f/4.5"),
    (10, "Nikkor 35-70mm f/3.5"),

    // AF Nikkor lenses (Autofocus era)
    (11, "Nikkor AF 50mm f/1.4D"),
    (12, "Nikkor AF 50mm f/1.8D"),
    (13, "Nikkor AF 35mm f/2D"),
    (14, "Nikkor AF 28mm f/2.8D"),
    (15, "Nikkor AF 24mm f/2.8D"),
    (16, "Nikkor AF 85mm f/1.4D"),
    (17, "Nikkor AF 85mm f/1.8D"),
    (18, "Nikkor AF 135mm f/2D DC"),
    (19, "Nikkor AF 105mm f/2.8D Macro"),
    (20, "Nikkor AF 60mm f/2.8D Micro"),

    // AF-D zoom lenses
    (21, "Nikkor AF 28-85mm f/3.5-4.5D"),
    (22, "Nikkor AF 35-70mm f/2.8D"),
    (23, "Nikkor AF 80-200mm f/2.8D ED"),
    (24, "Nikkor AF 70-210mm f/4-5.6D"),
    (25, "Nikkor AF 28-105mm f/3.5-4.5D"),
    (26, "Nikkor AF 24-85mm f/2.8-4D"),
    (27, "Nikkor AF 35-135mm f/3.5-4.5"),
    (28, "Nikkor AF 70-300mm f/4-5.6D ED"),
    (29, "Nikkor AF 28-200mm f/3.5-5.6D"),
    (30, "Nikkor AF 75-300mm f/4.5-5.6"),

    // Professional AF lenses
    (31, "Nikkor AF 300mm f/2.8D ED"),
    (32, "Nikkor AF 400mm f/2.8D ED"),
    (33, "Nikkor AF 500mm f/4D ED"),
    (34, "Nikkor AF 600mm f/4D ED"),
    (35, "Nikkor AF 200mm f/2 VR"),
    (36, "Nikkor AF 14mm f/2.8D ED"),
    (37, "Nikkor AF 20mm f/2.8D"),
    (38, "Nikkor AF 180mm f/2.8D ED"),
    (39, "Nikkor AF Fisheye 16mm f/2.8D"),
    (40, "Nikkor AF 17-35mm f/2.8D ED"),

    // AF-S lenses (Silent Wave Motor - modern autofocus)
    (119, "Nikkor AF-S DX 18-55mm f/3.5-5.6G VR"),
    (120, "Nikkor AF-S DX 18-55mm f/3.5-5.6G VR II"),
    (121, "Nikkor AF-S DX 55-200mm f/4-5.6G ED VR"),
    (122, "Nikkor AF-S DX 18-135mm f/3.5-5.6G ED VR"),
    (123, "Nikkor AF-S DX 55-300mm f/4.5-5.6G ED VR"),
    (124, "Nikkor AF-S DX 16-85mm f/3.5-5.6G ED VR"),
    (125, "Nikkor AF-S DX 18-200mm f/3.5-5.6G ED VR"),
    (126, "Nikkor AF-S DX 18-200mm f/3.5-5.6G ED VR II"),
    (127, "Nikkor AF-S DX 18-105mm f/3.5-5.6G ED VR"),
    (128, "Nikkor AF-S DX 10-24mm f/3.5-4.5G ED"),

    // AF-S DX telephoto and macro
    (129, "Nikkor AF-S DX 35mm f/1.8G"),
    (130, "Nikkor AF-S DX 40mm f/2.8G Micro"),
    (131, "Nikkor AF-S DX 85mm f/3.5G ED VR Micro"),
    (132, "Nikkor AF-S DX 12-24mm f/4G ED"),
    (133, "Nikkor AF-S DX 17-55mm f/2.8G ED"),
    (134, "Nikkor AF-S DX 18-70mm f/3.5-4.5G ED"),
    (135, "Nikkor AF-S DX 18-140mm f/3.5-5.6G ED VR"),
    (136, "Nikkor AF-S DX 16-80mm f/2.8-4E ED VR"),
    (137, "Nikkor AF-S DX 10-18mm f/4.5-5.6G VR"),
    (138, "Nikkor AF-S DX 55-300mm f/4.5-5.6G ED VR"),
    (139, "Nikkor AF-S DX 18-300mm f/3.5-5.6G ED VR"),
    (140, "Nikkor AF-S DX 18-300mm f/3.5-6.3G ED VR"),

    // Full-frame AF-S professional zooms
    (141, "Nikkor AF-S 14-24mm f/2.8G ED"),
    (142, "Nikkor AF-S 24-70mm f/2.8G ED"),
    (143, "Nikkor AF-S 24-120mm f/4G ED VR"),
    (144, "Nikkor AF-S 28-300mm f/3.5-5.6G ED VR"),
    (145, "Nikkor AF-S 16-35mm f/4G ED VR"),
    (146, "Nikkor AF-S 70-200mm f/4G ED VR"),
    (147, "Nikkor AF-S 24-70mm f/2.8G ED"),
    (148, "Nikkor AF-S 24-120mm f/4G ED VR"),
    (149, "Nikkor AF-S 80-400mm f/4.5-5.6G ED VR"),
    (150, "Nikkor AF-S 200-500mm f/5.6E ED VR"),

    // AF-S telephoto professional (super-telephoto)
    (151, "Nikkor AF-S 70-200mm f/2.8G ED VR"),
    (152, "Nikkor AF-S 70-200mm f/2.8G ED VR II"),
    (153, "Nikkor AF-S 200-400mm f/4G ED VR"),
    (154, "Nikkor AF-S 70-200mm f/2.8G ED VR II"),
    (155, "Nikkor AF-S 300mm f/2.8G ED VR"),
    (156, "Nikkor AF-S 400mm f/2.8G ED VR"),
    (157, "Nikkor AF-S 500mm f/4G ED VR"),
    (158, "Nikkor AF-S 600mm f/4G ED VR"),
    (159, "Nikkor AF-S 800mm f/5.6E FL ED VR"),
    (160, "Nikkor AF-S 200-400mm f/4G ED VR II"),

    // AF-S prime lenses
    (161, "Nikkor AF-S 35mm f/1.8G"),
    (162, "Nikkor AF-S 50mm f/1.8G"),
    (163, "Nikkor AF-S 85mm f/1.8G"),
    (164, "Nikkor AF-S 24mm f/1.4G ED"),
    (165, "Nikkor AF-S 35mm f/1.4G"),
    (166, "Nikkor AF-S 58mm f/1.4G"),
    (167, "Nikkor AF-S 85mm f/1.4G"),
    (168, "Nikkor AF-S 105mm f/1.4E ED"),
    (169, "Nikkor AF-S 28mm f/1.8G"),
    (170, "Nikkor AF-S 50mm f/1.4G"),

    // AF-S Micro (macro) lenses
    (171, "Nikkor AF-S VR Micro 105mm f/2.8G IF-ED"),
    (172, "Nikkor AF-S VR Micro 60mm f/2.8G ED"),
    (173, "Nikkor AF-S Micro 40mm f/2.8G"),

    // Nikkor Z-mount lenses (mirrorless system)
    (174, "Nikkor Z 24-70mm f/4 S"),
    (175, "Nikkor Z 14-30mm f/4 S"),
    (176, "Nikkor Z 35mm f/1.8 S"),
    (177, "Nikkor Z 50mm f/1.8 S"),
    (178, "Nikkor Z 24-70mm f/2.8 S"),
    (179, "Nikkor Z 70-200mm f/2.8 VR S"),
    (180, "Nikkor Z 58mm f/0.95 S Noct"),
    (181, "Nikkor Z 14-24mm f/2.8 S"),
    (182, "Nikkor Z 20mm f/1.8 S"),
    (183, "Nikkor Z 24mm f/1.8 S"),
    (184, "Nikkor Z 50mm f/1.2 S"),
    (185, "Nikkor Z 85mm f/1.8 S"),
    (186, "Nikkor Z MC 105mm f/2.8 VR S"),
    (187, "Nikkor Z MC 50mm f/2.8 Macro"),
    (188, "Nikkor Z 40mm f/2"),
    (189, "Nikkor Z 28mm f/2.8"),
    (190, "Nikkor Z 24-50mm f/4-6.3"),
    (191, "Nikkor Z 24-200mm f/4-6.3 VR"),
    (192, "Nikkor Z 100-400mm f/4.5-5.6 VR S"),
    (193, "Nikkor Z 800mm f/6.3 VR S"),

    // Z-mount telephoto primes
    (194, "Nikkor Z 400mm f/2.8 TC VR S"),
    (195, "Nikkor Z 400mm f/4.5 VR S"),
    (196, "Nikkor Z 600mm f/4 TC VR S"),
    (197, "Nikkor Z 800mm f/6.3 VR S"),
    (198, "Nikkor Z 600mm f/6.3 VR S"),

    // Z-mount DX (APS-C mirrorless)
    (199, "Nikkor Z DX 16-50mm f/3.5-6.3 VR"),
    (200, "Nikkor Z DX 50-250mm f/4.5-6.3 VR"),
    (201, "Nikkor Z DX 18-140mm f/3.5-6.3 VR"),
    (202, "Nikkor Z DX 24mm f/1.7"),
    (203, "Nikkor Z DX 12-28mm f/3.5-5.6 PZ VR"),

    // Third-party lenses commonly used with Nikon
    (210, "Sigma 18-35mm f/1.8 DC HSM Art"),
    (211, "Sigma 35mm f/1.4 DG HSM Art"),
    (212, "Sigma 50mm f/1.4 DG HSM Art"),
    (213, "Sigma 85mm f/1.4 DG HSM Art"),
    (214, "Sigma 24-70mm f/2.8 DG OS HSM Art"),
    (215, "Sigma 70-200mm f/2.8 DG OS HSM Sports"),
    (216, "Sigma 150-600mm f/5-6.3 DG OS HSM Contemporary"),
    (217, "Sigma 150-600mm f/5-6.3 DG OS HSM Sports"),
    (218, "Tamron SP 24-70mm f/2.8 Di VC USD G2"),
    (219, "Tamron SP 70-200mm f/2.8 Di VC USD G2"),
    (220, "Tamron SP 150-600mm f/5-6.3 Di VC USD G2"),
    (221, "Tamron SP 90mm f/2.8 Di VC USD Macro"),
    (222, "Tokina 11-16mm f/2.8 AT-X Pro DX II"),
    (223, "Tokina 11-20mm f/2.8 AT-X Pro DX"),
];

// ============================================================================
// STATIC LENS DATABASE
// ============================================================================

/// Static lens database using the unified LensDatabase infrastructure
static NIKON_LENS_DB: StaticLensDb = StaticLensDb::new(&NIKON_LENSES);

// ============================================================================
// PUBLIC API
// ============================================================================

/// Looks up a lens name from a Nikon lens ID
///
/// This is the backward-compatible API that returns Option<String>.
/// For new code, prefer using get_lens_database() for direct LensDatabase access.
///
/// # Arguments
/// * `lens_id` - The lens ID from LensData or other arrays
///
/// # Returns
/// * `Some(String)` - The lens model name if found
/// * `None` - If lens ID is not in database
pub fn lookup_lens_name(lens_id: u16) -> Option<String> {
    NIKON_LENS_DB.lookup(lens_id).map(|s| s.to_string())
}

/// Get reference to the Nikon lens database
///
/// Returns a reference to the static lens database implementing the LensDatabase trait.
/// This is the preferred API for new code as it avoids string allocations.
///
/// # Returns
/// Reference to the static Nikon lens database
///
/// # Example
/// ```ignore
/// use nikon_lens_database::get_lens_database;
///
/// let db = get_lens_database();
/// if let Some(name) = db.lookup(147) {
///     println!("Lens: {}", name);
/// }
/// ```
pub fn get_lens_database() -> &'static impl LensDatabase {
    &NIKON_LENS_DB
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_af_s_dx_lens() {
        // Nikkor AF-S DX 18-55mm f/3.5-5.6G VR
        let result = lookup_lens_name(119);
        assert_eq!(
            result,
            Some("Nikkor AF-S DX 18-55mm f/3.5-5.6G VR".to_string())
        );
    }

    #[test]
    fn test_lookup_professional_lens() {
        // Nikkor AF-S 24-70mm f/2.8G ED
        let result = lookup_lens_name(147);
        assert_eq!(result, Some("Nikkor AF-S 24-70mm f/2.8G ED".to_string()));
    }

    #[test]
    fn test_lookup_z_mount_lens() {
        // Nikkor Z 50mm f/1.8 S
        let result = lookup_lens_name(177);
        assert_eq!(result, Some("Nikkor Z 50mm f/1.8 S".to_string()));
    }

    #[test]
    fn test_lookup_z_noct_lens() {
        // Nikkor Z 58mm f/0.95 S Noct
        let result = lookup_lens_name(180);
        assert_eq!(result, Some("Nikkor Z 58mm f/0.95 S Noct".to_string()));
    }

    #[test]
    fn test_lookup_third_party_lens() {
        // Sigma 35mm f/1.4 DG HSM Art
        let result = lookup_lens_name(211);
        assert_eq!(result, Some("Sigma 35mm f/1.4 DG HSM Art".to_string()));
    }

    #[test]
    fn test_lookup_unknown_lens() {
        let result = lookup_lens_name(65000);
        assert_eq!(result, None);
    }

    #[test]
    fn test_database_size() {
        // Should have 139 lens entries
        assert_eq!(
            NIKON_LENSES.len(),
            139,
            "Expected 139 lens entries"
        );
    }

    #[test]
    fn test_lens_database_trait() {
        // Test using the LensDatabase trait directly
        let db = get_lens_database();

        // Test successful lookup
        assert_eq!(db.lookup(147), Some("Nikkor AF-S 24-70mm f/2.8G ED"));

        // Test failed lookup (use valid u16 value)
        assert_eq!(db.lookup(65000), None);
    }
}
