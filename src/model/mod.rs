use chrono::{DateTime, Utc};

pub const SIGNATURE_LEN: usize = 544;

pub type UtcDateTime = DateTime<Utc>;
pub type Md5Hash = [u8; 16];
pub type Signature = [i8; SIGNATURE_LEN];

pub mod read;
pub mod write;

pub struct AIMetadata {
    pub positive_prompt: String,
    pub negative_prompt: Option<String>,
    /// Steps count
    pub steps: u32,
    /// CFG scale
    pub scale: f32,
    pub sampler: String,
    /// Not sure if u32 is sufficient
    pub seed: u64,
    pub strength: f32,
    pub noise: f32,
}

/// Gelbooru-like types
pub enum TagType {
    Reserved  = 0,
    Artist    = 1,
    Character = 2,
    Title     = 3,
    Metadata  = 4,
    Tag       = 5,
}
