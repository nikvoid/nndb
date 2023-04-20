use actix_web::{Responder, get, web, post};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::{dao::{STORAGE, ElementStorage}, log_n_bail, model::{write, TagType}, util, service::{SCAN_FILES_LOCK, UPDATE_METADATA_LOCK, GROUP_ELEMENTS_LOCK, MAKE_THUMBNAILS_LOCK, self}, log_n_ok};

/// Tag autocompletion max tags
const TAG_LIMIT: u32 = 15;

/// Max size of log tail to send
const LOG_TAIL_SIZE: usize = 20_000;

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

#[derive(Serialize)]
pub struct ImportTasksStatus {
    scan_files: bool,
    update_metadata: bool,
    group_elements: bool,
    make_thumbnails: bool
}

/// Tag autocompletion
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

/// Get recent log
#[get("/api/read/log")]
pub async fn read_log() -> impl Responder {    
    
    let mut buf = vec![0; LOG_TAIL_SIZE];
    match util::get_log_tail(&mut buf).await {
        Ok(len) => {
            buf.truncate(len);
            Ok(buf)
        },
        Err(e) => log_n_bail!("failed to fetch log", ?e)
    }    
}

/// Get import tasks status 
#[get("/api/read/import")]
pub async fn import_status() -> impl Responder {
    let status = ImportTasksStatus {
        scan_files: SCAN_FILES_LOCK.inspect(),
        update_metadata: UPDATE_METADATA_LOCK.inspect(),
        group_elements: GROUP_ELEMENTS_LOCK.inspect(),
        make_thumbnails: MAKE_THUMBNAILS_LOCK.inspect()
    };

    web::Json(status)
}

/// Add tag(s) to element
#[post("/api/write/add_tags")]
pub async fn add_tags(query: web::Json<AddTagsRequest>) -> impl Responder {    
    let tags = query.tags
        .split_whitespace()
        // New tags will be created with Tag type, existing won't be changed
        .filter_map(|t| write::Tag::new(&t, None, TagType::Tag))
        .collect_vec();
    
    match STORAGE.lock().await.add_tags(query.element_id, &tags) {
        Ok(_) => log_n_ok!("added tags to element"),
        Err(e) => log_n_bail!("failed to add tags", ?e)        
    }
}

/// Delete tag from element
#[post("/api/write/delete_tag")]
pub async fn delete_tag(req: web::Json<DeleteTagRequest>) -> impl Responder {
    match STORAGE.lock()
        .await
        .remove_tag_from_element(req.element_id, &req.tag_name) {
        Ok(_) => log_n_ok!("removed tag from element"),
        Err(e) => log_n_bail!("failed to remove tag", ?e)
    }
}

/// Edit tag
#[post("/api/write/edit_tag")]
pub async fn edit_tag(req: web::Json<EditTagRequest>) -> impl Responder {
    let tag = match write::Tag::new(&req.tag_name, req.alt_name.clone(), req.tag_type) {
        Some(tag) => tag,
        None => log_n_bail!("failed to create tag struct")
    };

    match STORAGE.lock().await.update_tag(tag, req.hidden) {
        Ok(_) => log_n_ok!("edited tag"),
        Err(e) => log_n_bail!("failed to update tag", ?e)
    }
}

/// Manually start import task in strict sequence
#[get("/api/write/start_import")]
pub async fn start_import() -> impl Responder {
    info!("Starting manual import");
    tokio::spawn(async {
        match service::manual_import().await {
            Ok(_) => info!("Manual import finished"),
            Err(e) => error!(?e, "Manual import failed")
        }
    });
    
    ""
}

/// Update count of elements with tag
#[get("/api/write/update_tag_counts")]
pub async fn update_tag_count() -> impl Responder {
    match STORAGE.lock().await.update_tag_count() {
        Ok(_) => log_n_ok!("updated tag counts (manually)"),
        Err(e) => log_n_bail!("failed to update tag counts", ?e)
    }
}

/// Remove all internal grouping data
#[get("/api/write/clear_group_data")]
pub async fn clear_group_data() -> impl Responder {
    match STORAGE.lock().await.clear_groups() {
        Ok(_) => log_n_ok!("cleared group data"),
        Err(e) => log_n_bail!("failed to clear groups", ?e)
    }
}

/// Scan thumbnails folder and mark elements without thumbnail
#[get("/api/write/fix_thumbnails")]
pub async fn fix_thumbnails() -> impl Responder {
    match tokio::task::spawn_blocking(service::fix_thumbnails).await {
        Ok(_) => log_n_ok!("fixed thumbnails"),
        Err(e) => log_n_bail!("failed to fix thumbnails", ?e)
    }
}

/// Retry failed imports
#[get("/api/write/retry_imports")]
pub async fn retry_imports() -> impl Responder {
    match STORAGE.lock().await.unmark_failed_imports() {
        Ok(_) => log_n_ok!("cleared failed imports state"),
        Err(e) => log_n_bail!("failed to retry imports", ?e)
    }
}