use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::ParsedMeta;

/// Novel AI image metadata, Comment section
#[derive(Serialize, Deserialize)]
pub struct Metadata<'a> {
    pub steps: u32,
    pub sampler: &'a str,
    pub seed: i64,
    pub strength: Option<f32>,
    pub noise: Option<f32>,
    pub scale: f32,
    pub uc: Cow<'a, str>,
    // This field is merged from Description PNG metadata
    #[serde(default)]
    pub prompt: Cow<'a, str>,
}

fn to_cow<T: ToString>(value: &T) -> Cow<'static, str> {
    Cow::Owned(value.to_string())
}

pub fn parse_metadata(raw_meta: &str) -> ParsedMeta {
    // This meta is coming from server and should be already validated
    // on extracting stage 
    let meta: Metadata = serde_json::from_str(raw_meta)
        .expect("failed to parse metadata");
    
    let mut parsed = vec![
        ("Prompt".into(), meta.prompt, true),
        ("Negative prompt".into(), meta.uc, true),
        ("Steps".into(), to_cow(&meta.steps), false),
        ("CFG Scale".into(), to_cow(&meta.scale), false),
        ("Sampler".into(), Cow::Borrowed(meta.sampler), false),
        ("Seed".into(), to_cow(&meta.seed), false),
    ];

    if let Some(v) = meta.strength {
        parsed.push(("Strength".into(), to_cow(&v), false));
    }
    
    if let Some(v) = meta.noise {
        parsed.push(("Noise".into(), to_cow(&v), false));
    }
    
    parsed
}

pub fn pretty_raw_meta(raw_meta: &str) -> Cow<str> {
    let meta: Metadata = serde_json::from_str(raw_meta)
        .expect("failed to parse metadata");

    Cow::Owned(serde_json::to_string_pretty(&meta).unwrap()) 
}
