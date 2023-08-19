use serde::Serialize;

use super::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LinkProps<T = ()> 
where T: PartialEq + Default {
    pub route: Route,
    pub children: Children,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub query: T,
}

/// A link that can be clicked or opened in new tab.
/// On click it changes current page.
/// Otherwise a new tab with another app instance will be opened
#[function_component]
pub fn AppLink<T>(props: &LinkProps<T>) -> Html
where T: PartialEq + Serialize + Clone + Default + 'static {
    let nav = use_navigator()
        .expect("failed to access navigator");

    let route = props.route.clone();
    let query = props.query.clone();
    let onclick = Callback::from(move |ev: MouseEvent| {
        // Prevent redirect on click
        ev.prevent_default();
        nav.push_with_query(&route, &query)
            .expect("failed to push route");
    });
    
    let query = serde_urlencoded::to_string(&props.query)
        .expect("failed to serialize query");
    let href = format!("{}?{}", props.route.to_path(), query);
        
    html! {
        <a class={props.class.clone()} {href} {onclick}>
            { for props.children.iter() }
        </a> 
    }
}