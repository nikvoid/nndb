use std::{io::Cursor, borrow::Cow};

use anyhow::Context;
use serde::Deserialize;

use crate::{model::{write::{ElementMetadata, Tag}, TagType, AIMetadata}, dao::STORAGE};

use super::{MetadataParser, ElementPrefab, is_png};

pub struct NovelAI;

#[derive(Deserialize)]
struct Metadata<'a> {
    steps: u32,
    sampler: &'a str,
    seed: i64,
    strength: f32,
    noise: f32,
    scale: f32,
    uc: Cow<'a, str>
}

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

impl MetadataParser for NovelAI {
    /// Check if png contains `Software = NovelAI`
    fn can_parse(&self, element: &ElementPrefab) -> bool {
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

    /// Parse metadata on hash deriving stage, provided access to file data
    fn parse_metadata(
        &self, 
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
            
        let meta: Metadata = serde_json::from_str(others)?;

        let tags = parse_prompt(&prompt)
            .filter_map(|t| match STORAGE.lookup_alias(t) {
                Some(name) => Tag::new(&name, None, TagType::Tag),
                None => Tag::new(t, None, TagType::Tag)
            })
            .chain(Tag::new("novelai_generated", None, TagType::Metadata))
            .collect();
        
        Ok(ElementMetadata {
            src_link: None,
            src_time: None,
            group: Some(meta.seed),
            ai_meta: Some(AIMetadata {
                positive_prompt: prompt.to_string(),
                negative_prompt: Some(meta.uc.to_string()),
                steps: meta.steps,
                scale: meta.scale,
                sampler: meta.sampler.to_owned(),
                seed: meta.seed,
                strength: meta.strength,
                noise: meta.noise
            }),
            tags
        })
    }
}