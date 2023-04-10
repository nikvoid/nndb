use std::{str::FromStr, convert::Infallible};

use chrono::{DateTime, Utc};
use num_enum::{FromPrimitive, IntoPrimitive};

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
    /// Not sure if i32 is sufficient
    pub seed: i64,
    pub strength: f32,
    pub noise: f32,
}

/// Gelbooru-like types
#[derive(Clone, Copy, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TagType {
    Reserved  = 0,
    Artist    = 1,
    Character = 2,
    Title     = 3,
    Metadata  = 4,
    #[default]
    Tag       = 5,
}

impl FromStr for TagType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "artist" => Self::Artist,
            "character" => Self::Character,
            "title" => Self::Title,
            "metadata" => Self::Metadata,
            _ => Self::Tag,
        })
    }
}