use yew::set_custom_panic_hook;

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
    let nav = use_navigator()
        .expect("failed to access navigator");
    let search = use_search_query();

    // On submit change query and push index page
    let onsubmit = {
        Callback::from(move |query: String| {
            let search = SearchQuery { query };
            nav.push_with_query(&Route::Index, &search)
                .expect("failed to push route");
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
    // On panic display alert
    set_custom_panic_hook(Box::new(|info| {
        let info = format!("{info}");
        if let Some(window) = web_sys::window() {
            if let Err(e) = window.alert_with_message(&info) {
                web_sys::console::log_3(
                    &"failed to send alert".into(), 
                    &e, 
                    &info.into()
                );
            }
        } else {
            web_sys::console::log_1(&info.into());
        }
    }));

    html! {
        <BrowserRouter>
            <Root />
        </BrowserRouter>
    }
}


