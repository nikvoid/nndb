use maud::{Render, html, DOCTYPE, Markup, html_to};
use serde::Serialize;

use crate::config::CONFIG;

mod index;

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
        $crate::view::MaudFnWrapper(|buf: &mut String| html_to!{ buf, $($tt)* })
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

pub use index::index_page;

/// Static content 
struct Static<'a>(&'a str);
impl<'a> Render for Static<'a> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.static_files_path);
        buffer.push_str(self.0);
    }
}

/// Link to element in pool
struct ElementLink<'a>(&'a str);
impl<'a> Render for ElementLink<'a> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(&CONFIG.elements_path);
        buffer.push_str(self.0);
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
                    // script
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
                        span.head-span { (Button(resolve!(index), "Index")) }
                        span.head-span {
                            form autocomplete="off" action=(resolve!(/index)) method="get" {
                                input #search-box type="text" 
                                    name="query" value=(self.query)
                                    // TODO: Enable scripts
                                    onKeyUp=""; 
                                input type="submit" value="Search";
                                div.result #head-result hidden {}
                            } 
                            @if let Some(h) = &self.after_header { (h) };
                        }
                        @if let Some(cont) = &self.content { (cont) };
                        section.aside {
                            @if let Some(aside) = &self.aside { (aside) }
                        }                        
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

