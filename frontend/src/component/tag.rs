use nndb_common::search::{parse_query, Term};

use crate::component::{link::AppLink, input::InputAutocomplete};

use super::prelude::*;

/// Tag list props
#[derive(Properties, PartialEq)]
pub struct Props {
    pub content: Vec<Tag>,
    #[prop_or(true)]
    pub read_only: bool,
    /// Callback that will be called on save
    ///
    /// `(added, removed) -> ()`
    #[prop_or_default]
    pub oncommit: Callback<(Vec<String>, Vec<String>)>,
}

/// Element tag list, with links to tag edit page and search by tag
#[derive(Default)]
pub struct TagList {
    // If in edit mode, this vec will hold temporary state
    edit_list: Option<Vec<String>>,
    // New tag input visibility
    input_visible: bool,
}

pub enum Msg {
    StartEdit,
    StartInput,
    Remove(usize),
    Add(String),
    Commit,
    Reset,
}

impl Component for TagList {
    type Message = Msg;

    type Properties = Props;

    fn create(_: &Context<Self>) -> Self {
        Self::default()
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let tags: Html = match &self.edit_list {
            // Create list of tags by category
            None => enum_iterator::all::<TagType>().filter_map(|ty| {
                let mut tags: Vec<_> = ctx.props().content
                    .iter()
                    .filter(|t| t.tag_type == ty)
                    .collect();
                // Omit category if it is empty
                if tags.is_empty() {
                    return None
                };
        
                // Alphabet order
                tags.sort_by_key(|t| &t.name);
        
                let tags = tags.iter()
                    .map(|t| html! {
                        <>
                        // Link to tag edit page
                        <AppLink<()> 
                            class="tag-aside"
                            route={Route::Tag { id: t.id }}>
                            { "#" }
                        </AppLink<()>>
                        // Show elements with this tag
                        <AppLink<SearchQuery> 
                            class="tag-info"
                            route={Route::Index}
                            query={SearchQuery { query: t.name.clone() }}>
                            // Strikethrough if hidden
                            <@{if t.hidden { "s" } else { "div" }}>
                                { t.pretty_name() }
                                if let Some(alt_name) = &t.alt_name {
                                    <i>{ " " }{ alt_name }</i>
                                }
                            </@>
                            <div class="tag-count">
                                { t.count }
                            </div>
                        </AppLink<SearchQuery>>
                        </>                
                    });
       
                html! {
                    <>
                        <div class="section-label">
                            { ty.name_cap() }
                        </div>
                        { for tags }
                    </>
                }.into()
            })
            .collect(),

            // Just create list of tags without categories
            Some(edit) => {
                edit.iter().enumerate().map(|(idx, tag)| {
                    let onclick = ctx.link().callback(move |_| Msg::Remove(idx));
                    html! {
                        <>
                        <div class="tag-aside" {onclick}>
                            { "x" }
                        </div>
                        <div class="tag-info">
                            { tag.replace('_', " ") }
                        </div>
                        </>                        
                    }
                })
                .collect()
            }
        };
        
        let onclick_edit = ctx.link().callback(|_| Msg::StartEdit);
        let onclick_reset = ctx.link().callback(|_| Msg::Reset);
        let onclick_save = ctx.link().callback(|_| Msg::Commit);
        let onclick_add = ctx.link().callback(|_| Msg::StartInput);

        let onsubmit = ctx.link().callback(Msg::Add);
        
        html! {
            <div class="tag-list">
                <div class="header">
                    <div class="tags-label">
                        { "Tags" }
                    </div>
                    if !ctx.props().read_only {
                        <div class="edit-controls">
                            if self.edit_list.is_some() {
                                <div class="button" onclick={onclick_reset}>
                                    { "Reset" }
                                </div>
                                <div class="button" onclick={onclick_save}>
                                    { "Save" }
                                </div>
                                <div class="button" onclick={onclick_add}>
                                    { "+" }
                                </div>
                            } else {
                                <div class="button" onclick={onclick_edit}>
                                    { "Edit" }
                                </div>
                            }
                        </div>
                    }
                </div>
                if !ctx.props().read_only {
                    <div class="tag-input" hidden={!self.input_visible}>
                        <InputAutocomplete 
                            {onsubmit}
                            button_name={"Add"}/>
                    </div>
                }
                { tags }
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {

        match msg {
            Msg::StartEdit => {
                // Populate edit list from props
                let mut list: Vec<_> = ctx.props().content
                    .iter()
                    .map(|t| t.name.clone())
                    .collect();
                // Enforce alphabet order
                list.sort();
                self.edit_list = Some(list);
            }
            Msg::StartInput => {
                // Unhide input box
                self.input_visible = true;
            }
            Msg::Add(tags) => {
                let tags = parse_query(&tags)
                    .filter_map(|t| 
                        if let Term::Tag(true, slice) = t {
                            Some(slice.to_string())
                        } else {
                            None
                        }
                    );
                    
                // Add tags to list and preserve alphabetic order
                if let Some(list) = &mut self.edit_list {
                    list.extend(tags);    
                    list.sort();
                }
                // Mask input box
                self.input_visible = false;
            }
            Msg::Remove(tag_idx) => {
                // Remove tag by index
                if let Some(list) = &mut self.edit_list {
                    list.remove(tag_idx);
                }
            }
            Msg::Commit => {
                let list = self.edit_list.take().unwrap();
                let mut add_list = vec![];
                let mut remove_list = vec![];
                
                // Find tags that were removed
                for tag in &ctx.props().content {
                    if !list.contains(&tag.name) {
                        remove_list.push(tag.name.clone());
                    }
                }
                
                // Find tags that were added
                for tag in list {
                    if !ctx.props().content
                        .iter()
                        .any(|t| t.name == tag) {
                        add_list.push(tag);
                    }
                };
                
                ctx.props().oncommit.emit((add_list, remove_list));
            }
            Msg::Reset => {
                // Just drop edit list
                self.edit_list.take();
                self.input_visible = false;
            }
        }


        true
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        // We do not want edit list to be silently de-synced from props
        ctx.link().send_message(Msg::Reset);
        true
    }
}
