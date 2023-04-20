use maud::{Render, DOCTYPE, Markup, html_to};
use serde::Serialize;
use enum_iterator::all;

use crate::{config::CONFIG, model::{read::{Tag, Element, ElementMetadata}, TagType}, util::Crc32Hash};

mod index;
mod element;
mod tag;
mod api;
mod dashboard;

pub use index::index_page;
pub use element::element_page;
pub use tag::tag_page;
pub use dashboard::dashboard_page;
pub use api::tag_autocomplete;
pub use api::add_tags;
pub use api::delete_tag;
pub use api::edit_tag;
pub use api::read_log;
pub use api::import_status;
pub use api::start_import;
pub use api::update_tag_count;
pub use api::clear_group_data;
pub use api::fix_thumbnails;
pub use api::retry_imports;

/// Helper for writing nested html_to!
/// Basically a lazy html! that can be rendered(-to) on demand
#[macro_export]
macro_rules! html_in {
    ($($tt:tt)*) => {
        $crate::view::MaudFnWrapper(|buf: &mut String| maud::html_to!{ buf, $($tt)* })
    };
}

/// Compile-time url resolver.
/// We can't use expressions in #[get] macro, so this is better than 
/// writing each url by hand
#[macro_export]
macro_rules! resolve {
    // Index page
    (/index) => { "/index" };
    // Element page
    (/element/$eid:expr) => { $crate::html_in! { "/element/" ($eid) }};
    // Tag page
    (/tag/$name:expr) => { $crate::html_in! { "/tag/" ($name) }};
    // Dashboard page
    (/dashboard) => { "/dashboard" };
    ($($tt:tt)*) => { stringify!($($tt)*) };
}

/// Log error and return 500 status to client
#[macro_export]
macro_rules! log_n_bail {
    ($lit:literal $(, $($tt:tt)* )?) => {{
        tracing::error!($($($tt)*,)? $lit);
        return Err(actix_web::error::ErrorInternalServerError($lit));
    }};
}

/// Log info and return 200 status to client
#[macro_export]
macro_rules! log_n_ok {
    ($lit:literal $(, $($tt:tt)* )?) => {{
        tracing::info!($($($tt)*,)? $lit);
        return Ok($lit);
    }};
}

/// Wrapper for ergonomic interfacing serde_qs with maud
/// # Panics
/// Will panic if serde_qs fails to serialize `R`
struct QueryString<'a, R: Serialize>(&'a R);
impl<'a, R> Render for QueryString<'a, R> where R: Serialize {
    fn render_to(&self, buffer: &mut String) {
        // Should be safe as long as serde_qs outputs valid UTF-8
        // (It should be escaped to ascii, actually)
        let writer = unsafe { buffer.as_mut_vec() };
        
        serde_qs::to_writer(self.0, writer).unwrap();
    }
}

/// A bit of closure magic to work around nested html_to!
struct MaudFnWrapper<F>(F);
impl<F> Render for MaudFnWrapper<F> 
where F: Fn(&mut String) {
    fn render_to(&self, buffer: &mut String) {
        self.0(buffer)
    }
}

/// Static content 
struct Static<'a>(&'a str);
impl Render for Static<'_> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.static_files_path);
        buffer.push_str(self.0);
    }
}

/// Link to element in pool
struct ElementLink<'a>(&'a Element);
impl Render for ElementLink<'_> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.elements_path);
        buffer.push_str(&self.0.filename);
    }
}

/// Link to element thumbnail in pool
struct ElementThumbnail<'a>(&'a Element);
impl Render for ElementThumbnail<'_> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.thumbnails_path);
        buffer.push_str(&self.0.filename.split('.').next().unwrap());
        buffer.push_str(".jpeg");
    }
}

/// Element list unit
struct ElementListContainer<'a>(&'a Element);
impl Render for ElementListContainer<'_> {
    fn render_to(&self, buffer: &mut String) {
        let ident = html_in! { "ELEMENT_LIST_" (self.0.id) };
        html_to! { buffer,
            .image-container-list.image-container-list-video[self.0.animated] {
                a href=(resolve!(/element/self.0.id)) {
                    (ScriptVar(&ident, &ElementLink(self.0)))
                    img.def-img.image-list-element src=(ElementThumbnail(self.0))
                        alt={ @if self.0.broken { "broken" } @else { "no image" } }
                        onerror = { 
                            @if !self.0.animated {
                                "elementListOnError(this, " (ident) ")"
                            } 
                        }
                    ;
                }
            }
        }
    }
}

