use std::path::PathBuf;

use crate::import::Importer;

use super::*;

/// Tag to write. Internal primary key is crc32 name hash 
pub struct Tag {
    /// Primary name
    pub name: String,
    /// Alternative name
    pub alt_name: Option<String>,
    /// Type of tag
    pub tag_type: TagType,
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

/// Element metadatas and tags
pub struct ElementMetadata {
    /// Link to source (if was imported from other sources)
    pub src_link: Option<String>,
    /// Time when element was added to other source (if present)
    pub src_time: Option<UtcDateTime>,
    /// Stable Diffusion/etc metadata
    pub ai_meta: Option<AIMetadata>,
    /// External group info
    pub group: Option<u32>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}  