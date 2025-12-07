//! QuickTime atom to ExifTool tag mapping
//!
//! This module provides mappings from QuickTime FourCC codes to ExifTool-compatible
//! tag names, enabling proper tag extraction and comparison with ExifTool output.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Mapping from QuickTime FourCC codes to ExifTool tag names
static ATOM_TO_TAG: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // User data atoms (classic QuickTime)
    m.insert("©nam", "QuickTime:Title");
    m.insert("©ART", "QuickTime:Artist");
    m.insert("©alb", "QuickTime:Album");
    m.insert("©day", "QuickTime:ContentCreateDate");
    m.insert("©cmt", "QuickTime:Comment");
    m.insert("©gen", "QuickTime:Genre");
    m.insert("©wrt", "QuickTime:Composer");
    m.insert("©too", "QuickTime:Encoder");
    m.insert("©dir", "QuickTime:Director");
    m.insert("©prd", "QuickTime:Producer");
    m.insert("©lyr", "QuickTime:Lyrics");
    m.insert("©grp", "QuickTime:Grouping");
    m.insert("aART", "QuickTime:AlbumArtist");
    m.insert("tmpo", "QuickTime:BeatsPerMinute");
    m.insert("cprt", "QuickTime:Copyright");
    m.insert("desc", "QuickTime:Description");
    m.insert("ldes", "QuickTime:LongDescription");
    m.insert("trkn", "QuickTime:TrackNumber");
    m.insert("disk", "QuickTime:DiskNumber");
    m.insert("covr", "QuickTime:CoverArt");

    // Movie header atoms (mvhd)
    m.insert("mvhd_timescale", "QuickTime:TimeScale");
    m.insert("mvhd_duration", "QuickTime:Duration");
    m.insert("mvhd_rate", "QuickTime:PreferredRate");
    m.insert("mvhd_volume", "QuickTime:PreferredVolume");
    m.insert("mvhd_create", "QuickTime:CreateDate");
    m.insert("mvhd_modify", "QuickTime:ModifyDate");

    // Track header atoms (tkhd)
    m.insert("tkhd_create", "QuickTime:TrackCreateDate");
    m.insert("tkhd_modify", "QuickTime:TrackModifyDate");
    m.insert("tkhd_duration", "QuickTime:TrackDuration");
    m.insert("tkhd_layer", "QuickTime:TrackLayer");
    m.insert("tkhd_volume", "QuickTime:TrackVolume");
    m.insert("tkhd_id", "QuickTime:TrackID");

    // Media header atoms (mdhd)
    m.insert("mdhd_timescale", "QuickTime:MediaTimeScale");
    m.insert("mdhd_duration", "QuickTime:MediaDuration");
    m.insert("mdhd_create", "QuickTime:MediaCreateDate");
    m.insert("mdhd_modify", "QuickTime:MediaModifyDate");

    // Handler atoms (hdlr)
    m.insert("hdlr_type", "QuickTime:HandlerType");
    m.insert("hdlr_name", "QuickTime:HandlerDescription");
    m.insert("hdlr_vendor", "QuickTime:HandlerVendorID");

    // Sample description atoms (stsd) - video
    m.insert("stsd_codec", "QuickTime:CompressorID");
    m.insert("stsd_name", "QuickTime:CompressorName");
    m.insert("stsd_width", "QuickTime:ImageWidth");
    m.insert("stsd_height", "QuickTime:ImageHeight");
    m.insert("stsd_depth", "QuickTime:BitDepth");
    m.insert("stsd_xres", "QuickTime:XResolution");
    m.insert("stsd_yres", "QuickTime:YResolution");
    m.insert("stsd_sourcewidth", "QuickTime:SourceImageWidth");
    m.insert("stsd_sourceheight", "QuickTime:SourceImageHeight");

    // Sample description atoms (stsd) - audio
    m.insert("stsd_channels", "QuickTime:AudioChannels");
    m.insert("stsd_samplerate", "QuickTime:AudioSampleRate");
    m.insert("stsd_bitspersample", "QuickTime:AudioBitsPerSample");
    m.insert("stsd_audioformat", "QuickTime:AudioFormat");

    // Sample-to-time atoms (stts) - video frame rate
    m.insert("stts_framerate", "QuickTime:VideoFrameRate");

    m
});

