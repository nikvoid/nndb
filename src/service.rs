use std::{fmt::Write, io::Read, path::Path};
use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use rayon::prelude::*;
use tracing::{error, info};
use walkdir::WalkDir;
use itertools::Itertools;

use crate::{
    dao::{ElementStorage, STORAGE}, 
    import::{ElementPrefab, Importer, ANIMATION_EXTS, IMAGE_EXTS, TAG_TRIGGER}, 
    model::{write::{ElementToParse, self}, SIGNATURE_LEN, Signature}, 
    config::CONFIG
};

/// Experimentaly decided optimal image signature distance 
pub const SIGNATURE_DISTANCE_THRESHOLD: f32 = 0.35;

/// TODO: Provide a way to parse metadata on this stage 
pub fn hash_file(prefab: ElementPrefab) -> anyhow::Result<ElementToParse> {
    let importer_id = Importer::scan(&prefab);
    let importer = importer_id.get_singleton();

    let hash = importer.derive_hash(&prefab);
    
    let mut new_name = String::with_capacity(48);

    let filename = prefab.path.file_name()
        .context("Expected filename")?
        .to_str()
        .context("Failed to convert filename")?;
    
    let ext = filename
        .rsplit('.')
        .next()
        .context("Expected extension")?;

    for byte in hash {
        write!(new_name, "{byte:x}")?
    }

    new_name.push('.');
    new_name.push_str(ext);

    let animated = ANIMATION_EXTS.contains(&ext);   
    let (signature, broken) = match animated {
        false => 'blk: {
            let mut sign = [0; SIGNATURE_LEN];
            let img = image::load_from_memory(&prefab.data);
            
            if let Err(e) = img {
                error!(?e, filename, "failed to load image");
                break 'blk (None, true)
            }
            
            let v = image_match::get_image_signature(img.unwrap());           
            sign.clone_from_slice(&v);
            (Some(sign), false)
        },
        true => (None, false),
    };
     
    let element = ElementToParse {
        filename: new_name,
        orig_filename: filename.to_owned(),
        hash,
        importer_id,
        animated,
        signature,
        broken,
        path: prefab.path,
    };
    
    Ok(element)
}

/// Scan `CONFIG.input_folder` directory for new files and import them
pub fn scan_files() -> anyhow::Result<u32> {
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
        .map(|(path, is_anim)| -> anyhow::Result<ElementToParse> {
            let mut file = std::fs::File::open(&path)?;

            // TODO: Handle animations differently
            let element = match is_anim {
                true | false => {
                    let mut data = vec![];
                    file.read_to_end(&mut data)?;

                    let prefab = ElementPrefab {
                        path,
                        data,
                    };

                    hash_file(prefab)?
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

    let res = STORAGE.blocking_lock().add_elements(&elements);

    // TODO: Move to outer fn?
    match &res {
        Ok(count) => info!(?count, "added elements to db"),
        Err(e) => error!(?e, "failed to add elements"),
    }
    
    res
}

/// Extract tags from path
pub fn get_tags_from_path(path: &Path) -> Vec<write::Tag> {
    path.into_iter()
        .map(|p| p.to_str())
        .flatten()
        .filter(|seg| seg.starts_with(TAG_TRIGGER))
        .flat_map(|seg| seg.strip_prefix(TAG_TRIGGER).unwrap().split('.'))
        .tuples()
        .map(|(tag_type, tag)| write::Tag::new(tag, None, tag_type.parse().unwrap()))
        .collect()
} 

/// Fetch metadata for all pending imports
pub async fn update_metadata() -> anyhow::Result<()> {
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
                        Err(e) => error!(?e, ?imp, "failed to fetch metadata"),
                    }
                }
            }
        })
        .collect();

    // Wait for all importers to finish
    while let Some(_) = groups.next().await {}
    
    Ok(())
}

/// Get distance between 2 signatures.
/// Maximal(?) value is `100.00`
fn get_sig_distance(sig1: &Signature, sig2: &Signature) -> f32 {
    let sum: u32 = sig1.iter()
        .zip(sig2)
        .map(|(&ux, &vx)| (ux - vx).pow(2) as u32)
        .sum();

    (sum as f32).sqrt()
}

/// Group elements by their image signature
pub fn group_elements_by_signature() -> anyhow::Result<()> {
    // Get all signatures
    let group_metas = STORAGE.blocking_lock().get_groups()?;

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
                if get_sig_distance(
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
                                None => STORAGE.blocking_lock()
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
    let store = STORAGE.blocking_lock();
    for (group_id, elem_ids) in groups.0 {
        store.add_to_group(&elem_ids, Some(group_id))?;
    }
    
    Ok(())
}