use std::{io::Read, path::PathBuf};
use anyhow::{Context, anyhow};
use atomic::{Atomic, Ordering};
use futures::{stream::FuturesUnordered, StreamExt};
use rayon::prelude::*;
use reqwest::StatusCode;
use tokio::sync::mpsc::channel;
use tracing::{error, info};
use walkdir::WalkDir;
use itertools::Itertools;

use crate::{
    dao::{STORAGE, FutureBlock}, 
    import::{ElementPrefab, ANIMATION_EXTS, IMAGE_EXTS}, 
    model::write::{ElementWithMetadata, Wiki}, 
    CONFIG, util::{self, Procedure}
};

/// Experimentaly decided optimal image signature distance 
pub const SIGNATURE_DISTANCE_THRESHOLD: f32 = 35.0;

/// Width and height of thumbnails
pub const THUMBNAIL_SIZE: (u32, u32) = (256, 256);

/// Indicate state of scan_files()
pub static SCAN_FILES_LOCK: Procedure = Procedure::new();
/// Indicate state of update_metadata()
pub static UPDATE_METADATA_LOCK: Procedure = Procedure::new();
/// Indicate state of group_elements_by_signature()
pub static GROUP_ELEMENTS_LOCK: Procedure = Procedure::new();
/// Indicate state of make_thumbnails()
pub static MAKE_THUMBNAILS_LOCK: Procedure = Procedure::new();
/// Indicate state if update_danbooru_wikis()
pub static FETCH_WIKI_LOCK: Procedure = Procedure::new();

/// Scan `CONFIG.input_folder` directory for new files and import them.
/// Will do nothing if already running
pub async fn scan_files() -> anyhow::Result<u32> {
    let _guard = match SCAN_FILES_LOCK.begin() {
        Some(guard) => guard,
        None => return Ok(0) 
    };
    
    let (tx, mut rx) = channel(8);

    let updater = _guard.updater();
    
    // CPU-heavy task: read and hash files
    tokio::task::spawn_blocking(move || {
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

        updater.set_action_count(files.len() as u32);

        files.into_par_iter()
            .map(|(path, _)| -> anyhow::Result<ElementWithMetadata> {
                // TODO: Will write option return error if file is busy now?..
                let mut file = std::fs::File::options()
                    .write(true)
                    .read(true)
                    .open(&path)?;

                let element = {
                    let mut data = vec![];
                    file.read_to_end(&mut data)?;

                    let prefab = ElementPrefab {
                        path: path.clone(),
                        data,
                    };

                    util::hash_file(prefab)
                        .context(path.display().to_string())?
                };

                // Report that file was processed
                updater.increment();

                Ok(element)
            })
            // Add files to db in chunks of 1000
            .chunks(1000)
            .for_each(|chunk| {
                // Send data, cause we cant access tokio context on this thread
                tx.blocking_send(chunk).ok();
            });

        // tx should be dropped here
    });

    let mut count = 0;
    // This won't run if there are no elements in input folder
    while let Some(chunk) = rx.recv().await {
        let elements: Vec<_> = chunk
            .iter()
            .filter_map(|res| match res {
                Ok(r) => Some(r),
                Err(e) => {
                    error!(?e, "failed to hash element");
                    None   
                }
            })
            .collect();
        count += STORAGE.add_elements(&elements).await?;
    }

    Ok(count)
}


/// Fetch metadata for all pending imports.
/// Will do nothing if already running
pub async fn update_metadata() -> anyhow::Result<()> {
    let _guard = match UPDATE_METADATA_LOCK.begin() {
        Some(guard) => guard,
        None => return Ok(())
    };
    
    let imports = STORAGE
        .get_pending_imports()
        .await?;

    let updater = _guard.updater();
    updater.set_action_count(imports.len() as u32);

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
                    match importer.fetch_metadata(imp).await {
                            Ok(meta) => match STORAGE
                                .add_metadata(imp.id, meta)
                                .await {
                                    Ok(_) => (),
                                    Err(e) => error!(?e, ?imp, "failed to add metadata"),
                                }
                            Err(e) => {
                                error!(?e, ?imp, "failed to fetch metadata");
                                STORAGE.mark_failed_import(imp.id).await.ok();
                            },
                        }
                    }
                    updater.increment();
            }
        })
        .collect();

    // Wait for all importers to finish
    while groups.next().await.is_some() {}
    
    Ok(())
}

/// Group elements by their image signature.
/// Will do nothing if already running
pub async fn group_elements_by_signature() -> anyhow::Result<()> {
    let _guard = match GROUP_ELEMENTS_LOCK.begin() {
        Some(guard) => guard,
        None => return Ok(())
    };

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

    let updater = _guard.updater();
    
    // Get all signatures
    let group_metas = STORAGE.get_groups().await?;

    // Compare each signature with each other (except self)

    let groups = tokio::task::spawn_blocking(move || {
        // Find current autoincrement value to avoid calling to DB
        let current_group_id = group_metas
            .iter()
            .filter_map(|g| g.group_id)
            .max()
            .unwrap_or(1);

        let current_group_id = Atomic::new(current_group_id);

        // Get elements without assigned group
        let ungrouped = group_metas.iter()
            .filter(|m| m.group_id.is_none())
            .collect_vec();
        let groups = parking_lot::RwLock::new(Groups(vec![]));

        updater.set_action_count(ungrouped.len() as u32);
        
        ungrouped
            .par_iter()
            .map(|ungroup| {
                for pot in &group_metas {
                    if ungroup.element_id != pot.element_id 
                        && util::get_sig_distance(
                            &ungroup.signature,
                            &pot.signature
                        ) < SIGNATURE_DISTANCE_THRESHOLD {
                
                        let group_id = match pot.group_id {
                            // Add to known group
                            Some(g) => g,
                            // Create new group
                            None => match groups.read().get_group(pot.element_id) {
                                    // Get existing
                                    Some(id) => id,
                                    // Or create
                                    None => current_group_id.fetch_add(1, Ordering::Relaxed),
                                }
                        };
                        let mut groups = groups.write();
                        groups.add(group_id, ungroup.element_id);
                        groups.add(group_id, pot.element_id);
                    }
                }
                updater.increment();
            })
            .count();
        groups.into_inner()
    }).await?;
    
    // Add remaining
    for (group_id, elem_ids) in &groups.0 {
        STORAGE.add_to_group(elem_ids, Some(*group_id)).await?;
    }
    
    Ok(())
}

