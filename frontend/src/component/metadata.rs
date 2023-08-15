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
                        <a class="tag-page-link">
                            // TODO: Link
                            { "#" }
                        </a >
                        <a class="tag-info">
                            <div class="tag-name">
                                { t.pretty_name() }
                            </div>
                            <div class="tag-count">
                                { t.count }
                            </div>
                        </a>
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