//! GE format family tags (auto-generated)

use crate::core::{FormatFamily, TagDescriptor, TagId, ValueType};
use std::sync::LazyLock;
use std::collections::HashMap;

static TAGS: LazyLock<Vec<TagDescriptor>> = LazyLock::new(|| vec![
    TagDescriptor::new(TagId::new_numeric(0x0202), "GE:Macro".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "Macro tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0207), "GE:GEModel".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "GEModel tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0300), "GE:GEMake".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "GEMake tag".to_string(), vec!["Example".to_string()]),
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
