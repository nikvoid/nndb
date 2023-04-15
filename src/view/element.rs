use actix_web::{Responder, get, web, error::{ErrorNotFound, ErrorInternalServerError}};
use maud::{Render, html};
use tracing::error;

use crate::{dao::{STORAGE, ElementStorage}, view::{BaseContainer, AsideTags, ElementLink, TagEditForm, AsideMetadata, ScriptButton}};

#[get("/element/{id}")]
pub async fn element_page(id: web::Path<u32>) -> impl Responder {
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
    
    let page = BaseContainer {
        after_header: match elem.animated {
            false => Some(html! {
                span.head-span {
                    (ScriptButton("return fullSize(this);", "Full size"))
                }
            }),
            true => None
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
                        img.page-container #element
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