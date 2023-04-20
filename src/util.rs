use std::{path::Path, io::SeekFrom, sync::atomic::AtomicBool, sync::atomic::Ordering, time::Duration};
use anyhow::Context;
use futures::Future;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::error;
use std::fmt::Write;
use itertools::Itertools;
use crate::{model::{Signature, write::{self, ElementToParse, ElementWithMetadata}, SIGNATURE_LEN}, import::{TAG_TRIGGER, ElementPrefab, Importer, ANIMATION_EXTS}, config::CONFIG};

/// Derive crc32
pub trait Crc32Hash {
    /// Derive crc32
    fn crc32(&self) -> u32;
}

impl Crc32Hash for str {
    fn crc32(&self) -> u32 {
        crc32fast::hash(self.as_bytes())
    }
}

/// AtomicBool that will be automatically set to `false` on guard drop
pub struct AutoBool(AtomicBool);
impl AutoBool {
    /// Create new AutoBool in released state
    pub const fn new() -> Self {
        AutoBool(AtomicBool::new(false))
    }
        
    /// Acquire bool guard if bool is `false`
    pub fn acquire(&self) -> Option<AutoBoolGuard> {
        match self.0.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
            Ok(false) => Some(AutoBoolGuard(&self.0)),
            _ => None
        }
    }

    /// Get bool state
    pub fn inspect(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Guard that will set inner AtomicBool to `false` on drop
pub struct AutoBoolGuard<'a>(&'a AtomicBool);
impl Drop for AutoBoolGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

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

/// Read log tail to buf
pub async fn get_log_tail(buf: &mut [u8]) -> anyhow::Result<usize> {
    let mut f = tokio::fs::File::open(&CONFIG.log_file).await?;
    let size = f.seek(SeekFrom::End(0)).await?;
    f.seek(SeekFrom::Start(size.saturating_sub(buf.len() as u64))).await?;
    let read = f.read(buf).await?;
    Ok(read)
}

/// Spawn task that will periodically spawn future
pub async fn task_with_interval<F, Fut>(futs: F, interval: Duration) 
where 
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static {
    tokio::spawn(async move {
        loop {
            match tokio::spawn(futs()).await {
                Ok(_) => (),
                Err(e) => error!(?e, "failed to wait for future"),
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// Spawn task that will periodically spawn blocking task 
pub async fn blocking_task_with_interval<F>(f: F, interval: Duration) 
where F: Fn() -> () + Send + Sync + Clone + Copy + 'static {
    tokio::spawn(async move {
        loop {
            match tokio::task::spawn_blocking(f).await {
                Ok(_) => (),
                Err(e) => error!(?e, "failed to wait for blocking future"),
            }
            tokio::time::sleep(interval).await;
        }
    });
} 