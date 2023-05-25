use std::path::PathBuf;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::import::Importer;

use super::*;

/// Tag excape regex
pub static TAG_REX: Lazy<Regex> = Lazy::new(|| 
    Regex::new(r#"[\s:,.@#$*'"`|%{}\[\]]+"#).unwrap()
);

/// Tag to write. Internal primary key is crc32 name hash 
pub struct Tag {
    /// Primary name
    name: String,
    /// Alternative name
    alt_name: Option<String>,
    /// Type of tag
    tag_type: TagType,
}

impl Tag {
    /// Create new tag with escaped name.
    /// Returns `None` only is `name` is empty
    pub fn new(
        name: &str, 
        alt_name: Option<String>, 
        tag_type: TagType
    ) -> Option<Self> {
        if name.is_empty() {
            return None
        }
        
        let name = TAG_REX.replace_all(name, "_")
            .trim_matches('_')
            .to_lowercase();
        Some(Self {
            name,
            alt_name,
            tag_type
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn alt_name(&self) -> Option<&str> {
        self.alt_name.as_deref()
    }
    
    pub fn tag_type(&self) -> TagType { 
        self.tag_type 
    }
    
    /// Get crc32 hash of name
    pub fn name_hash(&self) -> u32 {
        crc32fast::hash(self.name.as_bytes())
    }
}

impl AsRef<Tag> for Tag {
    fn as_ref(&self) -> &Tag {
        self
    }
} 

/// Element that waits for metadata parse/download
#[derive(Debug)]
pub struct ElementToParse {
    /// Full path to element
    pub path: PathBuf,
    /// Name of file in files pool (derived from md5 hash of file)
    pub filename: String,
    /// Name that file had before rename
    pub orig_filename: String,
    /// Hash of whole file
    pub hash: Md5Hash,
    /// Importer that will be used for file
    pub importer_id: Importer,
    /// Whether element is animation
    pub animated: bool,
    /// Image matching signature
    pub signature: Option<Signature>,
    /// True if failed to read image
    pub broken: bool,
}

impl AsRef<ElementToParse> for ElementToParse {
    fn as_ref(&self) -> &ElementToParse {
        self
    }
}

/// Element metadatas and tags
pub struct ElementMetadata {
    /// Link to source (if was imported from other sources)
    pub src_link: Option<String>,
    /// Time when element was added to other source (if present)
    pub src_time: Option<UtcDateTime>,
    /// Stable Diffusion/etc metadata
    pub ai_meta: Option<AIMetadata>,
    /// External group info
    pub group: Option<i64>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}  

impl AsRef<ElementMetadata> for ElementMetadata {
    fn as_ref(&self) -> &ElementMetadata {
        self
    }
}

pub struct ElementWithMetadata(pub ElementToParse, pub Option<ElementMetadata>); 
impl AsRef<ElementWithMetadata> for ElementWithMetadata {
    fn as_ref(&self) -> &ElementWithMetadata {
        self
    }
}