use std::{io::Read, path::PathBuf};
use anyhow::{Context, anyhow};
use futures::{stream::FuturesUnordered, StreamExt};
use rayon::prelude::*;
use tracing::{error, info};
use walkdir::WalkDir;
use itertools::Itertools;

use crate::{
    dao::{ElementStorage, STORAGE}, 
    import::{ElementPrefab, ANIMATION_EXTS, IMAGE_EXTS}, 
    model::write::ElementWithMetadata, 
    config::CONFIG, util::{self, AutoBool}
};

/// Experimentaly decided optimal image signature distance 
pub const SIGNATURE_DISTANCE_THRESHOLD: f32 = 35.0;

/// Width and height of thumbnails
pub const THUMBNAIL_SIZE: (u32, u32) = (256, 256);

/// Indicate state of scan_files()
pub static SCAN_FILES_LOCK: AutoBool = AutoBool::new();
/// Indicate state of update_metadata()
pub static UPDATE_METADATA_LOCK: AutoBool = AutoBool::new();
/// Indicate state of group_elements_by_signature()
pub static GROUP_ELEMENTS_LOCK: AutoBool = AutoBool::new();
/// Indicate state of make_thumbnails()
pub static MAKE_THUMBNAILS_LOCK: AutoBool = AutoBool::new();

