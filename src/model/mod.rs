use std::{str::FromStr, convert::Infallible};

use chrono::{DateTime, Utc};
use enum_iterator::Sequence;
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Serialize, Deserialize};

pub const SIGNATURE_LEN: usize = 544;
pub const MD5_LEN: usize = 16;

pub type UtcDateTime = DateTime<Utc>;
pub type Md5Hash = [u8; MD5_LEN];
pub type Signature = [i8; SIGNATURE_LEN];

use crate::dao::SliceShim;

pub mod read;
pub mod write;

/// Generative Neural Network (SD primarily) metadata
#[derive(Default, sqlx::FromRow)]
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
#[derive(sqlx::FromRow)]
pub struct GroupMetadata {
    /// Id of the element
    pub element_id: u32,
    /// Image signature
    #[sqlx(try_from = "SliceShim<'a>")]
    pub signature: Signature,
    /// Element group
    pub group_id: Option<u32>,
}

/// Database summary
#[derive(sqlx::FromRow)]
pub struct Summary {
    /// Count of tags in DB
    pub tag_count: u32,
    /// Count of elements in DB
    pub element_count: u32,
}

/// Gelbooru-like types
#[derive(
    Clone, 
    Copy, 
    FromPrimitive,
    IntoPrimitive, 
    Sequence, 
    PartialEq, 
    Serialize, 
    Deserialize,
    sqlx::Type
)]
#[repr(u8)]
pub enum TagType {
    #[serde(alias = "service")]
    Service   = 0,
    #[serde(alias = "artist")]
    Artist    = 1,
    #[serde(alias = "character")]
    Character = 2,
    #[serde(alias = "title")]
    Title     = 3,
    #[serde(alias = "metadata")]
    Metadata  = 4,
    #[serde(alias = "tag")]
    #[default]
    Tag       = 5,
}

impl TagType {
    /// Get name of tag type
    pub fn label(self) -> &'static str {
        match self {
            TagType::Service => "service",
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