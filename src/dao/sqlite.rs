use std::{cell::RefCell, path::PathBuf};

use rusqlite::{Connection, named_params, Transaction};

use crate::model::{write::ElementToParse, Md5Hash};

use super::*;

const SQLITE_UP: &str = include_str!("sqlite_up.sql");

pub struct Sqlite(RefCell<Connection>);

trait ConnectionExt {
    fn add_element(&self, e: &ElementToParse) -> anyhow::Result<()>;
}

impl ConnectionExt for Transaction<'_> {
    fn add_element(&self, e: &ElementToParse) -> anyhow::Result<()> {
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
        let sig = e.signature
            .as_ref()
            .map(|s| bytemuck::cast_slice(s));
        
        import_stmt.execute((id, imp_id))?;
        group_stmt.execute((id, sig))?;

        Ok(())
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
    fn add_elements(&self, elements: &[ElementToParse]) -> anyhow::Result<()> {
        let mut hashes = self.get_hashes()?;
        let mut conn = self.0.borrow_mut();
        let mut o_path = PathBuf::from(&CONFIG.element_pool);
        for e in elements {
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
                // log error
                tx.add_element(&e).ok(); 
                
                // Move or copy elements
                // TODO: log error
                if CONFIG.testing_mode {
                    std::fs::copy(&e.path, &o_path).map(|_| ())
                } else {
                    std::fs::rename(&e.path, &o_path)
                }.ok();
            }
            o_path.pop();
            tx.commit()?;

            // Add recently inserted hash
            hashes.push(e.hash);
        }

        Ok(())
    }

    fn get_hashes(&self) -> anyhow::Result<Vec<Md5Hash>> {
        let v = self.0.borrow()
            .prepare("SELECT hash FROM element")?
            .query_map([], |r| r.get::<_, Md5Hash>(0))?
            .filter_map(|h| h.ok())
            .collect();
        Ok(v)
    }

}

