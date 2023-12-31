use std::{str::FromStr, convert::Infallible};

use chrono::{DateTime, Utc};
use enum_iterator::Sequence;
use serde::{Serialize, Deserialize};

use crate::metadata::MetadataSource;

pub type UtcDateTime = DateTime<Utc>;

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Tag {
    /// Tag id
    pub id: u32,
    /// Primary name
    #[cfg_attr(feature = "backend", sqlx(rename = "tag_name"))]
    pub name: String,
    /// Alternative name
    pub alt_name: Option<String>,
    /// Tag type
    pub tag_type: TagType,
    /// Count of elements with this tag
    pub count: u32,
    /// Group id of similar tags/aliases
    pub group_id: Option<u32>,
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
    /// External metadata
    pub ext_meta: Vec<ExternalMetadata>,
    /// Time when element was added to db
    pub add_time: UtcDateTime,
    /// Time when element was created/modified
    pub file_time: Option<UtcDateTime>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Associated {
    /// Source of grouping data
    pub source: MetadataSource,
    /// Group id
    pub value: i64,
    /// Associated elements
    pub elements: Vec<Element>
}

#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ExternalMetadata {
    /// Metadata source
    #[cfg_attr(feature = "backend", sqlx(rename = "importer_id"))]
    pub source: MetadataSource,
    /// Link to source (if was imported from other sources)
    pub src_link: Option<String>,
    /// Time when element was added to other source (if present)
    pub src_time: Option<String>,
    /// Raw Stable Diffusion/etc metadata
    pub raw_meta: Option<String>
}

/// Struct that represent state of some procedure, 
/// where there are many similar operations that can be counted
#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub enum TaskStatus {
    Running {
        /// Task done actions
        done: u32,
        /// Task overall actions
        actions: u32,
    },
    #[default]
    Sleep
}

/// Gelbooru-like types
#[cfg_attr(feature = "backend", derive(sqlx::Type))]
#[derive(
    Serialize, 
    Deserialize, 
    PartialEq, 
    Clone, 
    Copy, 
    Sequence, 
    Default,
)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum TagType {
    Service   = 0,
    Artist    = 1,
    Character = 2,
    Title     = 3,
    Metadata  = 4,
    #[default]
    Tag       = 5,
}

/// Database summary
#[cfg_attr(feature = "backend", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, PartialEq, Default)]
pub struct Summary {
    /// Count of tags in DB
    pub tag_count: u32,
    /// Count of elements in DB
    pub element_count: u32,
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
