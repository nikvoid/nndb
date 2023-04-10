use std::{cell::RefCell, path::PathBuf};

use rusqlite::{Connection, named_params, Transaction};
use tracing::error;

use crate::{model::{write::ElementToParse, Md5Hash}, service};

use super::*;

const SQLITE_UP: &str = include_str!("sqlite_up.sql");

pub struct Sqlite(RefCell<Connection>);

trait ConnectionExt {
    fn add_element(&self, e: &ElementToParse) -> anyhow::Result<u32>;

    fn get_tags_hashes(&self) -> anyhow::Result<Vec<u32>>;
}

impl ConnectionExt for Transaction<'_> {
    /// Returns element id
    fn add_element(&self, e: &ElementToParse) -> anyhow::Result<u32> {
        let mut el_stmt = self.prepare_cached( //sql
            "INSERT INTO element (filename, orig_name, hash, broken, animated)
            VALUES (:filename, :orig_name, :hash, :broken, :animated)"
        )?;

        let mut import_stmt = self.prepare_cached(
            "INSERT INTO import (element_id, importer_id) VALUES (?, ?)"
        )?;

        let mut group_stmt = self.prepare_cached(
            "INSERT INTO group_metadata (element_id, signature) VALUES (?, ?)"
        )?; 

        let id = el_stmt.insert(named_params! {
            ":filename": e.filename,
            ":orig_name": e.orig_filename,
            ":hash": e.hash,
            ":broken": e.broken,
            ":animated": e.animated, 
        })?;

        let imp_id: u8 = e.importer_id.into();
        import_stmt.execute((id, imp_id))?;

        if let Some(sig) = e.signature {
            group_stmt.execute((id, bytemuck::cast_slice(&sig)))?;
        }
        
        Ok(id as u32)
    }

    fn get_tags_hashes(&self) -> anyhow::Result<Vec<u32>> {
         let v = self
            .prepare("SELECT name_hash FROM tag")?
            .query_map([], |r| r.get::<_, u32>(0))?
            .filter_map(|h| h.ok())
            .collect();
        Ok(v)
    }
}

impl ElementStorage for Sqlite {
    fn init(url: &str) -> Self {
        let conn = Connection::open(url).unwrap();
        conn.execute_batch(SQLITE_UP).unwrap();
        Self(RefCell::new(conn))
    }

    /// Add all elements from slice.
    /// No changes will remain in DB on error.
    /// Also moves element from it's original path to element pool
    fn add_elements<E>(&self, elements: &[E]) -> anyhow::Result<u32>
    where E: AsRef<ElementToParse> {
        let mut hashes = self.get_hashes()?;
        let mut o_path = PathBuf::from(&CONFIG.element_pool);
        let mut count = 0;
        
        for e in elements {
            let e = e.as_ref();
            
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
                id = match tx.add_element(&e) {
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
                let tags = service::get_tags_from_path(&e.path);
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
        let hashes = tx.get_tags_hashes()?;
        {
            let mut tag_stmt = tx.prepare_cached( // sql
                "INSERT INTO tag (name_hash, tag_name, alt_name, tag_type)
                VALUES (?, ?, ?, ?)
                ON CONFLICT (name_hash) DO NOTHING"
            )?;

            let mut join_stmt = tx.prepare_cached( // sql 
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
        } 
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
        let m = metadata.as_ref();
        
        if !m.tags.is_empty() {
            self.add_tags(element_id, &m.tags)?;
        }
        
        let mut conn = self.0.borrow_mut();
        let tx = conn.transaction()?;

        {
            tx.prepare_cached(
                "DELETE FROM import WHERE element_id = ?"
            )?.execute((element_id,))?;

            tx.prepare_cached(
                "INSERT INTO metadata (element_id, src_link, src_time, ext_group)
                VALUES (?, ?, ?, ?)"
            )?.execute((element_id, &m.src_link, m.src_time, m.group))?;

            if let Some(ai) = &m.ai_meta {
                tx.prepare_cached(
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
        }
        tx.commit()?;

        drop(conn);
    
        Ok(())
    }
}

