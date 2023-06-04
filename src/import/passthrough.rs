use async_trait::async_trait;
use crate::model::{write::{ElementMetadata, Tag}, TagType};
use super::{MetadataParser, ElementPrefab};

/// Importer that does not fetch metadata at all
pub struct Passthrough;

#[async_trait]
impl MetadataParser for Passthrough {
    fn can_parse(&self, _: &ElementPrefab) -> bool { true }

    fn parse_metadata(
        &self, 
        _: &ElementPrefab
    ) -> anyhow::Result<ElementMetadata> {
        Ok(ElementMetadata {
            src_link: None,
            src_time: None,
            ai_meta: None,
            group: None,
            tags: vec![Tag::new("unknown_source", None, TagType::Metadata).unwrap()],
        })
    }
}