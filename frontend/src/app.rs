use yew::prelude::*;
use yew_router::prelude::*;
use crate::page::index::Index;

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
    }
}

pub fn switch(route: Route) -> Html {   
    match route {
        Route::Index => html! { <Index /> },
        Route::Dashboard => todo!(),
        Route::Element { id } => todo!(),
        Route::Tag { id } => todo!(),
    }
}

#[function_component]
pub fn App() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}


