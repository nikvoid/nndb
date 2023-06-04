use async_trait::async_trait;
use crate::model::{write::{ElementMetadata, Tag}, TagType, read::{PendingImport, self}};
use super::{MetadataParser, ElementPrefab, MetadataFetcher};

/// Importer that does not parse metadata at all
pub struct Passthrough;

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

/// Stub fetcher
pub struct Unknown;

#[async_trait]
impl MetadataFetcher for Unknown {
    /// Check if importer can get metadata for element
    fn supported(&self, import: &PendingImport) -> bool { false }
    
    /// Check if importer can fetch metadata now
    fn available(&self) -> bool { false }
    
    /// Fetch metadata for pending import (network access implied)
    async fn fetch_metadata(
        &self,
        import: &PendingImport
    ) -> anyhow::Result<Option<ElementMetadata>> {
        Ok(None)
    }
}