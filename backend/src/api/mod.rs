use actix_web::{Responder, get, web::{self, Json}, post};
use itertools::Itertools;
use nndb_common::*;
use tracing::{info, error};

use crate::{
    dao::STORAGE, 
    model::{write, TagType}, 
    util, 
    service::{
        SCAN_FILES_LOCK, UPDATE_METADATA_LOCK, GROUP_ELEMENTS_LOCK, 
        MAKE_THUMBNAILS_LOCK, self, FETCH_WIKI_LOCK
    }, 
    log_n_ok, 
    log_n_bail, 
};

mod convert;
mod macros;
use convert::IntoVec;

/// Tag autocompletion max tags
const TAG_LIMIT: u32 = 15;

/// Element search
#[post("/v1/search")]
pub async fn search_elements(Json(req): Json<SearchRequest>) -> impl Responder {
    match STORAGE
        .search_elements(&req.query, req.offset, Some(req.limit), req.tag_limit)
        .await {
        Ok((elems, tags, count)) => {
            Ok(Json(SearchResponse {
                elements: elems.into_vec(),
                tags: tags.into_vec(),
                count
            }))
        },
        Err(e) => {
            log_n_bail!("failed to perform search", ?e);
        }
    }
}

/// Tag autocompletion
#[post("/v1/autocomplete")]
pub async fn tag_autocomplete(query: web::Json<AutocompleteRequest>) -> impl Responder {
    match STORAGE.get_tag_completions(&query.0.input, TAG_LIMIT).await {
        Ok(res) => {
            Ok(Json(AutocompleteResponse {
                completions: res.into_vec()
            }))
        },
        Err(e) => {
            log_n_bail!("failed to complete tag", ?e);
        }
    }
}

/// Element data, metadata and associated
#[get("/v1/element/{id}")]
pub async fn element(id: web::Path<u32>) -> impl Responder {
    match STORAGE.get_element_data(*id).await {
        Ok(Some((element, meta))) => {
            let associated = match STORAGE.get_associated_elements(*id).await {
                Ok(v) => v,
                Err(e) => log_n_bail!("failed to fetch associated elements", ?e)  
            };
            
            Ok(Some(Json(MetadataResponse {
                element: element.into(),
                metadata: meta,
                associated: associated.into_vec()
            })))
        },
        Ok(None) => Ok(None),
        Err(e) => {
            log_n_bail!("failed to fetch element data", ?e);
        }
    }
}

/// Tag data and aliases
#[get("/v1/tag/{id}")]
pub async fn tag_data(id: web::Path<u32>) -> impl Responder {
    let tag = match STORAGE.get_tag_data_by_id(*id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(Json(None)),
        Err(e) => log_n_bail!("failed to get tag data", ?e)
    };

    let aliases = match STORAGE.get_tag_aliases(&tag.name).await {
        Ok(v) => v,
        Err(e) => log_n_bail!("failed to get tag aliases", ?e)  
    };
    
    Ok(Json(Some(TagResponse {
        tag,
        aliases
    })))
}

/// Edit tag
#[post("/v1/tag_edit")]
pub async fn tag_edit(req: Json<TagEditRequest>) -> impl Responder {
    let tag = match write::Tag::new(&req.new_name, req.alt_name.clone(), req.tag_type) {
        Some(tag) => tag,
        None => log_n_bail!("failed to create tag struct")
    };

    match STORAGE.update_tag(&req.tag_name, &tag, req.hidden).await {
        Ok(_) => log_n_ok!("edited tag"),
        Err(e) => log_n_bail!("failed to update tag", ?e)
    }
}

/// Alias tag
#[post("/v1/tag_alias")]
pub async fn tag_alias(req: Json<TagAliasRequest>) -> impl Responder {
    match search::parse_query(&req.query)
        .filter_map(|t| if let search::Term::Tag(true, tag) = t { Some(tag) } else { None })
        .next() {
        Some(to) => match STORAGE
            .alias_tag(&req.tag_name, to)
            .await {
            Ok(_) => log_n_ok!("aliased tag", tag=req.tag_name, to),
            Err(e) => log_n_bail!("failed to make alias", ?e)
        }
        None => log_n_bail!("tag definition not found"),
    }
}

/// Get recent log
#[post("/v1/log")]
pub async fn read_log(req: Json<LogRequest>) -> impl Responder {    
    let mut buf = Vec::with_capacity(req.read_size as _);
    match util::get_log_tail(&mut buf, req.read_size as _).await {
        Ok(_) => {
            let data = String::from_utf8_lossy(&buf).into_owned();
            Ok(Json(LogResponse{ data }))
        },
        Err(e) => log_n_bail!("failed to fetch log", ?e)
    }    
}

/// Get import tasks status 
#[get("/v1/status")]
pub async fn import_status() -> impl Responder {
    let status = StatusResponse {
        scan_files: SCAN_FILES_LOCK.state(),
        update_metadata: UPDATE_METADATA_LOCK.state(),
        group_elements: GROUP_ELEMENTS_LOCK.state(),
        make_thumbnails: MAKE_THUMBNAILS_LOCK.state(),
        wiki_fetch: FETCH_WIKI_LOCK.state(),
    };

    Json(status)
}

/// Add and remove tag(s) to element
#[post("/v1/tags_edit")]
pub async fn tags_edit(Json(req): Json<EditTagsRequest>) -> impl Responder {    
    let add_tags = req.add
        .iter()
        // New tags will be created with Tag type, existing won't be changed
        .filter_map(|t| write::Tag::new(t, None, TagType::Tag))
        .collect_vec();
    
    if let Err(e) = STORAGE.add_tags(Some(req.element_id), &add_tags).await {
        log_n_bail!("failed to add tags", ?e);      
    }

    for tag in &req.remove {
        if let Err(e) = STORAGE.remove_tag_from_element(req.element_id, tag).await {
            log_n_bail!("failed to remove tag", ?e);
        }
    }

    info!("Changed element tags:\n  added: {:?}\n  removed: {:?}", req.add, req.remove);

    Ok("null")
}

/// Joined backend control endpoint
#[post("/v1/control")]
pub async fn control(Json(req): Json<ControlRequest>) -> impl Responder {
    tokio::spawn(async move {
        info!("Processing control request {req:?}");
        let res = match req {
            ControlRequest::StartImport =>
                service::manual_import().await,
            ControlRequest::UpdateTagCount => 
                STORAGE.update_tag_count().await,
            ControlRequest::ClearGroupData => 
                STORAGE.clear_groups().await,
            ControlRequest::FixThumbnails => 
                match tokio::task::spawn_blocking(service::fix_thumbnails)
                    .await {
                Ok(res) => res,
                // Convert join error
                Err(e) => Err(e.into())
            },
            ControlRequest::RetryImports => 
                match STORAGE.unmark_failed_imports().await {
                    e @ Err(_) => e,
                    Ok(_) => service::update_metadata().await
                },
            ControlRequest::FetchWikis => 
                service::update_danbooru_wikis().await,
        };

        match res {
            Ok(_) => info!("Control request {req:?} finished successfully"),
            Err(e) => error!("Control request {req:?} failed: {e}")
        }
    });

    "null"
}

#[get("/v1/summary")]
pub async fn summary() -> impl Responder {
    match STORAGE.get_summary().await {
        Ok(summary) => Ok(Json(SummaryResponse {
            summary
        })),
        Err(e) => log_n_bail!("failed to get DB summary", ?e)
    }
}
