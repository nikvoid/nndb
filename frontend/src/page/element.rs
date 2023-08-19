use web_sys::HtmlElement;

use crate::component::{metadata::Metadata, element::ElementList, tag::TagList};

use super::prelude::*;

/// Element page props
#[derive(PartialEq, Properties)]
pub struct Props {
    pub id: u32
}

/// Element metadata loading state
#[derive(PartialEq, Default)]
pub enum State {
    #[default]
    Loading,
    Found(MetadataResponse),
    NotFound,    
}

/// Page that displays element, its metadata and associated/similar elements
#[derive(Default)]
pub struct ElementPage {
    element_data: State
}

pub enum Msg {
    Reload,
    Update(State),
    ChangeTags(Vec<String>, Vec<String>)
}

impl Component for ElementPage {
    type Message = Msg;

    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Reload);
        Self::default()
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
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

        let oncommit = ctx.link()
            .callback(|(add, remove)| Msg::ChangeTags(add, remove));
        
        match &self.element_data {
            State::Loading => html! {},
            State::Found(MetadataResponse { 
                element, 
                metadata, 
                associated 
            }) => {
                let associated = associated
                    .iter()
                    // Do not display empty groups or groups that have only this element
                    .filter(|assoc| 
                        !assoc.elements.is_empty()
                        && (
                            assoc.elements.len() != 1 
                            || assoc.elements[0] != *element
                        )
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
                            <TagList 
                                content={metadata.tags.clone()} 
                                read_only={false} 
                                {oncommit} />
                            <Metadata meta={metadata.clone()} />
                        </div>
                        <div id="element-container">
                            if element.animated {
                                <video 
                                    class="element-constrained" 
                                    controls=true 
                                    loop=true>
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

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => {
                // Just fetch element data
                let id = ctx.props().id;
                ctx.link().send_future(async move {
                    let opt: Option<MetadataResponse> = 
                        backend_get!("/v1/element/{}", id)
                        .await
                        .expect("failed to fetch element data");
                    let state = match opt {
                        Some(resp) => State::Found(resp),
                        None => State::NotFound
                    };
                    Msg::Update(state)
                });
                false
            },
            Msg::ChangeTags(add, remove) => {
                // On tag list commit send request to change element tags
                let element_id = ctx.props().id;
                ctx.link().send_future(async move {
                    let req = EditTagsRequest {
                        element_id,
                        add,
                        remove
                    };
                    let _: () = backend_post!(&req, "/v1/tags_edit")
                        .await
                        .expect("failed to send tags edit request");
                    Msg::Reload
                });
                false
            },
            Msg::Update(state) => {
                self.element_data = state;
                true
            },
        }
    }
}

