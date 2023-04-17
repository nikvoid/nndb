use actix_web::{Responder, get, web, post};
use itertools::Itertools;
use serde::Deserialize;

use crate::{dao::{STORAGE, ElementStorage}, log_n_bail, model::{write, TagType}};

const TAG_LIMIT: u32 = 15;

#[derive(Deserialize)]
pub struct AutocompleteRequest {
    input: String
}

#[derive(Deserialize)]
pub struct AddTagsRequest {
    element_id: u32,
    tags: String,
}

#[get("/api/read/autocomplete")]
pub async fn tag_autocomplete(query: web::Query<AutocompleteRequest>) -> impl Responder {
    match STORAGE.lock().await.get_tag_completions(query.0.input, TAG_LIMIT) {
        Ok(res) => {
            Ok(web::Json(res))
        },
        Err(e) => {
            log_n_bail!("failed to complete tag", ?e);
        }
    }
}

#[post("/api/write/add_tags")]
pub async fn add_tags(query: web::Json<AddTagsRequest>) -> impl Responder {    
    let tags = query.tags
        .split_whitespace()
        // New tags will be created with Tag type, existing won't be changed
        .filter_map(|t| write::Tag::new(&t, None, TagType::Tag))
        .collect_vec();
    
    match STORAGE.lock().await.add_tags(query.element_id, &tags) {
        Ok(_) => Ok(""),
        Err(e) => log_n_bail!("failed to add tags", ?e)        
    }
}