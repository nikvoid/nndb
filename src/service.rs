use std::fmt::Write;
use anyhow::Context;
use once_cell::sync::Lazy;

use crate::{dao::{ElementStorage, StorageBackend}, import::{ElementPrefab, Importer, ANIMATION_EXTS}, model::{write::ElementToParse, SIGNATURE_LEN}};

/// Global services
pub struct Service;

impl Service {
    /// Processes file hash and signature, cpu heavy
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
                
                if img.is_err() {
                    // TODO: log error
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
}
