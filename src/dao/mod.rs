mod sqlite;

use once_cell::sync::Lazy;
pub use sqlite::Sqlite;
use tokio::sync::Mutex;

use crate::{config::CONFIG, model::{write::{ElementToParse, Tag}, Md5Hash}};

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
}