//! MOI format family tags (auto-generated)

use crate::core::{FormatFamily, TagDescriptor, TagId, ValueType};
use std::sync::LazyLock;
use std::collections::HashMap;

static TAGS: LazyLock<Vec<TagDescriptor>> = LazyLock::new(|| vec![
    TagDescriptor::new(TagId::new_numeric(0x0000), "MOI:MOIVersion".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "MOIVersion tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0006), "MOI:DateTimeOriginal".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "DateTimeOriginal tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x000E), "MOI:Duration".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "Duration tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0080), "MOI:AspectRatio".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "AspectRatio tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0084), "MOI:AudioCodec".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "AudioCodec tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x4001), "MOI:MPEG".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "MPEG tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0086), "MOI:AudioBitrate".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "AudioBitrate tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x00DA), "MOI:VideoBitrate".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "VideoBitrate tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x813D), "MOI:5500000".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "5500000 tag".to_string(), vec!["Example".to_string()]),
]);

pub fn get_tags() -> &'static HashMap<String, TagDescriptor> {
    static MAP: LazyLock<HashMap<String, TagDescriptor>> = LazyLock::new(|| {
        let mut map = HashMap::with_capacity(TAGS.len());
        for tag in TAGS.iter() {
            map.insert(tag.tag_name.clone(), tag.clone());
        }
        map
    });
    &MAP
}
