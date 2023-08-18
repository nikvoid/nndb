use actix_web::{Responder, get, web::{self, Json}, post};
use anyhow::Context;
use itertools::Itertools;
use serde::Deserialize;
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
    search::{self, Term}, 
    log_n_ok, 
    log_n_bail, 
};

mod convert;
mod macros;
use convert::IntoVec;

/// Tag autocompletion max tags
const TAG_LIMIT: u32 = 15;

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
            let mut associated = vec![];
            // Get associated by signature
            if let Some(group) = element.group_id {
                match STORAGE.search_elements(
                    &format!("group:{group}"), 
                    0, 
                    None, 
                    0
                ).await {
                    Ok((by_sig, ..)) => {
                        associated.push(Associated {
                            key: "Signature".into(),
                            value: group as i64,
                            elements: by_sig.into_vec()
                        })
                    },
                    Err(e) => {
                        log_n_bail!("failed to fetch associted by signature: {e}")
                    }
                }
            }

            // Get associated by external source
            for (fetcher, group) in &meta.ext_groups {
                match STORAGE.search_elements(
                    &format!("extgroup:{group}"), 
                    0, 
                    None, 
                    0
                ).await {
                    Ok((by_ext, ..)) => {
                        associated.push(Associated {
                            key: fetcher.name().into(),
                            value: *group,
                            elements: by_ext.into_vec()
                        })
                    },
                    Err(e) => {
                        log_n_bail!("failed to fetch associted by external source: {e}")
                    }
                }
                
            }
            
            Ok(Some(Json(MetadataResponse {
                element: element.into(),
                metadata: meta.into(),
                associated
            })))
        },
        Ok(None) => Ok(None),
        Err(e) => {
            log_n_bail!("failed to fetch element data: {e}");
        }
    }
}

/// Tag data and aliases
#[get("/v1/tag/{id}")]
pub async fn tag_data(id: web::Path<u32>) -> impl Responder {
    let tag = match STORAGE.get_tag_data_by_id(*id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(Json(None)),
        Err(e) => log_n_bail!("failed to get tag data: {e}")
    };

    let aliases = match STORAGE.get_tag_aliases(&tag.name).await {
        Ok(v) => v,
        Err(e) => log_n_bail!("failed to get tag aliases: {e}")  
    };
    
    Ok(Json(Some(TagResponse {
        tag: tag.into(),
        aliases: aliases.into_vec()
    })))
}

/// Edit tag
#[post("/v1/tag_edit")]
pub async fn tag_edit(req: Json<TagEditRequest>) -> impl Responder {
    let tag = match write::Tag::new(&req.new_name, req.alt_name.clone(), req.tag_type.into()) {
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
        .filter_map(|t| if let Term::Tag(true, tag) = t { Some(tag) } else { None })
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

/// Add tag(s) to element
#[post("/api/write/add_tags")]
pub async fn add_tags(query: Json<AddTagsRequest>) -> impl Responder {    
    let tags = query.tags
        .split_whitespace()
        // New tags will be created with Tag type, existing won't be changed
        .filter_map(|t| write::Tag::new(t, None, TagType::Tag))
        .collect_vec();
    
    match STORAGE.add_tags(Some(query.element_id), &tags).await {
        Ok(_) => log_n_ok!("added tags to element"),
        Err(e) => log_n_bail!("failed to add tags", ?e)        
    }
}

/// Delete tag from element
#[post("/api/write/delete_tag")]
pub async fn delete_tag(req: Json<DeleteTagRequest>) -> impl Responder {
    match STORAGE
        .remove_tag_from_element(req.element_id, &req.tag_name)
        .await {
        Ok(_) => log_n_ok!("removed tag from element"),
        Err(e) => log_n_bail!("failed to remove tag", ?e)
    }
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
                STORAGE.unmark_failed_imports().await,
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
