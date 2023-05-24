//! Due to backend being selected at compile time, there is no need for traits
//! (Especially with async methods)
//! 
//! So just stick to API defined somewhere (here) instead.
//!
//! Sample stubbed implementation:
//! ```
//! impl Sqlite {
//!     /// Connect to url and init storage
//!     pub async fn init(url: &str) -> Result<Self, StorageError> {
//!         let pool = SqlitePool::connect(url).await?;
//!         sqlx::migrate!().run(&pool).await?;
//!         Ok(Self(pool))
//!     }
//! 
//!     /// Add all elements from slice (optionally with metadata)
//!     /// Returns count of new elements
//!     pub async fn add_elements<E>(&self, elements: &[E]) -> Result<u32, StorageError>
//!     where E: AsRef<ElementWithMetadata> {
//!         Ok(0)
//!     }
//! 
//!     /// Get all files' hashes
//!     pub async fn get_hashes(&self) -> Result<Vec<Md5Hash>, StorageError> {
//!         Ok(vec![])
//!     }
//! 
//!     /// Add all tags from slice
//!     pub async fn add_tags<T>(&self, element_id: Option<u32>, tags: &[T]) -> Result<(), StorageError>
//!     where T: AsRef<write::Tag> {
//!         Ok(())
//!     }
//! 
//!     /// Get all elements waiting on metadata
//!     pub async fn get_pending_imports(&self) -> Result<Vec<PendingImport>, StorageError> {
//!         Ok(vec![])
//!     }
//! 
//!     /// Add metadata for element -- and remove pending import
//!     pub async fn add_metadata<M>(&self, element_id: u32, metadata: M) -> Result<(), StorageError>
//!     where M: AsRef<write::ElementMetadata> + Send {
//!         Ok(())
//!     }
//! 
//!     /// Get all image signature groups stored in db
//!     pub async fn get_groups(&self) -> Result<Vec<GroupMetadata>, StorageError> {
//!         Ok(vec![])
//!     }
//! 
//!     /// Add all elements to group (or create new group with them)
//!     ///
//!     /// Returns group id
//!     pub async fn add_to_group(
//!         &self, 
//!         element_ids: &[u32],
//!         group: Option<u32>
//!     ) -> Result<u32, StorageError> {
//!         Ok(0)
//!     }
//! 
//!     /// Fetch elements from db, by query, with offset and limit.
//!     /// Returns `(elements, selection_tags, count_in_db)`
//!     pub async fn search_elements(
//!         &self, 
//!         query: &str,
//!         offset: u32, 
//!         limit: Option<u32>,
//!         tag_limit: u32,
//!     ) -> Result<(Vec<read::Element>, Vec<read::Tag>, u32), StorageError> {
//!         Ok((vec![], vec![], 0))
//!     }
//! 
//!     /// Get element data and metadata
//!     pub async fn get_element_data(
//!         &self, 
//!         id: u32,
//!     ) -> Result<Option<(read::Element, read::ElementMetadata)>, StorageError> {
//!         Ok(None)
//!     }
//! 
//!     /// Update count of elements with tag for each tag
//!     pub async fn update_tag_count(&self) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Tag autocompletion
//!     pub async fn get_tag_completions(&self, input: &str, limit: u32) -> Result<Vec<read::Tag>, StorageError> {
//!         Ok(vec![])
//!     }  
//! 
//!     /// Mark that `element_ids` have thumbnails
//!     pub async fn add_thumbnails(&self, element_ids: &[u32]) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Get full data for tag by it's primary name
//!     pub async fn get_tag_data(&self, name: &str) -> Result<Option<read::Tag>, StorageError> {
//!         Ok(None)
//!     } 
//! 
//!     /// Remove tag from element
//!     pub async fn remove_tag_from_element(&self, element_id: u32, tag_name: &str) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Update tag data
//!     pub async fn update_tag<T>(&self, tag: T, hidden: bool) -> Result<(), StorageError>
//!     where T: AsRef<write::Tag> + Send {
//!         Ok(())
//!     }
//! 
//!     /// Add `tag` to group that have `to` tag, or create new
//!     /// If `to` does not exist, it will be created 
//!     /// If `tag == to`, `tag` will be removed from group
//!     pub async fn alias_tag(&self, tag: &str, to: &str) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Get tag aliases group
//!     pub async fn get_tag_aliases(&self, tag: &str) -> Result<Vec<read::Tag>, StorageError> {
//!         Ok(vec![])
//!     }
//!     
//!     /// Get summary about tags and elements
//!     pub async fn get_summary(&self) -> Result<Summary, StorageError> {
//!         todo!()
//!     }
//! 
//!     /// Mark import as failed
//!     pub async fn mark_failed_import(&self, element_id: u32) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Mark that all elements don't have thumbnails
//!     pub async fn remove_thumbnails(&self) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Remove failed mark from failed imports
//!     pub async fn unmark_failed_imports(&self) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! 
//!     /// Remove internal grouping data
//!     pub async fn clear_groups(&self) -> Result<(), StorageError> {
//!         Ok(())
//!     }
//! }
//! ```

mod sqlitex;

use futures::Future;
use once_cell::sync::Lazy;
pub use sqlitex::Sqlite;

use crate::CONFIG;

pub type StorageBackend = Sqlite;

pub static STORAGE: Lazy<StorageBackend> = Lazy::new(|| {
    StorageBackend::init(&CONFIG.db_url).blocking_run().unwrap()
});

/// Helper trait to allow calling `futures::executor::block_on` postfix
pub trait FutureBlock {
    type Output;
    fn blocking_run(self) -> Self::Output; 
}

impl<T> FutureBlock for T where T: Future {
    type Output = <Self as Future>::Output;

    fn blocking_run(self) -> Self::Output {
        futures::executor::block_on(self)
    }
}

/// Wrapper for decoding blob into fixed size array
#[derive(sqlx::Type, Debug)]
#[sqlx(transparent)]
pub struct SliceShim<'a>(&'a [u8]);

impl<'a, const N: usize> TryFrom<SliceShim<'a>> for [i8; N] {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: SliceShim<'a>) -> Result<Self, Self::Error> {
        Self::try_from(bytemuck::cast_slice(value.0))
    }
}

impl<'a, const N: usize> TryFrom<SliceShim<'a>> for [u8; N] {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: SliceShim<'a>) -> Result<Self, Self::Error> {
        Self::try_from(bytemuck::cast_slice(value.0))
    }
} 