//! ZISRAW format family tags (auto-generated)

use crate::core::{FormatFamily, TagDescriptor, TagId, ValueType};
use std::sync::LazyLock;
use std::collections::HashMap;

static TAGS: LazyLock<Vec<TagDescriptor>> = LazyLock::new(|| vec![
    TagDescriptor::new(TagId::new_numeric(0x0020), "ZISRAW:ZISRAWVersion".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "ZISRAWVersion tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0030), "ZISRAW:PrimaryFileGUID".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "PrimaryFileGUID tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0040), "ZISRAW:FileGUID".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "FileGUID tag".to_string(), vec!["Example".to_string()]),
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
