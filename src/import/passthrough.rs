use crate::{model::{write::ElementMetadata, read::PendingImport}, dao::StorageBackend};

use super::{MetadataImporter, ElementPrefab};

/// Importer that does not fetch metadata at all
pub struct Passthrough;

impl MetadataImporter for Passthrough {
    fn can_parse(&self, _: &ElementPrefab) -> Option<bool> {
        Some(true)
    }

    fn after_hash_hook(&self, _: &ElementPrefab, _: &StorageBackend) -> anyhow::Result<()> {
        Ok(())
    }

    fn fetch_metadata(&self, _: PendingImport) -> anyhow::Result<crate::model::write::ElementMetadata> {
        // TODO: Add "unknown_source" tag
        Ok(ElementMetadata {
            src_link: None,
            src_time: None,
            ai_meta: None,
            group: None,
            tags: vec![],
        })
    }
}