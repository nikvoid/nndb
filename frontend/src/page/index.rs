use crate::{backend_post, component::element::ElementList};

use super::prelude::*;

#[function_component]
pub fn Index() -> Html {
    let resp = use_state(SearchResponse::default);
    
    {
        let resp = resp.clone();
        let req = SearchRequest {
            limit: 50,
            ..Default::default()
        };
        use_effect_with_deps(|_| {
            wasm_bindgen_futures::spawn_local(async move {
                let data = backend_post!(&req, "/v1/search")
                    .await
                    .unwrap();
                resp.set(data)
            });
        }, ())
    }
    
    html! {
        <main class="index-page">
            <ElementList content={resp.elements.clone()} />
        </main>
    }
}
