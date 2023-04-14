use actix_web::{Responder, get, web, error::{ErrorNotFound, ErrorInternalServerError}};
use maud::{Render, html};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{dao::{STORAGE, ElementStorage}, view::{BaseContainer, AsideTags, Button, Link, ElementLink, TagEditForm, AsideMetadata}, resolve};

#[derive(Deserialize, Serialize)]
pub struct Request {
    pub full: Option<bool>
}

#[get("/element/{id}")]
pub async fn element_page(id: web::Path<u32>, query: web::Query<Request>) -> impl Responder {
    let id = *id;
    
    let (elem, meta) = match STORAGE
        .lock()
        .await
        .get_element_data(id) {
            Ok(Some(data)) => data,
            Ok(None) => return Err(ErrorNotFound("no such element")),
            Err(e) => {
                error!(?e, "failed to fetch element data");
                return Err(ErrorInternalServerError("failed to fetch element data"));
            }
        };
    let full = query.full.unwrap_or(false);
    let page = BaseContainer {
        after_header: match full {
            true => None,
            false => Some(html! {
                span.head-span {
                    (Button(Link(resolve!(/element/id), &Request { 
                        full: Some(true) 
                    }), "Full size"))
                }
            })
        },
        content: Some(html! {
            .index-main {
                @match elem.animated {
                    true => {
                        video.page-container controls="" loop="" {
                            source src=(ElementLink(&elem));
                        }
                    }
                    false => {
                        img.page-container.page-container-full[full]
                            src=(ElementLink(&elem)) alt="image";
                        // div {} // TODO: ???
                    }
                }
            } 
        }),
        aside: Some(html! {
            (AsideTags(&meta.tags))
            (AsideMetadata(&meta))
            // TODO: Replace GET-query based updates with typescript onclicks
            (TagEditForm("", "add_tag", "Add tag"))
            // TODO: Element group
        }),
        ..Default::default()
    };
    
    Ok(page.render())
}