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
    let ai_meta = line_iter.next().into_iter().flat_map(|other| other
        .split(',')
        .filter_map(|m| m.split(':').collect_tuple())
        .map(|(k, v)| (k.trim(), Cow::Borrowed(v.trim())))
    );
    
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
