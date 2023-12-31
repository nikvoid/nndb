use std::path::PathBuf;

use nndb_common::search::TAG_REX;
use crate::import::Parser;

use super::*;

/// Tag to write
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
    pub importer_id: Parser,
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
    /// Raw Stable Diffusion/etc metadata
    pub raw_meta: Option<String>,
    /// External group info
    pub group: Option<i64>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}  

pub struct Wiki {
    /// Wiki title (primary tag name)
    pub title: String,
    /// Tag aliases
    pub aliases: Vec<String>,
    /// Tag type
    pub category: TagType,    
}

impl AsRef<ElementMetadata> for ElementMetadata {
    fn as_ref(&self) -> &ElementMetadata {
        self
    }
}

pub struct ElementWithMetadata(
    pub ElementToParse, 
    pub ElementMetadata,
    pub Parser
); 

impl AsRef<ElementWithMetadata> for ElementWithMetadata {
    fn as_ref(&self) -> &ElementWithMetadata {
        self
    }
}

impl AsRef<Wiki> for Wiki {
    fn as_ref(&self) -> &Wiki {
        self
    }
}