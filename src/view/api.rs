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

#[derive(Deserialize)]
pub struct DeleteTagRequest {
    element_id: u32,
    tag_name: String 
}

#[derive(Deserialize)]
pub struct EditTagRequest {
    tag_name: String,
    alt_name: Option<String>,
    tag_type: TagType,
    hidden: bool,
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

#[post("/api/write/delete_tag")]
pub async fn delete_tag(req: web::Json<DeleteTagRequest>) -> impl Responder {
    match STORAGE.lock()
        .await
        .remove_tag_from_element(req.element_id, &req.tag_name) {
        Ok(_) => Ok(""),
        Err(e) => log_n_bail!("failed to remove tag", ?e)
    }
}

#[post("/api/write/edit_tag")]
pub async fn edit_tag(req: web::Json<EditTagRequest>) -> impl Responder {
    let tag = match write::Tag::new(&req.tag_name, req.alt_name.clone(), req.tag_type) {
        Some(tag) => tag,
        None => log_n_bail!("failed to create tag struct", "")
    };

    match STORAGE.lock().await.update_tag(tag, req.hidden) {
        Ok(_) => Ok(""),
        Err(e) => log_n_bail!("failed to update tag", ?e)
    }
}