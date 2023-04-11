use maud::{Render, html, DOCTYPE, Markup};

use crate::config::CONFIG;

mod index;

/// Compile-time url resolver.
/// We can't use expressions in #[get] macro, so this is better than 
/// writing each url by hand
#[macro_export]
macro_rules! resolve {
    (index) => { "/index" };
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

/// Html <head>
struct HtmlHead;
impl Render for HtmlHead {
    fn render(&self) -> Markup {
        html! {
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

fn button(href: &str, text: &str) -> Markup {
    html! {
        a.button href=(href) {
            (text)
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
    fn render(&self) -> Markup {
        html! {
            (HtmlHead)
            body {
                #base-container { 
                    #upnav {
                        span.head-span { (button(resolve!(index), "Index")) }
                        span.head-span {
                            form autocomplete="off" action=(resolve!(index)) method="get" {
                                input #search-box type="text" 
                                    name="query" value=(self.query)
                                    // TODO: Enable scripts
                                    onKeyUp=""; 
                                input type="submit" value="Search";
                                div.result #head-result hidden {}
                            } 
                        }
                        @if let Some(h) = &self.after_header { (h) };
                        section.aside {
                            @if let Some(aside) = &self.aside { (aside) }
                        }                        
                    }
                }
                footer {
                    .footer-container {
                        (button(resolve!(admin), "Administration"))
                    }
                    @if let Some(f) = &self.footer { (f) }
                }
            }
        }
    }
}

