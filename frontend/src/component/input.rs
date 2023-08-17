use super::prelude::*;
use std::ops::Range;

use web_sys::{HtmlInputElement, KeyboardEvent};

#[derive(Properties, PartialEq)]
struct CompletionProps {
    pub result: String,
    pub children: Children
}

#[derive(Properties, PartialEq)]
pub struct InputProps {
    /// Callback that will be called on button press
    pub onsubmit: Callback<String>,
    /// Text on submit button
    #[prop_or("Search".into())]
    pub button_name: AttrValue,
    #[prop_or_default]
    pub value: String,
}


/// Input field with tag autocompletion
#[derive(Default)]
pub struct InputAutocomplete {
    content: Vec<Tag>,
    input_locked: bool,
    text: String,
    selected: Range<usize>,
    input: NodeRef,
}

pub enum Msg {
    Set(String),
    Parse,
    Term(String),
    Completions(Vec<Tag>),
    Selected(String),
    Submit,
}

impl Component for InputAutocomplete {
    type Message = Msg;

    type Properties = InputProps;

    fn create(_: &yew::Context<Self>) -> Self {
        Self::default()
    }

    fn view(&self, ctx: &yew::Context<Self>) -> Html {
        let onclick = ctx.link().callback(|_| Msg::Submit);
        let onclick_form = ctx.link().callback(|_| Msg::Parse);
        let onkeyup = ctx.link().callback(|e: KeyboardEvent| match e.key_code() {
            // If Enter pressed
            13 => Msg::Submit,
            _  => Msg::Parse
        });
        
        let input_ref = self.input.clone();

        let completions = self.content
            .iter()
            .map(|tag| {
                let name = tag.name.clone();
                // Possible completion result, that will be spliced into input on click
                let onclick = ctx.link()
                    .callback(move |_| Msg::Selected(name.clone()));
                html! {
                    <div class="tag-completion" {onclick}>
                        <div class="name">
                            { &tag.name }
                            if let Some(alt_name) = &tag.alt_name {
                                <i>
                                    { " " }
                                    { alt_name }
                                </i>
                            }
                        </div>
                        <div class="count">
                            { tag.count }
                        </div>
                    </div>
                }
            });
        
        html! {
            <div class="input-autocomplete">        
                <input 
                    type="text" 
                    {onkeyup} 
                    onclick={onclick_form} 
                    ref={input_ref} />
                <button {onclick}>{ &ctx.props().button_name }</button>
                <div class="completions" hidden={self.content.is_empty()}>
                    { for completions }
                </div>
            </div>
        }
    }

    fn changed(&mut self, ctx: &yew::Context<Self>, _old_props: &Self::Properties) -> bool {
        // Synchronize value with props
        ctx.link().send_message(Msg::Set(ctx.props().value.clone()));
        true
    }

    fn update(&mut self, ctx: &yew::Context<Self>, msg: Self::Message) -> bool {
        let input = self.input.cast::<HtmlInputElement>().unwrap();
        
        match msg {
            Msg::Set(value) => {
                input.set_value(&value);
                true
            }
            Msg::Parse => {
                self.content = vec![];
                
                // WARN: cursor is offset in characters, not bytes
                let cursor = input.selection_start().unwrap().unwrap() as usize;

                let text = input.value();
                
                let cursor = text
                    .char_indices()
                    // Trailing cursor case
                    .chain(Some((text.len(), '>')))
                    .nth(cursor)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                
                let start = text
                    .rmatch_indices(' ')
                    .find(|(idx, _)| idx < &cursor)
                    .map(|(idx, _)| idx + 1)
                    .unwrap_or(0);

                let end = text
                    .match_indices(' ')
                    .find(|(idx, _)| idx >= &cursor)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
               
                if !self.input_locked {
                    self.selected = start..end;
                    self.text = text.clone();
                    // Lock input until response from backend
                    self.input_locked = true;
                }

                ctx.link().send_message(Msg::Term(text[start..end].into()));
                false
            },
            Msg::Term(term) => {
                // On select request completions from backend
                ctx.link().send_future(async move {
                    let req = AutocompleteRequest {
                        input: term
                    };
                    let resp: AutocompleteResponse = 
                        backend_post!(&req, "/v1/autocomplete")
                        .await
                        .unwrap();
                    Msg::Completions(resp.completions)
                });
                false
            },
            Msg::Completions(cont) => {
                self.content = cont;
                self.input_locked = false;
                true
            },
            Msg::Selected(text) => {
                self.content = vec![];
                let mut value = self.text.clone();
                value.replace_range(self.selected.clone(), &text);
                input.set_value(&value);
                true
            }
            Msg::Submit => {
                ctx.props().onsubmit.emit(input.value());
                false
            },
        }
    }
}
