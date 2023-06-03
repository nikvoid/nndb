use std::io::Cursor;

use anyhow::bail;
use async_trait::async_trait;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{model::{read::PendingImport, write::{ElementMetadata, Tag}, TagType, AIMetadata}, dao::STORAGE};

use super::{MetadataImporter, ElementPrefab, is_png};

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

/// Stable diffusion webui
///
/// https://github.com/AUTOMATIC1111/stable-diffusion-webui
/// TODO: Support non-png/EXIF?
pub struct Webui;

#[async_trait]
impl MetadataImporter for Webui {
    /// Check if importer can get metadata for element
    fn can_parse(&self, element: &ElementPrefab) -> bool {
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

    /// Check if importer can parse file on hash deriving stage
    fn can_parse_in_place(&self) -> bool { true }

    /// Parse metadata on hash deriving stage, provided access to file data
    fn parse_metadata(
        &self, 
        element: &ElementPrefab
    ) -> anyhow::Result<ElementMetadata> {
        let mut cursor = Cursor::new(&element.data);
        let reader = png::Decoder::new(&mut cursor).read_info()?;
        let Some(params) = reader
            .info()
            .uncompressed_latin1_text
            .iter()
            .find(|e| e.keyword == "parameters")
            .map(|e| &e.text)
        else {
            bail!("`parameters` field not found")
        };

        let mut line_iter = params.lines().peekable();

        // Layout:
        // <prompt>
        // ...
        // Negative prompt: <neg_prompt>
        // ...
        // Steps: <steps>, Sampler: <sampler>, CFG Scale: ...
         
        let prompt = line_iter
            .peeking_take_while(|l| !l.starts_with("Negative prompt:"))
            .join(" ");

        let tags = parse_prompt(&prompt)
            .filter_map(|t| { 
                let name = STORAGE.lookup_alias(&t).unwrap_or(t);                
                Tag::new(&name, None, TagType::Tag)
            })
            // Append source tag 
            .chain(Tag::new("webui_generated", None, TagType::Metadata))
            .collect();        

        let neg_prompt = line_iter
            .peeking_take_while(|l| !l.starts_with("Steps")) 
            .map(|l| l.trim_start_matches("Negative prompt: "))
            .join(" ");

        let Some(other) = line_iter.next() else {
            bail!("Part of webui metadata is missing")
        };

        // Parse other metadata
        let mut ai_meta = other.split(',')
            .filter_map(|m| m.split(':').collect_tuple())
            .fold(AIMetadata::default(), |mut acc, (key, val)| {
                let val = val.trim();
                match key.trim() {
                    "Steps" => acc.steps = val.parse().unwrap_or_default(),
                    "Sampler" => acc.sampler = val.to_owned(),
                    "CFG scale" => acc.scale = val.parse().unwrap_or_default(),
                    "Seed" => acc.seed = val.parse().unwrap_or_default(),
                    "Denoising strength" => acc.strength = val.parse().unwrap_or_default(),
                    _ => ()
                }
                acc
            });
        
        ai_meta.positive_prompt = prompt;
        ai_meta.negative_prompt = Some(neg_prompt);

        Ok(ElementMetadata {
            src_link: None,
            src_time: None,
            group: Some(ai_meta.seed),
            ai_meta: Some(ai_meta),
            tags
        })
    }

    /// Fetch metadata for pending import (network access implied)
    async fn fetch_metadata(
        &self,
        _: &PendingImport
    ) -> anyhow::Result<ElementMetadata> {
        bail!("unimplemented")
    }
}