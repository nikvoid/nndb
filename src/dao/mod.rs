mod sqlite;

use once_cell::sync::Lazy;
pub use sqlite::Sqlite;
use tokio::sync::Mutex;

use crate::{config::CONFIG, model::{write::{ElementToParse, Tag, ElementMetadata}, Md5Hash, read::PendingImport, Signature, GroupMetadata}};

pub type StorageBackend = Sqlite;

// TODO: Decouple mutex with backend
pub static STORAGE: Lazy<Mutex<StorageBackend>> = Lazy::new(||
    Mutex::new(StorageBackend::init(&CONFIG.db_url))
); 

/// Trait for backing storage.
/// System designed with compile-time backend selection in mind
pub trait ElementStorage {
    /// Connect to url and init storage
    fn init(url: &str) -> Self;

    /// Add all elements from slice
    /// Returns count of new elements
    fn add_elements<E>(&self, elements: &[E]) -> anyhow::Result<u32>
    where E: AsRef<ElementToParse>;

    /// Get all files' hashes
    fn get_hashes(&self) -> anyhow::Result<Vec<Md5Hash>>;

    /// Add all tags from slice
    fn add_tags<T>(&self, element_id: u32, tags: &[T]) -> anyhow::Result<()>
    where T: AsRef<Tag>;

    /// Get all elements waiting on metadata
    fn get_pending_imports(&self) -> anyhow::Result<Vec<PendingImport>>;

    /// Add metadata for element -- and remove pending import
    fn add_metadata<M>(&self, element_id: u32, metadata: M) -> anyhow::Result<()>
    where M: AsRef<ElementMetadata>;

    /// Get all image signature groups stored in db
    fn get_groups(&self) -> anyhow::Result<Vec<GroupMetadata>>;

    /// Add all elements to group (or create new group with them)
    ///
    /// Returns group id
    fn add_to_group(
        &self, 
        element_ids: &[u32],
        group: Option<u32>
    ) -> anyhow::Result<u32>;
}