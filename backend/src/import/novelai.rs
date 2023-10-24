use std::{io::Cursor, borrow::Cow};
use anyhow::Context;
use nndb_common::NovelAIMetadata;

use crate::{model::{write::{ElementMetadata, Tag}, TagType}, dao::STORAGE};

use super::{ElementPrefab, is_png};


/// Parse NovelAI prompt
///
/// Reference: https://docs.novelai.net/
fn parse_prompt(prompt: &str) -> impl Iterator<Item = &str> + '_ {
    prompt
        .split(',')    
        .filter_map(|expr| {

        // Strip whitespaces
        let trim = expr.trim();
            
        // Trim strength control braces
        super::trim_braces(trim)
        })
        .flat_map(|expr| {
            // Split mixed tag
            expr.split('|')
                // Remove weigths
                .filter_map(|t| t.split(':').next())
        })
}

/// Check if png contains `Software = NovelAI`
pub fn can_parse(element: &ElementPrefab) -> bool {
    // PNG header
    if !is_png(element) {
        return false
    }
    
    let mut cursor = Cursor::new(&element.data);
    let dec = png::Decoder::new(&mut cursor);
    if let Ok(reader) = dec.read_info() {
        for entry in &reader.info().uncompressed_latin1_text {
            if let ("Software", "NovelAI") = 
                (entry.keyword.as_str(), entry.text.as_str()) {
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
    let dec = png::Decoder::new(&mut cursor);
    let reader = dec.read_info()?;
    
    // Prompt can be in iTXt entry or tEXt entry with key "Description"
    let prompt = reader.info().uncompressed_latin1_text.iter().find_map(|e| {
        match e.keyword.as_str() {
            "Description" => Some(Cow::Borrowed(&e.text)),
            _ => None
        }
    }).or_else(|| reader.info().utf8_text.iter().find_map(|e| {
        match e.keyword.as_str() {
            "Description" => Some(Cow::Owned(e.get_text().ok()?)),
            _ => None    
        }   
    })).context("prompt not found")?;
    
    let others = reader.info().uncompressed_latin1_text.iter().find_map(|e| 
        match e.keyword.as_str() {
            "Comment" => Some(&e.text),
            _ => None
        }
    ).context("novelai metadata not found")?;
        
    let tags = parse_prompt(&prompt)
        .filter_map(|t| {
            let name = STORAGE.get().and_then(|s| s.lookup_alias(t));
            Tag::new(name.as_deref().unwrap_or(t), None, TagType::Tag)
        })
        .chain(Tag::new("novelai_generated", None, TagType::Metadata))
        .collect();
    
    let mut meta: NovelAIMetadata = serde_json::from_str(others)?;
    
    // Merge prompt
    meta.prompt = prompt;

    let raw_meta = serde_json::to_string_pretty(&meta)?;
    
    Ok(ElementMetadata {
        src_link: None,
        src_time: None,
        group: Some(meta.seed),
        raw_meta: Some(raw_meta),
        tags
    })
}
