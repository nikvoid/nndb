use std::{borrow::Cow, iter::once};

use itertools::Itertools;

use crate::ParsedMeta;

/// Iterate through webui metadata parameters
///
/// Layout:
/// '''
/// <prompt>
/// ...
/// Negative prompt: <neg_prompt>
/// ...
/// Steps: <steps>, Sampler: <sampler>, CFG Scale: ...
/// '''
pub fn iter_metadata(raw: &str) -> impl Iterator<Item = (&str, Cow<'_, str>)> {
    let mut line_iter = raw.lines().peekable();
    
    let prompt = line_iter
        .peeking_take_while(|l| !l.starts_with("Negative prompt:"))
        .join(" ");
    
    let neg_prompt = line_iter
        .peeking_take_while(|l| !l.starts_with("Steps")) 
        .map(|l| l.trim_start_matches("Negative prompt: "))
        .join(" ");
    
    // Parse other metadata
    let ai_meta = line_iter.next().into_iter().flat_map(|other| {
        let mut param_slice = other;
        
        std::iter::from_fn(move || {
            let (k, rem) = param_slice.split_once(':')?;
            let rem = rem.trim();
            
            // Complex parameter with commas inside
            let v = if rem.starts_with('"') {
                // Find next quote
                let pos = rem.strip_prefix('"')?.find('"')? + 1;
                param_slice = &rem[pos..];
                
                // Return stripped complex value
                &rem[1..pos - 1]
            } else {
                let pos = rem.find(',').unwrap_or(rem.len());
                param_slice = &rem[pos..];

                &rem[..pos]
            };

            // Consume comma
            if let Some(comma) = param_slice.find(',') {
                param_slice = &param_slice[comma + 1..];   
            }
            
            Some((k.trim(), Cow::Borrowed(v.trim())))
        })
    });
    
    once(("Prompt", Cow::Owned(prompt)))
        .chain(once(("Negative prompt", Cow::Owned(neg_prompt))))
        .chain(ai_meta)
}

pub fn parse_metadata(raw_meta: &str) -> ParsedMeta {
    iter_metadata(raw_meta)
        .map(|(k, v)| {
            let wide = matches!(k, "Prompt" | "Negative prompt");
            (Cow::Borrowed(k), v, wide)
        })
        .collect()
}