/// Get ExifTool-compatible tag name for a QuickTime atom
///
/// # Arguments
///
/// * `atom_type` - The atom FourCC code (e.g., "©nam", "mvhd_create")
///
/// # Returns
///
/// The ExifTool-compatible tag name if found, None otherwise
///
/// # Example
///
/// ```
/// use oxidex::parsers::quicktime::tag_mapping::atom_to_exiftool_tag;
///
/// assert_eq!(atom_to_exiftool_tag("©nam"), Some("QuickTime:Title"));
/// assert_eq!(atom_to_exiftool_tag("mvhd_create"), Some("QuickTime:CreateDate"));
/// assert_eq!(atom_to_exiftool_tag("unknown"), None);
/// ```
pub fn atom_to_exiftool_tag(atom_type: &str) -> Option<&'static str> {
    ATOM_TO_TAG.get(atom_type).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_mapping() {
        assert_eq!(atom_to_exiftool_tag("©nam"), Some("QuickTime:Title"));
    }

    #[test]
    fn test_artist_mapping() {
        assert_eq!(atom_to_exiftool_tag("©ART"), Some("QuickTime:Artist"));
    }

    #[test]
    fn test_mvhd_create_mapping() {
        assert_eq!(
            atom_to_exiftool_tag("mvhd_create"),
            Some("QuickTime:CreateDate")
        );
    }

    #[test]
    fn test_mvhd_duration_mapping() {
        assert_eq!(
            atom_to_exiftool_tag("mvhd_duration"),
            Some("QuickTime:Duration")
        );
    }

    #[test]
    fn test_tkhd_id_mapping() {
        assert_eq!(atom_to_exiftool_tag("tkhd_id"), Some("QuickTime:TrackID"));
    }

    #[test]
    fn test_stsd_codec_mapping() {
        assert_eq!(
            atom_to_exiftool_tag("stsd_codec"),
            Some("QuickTime:CompressorID")
        );
    }

    #[test]
    fn test_unknown_atom() {
        assert_eq!(atom_to_exiftool_tag("xxxx"), None);
    }

    #[test]
    fn test_all_user_data_atoms() {
        // Verify key user data atoms are mapped
        assert!(atom_to_exiftool_tag("©nam").is_some());
        assert!(atom_to_exiftool_tag("©ART").is_some());
        assert!(atom_to_exiftool_tag("©alb").is_some());
        assert!(atom_to_exiftool_tag("©day").is_some());
        assert!(atom_to_exiftool_tag("©cmt").is_some());
        assert!(atom_to_exiftool_tag("©gen").is_some());
    }

    #[test]
    fn test_all_mvhd_atoms() {
        // Verify movie header atoms are mapped
        assert!(atom_to_exiftool_tag("mvhd_timescale").is_some());
        assert!(atom_to_exiftool_tag("mvhd_duration").is_some());
        assert!(atom_to_exiftool_tag("mvhd_rate").is_some());
        assert!(atom_to_exiftool_tag("mvhd_volume").is_some());
        assert!(atom_to_exiftool_tag("mvhd_create").is_some());
        assert!(atom_to_exiftool_tag("mvhd_modify").is_some());
    }

    #[test]
    fn test_all_tkhd_atoms() {
        // Verify track header atoms are mapped
        assert!(atom_to_exiftool_tag("tkhd_create").is_some());
        assert!(atom_to_exiftool_tag("tkhd_modify").is_some());
        assert!(atom_to_exiftool_tag("tkhd_duration").is_some());
        assert!(atom_to_exiftool_tag("tkhd_layer").is_some());
        assert!(atom_to_exiftool_tag("tkhd_id").is_some());
    }

    #[test]
    fn test_all_stsd_atoms() {
        // Verify sample description atoms are mapped
        assert!(atom_to_exiftool_tag("stsd_codec").is_some());
        assert!(atom_to_exiftool_tag("stsd_name").is_some());
        assert!(atom_to_exiftool_tag("stsd_width").is_some());
        assert!(atom_to_exiftool_tag("stsd_height").is_some());
        assert!(atom_to_exiftool_tag("stsd_channels").is_some());
        assert!(atom_to_exiftool_tag("stsd_samplerate").is_some());
    }
}