/// Html <head>
struct HtmlHead;
impl Render for HtmlHead {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="UTF-8";
                    link rel="stylesheet" href=(Static("styles.css"));
                    script src=(Static("script.js")) {}
                    title { "nndb" }
                }
            }
        }
    }
}

/// A button (<a class=button>)
struct Button<Href, Text>(Href, Text);
impl<Href, Text> Render for Button<Href, Text>
where 
    Href: Render, 
    Text: Render {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            a.button href=(self.0) {
                (self.1)
            }       
        }
    }
}

/// A button that calls script
struct ScriptButton<Script, Text>(Script, Text);
impl<Script, Text> Render for ScriptButton<Script, Text>
where 
    Script: Render, 
    Text: Render {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            a.button onclick={ (self.0) "; return false;" } href="?" {
                (self.1)
            }       
        }
    }
}

/// Compose link from url and GET query
struct Link<Url, QueryS>(Url, QueryS);
impl<Url, QueryS> Render for Link<Url, QueryS> 
where 
    Url: Render,
    QueryS: Serialize {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer, 
            (self.0) "?" (QueryString(&self.1))
        }
    }
}


/// Base template for any other page
#[derive(Default)]
struct BaseContainer<'a> {
    /// Block below page header
    after_header: Option<Markup>,
    /// Main content
    content: Option<Markup>,
    /// Block aside of content
    aside: Option<Markup>,
    /// Page footer
    footer: Option<Markup>,       

    /// Query parameter
    query: &'a str,
}

impl<'a> Render for BaseContainer<'a> {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            (HtmlHead)
            body {
                #base-container { 
                    #upnav {
                        span.head-span { (Button(resolve!(/index), "Index")) }
                        span.head-span {
                            form autocomplete="off" action=(resolve!(/index)) method="get" {
                                input #search-box type="text" 
                                    name="query" value=(self.query)
                                    onKeyUp="getCompletions(this, 'head-result')" 
                                    onclick="getCompletions(this, 'head-result')"
                                    onchange="searchBoxHook(this)";
                                input type="submit" value="Search";
                                .result #head-result hidden {}
                            } 
                            @if let Some(h) = &self.after_header { (h) };
                        }
                    }
                    @if let Some(cont) = &self.content { (cont) };
                    section.aside {
                        @if let Some(aside) = &self.aside { (aside) }
                    }
                }
                footer {
                    .footer-container {
                        (Button(resolve!(/dashboard), "Dashboard"))
                    }
                    @if let Some(f) = &self.footer { (f) }
                }
            }
        }
    }
}

/// Block with tags aside of element list/element page
struct AsideTags<'a>(&'a [Tag], Option<&'a Element>);
impl Render for AsideTags<'_> {
    fn render_to(&self, buffer: &mut String) {
        // Group tags by types (create iterator for each non-empty type)
        let types = all::<TagType>()
            .map(|typ| (typ, self.0.iter().filter(move |t| t.tag_type == typ)))
            .filter(|(_, iter)| iter.clone().next().is_some())
        ;
        
        html_to! { buffer,
            @for (typ, tags) in types {
                .tag { (typ.label()) }
                @for tag in tags {
                    .tag-container-grid {
                        a.tag.tag-hash href=(Link(resolve!(/tag/tag.name), tag::Request {
                            element_ref: (self.1.map(|e| e.id))
                        })) {
                            "#" 
                        }
                        a.tag.tag-block.tag-hidden[tag.hidden] href=(Link(resolve!(/index), 
                        index::Request {
                            query: Some(&tag.name),
                            page: None
                        })) {
                            @for part in tag.name.split("_") {
                                (part) " "     
                            }
                            (tag.count) 
                            @if let Some(alt) = &tag.alt_name {
                                br; span.tag-alt-name { (alt) }
                            }
                        }
                    }
                }
            } 
        }
    }
}

