use std::{str::FromStr, convert::Infallible};

use chrono::{DateTime, Utc};
use enum_iterator::Sequence;
use num_enum::{FromPrimitive, IntoPrimitive};

pub const SIGNATURE_LEN: usize = 544;

pub type UtcDateTime = DateTime<Utc>;
pub type Md5Hash = [u8; 16];
pub type Signature = [i8; SIGNATURE_LEN];

pub mod read;
pub mod write;

/// Generative Neural Network (SD primarily) metadata
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

/// Metadata for element group
pub struct GroupMetadata {
    /// Id of the element
    pub element_id: u32,
    /// Image signature
    pub signature: Signature,
    /// Element group
    pub group_id: Option<u32>,
}

/// Gelbooru-like types
#[derive(Clone, Copy, FromPrimitive, IntoPrimitive, Sequence, PartialEq)]
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

impl TagType {
    /// Get name of tag type
    pub fn label(self) -> &'static str {
        match self {
            TagType::Reserved => "service",
            TagType::Artist => "artist",
            TagType::Character => "character",
            TagType::Title => "title",
            TagType::Metadata => "metadata",
            TagType::Tag => "tag",
        }
    }
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