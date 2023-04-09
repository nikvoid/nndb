use std::{fmt::Write, io::Read};
use anyhow::{Context, bail};
use rayon::prelude::*;
use tracing::{error, info};
use walkdir::WalkDir;

use crate::{
    dao::{ElementStorage, STORAGE}, 
    import::{ElementPrefab, Importer, ANIMATION_EXTS, IMAGE_EXTS}, 
    model::{write::ElementToParse, SIGNATURE_LEN}, 
    config::CONFIG
};

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
        .into_iter()
        .filter_map(|res| match res {
            Ok(r) => Some(r),
            Err(e) => {
                error!(?e, "failed to hash element");
                None   
            }
        })
        .collect();

    let res = STORAGE.blocking_lock().add_elements(&elements);

    // TODO: Move to outer fn
    match &res {
        Ok(count) => info!(?count, "added elements to db"),
        Err(e) => error!(?e, "failed to add elements"),
    }
    
    res
}