/// Tag input form with autocomplete (TODO: Not yet) (action, id, submit_name)
struct TagEditForm<'a, OnSubmit>(OnSubmit, &'a str, &'a str);
impl<OnSubmit> Render for TagEditForm<'_, OnSubmit>
where OnSubmit: Render {
    fn render_to(&self, buffer: &mut String) {
        let ident = html_in! { "TAG_EDIT_FIELD_" (self.1.crc32()) };
        html_to! { buffer,
            form onsubmit=(self.0) {
                (ScriptVar(&ident, self.1))
                input.tag-field #{ (self.1) "_box" } name="tag" type="text"
                    onKeyUp={ "getCompletions(this, " (ident) ")" } 
                    onclick={ "getCompletions(this, " (ident) ")" };
                input type="submit" value=(self.2);
                .result #(self.1) hidden {}
            }            
        }
    }
}

/// param_name   param_data
/// 
/// For different parameters that usually rendered aside
struct BlockParam<'a, R>(&'a str, R);
impl<R> Render for BlockParam<'_, R>
where R: Render {
    fn render_to(&self, buffer: &mut String) {

        html_to! { buffer,
            .tag-container-grid {
                @if !self.0.is_empty() {
                    .tag.tag-hash { (self.0) }
                }
                .tag.tag-block { (self.1) }
            }
        }

    }
}

/// Aside block that displays element metadata
struct AsideMetadata<'a>(&'a ElementMetadata);
impl Render for AsideMetadata<'_> {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            .tag { "Time" }
            .tag-container-grid {
                b.tag.tag-block { "Added at " (self.0.add_time) }
            }
            @if let Some(time) = self.0.src_time {
                .tag-container-grid {
                    b.tag.tag-block { "Source: " (time) }
                }
            }
            @if let Some(link) = &self.0.src_link {
                .tag { "Source" }
                .tag-container-grid {
                    a.tag.tag-block href=(link) { (link) }
                }
            }
            @if let Some(ai) = &self.0.ai_meta {
                .tag { "NN Metadata" }
                (BlockParam("", html_in! { "Positive prompt" br; (ai.positive_prompt) }))
                @if let Some(neg) = &ai.negative_prompt {
                    (BlockParam("", html_in! { "Negative prompt" br; (neg) }))
                }
                (BlockParam("Sampler", &ai.sampler))
                (BlockParam("Seed", ai.seed))
                (BlockParam("Steps", ai.steps))
                (BlockParam("Scale", ai.scale))
                (BlockParam("Strength", ai.strength))
                (BlockParam("Noise", ai.noise))
            }
            @else if !self.0.tags.is_empty() {
                .tag { "Generated prompt" }
                (BlockParam("", html_in! {
                    // compose prompt from tags
                    @for tag in &self.0.tags {
                        (tag.name) ", "
                    }
                }))
            }
        }
    }
}

/// Variable that can be directly spliced into script chunk
pub enum ScriptVariable<'a> {
    Int(i64),
    String(&'a str),
    Render(&'a dyn Render),
} 

impl Render for ScriptVariable<'_> {
    fn render_to(&self, buffer: &mut String) {
        match self {
            ScriptVariable::Int(i) => i.render_to(buffer),
            ScriptVariable::String(s) => {
                buffer.push('\'');
                buffer.push_str(s);
                buffer.push('\'');
            },
            ScriptVariable::Render(r) => html_to! { buffer, "'" (r) "'" },
        }
    }
}

/// Represent value as `ScriptVariable`
pub trait AsScriptVariable {
    fn as_script_var(&self) -> ScriptVariable;
}

impl<T> AsScriptVariable for &T 
where T: Render {
    fn as_script_var(&self) -> ScriptVariable {
        ScriptVariable::Render(self)
    }
}

impl AsScriptVariable for &str {
    fn as_script_var(&self) -> ScriptVariable {
        ScriptVariable::String(self)
    }
} 

impl AsScriptVariable for u32 {
    fn as_script_var(&self) -> ScriptVariable {
        ScriptVariable::Int(*self as i64)
    }
}

/// Inject variable with (ident, value) into script
pub struct ScriptVar<Ident, Val>(Ident, Val);
impl<Ident, Val> Render for ScriptVar<Ident, Val>
where 
    Val: AsScriptVariable, 
    Ident: Render {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            script {
                "let " (self.0) " = " (self.1.as_script_var()) ";"
            }
        }
    }
}
