use futures::FutureExt;
use serde::{Serialize, Deserialize};
use crate::backend_post;
use crate::component::input::{Completion, InputAutocomplete};
use crate::page::index::{Index, IndexQuery};
use crate::component::prelude::*;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Index,
    #[at("/dashboard")]
    Dashboard,
    #[at("/element/:id")]
    Element {
        id: u32
    },
    #[at("/tag/:id")]
    Tag {
        id: u32
    },
    #[not_found]
    #[at("/not_found")]
    NotFound,
}

/// A context struct for passing search query around
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QueryContext {
    pub query: String
}

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

/// A shim to access location inside BrowserRouter
#[function_component]
fn Shim() -> Html {
    let location = use_location().unwrap();
    let nav = use_navigator().unwrap();
    
    // Try to seed context from current url -- if opened in new tab
    let init_context: QueryContext = location
        .query()
        .unwrap_or_default();
    let context = use_state(move || init_context);

    // On submit change context and push index page
    let onsubmit = {
        let context = context.clone();
        Callback::from(move |query: String| {
            nav.push_with_query(&Route::Index, &IndexQuery {
                query: Some(query.clone()),
                page: Some(1)
            }).unwrap();
            context.set(QueryContext { query });
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
        <ContextProvider<QueryContext> context={(*context).clone()} >
            <main>
                <div class="search-box">
                    <InputAutocomplete 
                        {onsubmit} 
                        {onselect} 
                        initial_value={context.query.clone()}/>
                </div>
                <div class="page-content">
                    <Switch<Route> render={switch} />
                </div>
            </main>
        </ContextProvider<QueryContext>>
    }
}

#[function_component]
pub fn App() -> Html {
    html! {
        <BrowserRouter>
            <Shim />
        </BrowserRouter>
    }
}


