use actix_files::Files;
use actix_web::{HttpServer, App, web::redirect};
use tracing::info;
use tracing_actix_web::TracingLogger;

use crate::config::CONFIG;

mod model;
mod dao;
mod import;
mod service;
mod config; 
mod util;
mod view;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
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

    HttpServer::new(|| {
        let mut app = App::new()
            .wrap(TracingLogger::default())
            .service(redirect("/", "/index"))
            .service(view::index_page)
            .service(view::element_page)
        ;

        // Serve static folder if needed
        app = match &CONFIG.static_folder {
            Some(folder) => app
                .service(Files::new(&CONFIG.static_files_path, folder))
                .service(Files::new(&CONFIG.elements_path, &CONFIG.element_pool)),
            None => app
        };

        app
    })
    .bind((CONFIG.bind_address.as_str(), CONFIG.port))?
    .run()
    .await
}
