use maud::{Render, DOCTYPE, Markup, html_to};
use serde::Serialize;
use enum_iterator::all;

use crate::{config::CONFIG, model::{read::{Tag, Element, ElementMetadata}, TagType}};

mod index;
mod element;

pub use index::index_page;
pub use element::element_page;

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

/// Helper for writing nested html_to!
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
    // Element page (not yet)
    (/element/$eid:expr) => { $crate::html_in!("/element/" ($eid) ) };
    ($($tt:tt)*) => { stringify!($($tt)*) };
}

/// Static content 
struct Static<'a>(&'a str);
impl<'a> Render for Static<'a> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.static_files_path);
        buffer.push_str(self.0);
    }
}

/// Link to element in pool
struct ElementLink<'a>(&'a Element);
impl<'a> Render for ElementLink<'a> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.elements_path);
        buffer.push_str(&self.0.filename);
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
            a.button onclick=(self.0) href="?" {
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
                                    // TODO: Enable scripts
                                    onKeyUp=""; 
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
                        (Button(resolve!(/admin), "Administration"))
                    }
                    @if let Some(f) = &self.footer { (f) }
                }
            }
        }
    }
}

/// Block with tags aside of element list/element page
struct AsideTags<'a>(&'a [Tag]);
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
                        a.tag.tag-hash href=(resolve!(/tag/t.name)) { "#" }
                        a.tag.tag-block href=(Link(resolve!(/index), index::Request {
                            query: Some(&tag.name),
                            page: None
                        })) {
                            (tag.name) " " (tag.count) 
                            @if let Some(alt) = &tag.alt_name {
                                br; (alt)
                            }
                        }
                    }
                }
            } 
        }
    }
}

/// Tag input form with autocomplete (TODO: Not yet) (action, id, submit_name)
struct TagEditForm<'a, Action>(Action, &'a str, &'a str);
impl<Action> Render for TagEditForm<'_, Action>
where Action: Render {
    fn render_to(&self, buffer: &mut String) {
        html_to! { buffer,
            form action=(self.0) {
                input name="tag" type="text"
                // TODO: Script  
                ;
                input type="submit" value=(self.2);
                .result #(self.1) hidden {}
            }            
        }
    }
}

/// Aside block that displays element metadata
struct AsideMetadata<'a>(&'a ElementMetadata);
impl Render for AsideMetadata<'_> {
    fn render_to(&self, buffer: &mut String) {
        /// # param_name
        /// param_data
        ///
        /// Helper
        struct Param<'a, R>(&'a str, R);
        impl<R> Render for Param<'_, R>
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
                (Param("", html_in! { "Positive prompt" br; (ai.positive_prompt) }))
                @if let Some(neg) = &ai.negative_prompt {
                    (Param("", html_in! { "Negative prompt" br; (neg) }))
                }
                (Param("Sampler", &ai.sampler))
                (Param("Seed", ai.seed))
                (Param("Steps", ai.steps))
                (Param("Scale", ai.scale))
                (Param("Strength", ai.strength))
                (Param("Noise", ai.noise))
            }
            @else if !self.0.tags.is_empty() {
                .tag { "Generated prompt" }
                (Param("", html_in! {
                    // compose prompt from tags
                    @for tag in &self.0.tags {
                        (tag.name) ", "
                    }
                }))
            }
        }
    }
}