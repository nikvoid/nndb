use anyhow::bail;
use async_trait::async_trait;


use crate::model::{write::{ElementMetadata, Tag}, read::PendingImport, TagType};

use super::{MetadataImporter, ElementPrefab};

/// Importer that does not fetch metadata at all
pub struct Passthrough;

#[async_trait]
impl MetadataImporter for Passthrough {
    fn can_parse(&self, _: &ElementPrefab) -> bool { true }

    fn can_parse_in_place(&self) -> bool { true }
    
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
    
    async fn fetch_metadata(
        &self,
        _: &PendingImport
    ) -> anyhow::Result<ElementMetadata> {
        bail!("unimplemented")
    }
}