/// Make thumbnails for all files that don't have one.
/// Will do nothing if already running
pub fn make_thumbnails() -> anyhow::Result<()> {
    let _guard = match MAKE_THUMBNAILS_LOCK.begin() {
        Some(guard) => guard,
        None => return Ok(())
    };

    let updater = _guard.updater();
    
    let no_thumbnail = STORAGE
        .search_elements("", 0, None, 0)
        .blocking_run()?.0
        .into_iter()
        // TODO: Thumbs for animated
        .filter(|e| !e.has_thumb && !e.animated)
        .collect_vec();

    updater.set_action_count(no_thumbnail.len() as u32);
    
    let elems: Vec<_> = no_thumbnail
        .into_par_iter()
        .map_with(
            (PathBuf::from(&CONFIG.element_pool.path), PathBuf::from(&CONFIG.thumbnails_folder.path)),
            |(pool, thumb), e| {
                pool.push(&e.filename);
                thumb.push(&e.filename);
                thumb.set_extension("jpeg");
            
                let err = util::make_thumbnail(pool, thumb, THUMBNAIL_SIZE);
                
                pool.pop();
                thumb.pop();

                updater.increment();
                
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

    STORAGE.add_thumbnails(&elems).blocking_run()?;

    Ok(())
}

/// Remove thumbnail mark from elements that don't actually have thumbnail
pub fn fix_thumbnails() -> anyhow::Result<()> {
    let _guard = match MAKE_THUMBNAILS_LOCK.begin() {
        Some(guard) => guard,
        None => return Ok(())
    };

    let mut elems = {
        STORAGE.remove_thumbnails().blocking_run()?;
        STORAGE.search_elements("", 0, None, 0).blocking_run()?.0
    };

    let thumbs = std::fs::read_dir(&CONFIG.thumbnails_folder.path)?
        .flat_map(|e| -> anyhow::Result<String> {
            let entry = e?;
            let path = entry.path();
            let filename = path
                .file_stem()
                .ok_or(anyhow!("expected file stem"))?
                .to_string_lossy();
            Ok(filename.into_owned())
        })
    .collect_vec();

    // Retain only elements that have thumbnail
    elems.retain(|e| {
        let filename = e.filename.split('.').next().unwrap();
        thumbs.iter().any(|t| t.starts_with(filename))
    });
    
    let ids = elems.into_iter()
        .map(|e| e.id)
        .collect_vec();

    // Return thumbnail mark
    STORAGE.add_thumbnails(&ids).blocking_run()?;
        
    Ok(())
}

/// Manually start import task in strict sequence
pub async fn manual_import() -> anyhow::Result<()> {
    scan_files().await?;
    info!("Scanned files");
    update_metadata().await?;
    info!("Updated metadata");
    group_elements_by_signature().await?;
    info!("Grouped images");
    tokio::task::spawn_blocking(make_thumbnails).await??;
    info!("Made thumbnails");

    Ok(())
}

/// Fetch danbooru wikis to get tags categories and translations.
/// This may take quite a bit of time.
pub async fn update_danbooru_wikis() -> anyhow::Result<()> {
    use crate::model::danbooru::*;
    let _guard = match FETCH_WIKI_LOCK.begin() {
        Some(guard) => guard,
        None => return Ok(())
    };

    let client = reqwest::Client::new();

    // Construct query
    let mut query = TagQuery {
        search: TagSearch {
            order: Order::Count,
        },
        pagination: PaginatedRequest {
            page: 0,
            limit: 1000,
            // Leave only necessary parts
            only: "name,category,wiki_page[other_names]".into(),
        },
    };

    let updater = _guard.updater();
    
    // Max page for unauthorized/non-premium user is 1000th page
    // 1M tags sorted by post count should be sufficient anyway
    updater.set_action_count(1000);
    
    loop {
        updater.increment();
        
        // Use format! because reqwest use serde_urlencoded which is subset(?)
        // of serde_qs
        let url = format!(
            "https://danbooru.donmai.us/tags.json?{}",
            serde_qs::to_string(&query)?
        );
        
        let data: Vec<TagEntry> = { 
            let resp = client.get(url)
            // Custom useragent is mandatory
            .header(
                "user-agent", 
                "i-just-need-to-fetch-tags"
            )
            .send()
            .await?;

            match resp.status() {
                // Pagination end
                StatusCode::GONE => vec![],
                StatusCode::OK => resp.json().await?,
                _ => { 
                    resp.error_for_status()?;
                    vec![]
                }
            }
        };

        if data.is_empty() {
            break;
        }

        // Convert to internal model
        let data: Vec<Wiki> = data
            .into_iter()
            .flat_map(|w| w.try_into().ok())
            .collect();

        STORAGE.add_wikis(&data).await?;

        query.pagination.page += 1;
    }

    // Tag aliases were updated, reload cache
    STORAGE.reload_tag_aliases_index().await?;
    
    Ok(())
}