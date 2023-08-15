use web_sys::HtmlImageElement;
use super::link::AppLink;

pub use super::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ListProps {
    pub content: Vec<Element>
}

#[function_component]
pub fn ElementList(props: &ListProps) -> Html {
    let elements = props.content
        .iter()
        .map(|e| {
            let class = classes!(
                "element-container",
                e.animated.then_some("animated")
            );
            
            let src = match (&e.thumb_url, e.animated) {
                (Some(url), _) => url,
                (None, false) => &e.url,
                (None, true) => ""
            }.to_string();

            let alt = if e.broken { "broken" } else { "no image" };

            // On error, try to load full image and remove this handler to avoid spam
            let url = e.url.clone();
            let animated = e.animated;
            let onerror = Callback::from(move |ev: Event| {
                let img = ev
                    .target_dyn_into::<HtmlImageElement>()
                    .expect("wrong element");
                if !animated {
                    img.set_src(&url);
                }
                img.set_onerror(None);
            });
            html! {
                <AppLink<()> {class} route={Route::Element { id: e.id }}>
                    <img {src} {alt} {onerror} />
                </AppLink<()> > 
            }
        });

    html! {
        <div class="element-list">
            { for elements }
        </div>
    }
}
