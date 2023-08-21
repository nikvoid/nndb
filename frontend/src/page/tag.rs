use web_sys::{HtmlInputElement, HtmlSelectElement};

use crate::component::{tag::TagList, input::InputAutocomplete};

use super::prelude::*;

/// Tag page props
#[derive(PartialEq, Properties)]
pub struct Props {
    pub id: u32
}

#[derive(Default)]
pub enum TagState {
    #[default]
    Loading,
    Found(TagResponse),
    NotFound,
}

/// Tag info/edit page
#[derive(Default)]
pub struct TagPage {
    tag_data: TagState,
    type_ref: NodeRef,
    name_ref: NodeRef,
    alt_name_ref: NodeRef,
    hidden_ref: NodeRef
}

/// Tag page messages
pub enum Msg {
    Reload,
    Update(TagState),
    Alias(String),
    Send
}

impl Component for TagPage {
    type Message = Msg;

    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Reload);
        Self::default()
    }

    fn view(&self, ctx: &Context<Self>) -> Html {

        match &self.tag_data {
            TagState::Loading => html! { },
            TagState::Found(TagResponse { tag, aliases }) => {
                let options = enum_iterator::all::<TagType>()
                    .map(|ty| {
                        html! {
                            <option 
                                value={ty.name()} 
                                selected={ty == tag.tag_type} >
                            { ty.name_cap() }
                            </option>
                        }
                    });
    
                let onsubmit = ctx.link().callback(|ev: SubmitEvent| {
                    ev.prevent_default();
                    Msg::Send
                });

                let onsubmit_alias = ctx.link().callback(Msg::Alias);

                let form = html! {
                    <form id="tag-edit" {onsubmit}>
                        <div class="label" id="tag-name-label">
                            { "Name" }
                        </div>
                        <div class="label" id="tag-alt-name-label">
                            { "Alternative name" }
                        </div>
                        <div class="label" id="tag-hidden-label">
                            { "Hidden" }
                        </div>
                        <div class="label" id="tag-type-label">
                            { "Type" }
                        </div>
                        <input 
                            ref={self.name_ref.clone()}
                            type="text" 
                            id="tag-name" 
                            value={tag.name.clone()} />
                        <input 
                            ref={self.alt_name_ref.clone()}
                            type="text" 
                            id="tag-alt-name"
                            value={tag.alt_name.clone().unwrap_or_default()} />
                        <select id="tag-type" ref={self.type_ref.clone()}>
                            { for options }
                        </select>
                        <input 
                            ref={self.hidden_ref.clone()}
                            type="checkbox" 
                            id="tag-hidden" 
                            checked={tag.hidden} />
                        <input 
                            type="submit" 
                            id="change-tag" 
                            value="Change tag" />
                        <div 
                            id="tag-alias" 
                            title="Alias to self removes element from any group">
                            <InputAutocomplete
                                onsubmit={onsubmit_alias}
                                button_name={"Add aliases"}
                                />
                        </div>
                    </form>
                };
        
                html! {
                    <div class="tag-page">
                        <div class="tag-infos">       
                            <div class="section-label">
                                { "Tag info" }
                            </div>
                            <div class="param-name">
                                { "ID" }
                            </div>
                            <div class="param-value">
                                { tag.id }
                            </div>
                            <div class="param-name">
                                { "Name" }
                            </div>
                            <div class="param-value">
                                { &tag.name }
                            </div>
                            if let Some(alt_name) = &tag.alt_name {
                                <div class="param-name">
                                    { "Alternative name" }
                                </div>
                                <div class="param-value">
                                    { alt_name }
                                </div>
                            }
                            <div class="param-name">
                                { "Type" }
                            </div>
                            <div class="param-value">
                                { tag.tag_type.name() }
                            </div>
                            <div class="param-name">
                                { "Hidden" }
                            </div>
                            <div class="param-value">
                                { tag.hidden }
                            </div>
                            <div class="param-name">
                                { "Images with tag" }
                            </div>
                            <div class="param-value">
                                { tag.count }
                            </div>
                    
                            if !aliases.is_empty() {
                                <div class="section-label">
                                    { "Tag aliases" }
                                </div>
                                <div class="section-data">
                                    <TagList content={aliases.clone()} />
                                </div>
                            }
                        </div>
                        { form }
                    </div>
                }
            },
            TagState::NotFound => html! {
                <Redirect<Route> to={Route::NotFound} />
            },
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => {
                let id = ctx.props().id;
                ctx.link().send_future(async move {
                    let resp: Option<TagResponse> = backend_get!("/v1/tag/{}", id)
                        .await
                        .expect("failed to fetch tag data");

                    if let Some(r) = resp {
                        Msg::Update(TagState::Found(r))
                    } else {
                        Msg::Update(TagState::NotFound)
                    }
                });
                false
            },
            Msg::Update(state) => {
                self.tag_data = state;
                true
            },
            Msg::Alias(aliases) => {
                let TagState::Found(resp) = &self.tag_data else { unreachable!() }; 
                let tag_name = resp.tag.name.clone();
                ctx.link().send_future(async move {
                    let req = TagAliasRequest {
                        tag_name,
                        query: aliases,
                    };
                    let _: () = backend_post!(&req, "/v1/tag_alias")
                        .await
                        .expect("failed to alias tag");
                    Msg::Reload
                });
                false
            },
            Msg::Send => {
                // Retrieve fields
                let new_name = self.name_ref.cast::<HtmlInputElement>()
                    .unwrap()
                    .value();
                let alt_name = self.alt_name_ref.cast::<HtmlInputElement>()
                    .unwrap()
                    .value();
                let hidden = self.hidden_ref.cast::<HtmlInputElement>()
                    .unwrap()
                    .checked();
                let tag_type = self.type_ref.cast::<HtmlSelectElement>()
                    .unwrap()
                    .selected_options()
                    .get_with_index(0)
                    .unwrap()
                    .get_attribute("value")
                    .unwrap()
                    .parse()
                    .unwrap();
                let TagState::Found(resp) = &self.tag_data else { unreachable!() }; 
                let tag_name = resp.tag.name.clone();
                ctx.link().send_future(async move {
                    let req = TagEditRequest {
                        tag_name,
                        new_name,
                        alt_name: (!alt_name.is_empty()).then_some(alt_name),
                        tag_type,
                        hidden,
                    };
                    // TODO: We may respond with updated tag state
                    let _: () = backend_post!(&req, "/v1/tag_edit")
                        .await
                        .expect("failed to edit tag");
                    Msg::Reload
                });
                false
            },
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        // Reload on tag id change
        ctx.link().send_message(Msg::Reload);
        true
    }
}
