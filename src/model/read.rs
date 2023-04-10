use crate::import::Importer;

use super::*;

pub struct Tag {
    /// Primary name
    pub name: String,
    /// Alternative name
    pub alt_name: Option<String>,
    /// Count of elements with this tag
    pub count: u32,
    /// Group id of similar tags/aliases
    pub group_id: u32,
}

pub struct Element {
    /// Name of file in files pool (derived from md5 hash of file)
    pub filename: String,
    /// Link to source (if was imported from other sources)
    pub src_link: Option<String>,
    /// Time when element was added to db
    pub add_time: UtcDateTime,
    /// Time when element was added to other source (if present)
    pub src_time: Option<UtcDateTime>,
    /// Name that file had before rename
    pub orig_filename: String,
    /// Group of similar images (decided by comparing image signatures)
    pub group_id: Option<u32>,
    /// Stable Diffusion/etc metadata
    pub ai_meta: Option<AIMetadata>,
    /// Field to mark broken file
    pub broken: bool,
    /// True if element has thumbnail
    pub has_thumb: bool,
    /// Whether element is animation
    pub animated: bool,
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

