use actix_web::{get, Responder};
use maud::{Render, html, html_to};

use crate::{view::{BaseContainer, ScriptButton}, dao::STORAGE, log_n_bail};

/// Takes (id, param_name, init_text) to create block with
/// concatenated param_name and init_text in span with id
/// `(param_name <span id=id>init_text</span>)`
struct IdParam<'a, R>(&'a str, &'a str, R);
impl<R> Render for IdParam<'_, R>
where R: Render {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            .tag-container-grid {
                .tag.tag-block { (self.1) span #(self.0.to_lowercase()) {(self.2)} }
            }
        }

    }
}

#[get("/dashboard")]
pub async fn dashboard_page() -> impl Responder {
    let summary = match STORAGE.get_summary().await {
        Ok(sum) => sum,
        Err(e) => log_n_bail!("failed to fetch summary", ?e)
    };
    
    let page = BaseContainer {
        aside: Some(html! {
            .tag { "Summary" }
            .tag.tag-block { "Tags: " (summary.tag_count) } 
            .tag.tag-block { "Elements: " (summary.element_count) }

            .tag { "Maintenance" }
            (ScriptButton("updateTagCountsOnClick()", "Update tag counts"))
            (ScriptButton("clearGroupsOnClick()", "Clear element group data"))
            (ScriptButton("fixThumbsOnClick()", "Fix thumbnails"))
            (ScriptButton("retryImportsOnClick()", "Retry imports"))
            (ScriptButton("fetchWikisOnClick()", "Fetch danbooru wikis"))

            .tag { "Import" }
            (IdParam("scan-files", "Scan files running: ", "unknown"))
            (IdParam("update-meta", "Update metadata running: ", "unknown"))
            (IdParam("group-elems", "Group elements running: ", "unknown"))
            (IdParam("make-thumbs", "Make thumbnails running: ", "unknown"))
            (IdParam("fetch-wikis", "Wikis are fetching: ", "unknown"))
            (ScriptButton("importBtnOnClick()", "Start import manually"))
        }),
        content: Some(html! {
            .index-main {
                .log-window-outline {
                    pre.log-window-inner.code #log-window readonly disabled wrap="off" {} 
                }
            }
        }),
        ..Default::default()
    };

    Ok(page.render())
}