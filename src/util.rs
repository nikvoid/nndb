use std::path::Path;
use anyhow::Context;
use tracing::error;
use std::fmt::Write;
use itertools::Itertools;
use crate::{model::{Signature, write::{self, ElementToParse, ElementWithMetadata}, SIGNATURE_LEN}, import::{TAG_TRIGGER, ElementPrefab, Importer, ANIMATION_EXTS}};


/// Get distance between 2 signatures.
/// Maximal(?) value is `100.00`
pub fn get_sig_distance(sig1: &Signature, sig2: &Signature) -> f32 {
    let sum: u32 = sig1.iter()
        .zip(sig2)
        .map(|(&ux, &vx)| (ux - vx).pow(2) as u32)
        .sum();

    (sum as f32).sqrt()
}

/// Extract tags from path
pub fn get_tags_from_path(path: &Path) -> Vec<write::Tag> {
    path.into_iter()
        .map(|p| p.to_str())
        .flatten()
        .filter(|seg| seg.starts_with(TAG_TRIGGER))
        .flat_map(|seg| seg.strip_prefix(TAG_TRIGGER).unwrap().split('.'))
        .tuples()
        .filter(|(_, tag)| !tag.is_empty())
        .map(|(tag_type, tag)| write::Tag::new(tag, None, tag_type.parse().unwrap()).unwrap())
        .collect()
} 

/// Derive file hash, signature, and, if possible, metadata
pub fn hash_file(prefab: ElementPrefab) -> anyhow::Result<ElementWithMetadata> {
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

    let metadata = match importer.can_parse_in_place() {
        true => Some(importer.parse_metadata(&prefab)?),
        false => None,
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
    
    Ok(ElementWithMetadata(element, metadata))
}

/// Make thumnbnail for image `src`.
/// Preserve aspect ratio
pub fn make_thumbnail(
    src: &Path, 
    thumb_out: &Path, 
    (max_width, max_height): (u32, u32) 
) -> anyhow::Result<()> {
    let img = image::open(src)?;    

    let ratio = img.width() as f32 / img.height() as f32;

    let (width, height) = 
    if ratio > 1.0 {
        (max_width, (max_height as f32 / ratio) as u32)
    } else {
        ((max_width as f32 * ratio) as u32, max_height)
    };
    
    let thumb = image::imageops::thumbnail(
        &img, 
        width.clamp(0, max_width), 
        height.clamp(0, max_height)
    );
    
    thumb.save(thumb_out)?;
    Ok(())
}