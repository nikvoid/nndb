pub use nndb_common::{TagType, AIMetadata, Summary, UtcDateTime};

pub const SIGNATURE_LEN: usize = 544;
pub const MD5_LEN: usize = 16;

pub type Md5Hash = [u8; MD5_LEN];
pub type Signature = [i8; SIGNATURE_LEN];

use crate::dao::SliceShim;

pub mod read;
pub mod write;
pub mod danbooru;

/// Metadata for element group
#[derive(sqlx::FromRow)]
pub struct GroupMetadata {
    /// Id of the element
    pub element_id: u32,
    /// Image signature
    #[sqlx(try_from = "SliceShim<'a>")]
    pub signature: Signature,
    /// Element group
    pub group_id: Option<u32>,
}

