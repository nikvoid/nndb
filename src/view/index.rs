use crate::{dao::{STORAGE, ElementStorage}, resolve};

use super::*;

use actix_web::{Responder, get, web};
use maud::html;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request<S> where S: AsRef<str> {
    query: Option<S>,
    page: Option<u32>,
}

#[get("/index")]
pub async fn index_page(query: web::Query<Request<String>>) -> impl Responder {
    let elements = STORAGE.lock().await.get_elements(0, 50).unwrap();

    BaseContainer {
        content: Some(html! {
            // TODO: Place page buttons here
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
        }),
        ..Default::default()
    }.render()
}