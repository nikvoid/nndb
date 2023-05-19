use std::path::PathBuf;

use crate::model::{Md5Hash, write::ElementMetadata, read::PendingImport};
use async_trait::async_trait;
use md5::{Digest, Md5};
use num_enum::{FromPrimitive, IntoPrimitive};

mod passthrough;
mod novelai;
mod webui;

pub const IMAGE_EXTS: &[&str] = &["png", "jpeg", "jpg", "gif", "avif", "webp"];
pub const ANIMATION_EXTS: &[&str] = &["mp4", "mov", "webm", "m4v"];

/// Name directory as `TAG.<tag_type>.<tag_name>.<tag_type>.<tag_name>...`
/// to add `<tag_name>...` to elements in this directory 
pub const TAG_TRIGGER: &str = "TAG.";

#[derive(FromPrimitive, IntoPrimitive, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Importer {
    /// No specific metadata
    #[default]
    Passthrough = 0,
    // Novel AI generations
    NovelAI     = 1,
    // Webui generations
    Webui       = 2,
}

impl Importer {
    /// Decide which importer to use with file
    pub fn scan(element: &ElementPrefab) -> Self  {
        match () {
            _ if webui::Webui.can_parse(element) => Self::Webui,
            _ if novelai::NovelAI.can_parse(element) => Self::NovelAI,
            _ => Self::Passthrough
        }
    }

    /// Get singleton for chosen importer
    pub fn get_singleton(self) -> &'static dyn MetadataImporter {
        match self {
            Self::Passthrough => &passthrough::Passthrough,
            Self::NovelAI => &novelai::NovelAI,
            Self::Webui => &webui::Webui,
        }
    }
}

/// Holder with element original filename and data 
pub struct ElementPrefab {
    pub path: PathBuf,
    pub data: Vec<u8>,
}

#[async_trait]
pub trait MetadataImporter: Sync {
    /// Check if importer can get metadata for element
    fn can_parse(&self, element: &ElementPrefab) -> bool;

    /// Check if importer can fetch metadata now
    fn available(&self) -> bool { true }
    
    /// Get hash based on file data or name
    fn derive_hash(
        &self,
        element: &ElementPrefab,
    ) -> Md5Hash {
        Md5::digest(&element.data).into()
    }

    /// Check if importer can parse file on hash deriving stage
    fn can_parse_in_place(&self) -> bool;

    /// Parse metadata on hash deriving stage, provided access to file data
    fn parse_metadata(
        &self, 
        element: &ElementPrefab
    ) -> anyhow::Result<ElementMetadata>;

    /// Fetch metadata for pending import (network access implied)
    async fn fetch_metadata(
        &self,
        element: &PendingImport
    ) -> anyhow::Result<ElementMetadata>;
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