/// Scan `CONFIG.input_folder` directory for new files and import them.
/// Will do nothing if already running
pub fn scan_files() -> anyhow::Result<u32> {
    let _guard = match SCAN_FILES_LOCK.acquire() {
        Some(guard) => guard,
        None => return Ok(0)
    };
    
    let files: Vec<_> = WalkDir::new(&CONFIG.input_folder)
        .into_iter()
        .filter_map(|e| {
            let e = match e {
                Ok(e) => e,
                Err(e) => {
                    error!(?e, "failed to get entry");
                    return None;
                },
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
        .map(|(path, is_anim)| -> anyhow::Result<ElementWithMetadata> {
            // TODO: Will write option return error if file is busy now?..
            let mut file = std::fs::File::options()
                .write(true)
                .read(true)
                .open(&path)?;

            // TODO: Handle animations differently
            let element = match is_anim {
                true | false => {
                    let mut data = vec![];
                    file.read_to_end(&mut data)?;

                    let prefab = ElementPrefab {
                        path: path.clone(),
                        data,
                    };

                    util::hash_file(prefab)
                        .context(path.display().to_string())?
                },
            };

            Ok(element)
        }).collect();

    let elements: Vec<_> = elements
        .iter()
        .filter_map(|res| match res {
            Ok(r) => Some(r),
            Err(e) => {
                error!(?e, "failed to hash element");
                None   
            }
        })
        .collect();

    STORAGE.blocking_lock().add_elements(&elements)
}


/// Fetch metadata for all pending imports.
/// Will do nothing if already running
pub async fn update_metadata() -> anyhow::Result<()> {
    let _guard = match UPDATE_METADATA_LOCK.acquire() {
        Some(guard) => guard,
        None => return Ok(())
    };
    let imports = STORAGE
        .lock()
        .await
        .get_pending_imports()?;

    let mut groups: FuturesUnordered<_> = imports.iter()
        .group_by(|imp| imp.importer_id)
        .into_iter()
        .map(|(_, group)| group.collect_vec())
        .filter(|group| !group.is_empty())
        // Run all importers concurrently 
        .map(|group| async {
            let importer = group.first().unwrap().importer_id.get_singleton();

            for imp in group {
                if importer.available() {
                    match importer.fetch_metadata(&imp).await {
                        Ok(meta) => match STORAGE
                            .lock()
                            .await
                            .add_metadata(imp.id, meta) {
                                Ok(_) => (),
                                Err(e) => error!(?e, ?imp, "failed to add metadata"),
                            }
                        Err(e) => {
                            error!(?e, ?imp, "failed to fetch metadata");
                            STORAGE.lock().await.mark_failed_import(imp.id).ok();
                        },
                    }
                }
            }
        })
        .collect();

    // Wait for all importers to finish
    while let Some(_) = groups.next().await {}
    
    Ok(())
}

/// Group elements by their image signature.
/// Will do nothing if already running
pub async fn group_elements_by_signature() -> anyhow::Result<()> {
    let _guard = match GROUP_ELEMENTS_LOCK.acquire() {
        Some(guard) => guard,
        None => return Ok(())
    };
    // Get all signatures
    let group_metas = STORAGE
        .lock()
        .await
        .get_groups()?;

    // Get elements without assigned group
    let ungrouped = group_metas.iter()
        .filter(|m| m.group_id.is_none())
        .collect_vec();

    /// Group registry
    struct Groups(Vec<(u32, Vec<u32>)>);
    impl Groups {
        /// Add element to group (and create it if needed)
        fn add(&mut self, group_id: u32, elem_id: u32) {
            let grp = self.0.iter_mut().find(|g| g.0 == group_id);
            match grp {
                Some((_, v)) => v.push(elem_id),
                None => self.0.push((group_id, vec![elem_id])),
            }
        }

        /// Get group for element
        fn get_group(&self, elem_id: u32) -> Option<u32> {
            self.0.iter()
                .find(|g| g.1.contains(&elem_id))
                .map(|g| g.0)
        }
    }

    let mut groups = Groups(vec![]);
    
    // Compare each signature with each other (except self)
    for elem in &ungrouped {
        for pot in &group_metas {
            if elem.element_id != pot.element_id {
                if util::get_sig_distance(
                    &elem.signature,
                    &pot.signature
                ) < SIGNATURE_DISTANCE_THRESHOLD {
                    let group_id = match pot.group_id {
                        // Add to known group
                        Some(g) => g,
                        // Create new group
                        None => match groups.get_group(pot.element_id) {
                                // Get existing
                                Some(id) => id,
                                // Or create
                                None => STORAGE
                                    .lock()
                                    .await
                                    .add_to_group(&[pot.element_id, elem.element_id], None)?,
                            }
                    };
                    groups.add(group_id, elem.element_id);
                    groups.add(group_id, pot.element_id);
                }
            }
        }
    }

    // Add remaining
    let store = STORAGE.lock().await;
    for (group_id, elem_ids) in groups.0 {
        store.add_to_group(&elem_ids, Some(group_id))?;
    }
    
    Ok(())
}

/// Update count of elements with tag for each tag
pub fn update_tag_count() -> anyhow::Result<()> {
    STORAGE.blocking_lock().update_tag_count()
}

/// Make thumbnails for all files that don't have one.
/// Will do nothing if already running
pub fn make_thumbnails() -> anyhow::Result<()> {
    let _guard = match MAKE_THUMBNAILS_LOCK.acquire() {
        Some(guard) => guard,
        None => return Ok(())
    };
    let elems: Vec<_> = STORAGE.blocking_lock()
        .search_elements("", 0, None, 0)?
        .0.into_par_iter()
        // TODO: Thumbs for animated
        .filter(|e| !e.has_thumb && !e.animated)
        .map_with(
            (PathBuf::from(&CONFIG.element_pool), PathBuf::from(&CONFIG.thumbnails_folder)),
            |(pool, thumb), e| {
                pool.push(&e.filename);
                thumb.push(&e.filename);
                thumb.set_extension("jpeg");
            
                let err = util::make_thumbnail(&pool, &thumb, THUMBNAIL_SIZE);
                
                pool.pop();
                thumb.pop();

                match err {
                    Ok(_) => Some(e.id),
                    Err(err) => {
                        error!(?err, e=e.filename, "failed to make thumbnail");
                        None
                    }
                }
            }
        )
        .filter_map(|opt| opt)
        .collect();

    STORAGE.blocking_lock().add_thumbnails(&elems)?;

    Ok(())
}

/// Remove thumbnail mark from elements that don't actually have thumbnail
pub fn fix_thumbnails() -> anyhow::Result<()> {
    let _guard = match MAKE_THUMBNAILS_LOCK.acquire() {
        Some(guard) => guard,
        None => return Ok(())
    };

    let mut elems = {
        let store = STORAGE.blocking_lock();
        store.remove_thumbnails()?;
        store.search_elements("", 0, None, 0)?.0
    };

    let thumbs = std::fs::read_dir(&CONFIG.thumbnails_folder)?.map(
        |e| -> anyhow::Result<String> {
            let entry = e?;
            let path = entry.path();
            let filename = path
                .file_stem()
                .ok_or(anyhow!("expected file stem"))?
                .to_string_lossy();
            Ok(filename.into_owned())
        }
    )
    .flatten()
    .collect_vec();

    // Retain only elements that have thumbnail
    elems.retain(|e| {
        let filename = e.filename.split('.').next().unwrap();
        thumbs.iter().find(|t| t.starts_with(filename))
            .is_some()
    });
    
    let ids = elems.into_iter()
        .map(|e| e.id)
        .collect_vec();

    // Return thumbnail mark
    STORAGE.blocking_lock().add_thumbnails(&ids)?;
        
    Ok(())
}

/// Manually start import task in strict sequence
pub async fn manual_import() -> anyhow::Result<()> {
    tokio::task::spawn_blocking(scan_files).await??;
    info!("Scanned files");
    update_metadata().await?;
    info!("Updated metadata");
    group_elements_by_signature().await?;
    info!("Grouped images");
    tokio::task::spawn_blocking(update_tag_count).await??;
    info!("Updated tag counts");
    tokio::task::spawn_blocking(make_thumbnails).await??;
    info!("Made thumbnails");

    Ok(())
}