use std::{time::Instant, path::PathBuf};
use import::{ANIMATION_EXTS, IMAGE_EXTS};
use rayon::prelude::*;
use walkdir::WalkDir;
use std::io::Read;
use anyhow::{Context, bail};
use config::CONFIG;

use crate::{import::ElementPrefab, service::Service, model::write::ElementToParse, dao::{STORAGE, ElementStorage}};

mod model;
mod dao;
mod import;
mod service;
mod config; 

/// Scan `CONFIG.input_folder` directory for new files and import them
fn scan_files() -> anyhow::Result<()> {
    let time = Instant::now();
    
    let files: Vec<_> = WalkDir::new(&CONFIG.input_folder)
        .into_iter()
        .filter_map(|e| {
            let e = match e {
                Ok(e) => e,
                Err(_) => todo!("log error"),
            };

            let path = e.path();   

            if !path.is_file() {
                return None;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            
            match ext.as_deref() {
                Some(e) if IMAGE_EXTS.contains(&e) => Some((path.to_owned(), false)),
                Some(e) if ANIMATION_EXTS.contains(&e) => Some((path.to_owned(), true)),
                _ => None,
            }
        })
        .collect();

    let elements: Vec<_> = files.into_par_iter()
        .map(|(path, is_anim)| {
            let mut file = std::fs::File::open(&path)?;

            let element = match is_anim {
                true => {
                    // TODO: Handle animations differently
                    bail!("todo");
                },
                false => {
                    let mut data = vec![];
                    file.read_to_end(&mut data)?;

                    let prefab = ElementPrefab {
                        path,
                        data,
                    };

                    Service::hash_file(prefab)?
                },
            };

            Ok(element)
        }).collect();

    println!("{}ms", time.elapsed().as_millis());

    let elements: Vec<_> = elements.into_iter().flatten().collect();
    // TODO: Report errors
    if let Err(e) = STORAGE.blocking_lock().add_elements(&elements) {
        //log error
    }

    println!("{}ms", time.elapsed().as_millis());

    Ok(())
}

fn main() {
    scan_files().unwrap();
}
