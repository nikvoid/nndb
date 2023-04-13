use crate::import::Importer;

use super::*;

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

