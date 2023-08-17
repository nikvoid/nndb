use crate::component::prelude::*;
use crate::component::input::InputAutocomplete;
use crate::component::link::AppLink;
use crate::page::element::ElementPage;
use crate::page::index::Index;
use crate::page::tag::TagPage;

pub fn switch(route: Route) -> Html {   
    match route {
        Route::Index => html! { <Index /> },
        Route::Element { id } => html! { <ElementPage {id} /> },
        Route::Tag { id } => html! { <TagPage {id} /> },
        // Route::Dashboard => todo!(),
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


