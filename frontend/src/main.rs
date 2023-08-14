mod app;
mod api;
mod page;
mod component;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
