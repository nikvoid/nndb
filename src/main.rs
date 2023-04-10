use tracing::info;

mod model;
mod dao;
mod import;
mod service;
mod config; 
mod util;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .init();
    info!("Starting service");
    tokio::task::spawn_blocking(service::scan_files)
        .await
        .unwrap()
        .unwrap();
    info!("Scanned files");
    service::update_metadata()
        .await
        .unwrap();
    info!("Updated metadata");
    tokio::task::spawn_blocking(service::group_elements_by_signature)
        .await
        .unwrap()
        .unwrap();
    info!("Group images");
}
