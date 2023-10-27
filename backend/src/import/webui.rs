//! Stable diffusion webui
//!
//! https://github.com/AUTOMATIC1111/stable-diffusion-webui
//! TODO: Support non-png/EXIF?
use std::io::Cursor;

use anyhow::bail;
use nndb_common::webui::iter_metadata;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{model::{write::{ElementMetadata, Tag}, TagType}, dao::STORAGE};

use super::{ElementPrefab, is_png};

/// Escaped with \ braces, etc
static ESCAPE_REX: Lazy<Regex> = Lazy::new(|| {
   Regex::new(r"\\(.)").unwrap()
});

/// Unescaped ([{}]) etc
static COMPLICATED_REX: Lazy<Regex> = Lazy::new(|| {
   Regex::new(r"[^\\][(){}\[\]:|]").unwrap()
});

/// Match prompt weight `prompt:0.001`
static WEIGHT_REX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r":-?[0-9]+(\.[0-9]+)?").unwrap()
});

/// Extract tags from Webui prompt
///
/// Reference: https://github.com/AUTOMATIC1111/stable-diffusion-webui/wiki/Features
fn parse_prompt(prompt: &str) -> impl Iterator<Item = String> + '_ {
    prompt
        .split(',')    
        .filter_map(|expr| {
            // Strip whitespaces
            let trim = expr.trim();

            // Trim strength control braces
            let body = super::trim_braces(trim)?;

            // At first, strip out weights
            let strip_weights = WEIGHT_REX.replace_all(body, "");

            // Webui prompts may be too complex to be splitted reasonably,
            // so just reject anything complicated
            if COMPLICATED_REX.is_match(&strip_weights) {
                None
            } else {
                // Unescape `\(` `\[` etc... 
                let escaped = ESCAPE_REX.replace_all(&strip_weights, "$1");
                Some(escaped.to_string())
            }
        })
}

/// Check if importer can get metadata for element
pub fn can_parse(element: &ElementPrefab) -> bool {
    if !is_png(element) {
        return false
    }
    
    let mut cursor = Cursor::new(&element.data);
    let dec = png::Decoder::new(&mut cursor);
    if let Ok(reader) = dec.read_info() {
        for entry in &reader.info().uncompressed_latin1_text {
            if entry.keyword.as_str() == "parameters" 
            && entry.text.contains("Negative prompt:") {
                return true
            }               
        }
    }
    
    false
}

pub fn extract_metadata(
    element: &ElementPrefab
) -> anyhow::Result<ElementMetadata> {
    let mut cursor = Cursor::new(&element.data);
    let reader = png::Decoder::new(&mut cursor).read_info()?;
    let Some(params) = reader
        .info()
        .uncompressed_latin1_text
        .iter()
        .find(|e| e.keyword == "parameters")
        .map(|e| e.text.clone())
    else {
        bail!("`parameters` field not found")
    };

    let mut meta_iter = iter_metadata(&params);

    let tags = parse_prompt(&meta_iter.next().unwrap().1)
        .filter_map(|t| { 
            let name = STORAGE.get().and_then(|s| s.lookup_alias(&t))
                .unwrap_or(t);
            Tag::new(&name, None, TagType::Tag)
        })
        // Append source tag 
        .chain(Tag::new("webui_generated", None, TagType::Metadata))
        .collect();        

    let Some((_, seed)) = meta_iter.find(|kv| kv.0 == "Seed") else {
        bail!("Seed parameter is missing")
    };

    drop(meta_iter);
    
    Ok(ElementMetadata {
        src_link: None,
        src_time: None,
        group: Some(seed.parse()?),
        raw_meta: Some(params),
        tags
    })
}
