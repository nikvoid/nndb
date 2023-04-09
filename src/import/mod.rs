use std::path::PathBuf;

use crate::model::{Md5Hash, write::ElementMetadata, read::PendingImport};
use md5::{Digest, Md5};
use num_enum::{FromPrimitive, IntoPrimitive};

mod passthrough;

pub const IMAGE_EXTS: &[&str] = &["png", "jpeg", "jpg"];
pub const ANIMATION_EXTS: &[&str] = &["mp4", "mov", "gif", "webm", "webp", "m4v", "avif"];

/// Name directory as `TAG.<tag_type>.<tag_name>.<tag_type>.<tag_name>...`
/// to add `<tag_name>...` to elements in this directory 
pub const TAG_TRIGGER: &str = "TAG.";

#[derive(FromPrimitive, IntoPrimitive, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Importer {
    /// No specific metadata
    #[default]
    Passthrough = 0,
}

impl Importer {
    /// Decide which importer to use with file
    pub fn scan(element: &ElementPrefab) -> Self  {
        [
            Self::Passthrough,
        ]
        .into_iter()
        .find(|imp| imp.get_singleton().can_parse(element))
        // Passthrough always returns true
        .unwrap()
    }

    /// Get singleton for chosen importer
    pub fn get_singleton(self) -> &'static dyn MetadataImporter {
        match self {
            Importer::Passthrough => &passthrough::Passthrough,
        }
    }
}

/// Holder with element original filename and data 
pub struct ElementPrefab {
    pub path: PathBuf,
    pub data: Vec<u8>,
}

pub trait MetadataImporter {
    /// Check if importer can get metadata for element
    fn can_parse(&self, element: &ElementPrefab) -> bool;

    /// Check if importer can fetch metadata now
    fn available(&self) -> bool { true }
    
    /// Get hash based on file data or name
    fn derive_hash(
        &self,
        element: &ElementPrefab,
    ) -> Md5Hash {
        Md5::digest(&element.data).into()
    }

    /// Fetch metadata for pending import
    fn fetch_metadata(&self, element: PendingImport) -> anyhow::Result<ElementMetadata>;
}