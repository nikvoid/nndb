use crate::{component::link::AppLink, app::QueryContext};

use super::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TagListProps {
    pub content: Vec<Tag>
}

#[function_component]
pub fn TagList(props: &TagListProps) -> Html {
    // Create list of tags by category
    let tags = enum_iterator::all::<TagType>()
        .filter_map(|ty| {
            let mut tags: Vec<_> = props.content
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
                            class="tag-page-link"
                            route={Route::Tag { id: t.id }}>
                            { "#" }
                        </AppLink<()>>
                        // Show elements with this tag
                        <AppLink<QueryContext> 
                            class="tag-info"
                            route={Route::Index}
                            query={QueryContext { query: t.name.clone() }}>
                            // Strikethrough if hidden
                            <@{if t.hidden { "s" } else { "div" }}>
                                { t.pretty_name() }
                            </@>
                            <div class="tag-count">
                                { t.count }
                            </div>
                        </AppLink<QueryContext>>
                    </>                
                });

           
            html! {
                <>
                    <div class="tag-type-name">
                        { ty.name() }
                    </div>
                    { for tags }
                </>
            }.into()
        });
    
    html! {
        <div class="tag-list">
            { for tags }
        </div>
    }
}