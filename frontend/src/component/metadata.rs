use crate::component::link::AppLink;

use super::prelude::*;

/// Tag list props
#[derive(Properties, PartialEq)]
pub struct TagListProps {
    pub content: Vec<Tag>
}

/// Element tag list, with links to tag edit page and search by tag
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
                    <div class="tag-type-name">
                        { ty.name_cap() }
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

/// Element metadata props
#[derive(PartialEq, Properties)]
pub struct MetadataProps {
    pub meta: ElementMetadata
}

/// Element metadata (excluding tags)
#[function_component]
pub fn Metadata(props: &MetadataProps) -> Html {
    let links = props.meta.src_links
        .iter()
        .map(|(src, href)| html! {
            <a class="section-part" href={href.clone()}>
                { src }{ ": " }{ href }
            </a>
        });

    let times = props.meta.src_times
        .iter()
        .map(|(src, time)| html! {
            <div class="section-part">
                { src }{ ": " }{ time }
            </div>
        });
    
    html! {
        <div class="element-metadata">
            // General data
            if !props.meta.src_links.is_empty() {
                <div class="section-label">
                    { "Source links" }
                </div>
                { for links }
            }
            <div class="section-label">
                { "Time" }
            </div>
            <div class="section-part">
                { "Added at: " }{ props.meta.add_time }
            </div>
            { for times }

            // Various SD parameters
            if let Some(ai) = &props.meta.ai_meta {
                <div class="section-label">
                    { "SD Metadata" }
                </div>
                <div class="section-label">
                    { "Positive prompt" }
                </div>
                <div class="section-part">
                    { &ai.positive_prompt }
                </div>
                if let Some(neg_prompt) = &ai.negative_prompt {
                    <div class="section-label">
                        { "Negative prompt" }
                    </div>
                    <div class="section-part">
                        { &neg_prompt }
                    </div>
                }
                <div class="param-name">
                    { "Steps" }
                </div>
                <div class="param-value">
                    { ai.steps }
                </div>
                <div class="param-name">
                    { "CFG Scale" }
                </div>
                <div class="param-value">
                    { ai.scale }
                </div>
                <div class="param-name">
                    { "Sampler" }
                </div>
                <div class="param-value">
                    { &ai.sampler }
                </div>
                <div class="param-name">
                    { "Seed" }
                </div>
                <div class="param-value">
                    { ai.seed }
                </div>
                <div class="param-name">
                    { "Denoising strength" }
                </div>
                <div class="param-value">
                    { ai.strength }
                </div>
                <div class="param-name">
                    { "Noise" }
                </div>
                <div class="param-value">
                    { ai.noise }
                </div>
            }
        </div>
    }
}