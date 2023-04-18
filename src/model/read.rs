use serde::Serialize;

use crate::import::Importer;

use super::*;

#[derive(Serialize)]
pub struct Tag {
    /// Primary name
    pub name: String,
    /// Alternative name
    pub alt_name: Option<String>,
    /// Tag type
    pub tag_type: TagType,
    /// Count of elements with this tag
    pub count: u32,
    /// Group id of similar tags/aliases
    pub group_id: Option<u32>,
    /// Is tag hidden
    pub hidden: bool,
}

pub struct Element {
    /// Element id
    pub id: u32,
    /// Name of file in files pool (derived from md5 hash of file)
    pub filename: String,
    /// Name that file had before rename
    pub orig_filename: String,
    /// Field to mark broken file
    pub broken: bool,
    /// True if element has thumbnail
    pub has_thumb: bool,
    /// Whether element is animation
    pub animated: bool,
    /// Group of similar images (decided by comparing image signatures)
    pub group_id: Option<u32>,
    /// Group info derived from external source
    pub group: Option<u32>,
}   

/// Element waiting for metadata download/parse
#[derive(Debug)]
pub struct PendingImport {
    /// Element id
    pub id: u32, 
    /// Importer assigned to element
    pub importer_id: Importer,
    /// Name that file had before rename
    pub orig_filename: String,
    /// Hash of whole file
    pub hash: Md5Hash,
}

/// Element metadatas and tags
pub struct ElementMetadata {
    /// Link to source (if was imported from other sources)
    pub src_link: Option<String>,
    /// Time when element was added to other source (if present)
    pub src_time: Option<UtcDateTime>,
    /// Time when element was added to db
    pub add_time: UtcDateTime,
    /// Stable Diffusion/etc metadata
    pub ai_meta: Option<AIMetadata>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}  
