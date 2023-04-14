use std::{cell::RefCell, path::PathBuf, rc::Rc};

use rusqlite::{Connection, named_params, Transaction, types::Value, vtab};
use tracing::error;

use crate::{model::{Md5Hash, GroupMetadata, SIGNATURE_LEN, AIMetadata}, util};

use super::*;

const SQLITE_UP: &str = include_str!("sqlite_up.sql");

pub struct Sqlite(RefCell<Connection>);

trait ConnectionExt {
    fn add_element(&self, e: &ElementWithMetadata) -> anyhow::Result<u32>;

    fn get_tags_hashes(&self) -> anyhow::Result<Vec<u32>>;
    
    fn add_tags<T>(&self, element_id: u32, tags: &[T]) -> anyhow::Result<()>
    where T: AsRef<Tag>;
    
    fn add_metadata<M>(&self, element_id: u32, metadata: M) -> anyhow::Result<()>
    where M: AsRef<ElementMetadata>;
}

impl ConnectionExt for Transaction<'_> {
    /// Returns element id
    /// Inner method without RefCell overhead
    fn add_element(&self, e: &ElementWithMetadata) -> anyhow::Result<u32> {
        let ElementWithMetadata(e, m) = e; 
        
        let mut el_stmt = self.prepare_cached( //sql
            "INSERT INTO element (filename, orig_name, hash, broken, animated)
            VALUES (:filename, :orig_name, :hash, :broken, :animated)"
        )?;

        let id = el_stmt.insert(named_params! {
            ":filename": e.filename,
            ":orig_name": e.orig_filename,
            ":hash": e.hash,
            ":broken": e.broken,
            ":animated": e.animated, 
        })?;

        match m {
            // Add metadata right here
            Some(meta) => {
                self.add_metadata(id as u32, meta)?;
            },
            // Insert import row
            None => {
                let mut import_stmt = self.prepare_cached(
                    "INSERT INTO import (element_id, importer_id) VALUES (?, ?)"
                )?;
                
                let imp_id: u8 = e.importer_id.into();
                import_stmt.execute((id, imp_id))?;
            },
        }

        if let Some(sig) = e.signature {
            let mut group_stmt = self.prepare_cached(
                "INSERT INTO group_metadata (element_id, signature) VALUES (?, ?)"
            )?; 

            group_stmt.execute((id, bytemuck::cast_slice(&sig)))?;
        }
        
        Ok(id as u32)
    }

    /// Inner method without RefCell overhead
    fn get_tags_hashes(&self) -> anyhow::Result<Vec<u32>> {
         let v = self
            .prepare("SELECT name_hash FROM tag")?
            .query_map([], |r| r.get::<_, u32>(0))?
            .filter_map(|h| h.ok())
            .collect();
        Ok(v)
    }

    /// Inner method without RefCell overhead
    fn add_tags<T>(&self, element_id: u32, tags: &[T]) -> anyhow::Result<()>
    where T: AsRef<Tag> {
        let hashes = self.get_tags_hashes()?;
        let mut tag_stmt = self.prepare_cached( // sql
            "INSERT INTO tag (name_hash, tag_name, alt_name, tag_type)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (name_hash) DO NOTHING"
        )?;

        let mut join_stmt = self.prepare_cached( // sql 
            "INSERT INTO element_tag (element_id, tag_hash)                 
            VALUES (?, ?)
            ON CONFLICT (element_id, tag_hash) DO NOTHING"
        )?;
        
        for t in tags {
            let t = t.as_ref();
            let hash = t.name_hash();

            // Insert if needed
            if !hashes.contains(&hash) {
                let typ: u8 = t.tag_type().into();
                tag_stmt.execute((hash, t.name(), t.alt_name(), typ))?;
                
            }
            
            join_stmt.execute((element_id, hash))?;
        }

        Ok(())
    }

    /// Inner method without RefCell overhead
    fn add_metadata<M>(&self, element_id: u32, metadata: M) -> anyhow::Result<()>
    where M: AsRef<ElementMetadata> {
        let m = metadata.as_ref();
        
        if !m.tags.is_empty() {
            self.add_tags(element_id, &m.tags)?;
        }

        self.prepare_cached(
            "DELETE FROM import WHERE element_id = ?"
        )?.execute((element_id,))?;

        self.prepare_cached(
            "INSERT INTO metadata (element_id, src_link, src_time, ext_group)
            VALUES (?, ?, ?, ?)"
        )?.execute((element_id, &m.src_link, m.src_time, m.group))?;

        if let Some(ai) = &m.ai_meta {
            self.prepare_cached( // sql
                "INSERT INTO ai_metadata 
                (element_id, positive_prompt, negative_prompt, steps, scale,
                sampler, seed, strength, noise)
                VALUES 
                (:element_id, :pos_prompt, :neg_prompt, :steps, :scale,
                :sampler, :seed, :strength, :noise)"
            )?.execute(named_params! {
                ":element_id": element_id,
                ":pos_prompt": ai.positive_prompt,
                ":neg_prompt": ai.negative_prompt,
                ":steps": ai.steps,
                ":scale": ai.scale,
                ":sampler": ai.sampler,
                ":seed": ai.seed,
                ":strength": ai.strength,
                ":noise": ai.noise,
            })?;
        }

        Ok(())
    }
}

