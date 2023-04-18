use actix_files::Files;
use actix_web::{HttpServer, App, web::redirect};
use tracing::info;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::fmt::writer::Tee;

use crate::config::CONFIG;

mod model;
mod dao;
mod import;
mod service;
mod config; 
mod util;
mod view;
mod search;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_file = std::fs::File::options()
        .append(true)
        .create(true)
        .open(&CONFIG.log_file)?;
    
    tracing_subscriber::fmt()
        .with_writer(Tee::new(std::io::stdout, log_file))
        .with_file(true)
        .with_line_number(true)
        .init();
    info!(addr=CONFIG.bind_address, port=CONFIG.port, "Starting server");
        
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
    info!("Grouped images");
    tokio::task::spawn_blocking(service::update_tag_count)
        .await
        .unwrap()
        .unwrap();
    info!("Updated tag counts");
    tokio::task::spawn_blocking(service::make_thumbnails)
        .await
        .unwrap()
        .unwrap();
    info!("Made thumbnails");

    HttpServer::new(|| {
        let mut app = App::new()
            .wrap(TracingLogger::default())
            .service(redirect("/", "/index"))
            .service(view::index_page)
            .service(view::element_page)
            .service(view::tag_autocomplete)
            .service(view::add_tags)
            .service(view::tag_page)
            .service(view::delete_tag)
            .service(view::edit_tag)
        ;

        // Serve static folder if needed
        app = match &CONFIG.static_folder {
            Some(folder) => app
                .service(Files::new(&CONFIG.static_files_path, folder))
                .service(Files::new(&CONFIG.thumbnails_path, &CONFIG.thumbnails_folder))
                .service(Files::new(&CONFIG.elements_path, &CONFIG.element_pool)),
            None => app
        };

        app
    })
    .bind((CONFIG.bind_address.as_str(), CONFIG.port))?
    .run()
    .await
}
