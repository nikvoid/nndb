use std::{path::Path, io::SeekFrom, sync::atomic::Ordering, time::{Duration, UNIX_EPOCH}, fmt::Display, process::Command};
use anyhow::{Context, bail};
use atomic::Atomic;
use futures::Future;
use md5::{Md5, Digest};
use nndb_common::{TaskStatus, UtcDateTime};
use once_cell::sync::OnceCell;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::error;
use itertools::Itertools;
use crate::{
    model::{Signature, 
        write::{self, ElementToParse, ElementWithMetadata}, 
        SIGNATURE_LEN, MD5_LEN
    },
    import::{TAG_TRIGGER, ElementPrefab, Parser, ANIMATION_EXTS},
    CONFIG
};

/// Procedure state, that will be set to default on drop
pub struct Procedure {
    running: Atomic<bool>,
    state: Atomic<(u32, u32)>
}

impl Procedure {
    /// Create new default state
    pub const fn new() -> Self {
        Self {
            running: Atomic::new(false),
            state: Atomic::new((0, 0))
        }
    }

    /// Indicate procedure beginning.
    /// Will return `Some(ProcedureGuard)` if not running already.
    pub fn begin(&self) -> Option<ProcedureGuard> {
        match self.running.compare_exchange(
            false,
            true, 
            Ordering::Acquire, 
            Ordering::Relaxed
        ) {
            Ok(false) => Some(ProcedureGuard(self)),
            _ => None
        }
    }

    /// Get procedure state
    pub fn state(&self) -> TaskStatus {
        match self.running.load(Ordering::Relaxed) {
            true => {
            let (done, actions) = self.state.load(Ordering::Relaxed);
                TaskStatus::Running { actions, done }
            }
            false => TaskStatus::Sleep
        }
    }
}

/// Procedure guard, that will set procedure state to default on drop
pub struct ProcedureGuard<'a>(&'a Procedure);

impl<'p> ProcedureGuard<'p> {
    /// Get procedure state updater
    pub fn updater(&self) -> ProcedureUpdater<'p> {
        ProcedureUpdater(self.0)
    }
}

impl Drop for ProcedureGuard<'_> {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::Relaxed);       
        self.0.state.store((0, 0), Ordering::Relaxed);      
    }
}

/// Updater, that can increment procedure's done actions count 
pub struct ProcedureUpdater<'a>(&'a Procedure);
impl ProcedureUpdater<'_> {
    /// Saturating increment count of processed actions
    pub fn increment(&self) {
        if self.0.running.load(Ordering::Relaxed) {
            self.0.state.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |mut state| 
                if state.0 < state.1 {
                    state.0 += 1;
                    Some(state)
                } else {
                    None
                }
            ).ok();
        }
    }

    /// Set action count and reset processed action count
    pub fn set_action_count(&self, count: u32) {
        if self.0.running.load(Ordering::Relaxed) {
            self.0.state.store((0, count), Ordering::Relaxed);
        }
    }
}


/// Wrapper for writing [u8] slice as continious hex string
pub struct AsHex<'a>(pub &'a [u8]);

impl Display for AsHex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
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
    path.iter()
        .filter_map(|p| p.to_str())
        .filter(|seg| seg.starts_with(TAG_TRIGGER))
        .flat_map(|seg| seg.strip_prefix(TAG_TRIGGER).unwrap().split('.'))
        .tuples()
        .filter(|(_, tag)| !tag.is_empty())
        .map(|(tag_type, tag)| write::Tag::new(tag, None, tag_type.parse().unwrap()).unwrap())
        .collect()
} 

/// Get last file modification date as [chrono::DateTime]
pub fn get_file_datetime(path: &Path) -> anyhow::Result<UtcDateTime> {
    let dur = path.metadata()?
        .modified()?
        .duration_since(UNIX_EPOCH)?;

    UtcDateTime::from_timestamp(dur.as_secs() as i64, 0)
        .context("failed to construct datetime")    
}

/// Derive file hash, signature, and, if possible, metadata
pub fn hash_file(prefab: ElementPrefab) -> anyhow::Result<ElementWithMetadata> {
    let parser_id = Parser::scan(&prefab);

    let hash: [u8; MD5_LEN] = Md5::digest(&prefab.data).into();    

    let filename = prefab.path.file_name()
        .context("Expected filename")?
        .to_str()
        .context("Failed to convert filename")?;
    
    let ext = filename
        .rsplit('.')
        .next()
        .context("Expected extension")?;

    let new_name = format!("{}.{ext}", AsHex(&hash));
    
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

    let metadata = parser_id.parse_metadata(&prefab)?;
     
    let element = ElementToParse {
        filename: new_name,
        orig_filename: filename.to_owned(),
        hash,
        importer_id: parser_id,
        animated,
        signature,
        broken,
        path: prefab.path,
    };
    
    Ok(ElementWithMetadata(element, metadata, parser_id))
}

/// Make thumnbnail for image `src`.
/// Preserve aspect ratio
pub fn make_thumbnail_image(
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

/// Make thumnbnail for animation `src`.
/// Preserve aspect ratio.
/// FFMpeg required
pub fn make_thumbnail_anim(
    src: &Path, 
    thumb_out: &Path, 
    (max_width, max_height): (u32, u32) 
) -> anyhow::Result<()> {
    let Some(ffpath) = &CONFIG.ffmpeg_path else {
        bail!("ffmpeg needed to generate animation thumbnail");
    };
    
    let mut ffmpeg = Command::new(ffpath)
        .arg("-i")
        .arg(src)
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-vf",
            // Thumbnail filter is slow, but the result is nice
            &format!("thumbnail,scale={max_width}:{max_height}:force_original_aspect_ratio=decrease"),
            "-frames:v",
            "1"
        ])
        .arg(thumb_out)
        .spawn()?;
    
    let status = ffmpeg.wait()?;
    if !status.success() {
        bail!("ffmpeg exited with {status}");
    }
    
    Ok(())
}

/// Read log tail to buf.
/// Note that due to log could become bigger during read or even be smaller than `bytes`, 
/// read bytes count won't always correspond to requested `bytes`.
pub async fn get_log_tail(buf: &mut Vec<u8>, bytes: u64) -> anyhow::Result<usize> {
    let mut f = tokio::fs::File::open(&CONFIG.log_file).await?;
    let size = f.seek(SeekFrom::End(0)).await?;
    let offset = SeekFrom::Start(size.saturating_sub(bytes));
    f.seek(offset).await?;
    let read = f.read_buf(buf).await?;
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
where F: Fn() + Send + Sync + Clone + Copy + 'static {
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

/// Lazy that can be manually initialized.
/// Taken directly from https://docs.rs/once_cell/latest/once_cell/index.html#lateinit
pub struct LateInit<T> { cell: OnceCell<T> }

impl<T> LateInit<T> {
    pub const fn new() -> Self {
        Self { cell: OnceCell::new() }
    }
    
    pub fn init(&self, value: T) {
        assert!(self.cell.set(value).is_ok())
    }
}

impl<T> std::ops::Deref for LateInit<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.cell.get().unwrap()
    }
}
