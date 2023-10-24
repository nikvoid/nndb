use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use futures::FutureExt;
use itertools::Itertools;
use moka::future::Cache;
use nndb_common::search::Term;
use nndb_common::{MetadataSource, search};
use sqlx::Executor;
use sqlx::{SqlitePool, migrate::MigrateDatabase, SqliteConnection};
use tokio::sync::RwLock;

use crate::import::{Fetcher, FetchStatus};
use crate::util;
use crate::{
    model::{
        write::{self, ElementWithMetadata}, 
        read::{self, PendingImport}, 
        Summary, Md5Hash, GroupMetadata, AIMetadata, UtcDateTime
    }, 
    CONFIG
};
use futures::{StreamExt, future::BoxFuture};

use tracing::{error, warn};

pub struct Sqlite {
    pool: SqlitePool,
    id_cache: Cache<String, Arc<Vec<u32>>>,
    alias_cache: RwLock<BTreeMap<String, String>>
}

pub type StorageError = anyhow::Error;

/// Private methods and associated functions
impl Sqlite {
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
        let ElementWithMetadata(e, meta, parser) = element; 

        // Try to get file modification time, this will fall back to 
        // CURRENT_TIMESTAMP in case of error
        let time = util::get_file_datetime(&e.path).ok();

        let hash = e.hash.as_slice();
        let id = sqlx::query!(
            r#"INSERT INTO element (
                filename, orig_filename, hash, broken, animated, add_time
            )
            VALUES (?, ?, ?, ?, ?, ?)"#,
            e.filename,
            e.orig_filename,
            hash,
            e.broken,
            e.animated,
            time
        )
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
        
        // Add metadata right here
        Self::add_metadata_tx(
            tx, 
            id as u32, 
            parser.metadata_source(),
            meta
        ).await?;

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
        source: MetadataSource,
        meta: &write::ElementMetadata
    ) -> Result<(), StorageError> {

        if !meta.tags.is_empty() {
            Self::add_tags_tx(tx, Some(element_id), &meta.tags).await?;
        }
       
        sqlx::query!(
            "INSERT INTO metadata (element_id, importer_id, src_link, src_time, ext_group)
            VALUES (?, ?, ?, ?, ?)",
            element_id, 
            source,
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

    async fn add_fetch_status_tx(
        tx: &mut SqliteConnection,
        element_id: u32, 
        fetcher: Fetcher,
        status: &FetchStatus
    ) -> Result<(), StorageError> {
        
        let query = match status {
            FetchStatus::Success(_) =>
                "INSERT INTO fetch_status (element_id, importer_id, failed, supported)
                VALUES (?, ?, 0, 1)",
            FetchStatus::Fail =>
                "INSERT INTO fetch_status (element_id, importer_id, failed, supported)
                VALUES (?, ?, 1, 1)
                ON CONFLICT (element_id, importer_id) DO UPDATE SET failed = failed + 1",
            FetchStatus::NotSupported =>
                "INSERT INTO fetch_status (element_id, importer_id, failed, supported)
                VALUES (?, ?, 0, 0)"
        };

        sqlx::query(query)
            .bind(element_id)
            .bind(fetcher)
            .execute(&mut *tx)
            .await?;
        
        Ok(())
    }

    /// Add all tags from slice, optionally add join rows for element.
    /// Increment tag's `count` if trying to insert duplicate and `element_id` is `Some`
    async fn add_tags_tx<T>(
        tx: &mut SqliteConnection,
        element_id: Option<u32>, 
        tags: &[T]
    ) -> Result<(), StorageError> 
    where T: AsRef<write::Tag> {    
        for t in tags {
            let t = t.as_ref();
            let name = t.name();
            let alt_name = t.alt_name();
            let typ = t.tag_type();
            
            if let Some(id) = element_id {
                // Update count if tag already exists
                sqlx::query!(
                    "INSERT INTO tag (tag_name, alt_name, tag_type, count)
                    VALUES (?, ?, ?, 1)
                    ON CONFLICT (tag_name) DO UPDATE SET count = count + 1",
                    name,
                    alt_name,
                    typ
                )
                .execute(&mut *tx)
                .await?;
                
                sqlx::query!(
                    "INSERT INTO element_tag (element_id, tag_id)                 
                    VALUES (?, (SELECT id FROM tag WHERE tag_name = ?))
                    ON CONFLICT (element_id, tag_id) DO NOTHING",
                    id, name
                )
                .execute(&mut *tx)
                .await?;
            } else {
                // Just insert tag
                sqlx::query!(
                    "INSERT INTO tag (tag_name, alt_name, tag_type)
                    VALUES (?, ?, ?)
                    ON CONFLICT (tag_name) DO NOTHING",
                    name,
                    alt_name,
                    typ
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
            WHERE tag_name = ?"
        )
        .bind(name)
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

            match (pos, Self::get_tag_data_tx(tx, name).await?) {
                // Return empty vec if one of positive tags not in db
                (true, None) => return Ok(vec![]),
                (true, Some(data)) => pos_tag_set.push(data.id),
                (false, Some(data)) => neg_tag_set.push(data.id),
                _ => ()
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

        for pos_id in &pos_tag_set {
            let opt: Option<i64> = sqlx::query_scalar!(
                "SELECT group_id FROM tag WHERE id = ?",
                pos_id
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
        
        // Count of positive tags
        let pos_tags: i64 = pos_tag_set.len() as i64;
        let ids = Self::with_temp_array_tx(tx, "mem", &arrays, |tx| async move {
            let ids: Vec<u32> = sqlx::query_scalar(
                &format!( // sql
                "
                SELECT DISTINCT e.id FROM element e
                JOIN element_tag et ON et.element_id = e.id
                JOIN tag t ON t.id = et.tag_id
                {join_group_meta}
                {join_metadata}
                WHERE 
                    1
                    {cond_group}
                    {cond_ext_group}
                GROUP BY e.id
                HAVING 
                    CASE ?1
                    -- Just exclude hidden
                    WHEN 0 THEN
                        sum(t.hidden) = 0
                    ELSE 
                        -- Include requested tags 
                        sum(
                            t.id IN mem.pos_tags OR t.group_id IN mem.pos_aliases
                        ) >= ?1
                        AND 
                        -- Exclude hidden and not requested tags
                        sum(
                            (t.id NOT IN mem.pos_tags AND t.group_id NOT IN mem.pos_aliases) 
                            AND t.hidden = 1
                        ) = 0
                    END
                    AND
                    -- Exclude negative tags
                    sum(t.id IN mem.neg_tags) = 0 
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
        let Some(data) = Self::get_tag_data_tx(tx, tag).await? else {
            return Ok(());
        };

        for alias in aliases {
            let alias = alias.as_ref();
            sqlx::query!(
                "INSERT INTO tag_alias (tag_id, alias)
                VALUES (?, ?)
                ON CONFLICT (alias) DO NOTHING",
                data.id, alias
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
        Ok(Self {
            pool,
            id_cache: Cache::new(64),
            alias_cache: RwLock::new(BTreeMap::new()), 
        })
    }

    /// Add all elements from slice (optionally with metadata)
    /// Returns count of new elements
    pub async fn add_elements<E>(&self, elements: &[E]) -> Result<u32, StorageError>
    where E: AsRef<ElementWithMetadata> {
        let mut hashes = self.get_hashes().await?;
        let mut o_path = PathBuf::from(&CONFIG.element_pool.path);
        let mut count = 0;
        
        for elem in elements {
            let ElementWithMetadata(e, ..) = elem.as_ref();

            // Deduplication
            if hashes.contains(&e.hash) {
                warn!(name=e.orig_filename, "duplicate, discarding");
                
                // Remove duplicated file if not in testing mode
                if !CONFIG.testing_mode {
                    std::fs::remove_file(&e.path).ok();
                }
                continue;
            }
            
            let mut tx = self.pool.begin().await?;
        
            let id = match Self::add_element_tx(&mut tx, elem.as_ref()).await {
                Ok(id) => id,
                Err(err) => {
                    error!(?err, name=e.orig_filename, "failed to add element");
                    continue;
                },
            }; 
            
            // Add tags derived from path to file
            let tags = util::get_tags_from_path(&e.path);
            if !tags.is_empty() {
                Self::add_tags_tx(&mut tx, Some(id), tags.as_slice()).await?;
            }
        
            o_path.push(&e.filename);
            
            // Move or copy elements
            if let Err(err) = if CONFIG.testing_mode {
                std::fs::copy(&e.path, &o_path).map(|_| ())
            } else {
                std::fs::rename(&e.path, &o_path)
            } {
                error!(?err, name=e.orig_filename, "failed to move file"); 
                o_path.pop();
                continue;
            }; 
            
            o_path.pop();

            tx.commit().await?;
            // Add recently inserted hash
            hashes.push(e.hash);

            count += 1;
        }
        
        // Invalidate element id cache
        self.id_cache.invalidate_all();
 
        Ok(count)
    }

    /// Get all files' hashes
    pub async fn get_hashes(&self) -> Result<Vec<Md5Hash>, StorageError> {
        let hashes = sqlx::query!(
            r#"SELECT hash FROM element"#
        )
        .map(|anon| anon.hash.try_into().unwrap())
        .fetch_all(&self.pool)
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
        let mut conn = self.pool.acquire().await?;
        Self::add_tags_tx(&mut conn, element_id, tags).await?;

        if element_id.is_some() {
            // Invalidate element id cache
            self.id_cache.invalidate_all();
        }

        Ok(())
    }

    /// Get all possible metadata fetch variants for elements.
    pub async fn get_pending_imports(&self) -> Result<Vec<PendingImport>, StorageError> {
        let fetchers: Vec<_> = enum_iterator::all::<Fetcher>()
            .filter(|&f| f.get_singleton().available())
            .collect();

        let mut conn = self.pool.acquire().await?;

        let imps = Self::with_temp_array_tx(&mut conn, "mem", &[("fetchers", &fetchers)], |conn| async {

            let imps = sqlx::query_as( // sql
                "SELECT e.*, f.value as importer_id
                FROM mem.fetchers as f, element e
                LEFT JOIN fetch_status s ON s.importer_id = f.value AND s.element_id = e.id  
                WHERE s.importer_id IS NULL
                ORDER BY f.value ASC" 
            )
            .fetch_all(&mut *conn)
            .await?;
            
            Ok(imps)  
        }.boxed())
        .await?;
        
        Ok(imps)
    }

    /// Add metadata for element
    pub async fn add_metadata(
        &self, 
        element_id: u32, 
        fetcher: Fetcher, 
        fetch_status: &FetchStatus
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;

        if let FetchStatus::Success(meta) = &fetch_status {
            Self::add_metadata_tx(&mut tx, element_id, fetcher.metadata_source(), meta).await?;
        }

        Self::add_fetch_status_tx(&mut tx, element_id, fetcher, fetch_status).await?;

        tx.commit().await?;
        
        // Invalidate element id cache
        self.id_cache.invalidate_all();

        Ok(())
    }

    /// Get all image signature groups stored in db
    pub async fn get_groups(&self) -> Result<Vec<GroupMetadata>, StorageError> {
        let metas = sqlx::query_as(
            "SELECT * FROM group_metadata"
        )
        .fetch_all(&self.pool)
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
        let mut tx = self.pool.begin().await?;

        let group_id = sqlx::query!(
            "REPLACE INTO group_ids (id) VALUES (?)",
            group
        )
        .execute(&mut *tx)
        .await?
        .last_insert_rowid() as u32;
        
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

        let mut conn = self.pool.acquire().await?;

        // Memoize ids
        let ids = match self.id_cache.get(query) {
            Some(ids) => ids,
            None => {
                let ids = Self::get_element_ids_by_query_tx(&mut conn, query).await?;
                self.id_cache.get_with_by_ref(query, async { Arc::new(ids) }).await
            }
        };
        
        let (elems, tags) =
        Self::with_temp_array_tx(&mut conn, "mem", &[("ids", &ids)], |conn| async move {
            let elems = sqlx::query_as( // sql
                "SELECT
                    e.*, g.group_id
                -- use mem.ids as base table to preserve ordering
                FROM mem.ids i
                LEFT JOIN element e ON i.value = e.id
                LEFT JOIN group_metadata g ON i.value = g.element_id
                LIMIT ? OFFSET ?",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *conn)
            .await?;
        
            let tags = sqlx::query_as( // sql
                "SELECT t.*
                FROM tag t
                JOIN element_tag et ON et.tag_id = t.id
                WHERE et.element_id IN mem.ids
                GROUP BY t.id                
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
        .fetch_optional(&self.pool)
        .await?;

        let Some(elem) = elem else {
            return Ok(None)
        };

        let src_links = sqlx::query!(
            r#"SELECT 
                m.importer_id as "importer_id: MetadataSource", 
                m.src_link as "src_link!"
            FROM metadata m
            WHERE m.element_id = ? AND m.src_link IS NOT NULL"#,
            id
        )
        .map(|anon| (anon.importer_id, anon.src_link))
        .fetch_all(&self.pool)
        .await?;
        
        let src_times = sqlx::query!(
            r#"SELECT 
                m.importer_id as "importer_id: MetadataSource", 
                m.src_time as "src_time!: UtcDateTime"
            FROM metadata m
            WHERE m.element_id = ? AND m.src_time IS NOT NULL"#,
            id
        )
        .map(|anon| (anon.importer_id, anon.src_time))
        .fetch_all(&self.pool)
        .await?;
        
        let add_time = sqlx::query_scalar( // sql
            "SELECT add_time
            FROM element
            WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        
        let ai_meta = sqlx::query_as( // sql
            "SELECT * FROM ai_metadata
            WHERE element_id = ?"    
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let tags = sqlx::query_as( // sql
            "SELECT t.*
            FROM tag t, element_tag et
            WHERE t.id = et.tag_id AND et.element_id = ?"
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let meta = read::ElementMetadata {
            src_links,
            src_times,
            add_time,
            ai_meta,
            tags,
        };
        
        Ok(Some((elem, meta)))
    }

    /// Get elements associated with this element
    pub async fn get_associated_elements(
        &self, 
        element_id: u32
    ) -> Result<Vec<read::Associated>, StorageError> {
        let mut conn = self.pool.acquire().await?;
        let mut output = vec![];

        // Fetch elements with similar signatures
        let by_sig: Vec<read::Element> = sqlx::query_as( //sql
            "SELECT e2.*, gm2.group_id
            FROM element e1
            JOIN group_metadata gm1 ON gm1.element_id = e1.id
            JOIN group_metadata gm2 ON gm2.group_id = gm1.group_id
            JOIN element e2 ON e2.id = gm2.element_id
            WHERE gm2.group_id IS NOT NULL AND e1.id = ?",
        )
        .bind(element_id)
        .fetch_all(&mut *conn)
        .await?;

        if !by_sig.is_empty() {
            output.push(read::Associated {
                source: MetadataSource::Signature,
                // Fine if group is not empty
                id: by_sig[0].group_id.unwrap() as i64,
                elements: by_sig,
            })
        }
        
        // Fetch element ids with grouping data
        let groups: Vec<(u32, MetadataSource, i64)> = sqlx::query!( //sql
            r#"
            SELECT 
                m2.element_id as "element_id!: u32",
                m2.importer_id as "source!: MetadataSource",
                m2.ext_group as "ext_group!: i64"
            FROM metadata m2
            JOIN metadata m1 ON m1.ext_group = m2.ext_group
            WHERE m1.element_id = ? AND m2.ext_group IS NOT NULL
            ORDER BY m2.importer_id, m2.ext_group"#,
            element_id
        )
        .map(|anon| (anon.element_id, anon.source, anon.ext_group))
        .fetch_all(&mut *conn)
        .await?;
        
        let mut subgroups: Vec<((MetadataSource, i64), Vec<u32>)> = vec![];

        for (key, group) in &groups
            .into_iter()
            .group_by(|(_, src, group)| (*src, *group))
        {
            let group = group
                .map(|g| g.0)
                .collect();
            subgroups.push((key, group));
        }

        // Fetch elements data
        for ((source, group_id), ids) in subgroups {
            let elements = Self::with_temp_array_tx(
                &mut conn, 
                "mem", 
                &[("ids", &ids)], 
                |tx| async {
                    let elems: Vec<read::Element> = sqlx::query_as(
                        "SELECT e.*, gm.group_id 
                        FROM element e
                        LEFT JOIN group_metadata gm ON gm.element_id = e.id
                        WHERE e.id IN mem.ids"
                    )
                    .fetch_all(&mut *tx)
                    .await?;
                    
                    Ok(elems)
                }.boxed()
            ).await?;

            output.push(read::Associated {
                source,
                id: group_id,
                elements,
            });
        }
        
        Ok(output)
    }

    /// Update count of elements with tag for each tag.
    pub async fn update_tag_count(&self) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        
        let counts = sqlx::query!(
            "SELECT tag_id, count(tag_id) as count
            FROM element_tag
            GROUP BY tag_id"
        )
        .fetch_all(&mut *tx)
        .await?;

        // Reset all counts
        sqlx::query!(
            "UPDATE tag SET count = 0"
        )
        .execute(&mut *tx)
        .await?;
        
        // Set counts back one by one
        for record in counts {
            sqlx::query!(
                "UPDATE tag SET count = ?
                WHERE id = ?",
                record.count, record.tag_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        
        Ok(())
    }

    /// Tag autocompletion
    pub async fn get_tag_completions(&self, input: &str, limit: u32) -> Result<Vec<read::Tag>, StorageError> {
        let fmt = format!("%{}%", input);
        let tags = sqlx::query_as( // sql
            "SELECT * FROM tag
            WHERE (tag_name LIKE ?1 OR alt_name IS NOT NULL AND alt_name LIKE ?1) AND hidden = 0
            ORDER BY count DESC
            LIMIT ?2"
        )
        .bind(fmt)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(tags)
    }  

    /// Mark that `element_ids` have thumbnails
    pub async fn add_thumbnails(&self, element_ids: &[u32]) -> Result<(), StorageError> {
        let mut conn = self.pool.acquire().await?;
        
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
    pub async fn get_tag_data_by_name(&self, name: &str) -> Result<Option<read::Tag>, StorageError> {
        let mut conn = self.pool.acquire().await?;
        Self::get_tag_data_tx(&mut conn, name).await
    } 
    
    /// Get full data for tag by id
    pub async fn get_tag_data_by_id(&self, id: u32) -> Result<Option<read::Tag>, StorageError> {
        let opt = sqlx::query_as(
            "SELECT * FROM tag
            WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(opt)
    } 

    /// Remove tag from element
    pub async fn remove_tag_from_element(&self, element_id: u32, tag_name: &str) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;

        let Some(data) = Self::get_tag_data_tx(&mut tx, tag_name).await? else {
            return Ok(())
        };
        
        let rows = sqlx::query!(
            "DELETE FROM element_tag
            WHERE element_id = ? AND tag_id = ?",
            element_id, data.id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows > 0 {
            sqlx::query!(
                "UPDATE tag SET count = count - 1
                WHERE id = ?",
                data.id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        
        Ok(())
    }

    /// Update tag data
    pub async fn update_tag(&self, name: &str, tag: &write::Tag, hidden: bool) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
                
        let Some(data) = Self::get_tag_data_tx(&mut tx, name).await? else {
            return Ok(())
        };
        
        sqlx::query(
            "UPDATE tag SET tag_name = ?, alt_name = ?, tag_type = ?, hidden = ?
            WHERE id = ?"
        )
        .bind(tag.name())
        .bind(tag.alt_name())
        .bind(tag.tag_type())
        .bind(hidden)
        .bind(data.id)
        .execute(&mut *tx)
        .await?;
        
        // If alias with new tag name exists, remove it
        sqlx::query(
            "DELETE FROM tag_alias WHERE alias = ? AND tag_id = ?",
        )
        .bind(tag.name())
        .bind(data.id)
        .execute(&mut *tx)
        .await?;

        // If tag primary name was changed, insert former name as alias 
        // to remap this tag during import
        if name != tag.name() {
            sqlx::query!(
                "INSERT INTO tag_alias (tag_id, alias) VALUES (?1, ?2)
                ON CONFLICT (alias) WHERE tag_id = ?1 DO NOTHING",
                data.id, name
            )
            .execute(&mut *tx)
            .await?;

            self.alias_cache.write().await.insert(name.into(), tag.name().into());
        }

        tx.commit().await?;
        
        Ok(())
    }

    /// Add `tag` to group that have `to` tag, or create new
    /// If `to` does not exist, it will be created 
    /// If `tag == to`, `tag` will be removed from group
    pub async fn alias_tag(&self, from_name: &str, to_name: &str) -> Result<(), StorageError> {
        // Special case: alias to self - remove from group 
        if from_name == to_name {
            sqlx::query(
                "UPDATE tag SET group_id = NULL
                WHERE tag_name = ?"
            )
            .bind(from_name)
            .execute(&self.pool)
            .await?;

            return Ok(())
        }
        
        let Some(tag) = self.get_tag_data_by_name(from_name).await? else {
            anyhow::bail!("no such tag");
        };
        
        let alias_to = self.get_tag_data_by_name(to_name).await?;

        // Start transaction
        let mut tx = self.pool.begin().await?;
        
        let alias_to = match alias_to {
            Some(to) => to,
            // Add new tag
            None => {
                let Some(alias) = write::Tag::new(to_name, None, tag.tag_type) else {
                    anyhow::bail!("expected alias name");
                };
                Self::add_tags_tx(&mut tx, None, &[alias]).await?;
                // If add_tags suceeded, tag should be present
                Self::get_tag_data_tx(&mut tx, to_name).await?.unwrap()
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
            WHERE tag_name in (?, ?)"
        )
        .bind(group_id)
        .bind(from_name)
        .bind(to_name)
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
                WHERE t.group_id = tg.id AND t.tag_name = ?
            )"
        )
        .bind(tag)
        .fetch_all(&self.pool)
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
        .fetch_one(&self.pool)
        .await?;

        Ok(summary)
    }

    /// Mark that all elements don't have thumbnails
    pub async fn remove_thumbnails(&self) -> Result<(), StorageError> {
        sqlx::query!(
            "UPDATE element SET has_thumb = 0"
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Remove failed mark from failed imports
    pub async fn unmark_failed_imports(&self) -> Result<(), StorageError> {
        sqlx::query!(
            "DELETE FROM fetch_status WHERE failed > 0 AND supported = 1"
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Remove internal grouping data
    pub async fn clear_groups(&self) -> Result<(), StorageError> {
        sqlx::query!(
            "DELETE FROM group_ids"
        )
        .execute(&self.pool)
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
        
        let mut tx = self.pool.begin().await?;

        let tags: Vec<_> = data
            .iter()
            .map(|d| &d.0)
            .collect();

        Self::add_tags_tx(&mut tx, None, &tags).await?;
        
        for (tag, aliases) in data {
            Self::add_tag_aliases_tx(&mut tx, tag.name(), aliases).await?;
        }

        tx.commit().await?;

        Ok(())
    }

    /// Loads tag aliases to memory in order to speed up multiple lookups 
    pub async fn reload_tag_aliases_index(&self) -> Result<(), StorageError> {
        let mut stream = sqlx::query!(
            "SELECT alias, tag_name
            FROM tag t 
            JOIN tag_alias a ON a.tag_id = t.id",
        )
        .map(|anon| (anon.alias, anon.tag_name))
        .fetch(&self.pool);

        let mut write = self.alias_cache.write().await;

        write.clear();
        
        while let Some(Ok((k, v))) = stream.next().await {
            write.insert(k, v);
        }
        
        Ok(())
    }

    /// Look for tag that corresponds to alias.
    /// Can't be called from async code
    pub fn lookup_alias(&self, alias: &str) -> Option<String> {
        self.alias_cache.blocking_read().get(alias).cloned() 
    }
    
    /// Look for tag that corresponds to alias (async).
    pub async fn lookup_alias_async(&self, alias: &str) -> Option<String> {
        self.alias_cache.read().await.get(alias).cloned() 
    }
}

