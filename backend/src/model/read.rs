use nndb_common::MetadataSource;

use crate::import::Fetcher;
use crate::dao::SliceShim;

pub use nndb_common::ElementMetadata;
pub use nndb_common::Tag;

use super::*;

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
}   

/// Element waiting for metadata download/parse
#[derive(Debug, sqlx::FromRow)]
pub struct PendingImport {
    /// Element id
    pub id: u32, 
    /// Importer assigned to element
    pub importer_id: Fetcher,
    /// Name that file had before rename
    pub orig_filename: String,
    /// Hash of whole file
    #[sqlx(try_from = "SliceShim<'a>")]
    pub hash: Md5Hash,
}

/// Associated elements
pub struct Associated {
    /// Grouping source
    pub source: MetadataSource,
    /// Group id
    pub id: i64,
    /// Elements in group
    pub elements: Vec<Element>
}
