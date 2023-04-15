use actix_web::{Responder, get, web, error::ErrorInternalServerError};
use serde::Deserialize;
use tracing::error;

use crate::dao::{STORAGE, ElementStorage};

const TAG_LIMIT: u32 = 15;

#[derive(Deserialize)]
pub struct Request {
    input: String
}

#[get("/api/autocomplete")]
pub async fn tag_autocomplete(query: web::Query<Request>) -> impl Responder {
    match STORAGE.lock().await.get_tag_completions(query.0.input, TAG_LIMIT) {
        Ok(res) => {
            Ok(web::Json(res))
        },
        Err(e) => {
            error!(?e, "failed to complete tag");
            Err(ErrorInternalServerError("failed to complete tag"))
        }
    }
}