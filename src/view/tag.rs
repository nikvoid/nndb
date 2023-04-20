use actix_web::{get, Responder, web, error::ErrorNotFound};
use enum_iterator::all;
use maud::{Render, html_to, html};
use serde::{Deserialize, Serialize};

use crate::{
    model::{read::Tag, TagType}, 
    view::{BaseContainer, ScriptButton, ScriptVar}, 
    dao::{STORAGE, ElementStorage}, 
    log_n_bail, html_in
};

pub struct TagInfo<'a>(&'a Tag);
impl Render for TagInfo<'_> {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            .tag { "Tag info" }
            .tag.tag-block { "Name: " (&self.0.name) }
            @if let Some(alt) = &self.0.alt_name {
                .tag.tag-block { "Name alias: " (alt) }
            }
            .tag.tag-block { "Type: " (self.0.tag_type.label()) }
            .tag.tag-block { "Hidden: " (self.0.hidden) }
            .tag.tag-block { "Images with this tag: " (self.0.count) }
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Request {
    pub element_ref: Option<u32>
}

#[get("/tag/{name}")]
pub async fn tag_page(name: web::Path<String>, query: web::Query<Request>) -> impl Responder {

    let tag = match STORAGE.lock().await.get_tag_data(&*name) {
        Ok(Some(tag)) => tag,
        Ok(None) => return Err(ErrorNotFound("no such tag")),
        Err(e) => log_n_bail!("failed to get tag data", ?e),
    };

    let content = BaseContainer {
        content: Some(html! {
            .index-main {
                (ScriptVar("TAG_NAME", &*tag.name))
                form onsubmit={
                    "editTagOnClick(event, this, TAG_NAME)"
                } {
                    "tag type"
                    br;
                    select.set-type {
                        @for typ in all::<TagType>() {
                            option value=(typ.label()) 
                                selected[tag.tag_type == typ] {
                                (typ.label())
                            }
                        }
                    }
                    br;
                    input.alt-name type="text" 
                        value=(tag.alt_name.as_deref().unwrap_or_default());
                    br;
                    input.is-hidden type="checkbox" checked[tag.hidden]; "  hidden"
                    br;
                    input type="submit" value="Change tag";                
                }                
                @if let Some(ref_elem) = query.element_ref {
                    (ScriptVar("ELEMENT_ID", ref_elem))
                    div style="margin-top: 10px" {
                        (ScriptButton(
                            "deleteTagOnClick(ELEMENT_ID, TAG_NAME)",
                            "Delete tag from image"
                        ))
                    }
                }
            }
        }),
        aside: Some(TagInfo(&tag).render()),
        ..Default::default() 
    };

    Ok(content.render())
}