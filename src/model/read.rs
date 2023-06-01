use serde::Serialize;

use crate::import::Importer;
use crate::dao::SliceShim;

use super::*;

#[derive(Serialize, sqlx::FromRow)]
pub struct Tag {
    /// Tag id
    pub id: u32,
    /// Primary name
    #[sqlx(rename = "tag_name")]
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

#[derive(sqlx::FromRow)]
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
    #[sqlx(rename = "ext_group")]
    pub group: Option<i64>,
}   

/// Element waiting for metadata download/parse
#[derive(Debug, sqlx::FromRow)]
pub struct PendingImport {
    /// Element id
    pub id: u32, 
    /// Importer assigned to element
    pub importer_id: Importer,
    /// Name that file had before rename
    pub orig_filename: String,
    /// Hash of whole file
    #[sqlx(try_from = "SliceShim<'a>")]
    pub hash: Md5Hash,
}

/// Element metadatas and tags
#[derive(Default, sqlx::FromRow)]
pub struct ElementMetadata {
    /// Link to source (if was imported from other sources)
    pub src_link: Option<String>,
    /// Time when element was added to other source (if present)
    pub src_time: Option<UtcDateTime>,
    /// Time when element was added to db
    pub add_time: UtcDateTime,
    /// Stable Diffusion/etc metadata
    #[sqlx(skip)]
    pub ai_meta: Option<AIMetadata>,
    /// Tags of the element
    #[sqlx(skip)]
    pub tags: Vec<Tag>,
}  
