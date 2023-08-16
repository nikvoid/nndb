use crate::{backend_post, component::{element::ElementList, paginator::Paginator, metadata::TagList}, app::QueryContext};

use super::prelude::*;

/// Count of elements displayed on single page
const ELEMENTS_ON_PAGE: u32 = 50;

/// Count of displayed selection tags
const TAGS_ON_PAGE: u32 = 50;

#[function_component]
pub fn Index() -> Html {
    let query = use_context::<QueryContext>()
        .expect("query context")
        .query;

    // Pages start from 1
    let page = use_state(|| 1);
    let resp = use_state(SearchResponse::default);
    
    {
        let resp = resp.clone();
        let page = page.clone();
        let query = query.clone();
        use_effect_with_deps(|(query, page)| {
            let req = SearchRequest {
                query: query.clone(),
                offset: (**page - 1) * ELEMENTS_ON_PAGE,
                limit: ELEMENTS_ON_PAGE,
                tag_limit: TAGS_ON_PAGE
            };

            wasm_bindgen_futures::spawn_local(async move {
                let data = backend_post!(&req, "/v1/search")
                    .await
                    .unwrap();
                resp.set(data);
            });
        }, (query, page));
    }
    
    let onpage = {
        let page = page.clone();
        Callback::from(move |new_page| {
            page.set(new_page);
        })
    };

    let max_page = resp.count / ELEMENTS_ON_PAGE + 1;   
    html! {
        <div class="index-page">
            <div class="metadata">
                <div class="element-count">
                    { "Elements found: " } { resp.count }
                </div>
                <TagList content={resp.tags.clone()}/>
            </div>
            <div class="elements">
                <div class="paginator-top">
                    <Paginator 
                        current={*page}
                        {max_page}
                        onclick={onpage.clone()}
                    />
                </div>
                <ElementList content={resp.elements.clone()} />
                <div class="paginator-bottom">
                    <Paginator 
                        current={*page}
                        {max_page}
                        onclick={onpage}
                    />
                </div>
            </div>
        </div>
    }
}
