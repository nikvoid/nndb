use actix_web::{get, Responder};
use maud::{Render, html};

use crate::{view::{BaseContainer, BlockParam, ScriptButton}, dao::{STORAGE, ElementStorage}, log_n_bail, html_in};


#[get("/dashboard")]
pub async fn dashboard_page() -> impl Responder {
    let summary = match STORAGE.lock().await.get_summary() {
        Ok(sum) => sum,
        Err(e) => log_n_bail!("failed to fetch summary", ?e)
    };
    
    let page = BaseContainer {
        aside: Some(html! {
            .tag { "Summary" }
            (BlockParam("", html_in! { "Tags: " (summary.tag_count) })) 
            (BlockParam("", html_in! { "Elements: " (summary.element_count) })) 

            // TODO
            .tag { "Maintenance" }
            (ScriptButton("", "Update tag counts"))
            (ScriptButton("", "Clear element group data"))
            (ScriptButton("", "Fix thumbnails"))
            (ScriptButton("", "Retry imports"))

            // TODO
            .tag { "Manual import" }
            (ScriptButton("", "Start import manually"))
        }),
        content: Some(html! {
            .log-window-outline {
                // textarea.log-window-inner.code #log-window readonly disabled wrap="off" {} 
                pre.log-window-inner.code #log-window readonly disabled wrap="off" {} 
            }
        }),
        ..Default::default()
    };

    Ok(page.render())
}