use crate::component::prelude::*;
use crate::component::input::{Completion, InputAutocomplete};
use crate::component::link::AppLink;
use crate::page::index::Index;

pub fn switch(route: Route) -> Html {   
    match route {
        Route::Index => html! { <Index /> },
        // Route::Dashboard => todo!(),
        // Route::Element { id } => todo!(),
        // Route::Tag { id } => todo!(),
        _ => html! {
            <div class="label">{ "Not Found" }</div>
        }
    }
}

/// A shim to access location inside BrowserRouter -- and page root
#[function_component]
fn Root() -> Html {
    let nav = use_navigator().unwrap();
    let query = use_search_query().query;
    web_sys::console::log_1(&query.as_str().into());

    // On submit change context and push index page
    let onsubmit = {
        Callback::from(move |query: String| {
            let ctx = SearchQuery { query };
            nav.push_with_query(&Route::Index, &ctx).unwrap();
        })
    };

    // On select request autocompletions from backend
    let onselect = Callback::from(|term| async move { 
        let req = AutocompleteRequest {
            input: term
        };
        let resp: AutocompleteResponse = backend_post!(&req, "/v1/autocomplete")
            .await
            .unwrap();
        resp.completions
            .into_iter()
            .map(|tag| {
                Completion {
                    inner: html! {
                        <div class="tag-completion">
                            <div class="name">
                                { &tag.name }
                                if let Some(alt_name) = &tag.alt_name {
                                    <i>
                                        { " " }
                                        { alt_name }
                                    </i>
                                }
                            </div>
                            <div class="count">
                                { tag.count }
                            </div>
                        </div>
                    },
                    name: tag.name
                }
            })
            .collect()
    }.boxed_local());
  
    html! {
        <main>
            <div class="search-box">
                <AppLink<()> 
                    class="button" 
                    route={Route::Index} >
                    { "To Index" }
                </AppLink<()>>
                <InputAutocomplete 
                    {onsubmit} 
                    {onselect} 
                    value={query.clone()}/>
            </div>
            <div class="page-content">
                <Switch<Route> render={switch} />
            </div>
        </main>
    }
}

#[function_component]
pub fn App() -> Html {
    html! {
        <BrowserRouter>
            <Root />
        </BrowserRouter>
    }
}


