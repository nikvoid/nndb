use yew::prelude::*;
use yew_router::prelude::*;
use serde::{Serialize, Deserialize};

/// A struct for passing search query around
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SearchQuery {
    pub query: String
}

/// Retrieve search query from location url
#[hook]
pub fn use_search_query() -> SearchQuery {
    let location = use_location().unwrap();
    // Try to seed context from current url -- if opened in new tab
    let context: SearchQuery = location
        .query()
        .unwrap_or_default();
    
    context
}

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
