use crate::component::prelude::*;
use crate::component::input::InputAutocomplete;
use crate::component::link::AppLink;
use crate::page::dashboard::Dashboard;
use crate::page::element::ElementPage;
use crate::page::index::Index;
use crate::page::tag::TagPage;

pub fn switch(route: Route) -> Html {   
    match route {
        Route::Index => html! { <Index /> },
        Route::Element { id } => html! { <ElementPage {id} /> },
        Route::Tag { id } => html! { <TagPage {id} /> },
        Route::Dashboard => html! { <Dashboard /> },
        _ => html! {
            <div class="label">{ "Not Found" }</div>
        }
    }
}

/// A shim to access location inside BrowserRouter -- and page root
#[function_component]
fn Root() -> Html {
    let nav = use_navigator().unwrap();
    let search = use_search_query();

    // On submit change query and push index page
    let onsubmit = {
        Callback::from(move |query: String| {
            let search = SearchQuery { query };
            nav.push_with_query(&Route::Index, &search).unwrap();
        })
    };
  
    html! {
        <main>
            <div class="search-box">
                <AppLink<()> 
                    class="index-button" 
                    route={Route::Index} >
                    { "To Index" }
                </AppLink<()>>
                <InputAutocomplete 
                    {onsubmit} 
                    value={search.query.clone()}/>
                <AppLink<SearchQuery> 
                    class="dashboard-button" 
                    route={Route::Dashboard} 
                    query={search.clone()}>
                    { "Dashboard" }
                </AppLink<SearchQuery>>
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


