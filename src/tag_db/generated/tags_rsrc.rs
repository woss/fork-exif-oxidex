//! RSRC format family tags (auto-generated)

use crate::core::{FormatFamily, TagDescriptor, TagId, ValueType};
use std::sync::LazyLock;
use std::collections::HashMap;

static TAGS: LazyLock<Vec<TagDescriptor>> = LazyLock::new(|| vec![
    TagDescriptor::new(TagId::new_numeric(0x75AE), "RSRC:PhotoshopInfo".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "PhotoshopInfo tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0xD359), "RSRC:Font".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "Font tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0xB117), "RSRC:PostscriptFont".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "PostscriptFont tag".to_string(), vec!["Example".to_string()]),
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
