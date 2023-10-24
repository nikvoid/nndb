use std::path::PathBuf;

use crate::model::{write::{ElementMetadata, Tag}, read::PendingImport};
use enum_iterator::Sequence;
use nndb_common::{MetadataSource, TagType};

mod novelai;
mod webui;
mod pixiv;

pub const IMAGE_EXTS: &[&str] = &["png", "jpeg", "jpg", "gif", "avif", "webp"];
pub const ANIMATION_EXTS: &[&str] = &["mp4", "mov", "webm", "m4v"];

/// Name directory as `TAG.<tag_type>.<tag_name>.<tag_type>.<tag_name>...`
/// to add `<tag_name>...` to elements in this directory 
pub const TAG_TRIGGER: &str = "TAG.";

/// Holder with element original filename and data 
pub struct ElementPrefab {
    pub path: PathBuf,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, sqlx::Type)]
#[repr(u8)]
pub enum Parser {
    /// No specific metadata
    Passthrough = 0,
    // Novel AI generations
    NovelAI     = 1,
    // Webui generations
    Webui       = 2,
}

impl Parser {
    /// Decide which importer to use with file
    pub fn scan(element: &ElementPrefab) -> Self  {
        match () {
            _ if webui::can_parse(element) => Self::Webui,
            _ if novelai::can_parse(element) => Self::NovelAI,
            _ => Self::Passthrough
        }
    }

    /// Get metadata source of the parser
    pub fn metadata_source(&self) -> MetadataSource {
        match self {
            Parser::Passthrough => MetadataSource::Passthrough,
            Parser::NovelAI => MetadataSource::NovelAI,
            Parser::Webui => MetadataSource::Webui,
        }
    }
    
    /// Parse metadata on hash deriving stage, provided access to file data
    pub fn parse_metadata(
        &self, 
        element: &ElementPrefab
    ) -> anyhow::Result<ElementMetadata> {
        match self {
            Parser::Passthrough => Ok(ElementMetadata {
                src_link: None,
                src_time: None,
                ai_meta: None,
                group: None,
                tags: vec![Tag::new("unknown_source", None, TagType::Metadata).unwrap()],
            }),
            Parser::NovelAI => novelai::parse_metadata(element),
            Parser::Webui => webui::parse_metadata(element),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, sqlx::Type, Sequence)]
#[repr(u8)]
pub enum Fetcher {
    /// Pixiv work metadata
    Pixiv   = 101,
}

impl Fetcher {
    /// Get metadata source corresponding to fetcher
    pub fn metadata_source(&self) -> MetadataSource {
        match self {
            Fetcher::Pixiv => MetadataSource::Pixiv,
        }
    }
    
    /// Check if importer can get metadata for element
    pub fn supported(&self, import: &PendingImport) -> bool {
        match self {
            Fetcher::Pixiv => pixiv::PIXIV.supported(import),
        }
    }
    
    /// Check if importer can fetch metadata now
    pub fn available(&self) -> bool { 
        match self {
            Fetcher::Pixiv => pixiv::PIXIV.available(),
        }    
    }
    
    /// Fetch metadata for pending import (network access implied)
    pub async fn fetch_metadata(
        &self,
        import: &PendingImport
    ) -> anyhow::Result<Option<ElementMetadata>> {
        match self {
            Fetcher::Pixiv => pixiv::PIXIV.fetch_metadata(import).await,
        }
    }
}

pub enum FetchStatus {
    Success(ElementMetadata),
    Fail,
    NotSupported,
}

// Check PNG header
fn is_png(element: &ElementPrefab) -> bool {
    element.data.starts_with(
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
    )    
}

/// Trim pairs of ({[]}) braces expr wrapped into
fn trim_braces(expr: &str) -> Option<&str> {
    // Count braces
    let braces = expr
        .chars()
        .zip(expr.chars().rev())
        .take_while(|&cursors| matches!(
            cursors, 
            | ('{', '}') 
            | ('[', ']')
            | ('(', ')')))
        .count();
    
    expr.get(braces..expr.len() - braces)
}