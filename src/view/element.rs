use actix_web::{Responder, get, web, error::ErrorNotFound};
use maud::{Render, html};
use tracing::error;

use crate::{
    dao::STORAGE, 
    view::{
        BaseContainer, AsideTags, ElementLink, 
        TagEditForm, AsideMetadata, ScriptButton, 
        ElementListContainer, ScriptVar
    }, log_n_bail};

#[get("/element/{id}")]
pub async fn element_page(id: web::Path<u32>) -> impl Responder {
    let id = *id;

    let (elem, meta) = match STORAGE
        .get_element_data(id)
        .await {
            Ok(Some(data)) => data,
            Ok(None) => return Err(ErrorNotFound("no such element")),
            Err(e) => log_n_bail!("failed to fetch element data", ?e),
        };
    
    let associated = match elem.group_id {
        Some(group_id) => { 
            let res = STORAGE
                .search_elements(&format!("group:{}", group_id), 0, None, 0)
                .await
                .map(|(res, ..)| res);
            
            Some(res)
        }
        None => None,
    };
    
    let associated_ext = match elem.group {
        Some(group_id) => { 
            let res = STORAGE
                .search_elements(&format!("extgroup:{}", group_id), 0, None, 0)
                .await
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
    
    let associated_ext = match associated_ext.transpose() {
        Ok(elems) => elems,
        Err(e) => {
            error!(?e, "failed to fetch external element group");
            None
        }
    };

    let assoc = associated.iter()
        .chain(associated_ext.iter())
        .flatten();

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
                    }
                }
            } 
            @if assoc.clone().count() > 0 {
                .index-side { 
                    @for e in assoc {
                        (ElementListContainer(e))
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