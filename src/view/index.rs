use super::*;

use actix_web::{Responder, get};
use maud::html;

#[get("/index")]
pub async fn index_page() -> impl Responder {
    BaseContainer {
        content: Some(html! {
            // TODO: Place page buttons here
            .index-main {
                .list-container {
                    // TODO: Place element list here
                }
            }
        }),
        ..Default::default()
    }.render()
}