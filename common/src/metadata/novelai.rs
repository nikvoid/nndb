use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::ParsedMeta;

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

fn kv(key: &str, val: impl ToString, wide: bool) -> (String, String, bool) {
    (key.to_string(), val.to_string(), wide)
}

pub fn parse_metadata(raw_meta: &str) -> ParsedMeta {
    // This meta is coming from server and should be already validated
    // on extracting stage 
    let meta: Metadata = serde_json::from_str(raw_meta)
        .expect("failed to parse metadata");
    
    let mut parsed = vec![
        kv("Prompt", meta.prompt, true),
        kv("Negative prompt", meta.uc, true),
        kv("Steps", meta.steps, false),
        kv("CFG Scale", meta.scale, false),
        kv("Sampler", meta.sampler, false),
        kv("Seed", meta.seed, false),
    ];

    if let Some(v) = meta.strength {
        parsed.push(kv("Strength", v, false));
    }
    
    if let Some(v) = meta.noise {
        parsed.push(kv("Noise", v, false));
    }
    
    parsed
}

pub fn pretty_raw_meta(raw_meta: &str) -> String {
    let meta: Metadata = serde_json::from_str(raw_meta)
        .expect("failed to parse metadata");

    serde_json::to_string_pretty(&meta).unwrap() 
}
