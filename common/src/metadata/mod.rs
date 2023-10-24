//! Raw metadata parsers and extractors

use std::borrow::Cow;

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

pub mod novelai;
pub mod webui;

/// Source of grouping data and/or metadata
#[cfg_attr(feature = "backend", derive(sqlx::Type))]
#[derive(
    Clone, 
    Copy, 
    Debug,
    PartialEq,
    Sequence,
    Serialize,
    Deserialize,
    PartialOrd,
    Ord,
    Eq,
)]
#[repr(u8)]
pub enum MetadataSource {
    /// Stub value
    Passthrough = 0,
    /// Stable diffusion seed
    NovelAI     = 1,
    /// Stable diffusion seed
    Webui       = 2,
    /// Image signature (id doesn't recorded to db)
    Signature   = 100,
    /// Pixiv illust id
    Pixiv       = 101
}

/// (key, value, should_be_wide)
pub type ParsedMeta<'a> = Vec<(Cow<'a, str>, Cow<'a, str>, bool)>;

impl MetadataSource {
    pub fn group_name(&self) -> &'static str {
        match self {
            MetadataSource::Passthrough => "Passthrough stub. You should not see this.",
            MetadataSource::Signature => "Signature",
            MetadataSource::Webui => "Webui generation seed",
            MetadataSource::NovelAI => "NovelAI generation seed",
            MetadataSource::Pixiv => "Pixiv illust",
        }
    }

    pub fn metadata_name(&self) -> &'static str {
        match self {
            MetadataSource::Passthrough => "Passthrough stub. You should not see this.",
            MetadataSource::Signature => "Signature",
            MetadataSource::Webui => "Webui SD Metadata",
            MetadataSource::NovelAI => "NovelAI SD Metadata",
            MetadataSource::Pixiv => "Pixiv illust metadata",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            MetadataSource::Passthrough => "Passthrough stub. You should not see this.",
            MetadataSource::Signature => "Signature",
            MetadataSource::Webui => "Webui",
            MetadataSource::NovelAI => "NovelAI",
            MetadataSource::Pixiv => "Pixiv",
        }
    }

    /// Extract key-value pairs from raw metadata
    pub fn additional_info<'m>(&self, raw_meta: &'m str) -> ParsedMeta<'m> {
        match self {
            MetadataSource::NovelAI => novelai::parse_metadata(raw_meta),
            MetadataSource::Webui => webui::parse_metadata(raw_meta),
            _ => vec![]
        }
    }
}
