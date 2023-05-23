use std::path::PathBuf;

use sqlx::Executor;
use sqlx::{SqlitePool, migrate::MigrateDatabase, SqliteConnection};

use crate::util;
use crate::{
    model::{
        write::{self, ElementWithMetadata}, 
        MD5_LEN, 
        SIGNATURE_LEN,
        read::{self, PendingImport}, 
        Summary, Md5Hash, GroupMetadata, AIMetadata
    }, 
    config::CONFIG
};
use futures::StreamExt;

use tracing::error;
use super::SliceShim;

pub struct Sqlite(SqlitePool);

// TEMP:
pub type StorageError = anyhow::Error;

/// Private methods and associated functions
impl Sqlite {
    async fn get_tags_hashes_tx(
        tx: &mut SqliteConnection
    ) -> Result<Vec<u32>, StorageError> {
        let hashes = sqlx::query_scalar!(
            r#"SELECT name_hash as "h: u32" FROM tag"#
        )
        .fetch_all(tx)
        .await?;

        Ok(hashes)
    }

    async fn add_ai_metadata_tx(
        tx: &mut SqliteConnection,
        element_id: u32, 
        ai_meta: &AIMetadata
    ) -> Result<(), StorageError> {
        sqlx::query!(
            "INSERT INTO ai_metadata 
            (element_id, positive_prompt, negative_prompt, steps, scale,
            sampler, seed, strength, noise)
            VALUES 
            (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            element_id, 
            ai_meta.positive_prompt,
            ai_meta.negative_prompt,
            ai_meta.steps,
            ai_meta.scale,
            ai_meta.sampler,
            ai_meta.seed,
            ai_meta.strength,
            ai_meta.noise
        )
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn add_element_tx(
        tx: &mut SqliteConnection, 
        element: &ElementWithMetadata
    ) -> Result<u32, StorageError> {
        let ElementWithMetadata(e, m) = element; 

        let hash = e.hash.as_slice();
        let id = sqlx::query!(
            "INSERT INTO element (filename, orig_filename, hash, broken, animated)
            VALUES (?, ?, ?, ?, ?)",
            e.filename,
            e.orig_filename,
            hash,
            e.broken,
            e.animated
        )
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
        
        match m {
            // Add metadata right here
            Some(meta) => {
                Self::add_metadata_tx(tx, id as u32, meta).await?;
            },
            // Insert import row
            None => {
                let imp_id: u8 = e.importer_id.into();
                sqlx::query!(
                    "INSERT INTO import (element_id, importer_id) VALUES (?, ?)",
                    id,
                    imp_id
                )
                .execute(&mut *tx)
                .await?;
            },
        }

        if let Some(sig) = e.signature {
            let sig = bytemuck::cast_slice(&sig);
            sqlx::query!(
                "INSERT INTO group_metadata (element_id, signature) VALUES (?, ?)",
                id,
                sig
            )
            .execute(tx)
            .await?;
        }
        
        Ok(id as u32)
    }
    
    // FIXME:
    async fn add_metadata_tx(
        tx: &mut SqliteConnection,
        element_id: u32, 
        meta: &write::ElementMetadata
    ) -> Result<(), StorageError> {

        if !meta.tags.is_empty() {
            Self::add_tags_tx(tx, Some(element_id), &meta.tags).await?;
        }

        sqlx::query!(
            "DELETE FROM import WHERE element_id = ?",
            element_id
        )
        .execute(&mut *tx)
        .await?;
        
        sqlx::query!(
            "INSERT INTO metadata (element_id, src_link, src_time, ext_group)
            VALUES (?, ?, ?, ?)",
            element_id, 
            meta.src_link,
            meta.src_time,
            meta.group
        )
        .execute(&mut *tx)
        .await?;
        
        if let Some(ai) = &meta.ai_meta {
            Self::add_ai_metadata_tx(tx, element_id, ai).await?;
        }

        Ok(())
    }

    async fn add_tags_tx<T>(
        tx: &mut SqliteConnection,
        element_id: Option<u32>, 
        tags: &[T]
    ) -> Result<(), StorageError> 
    where T: AsRef<write::Tag> {    
        let hashes = Self::get_tags_hashes_tx(&mut *tx).await?;
    
        for t in tags {
            let t = t.as_ref();
            let hash = t.name_hash();
            let name = t.name();
            let alt_name = t.alt_name();
            let typ: u8 = t.tag_type().into();

            if !hashes.contains(&hash) {
                sqlx::query!(
                    "INSERT INTO tag (name_hash, tag_name, alt_name, tag_type)
                    VALUES (?, ?, ?, ?)
                    ON CONFLICT (name_hash) DO NOTHING",
                    hash,
                    name,
                    alt_name,
                    typ
                )
                .execute(&mut *tx)
                .await?;
            }
            
            if let Some(id) = element_id {
                sqlx::query!(
                    "INSERT INTO element_tag (element_id, tag_hash)                 
                    VALUES (?, ?)
                    ON CONFLICT (element_id, tag_hash) DO NOTHING",
                    id, 
                    hash
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        Ok(())
    }
}

/// Public API
impl Sqlite {
    /// Connect to url and init storage
    pub async fn init(url: &str) -> Result<Self, StorageError> {
        // Create if not exists
        if !sqlx::Sqlite::database_exists(url).await? {
            sqlx::Sqlite::create_database(url).await?;
        }
        let pool = SqlitePool::connect(url).await?;
        // Apply migrations if needed
        sqlx::migrate!().run(&pool).await?;
        Ok(Self(pool))
    }

    /// Add all elements from slice (optionally with metadata)
    /// Returns count of new elements
    pub async fn add_elements<E>(&self, elements: &[E]) -> Result<u32, StorageError>
    where E: AsRef<ElementWithMetadata> {
        let mut hashes = self.get_hashes().await?;
        let mut o_path = PathBuf::from(&CONFIG.element_pool);
        let mut count = 0;
        
        for elem in elements {
            let ElementWithMetadata(e, _) = elem.as_ref();
            
            // Deduplication
            match (hashes.contains(&e.hash), &CONFIG.testing_mode) {
                (true, true) => continue,
                (true, false) => {
                    std::fs::remove_file(&e.path).ok();
                },
                _ => ()
            };
            
            let mut tx = self.0.begin().await?;
        
            o_path.push(&e.filename);
            
            let id = match Self::add_element_tx(&mut tx, elem.as_ref()).await {
                Ok(id) => Some(id),
                Err(err) => {
                    error!(?err, name=e.orig_filename, "failed to add element");
                    None
                },
            }; 
        
            // Move or copy elements
            if let Err(err) = if CONFIG.testing_mode {
                std::fs::copy(&e.path, &o_path).map(|_| ())
            } else {
                std::fs::rename(&e.path, &o_path)
            } {
                error!(?err, name=e.orig_filename, "failed to move file"); 
            }; 
            
            o_path.pop();

            // Add tags derived from path to file
            if let Some(id) = id {
                let tags = util::get_tags_from_path(&e.path);
                if !tags.is_empty() {
                    Self::add_tags_tx(&mut tx, Some(id), tags.as_slice()).await?;
                }
            }
            
            tx.commit().await?;
            // Add recently inserted hash
            hashes.push(e.hash);

            count += 1;
        }
 
        Ok(count)
    }

    /// Get all files' hashes
    pub async fn get_hashes(&self) -> Result<Vec<Md5Hash>, StorageError> {
        let hashes = sqlx::query!(
            r#"SELECT hash FROM element"#
        )
        .map(|anon| anon.hash.try_into().unwrap())
        .fetch_all(&self.0)
        .await?;
        
        Ok(hashes)
    }

    /// Add all tags from slice
    pub async fn add_tags<T>(
        &self, 
        element_id: Option<u32>, 
        tags: &[T]
    ) -> Result<(), StorageError>
    where T: AsRef<write::Tag> {
        let mut conn = self.0.acquire().await?;
        Self::add_tags_tx(&mut conn, element_id, tags).await
    }

    /// Get all elements waiting on metadata
    pub async fn get_pending_imports(&self) -> Result<Vec<PendingImport>, StorageError> {
        let v = sqlx::query_as( // sql
            "SELECT e.*
            FROM element e, import i
            WHERE e.id = i.element_id AND i.failed = 0
            ORDER BY i.importer_id ASC"
        )
        .fetch_all(&self.0)
        .await?;
        
        Ok(v)
    }

    /// Add metadata for element -- and remove pending import
    pub async fn add_metadata<M>(&self, element_id: u32, metadata: M) -> Result<(), StorageError>
    where M: AsRef<write::ElementMetadata> + Send {
        let m = metadata.as_ref();
        let mut conn = self.0.acquire().await?;
        Self::add_metadata_tx(&mut conn, element_id, m).await
    }

    /// Get all image signature groups stored in db
    pub async fn get_groups(&self) -> Result<Vec<GroupMetadata>, StorageError> {
        let metas = sqlx::query_as(
            "SELECT * FROM group_metadata"
        )
        .fetch_all(&self.0)
        .await?;
        
        Ok(metas)
    }

    /// Add all elements to group (or create new group with them)
    ///
    /// Returns group id
    pub async fn add_to_group(
        &self, 
        element_ids: &[u32],
        group: Option<u32>
    ) -> Result<u32, StorageError> {
        Ok(0)
    }

    /// Fetch elements from db, by query, with offset and limit.
    /// Returns `(elements, selection_tags, count_in_db)`
    pub async fn search_elements(
        &self, 
        query: &str,
        offset: u32, 
        limit: Option<u32>,
        tag_limit: u32,
    ) -> Result<(Vec<read::Element>, Vec<read::Tag>, u32), StorageError> {
        let limit = limit.unwrap_or(u32::MAX);

        let elems = sqlx::query_as( // sql
            "SELECT 
                e.*, g.group_id, m.ext_group
            FROM element e
            LEFT JOIN group_metadata g ON g.element_id = e.id
            LEFT JOIN metadata m ON m.element_id = e.id
            -- WHERE e.id in rarray(?)
            LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.0)
        .await?;

        // TEMP:
        let count = sqlx::query_scalar!(
            "SELECT count(*) FROM element"
        )
        .fetch_one(&self.0)
        .await?;

        let tags = sqlx::query_as( // sql
            "SELECT t.*
            FROM tag t
            WHERE t.name_hash IN (
                SELECT tag_hash FROM element_tag
                -- WHERE element_id in  
            )
            ORDER BY t.count DESC
            LIMIT ?",
        )
        .bind(tag_limit)
        .fetch_all(&self.0)
        .await?;
        
        Ok((elems, tags, count as u32))
    }

    /// Get element data and metadata
    pub async fn get_element_data(
        &self, 
        id: u32,
    ) -> Result<Option<(read::Element, read::ElementMetadata)>, StorageError> {
        let elem = sqlx::query_as( // sql
            "SELECT 
                e.*, g.group_id, m.ext_group
            FROM element e
            LEFT JOIN group_metadata g ON g.element_id = e.id
            LEFT JOIN metadata m ON m.element_id = e.id
            WHERE e.id = ?"
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?;

        let Some(elem) = elem else {
            return Ok(None)
        };

        let meta: Option<read::ElementMetadata> = sqlx::query_as( // sql
            "SELECT
                m.src_link,
                m.src_time,
                e.add_time
            FROM metadata m
            INNER JOIN element e ON e.id = m.element_id
            WHERE m.element_id = ?"
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?;
        
        let Some(mut meta) = meta else {
            return Ok(None)
        };

        meta.ai_meta = sqlx::query_as( // sql
            "SELECT * FROM ai_metadata
            WHERE element_id = ?"    
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?;

        meta.tags = sqlx::query_as( // sql
            "SELECT t.*
            FROM tag t, element_tag et
            WHERE t.name_hash = et.tag_hash AND et.element_id = ?"
        )
        .bind(id)
        .fetch_all(&self.0)
        .await?;
        
        Ok(Some((elem, meta)))
    }

    /// Update count of elements with tag for each tag
    pub async fn update_tag_count(&self) -> Result<(), StorageError> {
        Ok(())
    }

    /// Tag autocompletion
    pub async fn get_tag_completions(&self, input: &str, limit: u32) -> Result<Vec<read::Tag>, StorageError> {
        Ok(vec![])
    }  

    /// Mark that `element_ids` have thumbnails
    pub async fn add_thumbnails(&self, element_ids: &[u32]) -> Result<(), StorageError> {
        Ok(())
    }

    /// Get full data for tag by it's primary name
    pub async fn get_tag_data(&self, name: &str) -> Result<Option<read::Tag>, StorageError> {
        Ok(None)
    } 

    /// Remove tag from element
    pub async fn remove_tag_from_element(&self, element_id: u32, tag_name: &str) -> Result<(), StorageError> {
        Ok(())
    }

    /// Update tag data
    pub async fn update_tag<T>(&self, tag: T, hidden: bool) -> Result<(), StorageError>
    where T: AsRef<write::Tag> + Send {
        Ok(())
    }

    /// Add `tag` to group that have `to` tag, or create new
    /// If `to` does not exist, it will be created 
    /// If `tag == to`, `tag` will be removed from group
    pub async fn alias_tag(&self, tag: &str, to: &str) -> Result<(), StorageError> {
        Ok(())
    }

    /// Get tag aliases group
    pub async fn get_tag_aliases(&self, tag: &str) -> Result<Vec<read::Tag>, StorageError> {
        Ok(vec![])
    }
    
    /// Get summary about tags and elements
    pub async fn get_summary(&self) -> Result<Summary, StorageError> {
        todo!()
    }

    /// Mark import as failed
    pub async fn mark_failed_import(&self, element_id: u32) -> Result<(), StorageError> {
        Ok(())
    }

    /// Mark that all elements don't have thumbnails
    pub async fn remove_thumbnails(&self) -> Result<(), StorageError> {
        Ok(())
    }

    /// Remove failed mark from failed imports
    pub async fn unmark_failed_imports(&self) -> Result<(), StorageError> {
        Ok(())
    }

    /// Remove internal grouping data
    pub async fn clear_groups(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

