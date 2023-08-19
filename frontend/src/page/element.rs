use web_sys::HtmlElement;

use crate::component::{metadata::{TagList, Metadata}, element::ElementList};

use super::prelude::*;

/// Element page props
#[derive(PartialEq, Properties)]
pub struct Props {
    pub id: u32
}

/// Element metadata loading state
#[derive(PartialEq)]
enum State {
    Loading,
    Found(MetadataResponse),
    NotFound,    
}

/// Element page, that displays element, its metadata and associated/similar elements
#[function_component]
pub fn ElementPage(props: &Props) -> Html {
    let resp = use_state(|| State::Loading);
    
    {
        // React on element id changes
        let resp = resp.clone();
        use_effect_with_deps(move |id| {
            let id = *id;
            wasm_bindgen_futures::spawn_local(async move {
                let opt: Option<MetadataResponse> = backend_get!("/v1/element/{}", id)
                    .await
                    .expect("failed to fetch element data");
                resp.set(match opt {
                    Some(resp) => State::Found(resp),
                    None => State::NotFound
                });
            })    
        }, props.id)
    }

    // Disable/enable constraints on element click
    let onclick = Callback::from(|ev: MouseEvent| {
        let target: HtmlElement = ev.target_dyn_into().unwrap();
        let class = if target.class_name().is_empty() {
            "element-constrained"
        } else {
            ""
        };
        target.set_class_name(class);
    });

    match &*resp {
        State::Loading => html! { },
        State::Found(MetadataResponse { element, metadata, associated }) => {
            let associated = associated
                .iter()
                // Do not display empty groups or groups that have only this element
                .filter(|assoc| 
                    !assoc.elements.is_empty()
                    && (assoc.elements.len() != 1 || assoc.elements[0] != *element)
                )
                .map(|Associated { key, value, elements }| {
                    html! {
                        <>
                            <div class="group-label">
                                { key } { ": " } { value }
                            </div>
                            <ElementList content={elements.clone()} />
                        </>
                    }                    
                });
            
            html! {
                <div class="element-page">
                    <div class="metadata">
                        <TagList content={metadata.tags.clone()} />
                        <Metadata meta={metadata.clone()} />
                    </div>
                    <div id="element-container">
                        if element.animated {
                            <video class="element-constrained" controls=true loop=true>
                                <source src={element.url.clone()} />
                            </video>                
                        } else {
                            <img 
                                class="element-constrained" 
                                src={element.url.clone()} 
                                {onclick} />
                        }
                    </div>
                    <div class="associated">
                        { for associated }
                    </div>
                </div>
            }
        },
        State::NotFound => html! {
            <Redirect<Route> to={Route::NotFound} />
        }
    }
}