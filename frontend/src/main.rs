mod app;
mod api;
mod page;
mod component;
mod route;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
