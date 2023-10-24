use super::prelude::*;

/// Element metadata props
#[derive(PartialEq, Properties)]
pub struct MetadataProps {
    pub meta: ElementMetadata
}

/// Element metadata (excluding tags)
#[function_component]
pub fn Metadata(props: &MetadataProps) -> Html {
    let mut links = props.meta.ext_meta
        .iter()
        .filter_map(|m| m.src_link.as_ref().map(|href|
            html! {
                <a class="section-part" href={href.clone()}>
                    { m.source.name() }{ ": " }{ href }
                </a>
            }
        ))
        .peekable();

    let times = props.meta.ext_meta
        .iter()
        .filter_map(|m| m.src_time.as_ref().map(|time|
            html! {
                <div class="section-part">
                    { m.source.name() }{ ": " }{ time }
                </div>
            }
        ));

    let metadata_sections = props.meta.ext_meta
        .iter()
        .filter_map(|m| m.raw_meta.as_deref().map(|meta| {
            let params = m.source.additional_info(meta).into_iter()
                .map(|(k, v, wide)| html! {
                    if wide {
                        <div class="section-label">
                            { k }
                        </div>
                        <div class="section-part">
                            { v }
                        </div>
                    } else {
                        <div class="param-name">
                            { k }
                        </div>
                        <div class="param-value">
                            { v }
                        </div>
                    }
                });

            html! {
                <>
                    <div class="section-label">
                        { m.source.metadata_name() }
                    </div>
                    { for params }
                </>
            }
        }));
    
    html! {
        <div class="element-metadata">
            // General data
            if links.peek().is_some() {
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
            <div class="section-part">
                { "Created at: " }{ props.meta.file_time }
            </div>
            { for times }
            { for metadata_sections }
        </div>
    }
}