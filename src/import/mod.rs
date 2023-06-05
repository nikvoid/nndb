use std::path::PathBuf;

use crate::model::{write::ElementMetadata, read::PendingImport};
use async_trait::async_trait;
use enum_iterator::Sequence;
use num_enum::{FromPrimitive, IntoPrimitive};

mod unknown;
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

#[derive(FromPrimitive, IntoPrimitive, Clone, Copy, Debug, PartialEq, sqlx::Type)]
#[repr(u8)]
pub enum Parser {
    /// No specific metadata
    #[default]
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
            _ if webui::Webui.can_parse(element) => Self::Webui,
            _ if novelai::NovelAI.can_parse(element) => Self::NovelAI,
            _ => Self::Passthrough
        }
    }

    /// Get singleton for chosen importer
    pub fn get_singleton(self) -> &'static dyn MetadataParser {
        match self {
            Self::Passthrough => &unknown::Passthrough,
            Self::NovelAI => &novelai::NovelAI,
            Self::Webui => &webui::Webui,
        }
    }
}

#[derive(FromPrimitive, IntoPrimitive, Clone, Copy, Debug, PartialEq, sqlx::Type, Sequence)]
#[repr(u8)]
pub enum Fetcher {
    /// Just stub
    #[default]
    Unknown = 100,
    /// Pixiv work metadata
    Pixiv   = 101,
}

impl Fetcher {
    /// Get singleton for chosen importer
    pub fn get_singleton(self) -> &'static dyn MetadataFetcher {
        match self {
            Self::Unknown => &unknown::Unknown,
            Self::Pixiv => &*pixiv::PIXIV,
        }
    }
    
    /// Gett fetcher name
    pub fn name(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Pixiv => "Pixiv",
        }
    }
}

pub enum FetchStatus {
    Success(ElementMetadata),
    Fail,
    NotSupported,
}

pub trait MetadataParser: Sync {
    /// Check if importer can get metadata for element
    fn can_parse(&self, element: &ElementPrefab) -> bool;

    /// Parse metadata on hash deriving stage, provided access to file data
    fn parse_metadata(
        &self, 
        element: &ElementPrefab
    ) -> anyhow::Result<ElementMetadata>;

}

#[async_trait]
pub trait MetadataFetcher: Sync {
    /// Check if importer can get metadata for element
    fn supported(&self, import: &PendingImport) -> bool;
    
    /// Check if importer can fetch metadata now
    fn available(&self) -> bool { true }
    
    /// Fetch metadata for pending import (network access implied)
    async fn fetch_metadata(
        &self,
        import: &PendingImport
    ) -> anyhow::Result<Option<ElementMetadata>>;
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