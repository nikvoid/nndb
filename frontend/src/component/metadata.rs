use super::prelude::*;

/// Element metadata props
#[derive(PartialEq, Properties)]
pub struct MetadataProps {
    pub meta: ElementMetadata,
    pub on_show_raw_meta: Callback<String>
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

    let make_html = |html: &str| {
        Html::from_html_unchecked(format!("<div>{html}</div>").into())
    };
    
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
                            { make_html(&v) }
                        </div>
                    } else {
                        <div class="param-name">
                            { k }
                        </div>
                        <div class="param-value">
                            { make_html(&v) }
                        </div>
                    }
                });

            let onclick = {
                let on_show = props.on_show_raw_meta.clone();
                let raw_meta = meta.to_string();
                let source = m.source;
                Callback::from(move |_| {
                    let pretty = source.pretty_raw_meta(&raw_meta);
                    on_show.emit(pretty)
                })
            };

            html! {
                <>
                    <div class="external-meta-header">
                        <div class="meta-label">
                            { m.source.metadata_name() }
                        </div>
                        <div class="show-btn button" {onclick}>
                            { "Raw" }
                        </div>
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