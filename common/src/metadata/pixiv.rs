use serde_json::Value;

use super::ParsedMeta;

fn string(value: &Value, path: &str) -> Option<String> {
    Some(match value.pointer(path)? {
        Value::String(s) => s.clone(),
        other => other.to_string()
    })
}

fn kv(json: &Value, key: &str, val_path: &str, wide: bool) -> Option<(String, String, bool)> {
    string(json, val_path)
        .filter(|v| !v.is_empty())
        .map(|v| (key.to_string(), v, wide))
}

fn try_parse_meta(ill: Value) -> Option<ParsedMeta> {
    if !ill.is_object() {
        return None
    }
    
    let vec: Vec<_> = [
        kv(&ill, "Username", "/user/name", false),
        kv(&ill, "Account", "/user/account", false),
        kv(&ill, "Title", "/title", false),
        kv(&ill, "Description", "/caption", true),
        kv(&ill, "Type", "/type", false),
        kv(&ill, "Views", "/total_view", false),
        kv(&ill, "Pages", "/page_count", false),
    ]
    .into_iter()
    .flatten()
    .collect();

    if vec.is_empty() {
        return None;
    }
    
    Some(vec)
}

pub fn parse_metadata(raw_meta: &str) -> ParsedMeta {
    // Unfortunately there is no feature to leave only data models in pixivcrab
    let ill: Value = serde_json::from_str(raw_meta).unwrap();
    try_parse_meta(ill).unwrap_or_default()
}

pub fn pretty_raw_meta(raw_meta: &str) -> String {
    let meta: Value = serde_json::from_str(raw_meta).unwrap();

    serde_json::to_string_pretty(&meta).unwrap() 
}
