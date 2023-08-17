use std::{str::FromStr, convert::Infallible};

use chrono::{DateTime, Utc};
use enum_iterator::Sequence;
use serde::{Serialize, Deserialize};

pub type UtcDateTime = DateTime<Utc>;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Tag {
    /// Tag id
    pub id: u32,
    /// Primary name
    pub name: String,
    /// Alternative name
    pub alt_name: Option<String>,
    /// Tag type
    pub tag_type: TagType,
    /// Count of elements with this tag
    pub count: u32,
    /// Is tag hidden
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Element {
    /// Element id
    pub id: u32,
    /// Url to file
    pub url: String,
    /// Field to mark broken file
    pub broken: bool,
    /// Url to ile thumbnail
    pub thumb_url: Option<String>,
    /// Whether element is animation
    pub animated: bool,
}   

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ElementMetadata {
    /// Link to source (if was imported from other sources)
    pub src_links: Vec<(String, String)>,
    /// Time when element was added to other source (if present)
    pub src_times: Vec<(String, UtcDateTime)>,
    /// Time when element was added to db
    pub add_time: UtcDateTime,
    /// Stable Diffusion/etc metadata
    pub ai_meta: Option<AIMetadata>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Associated {
    /// Associated by this key
    pub key: String,
    /// Key value
    pub value: i64,
    /// Associated elements
    pub elements: Vec<Element>
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct AIMetadata {
    /// Positive prompt
    pub positive_prompt: String,
    /// Negative prompt
    pub negative_prompt: Option<String>,
    /// Step count
    pub steps: u32,
    /// CFG scale
    pub scale: f32,
    /// Used sampler
    pub sampler: String,
    /// Generation seed
    pub seed: i64,
    /// Denoising strength
    pub strength: f32,
    /// Noise
    pub noise: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Sequence, Default)]
#[serde(rename_all = "lowercase")]
pub enum TagType {
    Service,
    Artist,
    Character,
    Title,
    Metadata,
    #[default]
    Tag,
}

impl Tag {
    /// Name with spaces as word separators
    pub fn pretty_name(&self) -> String {
        self.name.replace('_', " ")
    }
}

impl FromStr for TagType {
    type Err = Infallible;

    /// Parse lowercase str to get tag type.
    /// In case of unknown type returns [TagType::Tag].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ty = match s {
            "service" => Self::Service,
            "artist" => Self::Artist,
            "character" => Self::Character,
            "title" => Self::Title,
            "metadata" => Self::Metadata,
            _ => Self::Tag,
        };
        Ok(ty)
    }
}

impl TagType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            TagType::Service => "service",
            TagType::Artist => "artist",
            TagType::Character => "character",
            TagType::Title => "title",
            TagType::Metadata => "metadata",
            TagType::Tag => "tag",
        }
    }
    /// Get capitalized type name
    pub fn name_cap(&self) -> &'static str {
        match self {
            TagType::Service => "Service",
            TagType::Artist => "Artist",
            TagType::Character => "Character",
            TagType::Title => "Title",
            TagType::Metadata => "Metadata",
            TagType::Tag => "Tag",
        }
    }
}
