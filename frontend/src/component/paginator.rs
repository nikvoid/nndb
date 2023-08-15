use super::prelude::*;
use web_sys::window;


#[derive(Properties, PartialEq)]
pub struct Props {
    pub current: u32,
    pub max_page: u32,
    pub onclick: Callback<u32>,

    #[prop_or(5)]
    pub lookaround: u32,
    #[prop_or(1)]
    pub jump: u32,
    #[prop_or(None)]
    pub scroll_to_x: Option<f64>,
}

#[function_component]
pub fn Paginator(props: &Props) -> Html {
    let window = window().unwrap();
    
    let pages = {
        let left = props.current
            .saturating_sub(props.lookaround)
            .clamp(1, props.max_page);
        let left_off = props.jump.clamp(0, left.saturating_sub(1));
        let left_jump = 1 + left_off;
    
        let right = (props.current + props.lookaround).clamp(1, props.max_page);
        let right_off = props.jump.clamp(0, props.max_page - right); 
        let right_jump = props.max_page - right_off + 1; 
        (1..left_jump)
            .chain(left..=right)
            .chain(right_jump..=props.max_page)
    }
    .map(|page| {
        let onclk = props.onclick.clone();
        let scroll_to_x = props.scroll_to_x;
        let window = window.clone();

        let class = classes!(
            "button",
            (props.current == page).then_some("active")
        );

        let onclick = Callback::from(move |_| {
            if let Some(x) = scroll_to_x {
               window.scroll_to_with_x_and_y(x, 0.);
            }
            onclk.emit(page)
        });

        html! {
        <>
            <a {class} {onclick}>{ page }</a>
        </>
        }
    });
    
    html! {
        <div class="paginator">
            { for pages }
        </div>
    }
}
