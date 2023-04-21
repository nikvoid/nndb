mod sqlite;

use once_cell::sync::Lazy;
pub use sqlite::Sqlite;
use tokio::sync::Mutex;

use crate::{config::CONFIG, model::{write::{Tag, ElementMetadata, ElementWithMetadata}, Md5Hash, read::{PendingImport, self}, GroupMetadata, Summary}};

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

    /// Add all elements from slice (optionally with metadata)
    /// Returns count of new elements
    fn add_elements<E>(&self, elements: &[E]) -> anyhow::Result<u32>
    where E: AsRef<ElementWithMetadata>;

    /// Get all files' hashes
    fn get_hashes(&self) -> anyhow::Result<Vec<Md5Hash>>;

    /// Add all tags from slice
    fn add_tags<T>(&self, element_id: Option<u32>, tags: &[T]) -> anyhow::Result<()>
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

    /// Fetch elements from db, by query, with offset and limit.
    /// Returns `(elements, selection_tags, count_in_db)`
    fn search_elements<Q>(
        &self, 
        query: Q,
        offset: u32, 
        limit: Option<u32>,
        tag_limit: u32,
    ) -> anyhow::Result<(Vec<read::Element>, Vec<read::Tag>, u32)>
    where Q: AsRef<str>;

    /// Get element data and metadata
    fn get_element_data(
        &self, 
        id: u32,
    ) -> anyhow::Result<Option<(read::Element, read::ElementMetadata)>>;

    /// Update count of elements with tag for each tag
    fn update_tag_count(&self) -> anyhow::Result<()>;

    /// Tag autocompletion
    fn get_tag_completions<I>(&self, input: I, limit: u32) -> anyhow::Result<Vec<read::Tag>>
    where I: AsRef<str>;  

    /// Mark that `element_ids` have thumbnails
    fn add_thumbnails(&self, element_ids: &[u32]) -> anyhow::Result<()>;

    /// Get full data for tag by it's primary name
    fn get_tag_data<N>(&self, name: N) -> anyhow::Result<Option<read::Tag>>
    where N: AsRef<str>; 

    /// Remove tag from element
    fn remove_tag_from_element<N>(&self, element_id: u32, tag_name: N) -> anyhow::Result<()>
    where N: AsRef<str>;

    /// Update tag data
    fn update_tag<T>(&self, tag: T, hidden: bool) -> anyhow::Result<()>
    where T: AsRef<Tag>;

    /// Add `tag` to group that have `to` tag, or create new
    /// If `to` does not exist, it will be created 
    /// If `tag == to`, `tag` will be removed from group
    fn alias_tag<N, Nt>(&self, tag: N, to: Nt) -> anyhow::Result<()>
    where 
        N: AsRef<str>,
        Nt: AsRef<str>;

    /// Get tag aliases group
    fn get_tag_aliases<N>(&self, tag: N) -> anyhow::Result<Vec<read::Tag>>
    where N: AsRef<str>;
    
    /// Get summary about tags and elements
    fn get_summary(&self) -> anyhow::Result<Summary>;

    /// Mark import as failed
    fn mark_failed_import(&self, element_id: u32) -> anyhow::Result<()>;

    /// Mark that all elements don't have thumbnails
    fn remove_thumbnails(&self) -> anyhow::Result<()>;

    /// Remove failed mark from failed imports
    fn unmark_failed_imports(&self) -> anyhow::Result<()>;

    /// Remove internal grouping data
    fn clear_groups(&self) -> anyhow::Result<()>;
}