use std::ops::RangeInclusive;

use crate::{dao::{STORAGE, ElementStorage}, resolve};

use super::*;

use actix_web::{Responder, get, web, error::ErrorInternalServerError};
use maud::html;
use serde::{Deserialize, Serialize};
use tracing::error;

const ELEMENTS_ON_PAGE: u32 = 50;
const PAGES_LOOKAROUND: u32 = 5;
/// Count of tags for element selection, that will be shown 
/// in [AsideTags] 
const SELECTION_TAGS_COUNT: u32 = 50;

/// Get range of pages (buttons) to display
fn get_pages(max_page: u32, current: u32) -> RangeInclusive<u32> {
    let left = current.saturating_sub(PAGES_LOOKAROUND).clamp(1, max_page);
    let right = (current + PAGES_LOOKAROUND).clamp(1, max_page);
    left..=right
}

/// Page buttons row (maxpage, current, query)
struct PageButtons<'a>(u32, u32, Option<&'a str>);
impl Render for PageButtons<'_> {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            .button-h-list-container {
                @for page in get_pages(self.0, self.1) {
                    a.button.button-large.selected[page == self.1]
                        href=(Link(resolve!(/index), &Request { 
                            query: self.2,
                            page: Some(page) 
                        })) {
                        (page)
                    }
                }
            }
        }
    }
} 

// Get request for index (element list) page
#[derive(Serialize, Deserialize)]
pub struct Request<S> where S: AsRef<str> {
    pub query: Option<S>,
    pub page: Option<u32>,
}

#[get("/index")]
pub async fn index_page(query: web::Query<Request<String>>) -> impl Responder {
    let page = match query.0.page {
        Some(page) => page,
        None => 1
    };

    let query_str = match &query.0.query {
        Some(q) => q,
        None => ""
    };

    let offset = (page - 1) * ELEMENTS_ON_PAGE;
    let (elements, tags, count) = match STORAGE.lock()
        .await
        .search_elements(
            query_str,
            offset, 
            ELEMENTS_ON_PAGE, 
            SELECTION_TAGS_COUNT
        ) {
            Ok(out) => out,
            Err(e) => {
                error!(?e, "failed to perform search");
                return Err(ErrorInternalServerError("failed to perform search"))
            }
        };
    let maxpage = (count / ELEMENTS_ON_PAGE) + 1;
    
    let answ = BaseContainer {
        content: Some(html! {
            (PageButtons(maxpage, page, query.0.query.as_deref()))
            .index-main {
                // FIXME: Fix typo in class
                .list-constainer {
                    @for e in elements {
                        .image-container-list {
                            a href={ 
                                (Link(resolve!(/element/e.id), &query.0)) 
                            } {  
                                // TODO: Thumbnails, error handling, animation
                                img.def-img.image-list-element src=(ElementLink(&e.filename))
                                ;
                            }
                        }
                    }
                }
            }
            (PageButtons(maxpage, page, query.0.query.as_deref()))
        }),
        aside: Some(AsideTags(&tags).render()),
        query: query_str, 
        ..Default::default()
    }.render();

    Ok(answ)
}