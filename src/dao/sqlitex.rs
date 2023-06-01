use std::path::PathBuf;

use futures::FutureExt;
use scc::HashIndex;
use sqlx::Executor;
use sqlx::{SqlitePool, migrate::MigrateDatabase, SqliteConnection};

use crate::model::TagType;
use crate::search::{self, Term};
use crate::util::{self, Crc32Hash};
use crate::{
    model::{
        write::{self, ElementWithMetadata}, 
        read::{self, PendingImport}, 
        Summary, Md5Hash, GroupMetadata, AIMetadata
    }, 
    CONFIG
};
use futures::{StreamExt, future::BoxFuture};

use tracing::error;

pub struct Sqlite(SqlitePool);

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

    async fn get_tag_data_tx(
        tx: &mut SqliteConnection,
        name: &str
    ) -> Result<Option<read::Tag>, StorageError> {
        let data = sqlx::query_as(
            "SELECT * FROM tag
            WHERE name_hash = ?"
        )
        .bind(name.crc32())
        .fetch_optional(&mut *tx)
        .await?;
            
        Ok(data)
    }

    /// Create temporary tables with values in in-memory DB
    /// available with `<db_name>.<table_name>`.
    /// 
    /// Future must be `.boxed()`
    async fn with_temp_array_tx<T, F, Out>(
        tx: &mut SqliteConnection,
        db_name: &str,
        tables: &[(&str, &[T])],
        mut inner: F
    ) -> Result<Out, StorageError>
    where 
        for<'a> T: sqlx::Type<sqlx::Sqlite>
            + sqlx::Encode<'a, sqlx::Sqlite>
            + Clone 
            + Send,
        for<'a> F: FnMut(
            &'a mut SqliteConnection
        ) -> BoxFuture<'a, Result<Out, StorageError>>,
    {
        // 1. Open aux in-memory DB
        let mut stmt = format!( // sql
            "ATTACH ':memory:' AS {db_name};"
        );        

        // 2. Construct statement for creating temp arrays
        for (table, data) in tables {
            stmt.push_str(&format!( // sql   
                "CREATE TABLE {db_name}.{table} (
                    value BLOB
                );"
            ));
            if !data.is_empty() {
                stmt.push_str(&format!(
                    "INSERT INTO {db_name}.{table} (value) VALUES " 
                ));
                
                for _ in *data {
                    stmt.push_str("(?),");
                }

                stmt.pop();
                stmt.push_str(";\n");
            }
        }
        
        let mut query = sqlx::query(&stmt);

        for (_, data) in tables {
            for val in *data {
                query = query.bind(val.clone());
            }
        }
        
        // 3. Execute stmt for in-mem DB
        tx.execute_many(query).count().await;
        
        // 4. Use in-mem DB table
        let res = inner(&mut *tx).await;   
       
        // 5. Close in-mem DB
        sqlx::query(
            &format!("DETACH {db_name}")
        )
        .execute(&mut *tx)
        .await?;
        
        res
    }

    /// Get ids of elements that this search query should get
    async fn get_element_ids_by_query_tx(
        tx: &mut SqliteConnection,
        query: &str
    ) -> Result<Vec<u32>, StorageError> {
        
        let mut pos_tag_set = vec![];
        let mut neg_tag_set = vec![];
        
        for tag in search::parse_query(query) {
            let Term::Tag(pos, name) = tag else { continue };
            let hash = name.crc32();

            if pos {
                pos_tag_set.push(hash);
            } else {
                neg_tag_set.push(hash);
            }
        }
        
        let mut group = None;
        let mut ext_group = None;
        for meta in search::parse_query(query) {
            match meta {
                Term::Tag(..) => continue,
                Term::Group(id) => group = Some(id),
                Term::ExtGroup(id) => ext_group = Some(id),
            }
        }

        let mut pos_aliases = vec![];

        for hash in &pos_tag_set {
            let opt: Option<i64> = sqlx::query_scalar!(
                "SELECT group_id FROM tag WHERE name_hash = ?",
                hash
            )
            .fetch_optional(&mut *tx)
            .await?
            .flatten();

            pos_aliases.extend(opt.map(|g| g as u32).iter());            
        }
        
        let arrays = [
            ("pos_tags", pos_tag_set.as_slice()),
            ("neg_tags", neg_tag_set.as_slice()),
            ("pos_aliases", pos_aliases.as_slice()),
        ];
        
        // Combined checksum
        let pos_tags: i64 = pos_tag_set.len() as i64;
        let ids = Self::with_temp_array_tx(tx, "mem", &arrays, |tx| async move {
            let ids: Vec<u32> = sqlx::query_scalar(
                &format!( // sql
                "
                SELECT DISTINCT e.id FROM element e
                JOIN element_tag et ON et.element_id = e.id
                JOIN tag t ON t.name_hash = et.tag_hash
                {join_group_meta}
                {join_metadata}
                -- When no pos tags: exclude hidden (by clearing group)
                -- When hidden pos tag explicitly requested: include only elements with this hidden tag
                -- Exclude elements that have any neg tag
                WHERE 
                    CASE ?1
                    WHEN 0 THEN 1
                    ELSE   
                        et.tag_hash IN mem.pos_tags OR t.group_id IN mem.pos_aliases 
                        -- Pass this to HAVING       
                        OR et.tag_hash IN mem.neg_tags OR t.hidden = 1
                    END 
                    {cond_group}
                    {cond_ext_group}
                GROUP BY e.id
                HAVING 
                    CASE ?1
                    WHEN 0 THEN
                        sum(t.hidden) = 0
                    ELSE 
                        sum(
                            et.tag_hash IN mem.pos_tags 
                            OR t.group_id IN mem.pos_aliases AND t.hidden = 0
                        ) >= ?1
                    END
                    AND
                    sum(et.tag_hash IN mem.neg_tags) = 0 
                ORDER BY e.add_time DESC",
                // Add joins on demand
                join_metadata = ext_group.is_some()
                    .then_some("JOIN metadata m ON m.element_id = e.id")
                    .unwrap_or_default(),
                join_group_meta = group.is_some()
                    .then_some("JOIN group_metadata g ON g.element_id = e.id")
                    .unwrap_or_default(),
                // Bind conditionals directly, they're integers anyway
                cond_group = group
                    .map(|id| format!("AND g.group_id = {id}"))
                    .unwrap_or_default(),
                cond_ext_group = ext_group
                    .map(|id| format!("AND m.ext_group = {id}"))
                    .unwrap_or_default(),
            ))
            .bind(pos_tags)
            .fetch_all(&mut *tx)
            .await?;
            
            Ok(ids)
        }.boxed())
        .await?;

        
        Ok(ids)
    }
    
    /// Add tag aliases to db
    async fn add_tag_aliases_tx<A>(
        tx: &mut SqliteConnection, 
        tag: &str,
        aliases: &[A]
    ) -> Result<(), StorageError>
    where A: AsRef<str> {
        let hash = tag.crc32();
        
        for alias in aliases {
            let alias = alias.as_ref();
            sqlx::query!(
                "INSERT INTO tag_alias (tag_hash, alias)
                VALUES (?, ?)
                ON CONFLICT (alias) DO NOTHING",
                hash, alias
            )
            .execute(&mut *tx)
            .await?;
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
        let mut o_path = PathBuf::from(&CONFIG.element_pool.path);
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
        let mut tx = self.0.begin().await?;

        let group_id = match group {
            None => sqlx::query!("INSERT INTO group_ids (id) VALUES (NULL)")
                .execute(&mut *tx)
                .await?
                .last_insert_rowid() as u32,
            Some(id) => id
        };

        for id in element_ids {
            sqlx::query!(
                "UPDATE group_metadata SET group_id = ? WHERE element_id = ?",
                group_id, id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        
        Ok(group_id)
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

        let mut conn = self.0.acquire().await?;

        let ids = Self::get_element_ids_by_query_tx(&mut conn, query).await?;
        
        let (elems, tags) =
        Self::with_temp_array_tx(&mut conn, "mem", &[("ids", &ids)], |conn| async move {
            let elems = sqlx::query_as( // sql
                "SELECT 
                    e.*, g.group_id, m.ext_group
                FROM element e
                LEFT JOIN group_metadata g ON g.element_id = e.id
                LEFT JOIN metadata m ON m.element_id = e.id
                WHERE e.id in mem.ids
                LIMIT ? OFFSET ?",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *conn)
            .await?;
        
            let tags = sqlx::query_as( // sql
                "SELECT t.*
                FROM tag t
                WHERE t.name_hash IN (
                    SELECT tag_hash FROM element_tag
                    WHERE element_id in mem.ids  
                )
                ORDER BY t.count DESC
                LIMIT ?",
            )
            .bind(tag_limit)
            .fetch_all(&mut *conn)
            .await?;
            
            Ok((elems, tags))
        }.boxed())
        .await?;
        
        
        Ok((elems, tags, ids.len() as u32))
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

        // Nested part of metadata is fetched below 
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
        sqlx::query!(
            "UPDATE tag SET count = (
                SELECT count(*) FROM element_tag WHERE tag_hash = name_hash
            )"
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }

    /// Tag autocompletion
    pub async fn get_tag_completions(&self, input: &str, limit: u32) -> Result<Vec<read::Tag>, StorageError> {
        let fmt = format!("%{}%", input);
        let tags = sqlx::query_as(
            "SELECT * FROM tag
            WHERE tag_name LIKE ? AND hidden = 0
            ORDER BY count DESC
            LIMIT ?"
        )
        .bind(fmt)
        .bind(limit)
        .fetch_all(&self.0)
        .await?;
        
        Ok(tags)
    }  

    /// Mark that `element_ids` have thumbnails
    pub async fn add_thumbnails(&self, element_ids: &[u32]) -> Result<(), StorageError> {
        let mut conn = self.0.acquire().await?;
        
        Self::with_temp_array_tx(&mut conn, "mem", &[("ids", element_ids)], |tx| async {
            sqlx::query(
                "UPDATE element SET has_thumb = 1
                WHERE id IN mem.ids"
            )
            .execute(&mut *tx)
            .await?;
            
            Ok(())
        }.boxed()).await?;
        
        Ok(())
    }

    /// Get full data for tag by it's primary name
    pub async fn get_tag_data(&self, name: &str) -> Result<Option<read::Tag>, StorageError> {
        let mut conn = self.0.acquire().await?;
        Self::get_tag_data_tx(&mut conn, name).await
    } 

    /// Remove tag from element
    pub async fn remove_tag_from_element(&self, element_id: u32, tag_name: &str) -> Result<(), StorageError> {
        let mut tx = self.0.begin().await?;
        let hash = tag_name.crc32();

        let rows = sqlx::query!(
            "DELETE FROM element_tag
            WHERE element_id = ? AND tag_hash = ?",
            element_id, hash
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows > 0 {
            sqlx::query!(
                "UPDATE tag SET count = count - 1
                WHERE name_hash = ?",
                hash
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        
        Ok(())
    }

    /// Update tag data
    pub async fn update_tag(&self, tag: &write::Tag, hidden: bool) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE tag SET alt_name = ?, tag_type = ?, hidden = ?
            WHERE name_hash = ?"
        )
        .bind(tag.alt_name())
        .bind(tag.tag_type())
        .bind(hidden)
        .bind(tag.name_hash())
        .execute(&self.0)
        .await?;
        
        Ok(())
    }

    /// Add `tag` to group that have `to` tag, or create new
    /// If `to` does not exist, it will be created 
    /// If `tag == to`, `tag` will be removed from group
    pub async fn alias_tag(&self, tag: &str, to: &str) -> Result<(), StorageError> {
        // Special case: alias to self - remove from group 
        if tag == to {
            sqlx::query(
                "UPDATE tag SET group_id = NULL
                WHERE name_hash = ?"
            )
            .bind(tag.crc32())
            .execute(&self.0)
            .await?;

            return Ok(())
        }
        
        let Some(tag) = self.get_tag_data(tag).await? else {
            anyhow::bail!("no such tag");
        };
        
        let alias_to = self.get_tag_data(to).await?;

        // Start transaction
        let mut tx = self.0.begin().await?;
        
        let alias_to = match alias_to {
            Some(to) => to,
            // Add new tag
            None => {
                let Some(alias) = write::Tag::new(to, None, tag.tag_type) else {
                    anyhow::bail!("expected alias name");
                };
                Self::add_tags_tx(&mut tx, None, &[alias]).await?;
                // If add_tags suceeded, tag should be present
                Self::get_tag_data_tx(&mut tx, to).await?.unwrap()
            } 
        };

        // Get group or insert new
        let group_id = match alias_to.group_id {
            Some(id) => id,
            None => {
                sqlx::query!(
                    "INSERT INTO tag_group (id) VALUES (NULL)",
                )
                .execute(&mut *tx)
                .await?
                .last_insert_rowid() as u32
            }
        };

        sqlx::query(
            "UPDATE tag SET group_id = ?
            WHERE name_hash in (?, ?)"
        )
        .bind(group_id)
        .bind(tag.name.crc32())
        .bind(alias_to.name.crc32())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        
        Ok(())
    }

    /// Get tag aliases group
    pub async fn get_tag_aliases(&self, tag: &str) -> Result<Vec<read::Tag>, StorageError> {
        let aliases = sqlx::query_as(
            "SELECT * FROM tag
            WHERE group_id = (
                SELECT tg.id FROM tag_group tg, tag t
                WHERE t.group_id = tg.id AND t.name_hash = ?
            )"
        )
        .bind(tag.crc32())
        .fetch_all(&self.0)
        .await?;
        
        Ok(aliases)
    }
    
    /// Get summary about tags and elements
    pub async fn get_summary(&self) -> Result<Summary, StorageError> {
        let summary = sqlx::query_as(
            "SELECT 
                (SELECT count(*) FROM tag) as tag_count, 
                (SELECT count(*) FROM element) as element_count"
        )
        .fetch_one(&self.0)
        .await?;

        Ok(summary)
    }

    /// Mark import as failed
    pub async fn mark_failed_import(&self, element_id: u32) -> Result<(), StorageError> {
        sqlx::query!(
            "UPDATE import SET failed = 1
            WHERE element_id = ?",
            element_id
        )
        .execute(&self.0)
        .await?;
        
        Ok(())
    }

    /// Mark that all elements don't have thumbnails
    pub async fn remove_thumbnails(&self) -> Result<(), StorageError> {
        sqlx::query!(
            "UPDATE element SET has_thumb = 0"
        )
        .execute(&self.0)
        .await?;
        
        Ok(())
    }

    /// Remove failed mark from failed imports
    pub async fn unmark_failed_imports(&self) -> Result<(), StorageError> {
        sqlx::query!(
            "UPDATE import SET failed = 0"
        )
        .execute(&self.0)
        .await?;
        
        Ok(())
    }

    /// Remove internal grouping data
    pub async fn clear_groups(&self) -> Result<(), StorageError> {
        sqlx::query!(
            "DELETE FROM group_ids"
        )
        .execute(&self.0)
        .await?;
        
        Ok(())
    }

    /// Add danbooru wikis to db
    pub async fn add_wikis<W>(&self, wikis: &[W]) -> Result<(), StorageError>
    where W: AsRef<write::Wiki> {
        let data: Vec<_> = wikis
            .iter()
            .flat_map(|w| 
                write::Tag::new(&w.as_ref().title, None, w.as_ref().category)
                    .map(|t| (t, &w.as_ref().aliases))
            )
            .collect();
        
        let mut tx = self.0.begin().await?;

        let tags: Vec<_> = data
            .iter()
            .map(|d| &d.0)
            .collect();

        Self::add_tags_tx(&mut tx, None, &tags).await?;
        
        for (tag, aliases) in data {
            Self::add_tag_aliases_tx(&mut tx, tag.name(), &aliases).await?;
        }

        tx.commit().await?;

        Ok(())
    }

    /// Loads tag aliases to memory in order to speed up multiple lookups 
    pub async fn load_tag_aliases_index(&self, index: &HashIndex<String, String>) -> Result<(), StorageError> {
        let mut stream = sqlx::query!(
            "SELECT alias, tag_name
            FROM tag t 
            JOIN tag_alias a ON a.tag_hash = t.name_hash",
        )
        .map(|anon| (anon.alias, anon.tag_name))
        .fetch(&self.0);

        index.clear_async().await;
        
        while let Some(Ok((k, v))) = stream.next().await {
            index.insert_async(k, v).await.ok();
        }
        
        Ok(())
    }
}

