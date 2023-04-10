use async_trait::async_trait;


use crate::model::{write::{ElementMetadata, Tag}, read::PendingImport, TagType};

use super::{MetadataImporter, ElementPrefab};

/// Importer that does not fetch metadata at all
pub struct Passthrough;

#[async_trait]
impl MetadataImporter for Passthrough {
    fn can_parse(&self, _: &ElementPrefab) -> bool { true }

    async fn fetch_metadata(
        &self,
        _: &PendingImport
    ) -> anyhow::Result<ElementMetadata> {
        
        Ok(ElementMetadata {
            src_link: None,
            src_time: None,
            ai_meta: None,
            group: None,
            tags: vec![Tag::new("unknown_source", None, TagType::Metadata)],
        })
    }
}