//! Opus format family tags (auto-generated)

use crate::core::{FormatFamily, TagDescriptor, TagId, ValueType};
use std::sync::LazyLock;
use std::collections::HashMap;

static TAGS: LazyLock<Vec<TagDescriptor>> = LazyLock::new(|| vec![
    TagDescriptor::new(TagId::new_numeric(0x96DF), "Opus:Header".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "Header tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0xFD18), "Opus:Comments".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "Comments tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0000), "Opus:OpusVersion".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "OpusVersion tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0001), "Opus:AudioChannels".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "AudioChannels tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0004), "Opus:SampleRate".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "SampleRate tag".to_string(), vec!["Example".to_string()]),
    TagDescriptor::new(TagId::new_numeric(0x0008), "Opus:OutputGain".to_string(), FormatFamily::MakerNotes, false, ValueType::String, "OutputGain tag".to_string(), vec!["Example".to_string()]),
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
