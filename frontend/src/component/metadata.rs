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

            // FIXME:
            // Various SD parameters
            // if let Some(ai) = &props.meta.ai_meta {
            //     <div class="section-label">
            //         { "SD Metadata" }
            //     </div>
            //     <div class="section-label">
            //         { "Positive prompt" }
            //     </div>
            //     <div class="section-part">
            //         { &ai.positive_prompt }
            //     </div>
            //     if let Some(neg_prompt) = &ai.negative_prompt {
            //         <div class="section-label">
            //             { "Negative prompt" }
            //         </div>
            //         <div class="section-part">
            //             { &neg_prompt }
            //         </div>
            //     }
            //     <div class="param-name">
            //         { "Steps" }
            //     </div>
            //     <div class="param-value">
            //         { ai.steps }
            //     </div>
            //     <div class="param-name">
            //         { "CFG Scale" }
            //     </div>
            //     <div class="param-value">
            //         { ai.scale }
            //     </div>
            //     <div class="param-name">
            //         { "Sampler" }
            //     </div>
            //     <div class="param-value">
            //         { &ai.sampler }
            //     </div>
            //     <div class="param-name">
            //         { "Seed" }
            //     </div>
            //     <div class="param-value">
            //         { ai.seed }
            //     </div>
            //     <div class="param-name">
            //         { "Denoising strength" }
            //     </div>
            //     <div class="param-value">
            //         { ai.strength }
            //     </div>
            //     <div class="param-name">
            //         { "Noise" }
            //     </div>
            //     <div class="param-value">
            //         { ai.noise }
            //     </div>
            // }
        </div>
    }
}