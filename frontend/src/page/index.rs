use serde::{Serialize, Deserialize};

use crate::{backend_post, component::{element::{ElementList, Route}, paginator::Paginator, metadata::TagList}};

use super::prelude::*;

/// Count of elements displayed on single page
const ELEMENTS_ON_PAGE: u32 = 50;

/// Count of displayed selection tags
const TAGS_ON_PAGE: u32 = 50;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct IndexQuery {
    pub query: Option<String>,
    pub page: Option<u32>
}

#[function_component]
pub fn Index() -> Html {
    let query: IndexQuery = use_location()
        .expect("cannot access location")
        // If location has been found, this should be infallible due to Options
        .query()
        .unwrap();

    let resp = use_state(SearchResponse::default);
    
    {
        let resp = resp.clone();
        use_effect_with_deps(|query| {
            // Pages start from 1
            let page = query.page.unwrap_or(1);
            let req = SearchRequest {
                query: query.query.clone().unwrap_or_default(),
                offset: (page - 1) * ELEMENTS_ON_PAGE,
                limit: ELEMENTS_ON_PAGE,
                tag_limit: TAGS_ON_PAGE
            };

            wasm_bindgen_futures::spawn_local(async move {
                let data = backend_post!(&req, "/v1/search")
                    .await
                    .expect("failed to fetch elements");
                resp.set(data);
            });
        }, query.clone());
    }
    
    let current = query.page.unwrap_or(1);
    let onpage = {
        let nav = use_navigator()
            .expect("failed to access navigator");
        Callback::from(move |new_page| {
            nav.push_with_query(&Route::Index, &IndexQuery {
                page: Some(new_page),
                ..query.clone()
            })
            .expect("failed to push index route");
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
                        {current}
                        {max_page}
                        onclick={onpage.clone()}
                    />
                </div>
                <ElementList content={resp.elements.clone()} />
                <div class="paginator-bottom">
                    <Paginator 
                        {current}
                        {max_page}
                        onclick={onpage}
                        scroll_to_x={0.}
                    />
                </div>
            </div>
        </div>
    }
}
