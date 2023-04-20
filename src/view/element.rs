use actix_web::{Responder, get, web, error::ErrorNotFound};
use maud::{Render, html};
use tracing::error;

use crate::{
    dao::{STORAGE, ElementStorage}, 
    view::{
        BaseContainer, AsideTags, ElementLink, 
        TagEditForm, AsideMetadata, ScriptButton, 
        ElementListContainer, ScriptVar
    }, log_n_bail};

#[get("/element/{id}")]
pub async fn element_page(id: web::Path<u32>) -> impl Responder {
    let id = *id;

    let (elem, meta) = match STORAGE
        .lock()
        .await
        .get_element_data(id) {
            Ok(Some(data)) => data,
            Ok(None) => return Err(ErrorNotFound("no such element")),
            Err(e) => log_n_bail!("failed to fetch element data", ?e),
        };
    
    let associated = match elem.group_id {
        Some(group_id) => { 
            let res = STORAGE
                .lock()
                .await
                .search_elements(format!("group:{}", group_id), 0, None, 0)
                .map(|(res, ..)| res);
            
            Some(res)
        }
        None => None,
    };

    let associated = match associated.transpose() {
        Ok(elems) => elems,
        Err(e) => {
            error!(?e, "failed to fetch element group");
            None
        }
    };
    
    let page = BaseContainer {
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
            @if let Some(elems) = associated {
                .index-side { 
                    @for e in elems {
                        (ElementListContainer(&e))
                    } 
                }
            }
        }),
        aside: Some(html! {
            @if !elem.animated {
                (ScriptButton("fullSize(this)", "Full size"))
            }
            (AsideTags(&meta.tags, Some(&elem)))
            (AsideMetadata(&meta))
            (ScriptVar("ELEMENT_ID", elem.id))
            (TagEditForm(
                "addTagOnSubmit(event, this, ELEMENT_ID)", 
                "add_tag_compl", 
                "Add tag"
            ))
        }),
        ..Default::default()
    };
    
    Ok(page.render())
}