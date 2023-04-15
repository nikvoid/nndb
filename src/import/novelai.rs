use std::{io::Cursor, borrow::Cow};

use anyhow::{bail, Context};
use async_trait::async_trait;
use serde::Deserialize;

use crate::model::{read::PendingImport, write::{ElementMetadata, Tag}, TagType, AIMetadata};

use super::{MetadataImporter, ElementPrefab};

pub struct NovelAI;

#[derive(Deserialize)]
struct Metadata<'a> {
    steps: u32,
    sampler: &'a str,
    seed: i64,
    strength: f32,
    noise: f32,
    scale: f32,
    uc: &'a str
}

/// NovelAI Tag syntax
/// Can contain multiple tags in one clause
enum NovelAITag<'a> {
    Simple(&'a str),
    Mixed(Vec<&'a str>)
}

impl<'a> NovelAITag<'a> {
    fn parse(expr: &'a str) -> Option<Self> {
        // Strip whitespaces
        let trim = expr.trim();

        // Get count of strength control braces tag wrapped into
        let braces = trim
            .chars()
            .zip(trim.chars().rev())
            .take_while(|&(s, e)| match (s, e) {
                | ('{', '}') 
                | ('[', ']')
                | ('(', ')') => true,
                _ => false
            })
            .count();

        let body = trim.get(braces..trim.len() - braces)?;

        if body.is_empty() {
            return None
        }
                
        if body.contains("|") {
            let opt: Option<Vec<_>> = body
                .split("|")
                // Strip weights
                .map(|t| t.split(':').next())
                .collect();

            opt.map(|tags| NovelAITag::Mixed(tags))
        } else {
            Some(NovelAITag::Simple(body))
        }
    }
}

#[async_trait]
impl MetadataImporter for NovelAI {
    /// Check if png contains `Software = NovelAI`
    fn can_parse(&self, element: &ElementPrefab) -> bool {
        // PNG header
        if !element.data.starts_with(
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        ) {
            return false
        }
        
        let mut cursor = Cursor::new(&element.data);
        let dec = png::Decoder::new(&mut cursor);
        if let Ok(reader) = dec.read_info() {
            for entry in &reader.info().uncompressed_latin1_text {
                match (entry.keyword.as_str(), entry.text.as_str()) {
                    ("Software", "NovelAI") => return true,
                    _ => ()
                } 
            }
        }
        false
    }

    fn can_parse_in_place(&self) -> bool { true }

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
            
        let meta: Metadata = serde_json::from_str(&others)?;

        // This is mostly heuristic... assume that tags are split by commas
        let nai_tags = prompt
            .split(",")
            // Just ignore malformed tags
            .filter_map(|tag| NovelAITag::parse(tag));

        let mut tags = vec![Tag::new(
            "novelai_generated",
            None,
            TagType::Metadata
        ).unwrap()];

        // Push all tags
        for tag in nai_tags {
            match tag {
                NovelAITag::Simple(tag) => tags.extend(
                    Tag::new(tag, None, TagType::Tag).into_iter()
                ),
                NovelAITag::Mixed(m_tags) => tags.extend(m_tags.iter().map(
                    |tag| Tag::new(tag, None, TagType::Tag)
                ).flatten())
            }
        }

        Ok(ElementMetadata {
            src_link: None,
            src_time: None,
            group: None,
            ai_meta: Some(AIMetadata {
                positive_prompt: prompt.to_string(),
                negative_prompt: Some(meta.uc.to_owned()),
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

    /// Fetch metadata for pending import (network access implied)
    async fn fetch_metadata(
        &self,
        _: &PendingImport
    ) -> anyhow::Result<ElementMetadata> {
        bail!("unimplemented")
    }
}