impl ElementStorage for Sqlite {
    fn init(url: &str) -> Self {
        let conn = Connection::open(url).unwrap();
        vtab::array::load_module(&conn).unwrap();
        conn.execute_batch(SQLITE_UP).unwrap();
        Self(RefCell::new(conn))
    }

    /// Add all elements from slice.
    /// No changes will remain in DB on error.
    /// Also moves element from it's original path to element pool
    fn add_elements<E>(&self, elements: &[E]) -> anyhow::Result<u32>
    where E: AsRef<ElementWithMetadata> {
        let mut hashes = self.get_hashes()?;
        let mut o_path = PathBuf::from(&CONFIG.element_pool);
        let mut count = 0;
        
        for elem in elements {
            let ElementWithMetadata(e, _) = elem.as_ref();
            
            let mut conn = self.0.borrow_mut(); 
            let id: Option<u32>; 
            
            // Deduplication
            match (hashes.contains(&e.hash), &CONFIG.testing_mode) {
                (true, true) => continue,
                (true, false) => {
                    std::fs::remove_file(&e.path).ok();
                },
                _ => ()
            };
        
            let tx = conn.transaction()?;
            o_path.push(&e.filename);
            {                
                id = match tx.add_element(elem.as_ref()) {
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
            }
            
            o_path.pop();
            tx.commit()?;

            // Add recently inserted hash
            hashes.push(e.hash);

            count += 1;

            // Drop borrow
            drop(conn);        

            // Add tags derived from path to file
            if let Some(id) = id {
                let tags = util::get_tags_from_path(&e.path);
                if !tags.is_empty() {
                    self.add_tags(id, tags.as_slice())?;
                }
            }
        }
 
        Ok(count)
    }

    fn get_hashes(&self) -> anyhow::Result<Vec<Md5Hash>> {
        let v = self.0.borrow()
            .prepare("SELECT hash FROM element")?
            .query_map([], |r| r.get::<_, Md5Hash>(0))?
            .filter_map(|h| h.ok())
            .collect();
        Ok(v)
    }

    fn add_tags<T>(&self, element_id: u32, tags: &[T]) -> anyhow::Result<()>
    where T: AsRef<Tag> {
        let mut conn = self.0.borrow_mut();
        let tx = conn.transaction()?;
        tx.add_tags(element_id, tags)?;
        tx.commit()?;
        Ok(())
    }

    /// Get elements without metadata, awaiting for import
    /// Elements are ordered by importer_id
    fn get_pending_imports(&self) -> anyhow::Result<Vec<PendingImport>> {
        let v: Result<Vec<_>, _> = self.0.borrow().prepare( // sql
            "SELECT e.id, i.importer_id, e.hash, e.orig_name
            FROM element e, import i
            WHERE e.id = i.element_id
            ORDER BY i.importer_id ASC"
        )?.query_map([], |r| Ok(PendingImport {
            id: r.get(0)?,
            importer_id: r.get::<_, u8>(1)?.into(),
            orig_filename: r.get(3)?,
            hash: r.get(2)?,
        })
        )?.collect();
        
        v.map_err(|e| e.into())
    }

    fn add_metadata<M>(&self, element_id: u32, metadata: M) -> anyhow::Result<()>
    where M: AsRef<ElementMetadata> {
        let mut conn = self.0.borrow_mut();
        let tx = conn.transaction()?;
        tx.add_metadata(element_id, metadata)?;
        tx.commit()?;
        Ok(())
    }

    fn get_groups(&self) -> anyhow::Result<Vec<GroupMetadata>> {
        let res: Result<Vec<_>, _> = self.0.borrow()
            .prepare("SELECT element_id, signature, group_id FROM group_metadata")?
            .query_map([], |r| Ok(GroupMetadata {
                element_id: r.get(0)?,
                signature: {
                    let blob: [u8; SIGNATURE_LEN] = r.get(1)?;
                    let slice: &[i8] = bytemuck::cast_slice(&blob);
                    slice.try_into().unwrap()
                },
                group_id: r.get(2)?,
            }))?
            .collect();
        Ok(res?)
    }

    fn add_to_group(
        &self, 
        element_ids: &[u32],
        group: Option<u32>
    ) -> anyhow::Result<u32> {
        let mut conn = self.0.borrow_mut();
        let tx = conn.transaction()?;

        // Create new group if needed
        let group_id = match group {
            None => tx.prepare_cached("INSERT INTO group_ids (id) VALUES (NULL)")?
                .insert([])? as u32,
            Some(id) => id,
        };

        {
            let mut group_stmt = tx.prepare_cached(
                "UPDATE group_metadata SET group_id = ? WHERE element_id = ?"
            )?;
            
            for id in element_ids {
                group_stmt.execute((group_id, id))?;
            }
        }

        tx.commit()?;

        Ok(group_id)
    }

    /// TODO: Add tag search capabilities
    fn search_elements<Q>(
        &self, 
        query: Q,
        offset: u32, 
        limit: u32,
        tag_limit: u32,
    ) -> anyhow::Result<(Vec<read::Element>, Vec<read::Tag>, u32)>
    where Q: AsRef<str> {
        let conn = self.0.borrow();

        // Map ids to Rc<Vec<Value::Integer>> to use them in rarray()
        let ids: Result<Vec<_>, _> = conn.prepare_cached(
            // TODO: Where clause
            "SELECT id FROM element"
        )?.query_map([], |r| Ok(Value::Integer(r.get(0)?)))?
        .collect();
        let ids = Rc::new(ids?);
        
        // Fail if one of elements failed to fetch
        let elems: Result<Vec<_>, _> = conn.prepare_cached( // sql
            "SELECT 
            e.id, e.filename, e.orig_name, e.broken, e.has_thumb, e.animated,
            g.group_id,
            m.ext_group
            FROM element e
            LEFT JOIN group_metadata g ON g.element_id = e.id
            LEFT JOIN metadata m ON m.element_id = e.id
            WHERE e.id in rarray(?)
            ORDER BY e.add_time DESC
            LIMIT ? OFFSET ?"
        )?.query_map((&ids, limit, offset), |r| Ok(read::Element {
            id: r.get(0)?,
            filename: r.get(1)?,
            orig_filename: r.get(2)?,
            broken: r.get(3)?,
            has_thumb: r.get(4)?,
            animated: r.get(5)?,
            group_id: r.get(6)?,
            group: r.get(7)?,
        }))?    
        .collect();

        let elems = elems?;

        let tags: Result<Vec<_>, _> = conn.prepare_cached( // sql
            "SELECT t.tag_name, t.alt_name, t.tag_type, t.group_id,
            (
                SELECT count(et.element_id) FROM element_tag et
                WHERE et.tag_hash = t.name_hash    
            ) as tag_count
            FROM tag t
            WHERE t.hidden = 0 AND t.name_hash IN (
                SELECT tag_hash FROM element_tag
                WHERE element_id in rarray(?) 
            )
            ORDER BY tag_count DESC
            LIMIT ?"
        )?.query_map((&ids, tag_limit,), |r| Ok(read::Tag {
            name: r.get(0)?,
            alt_name: r.get(1)?,
            tag_type: {
                let raw: u8 = r.get(2)?;       
                raw.into()
            },
            group_id: r.get(3)?,
            count: r.get(4)?,
        }))?
        .collect();
            
        Ok((elems, tags?, ids.len() as u32))
    }

    fn get_element_data(
        &self, 
        id: u32,
    ) -> anyhow::Result<Option<(read::Element, read::ElementMetadata)>> {
        let conn = self.0.borrow();

        let data = conn.prepare_cached( //sql
            "SELECT e.filename, e.orig_name, e.broken, e.has_thumb, e.animated, e.add_time,
            gm.group_id, m.src_link, m.src_time, m.ext_group, 
            a.element_id, a.positive_prompt, a.negative_prompt, a.steps, a.scale,
            a.sampler, a.seed, a.strength, a.noise
            FROM element e
            LEFT JOIN group_metadata gm ON gm.element_id = e.id
            LEFT JOIN metadata m ON m.element_id = e.id
            LEFT JOIN ai_metadata a ON a.element_id = e.id
            WHERE e.id = ?"
        )?.query_map((id,), |r| {
            let elem = read::Element {
                id,
                filename: r.get(0)?,
                orig_filename: r.get(1)?,
                broken: r.get(2)?,
                has_thumb: r.get(3)?,
                animated: r.get(4)?,
                group_id: r.get(6)?,
                group: r.get(10)?,
            };

            // Get ai metadata if it exists for element
            let ai_meta_id: Option<u32> = r.get(11)?;
            let ai_meta = match ai_meta_id {
                Some(_) => Some(AIMetadata {
                    positive_prompt: r.get(12)?,
                    negative_prompt: r.get(13)?,
                    steps: r.get(14)?,
                    scale: r.get(15)?,
                    sampler: r.get(16)?,
                    seed: r.get(17)?,
                    strength: r.get(18)?,
                    noise: r.get(19)?,
                }),
                None => None,
            };

            let meta = read::ElementMetadata {
                src_link: r.get(7)?,
                src_time: r.get(8)?,
                ai_meta,
                tags: vec![],
                add_time: r.get(5)?,
            };

            Ok((elem, meta))
        })?.next();

        match data {
            Some(data) => {
                let (elem, mut meta) = data?;
                
                let tags: Result<Vec<_>, _> = conn.prepare_cached( // sql
                    "SELECT t.tag_name, t.alt_name, t.tag_type, t.group_id
                    FROM tag t, element_tag et
                    WHERE t.name_hash = et.tag_hash AND et.element_id = ?"
                )?.query_map((id,), |r| Ok(read::Tag {
                    name: r.get(0)?,
                    alt_name: r.get(1)?,
                    tag_type: {
                        let raw: u8 = r.get(2)?;
                        raw.into()        
                    },
                    // TODO: Count
                    count: 0,
                    group_id: r.get(3)?,
                }))?
                .collect();

                meta.tags = tags?;
                Ok(Some((elem, meta)))
            }
            None => {
                Ok(None)
            }
        }
    }
}

