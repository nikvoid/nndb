use std::time::Duration;

use actix_files::Files;
use actix_web::{HttpServer, App, web::redirect};
use tracing::{info, error};
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

/// Spawn periodic import tasks
async fn import_spawner() {
    // Different delays are used here to drive tasks out of sync
    // TODO: Random delays?..
    
    util::blocking_task_with_interval(|| match service::scan_files() {
        Ok(count) => info!(count, "added elements to db"),
        Err(e) => error!(?e, "failed to scan files"),
    }, Duration::from_secs(300)).await;

    util::task_with_interval(|| async {
        match service::update_metadata().await {
            Ok(_) => info!("updated metadata"),
            Err(e) => error!(?e, "failed to update metadata"),
        }        
    }, Duration::from_secs(310)).await;

    util::task_with_interval(|| async {
        match service::group_elements_by_signature().await {
            Ok(_) => info!("grouped elements"),
            Err(e) => error!(?e, "failed to group elements"),
        }        
    }, Duration::from_secs(320)).await;

    util::blocking_task_with_interval(|| match service::update_tag_count() {
        Ok(_) => info!("updated tag count"),
        Err(e) => error!(?e, "failed to update tag count"),
    }, Duration::from_secs(325)).await;

    util::blocking_task_with_interval(|| match service::make_thumbnails() {
        Ok(_) => info!("made thumbnails"),
        Err(e) => error!(?e, "failed to make thumbnails"),
    }, Duration::from_secs(330)).await;
}

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

    info!("Spawning import tasks");
    tokio::spawn(import_spawner()).await.unwrap();

    info!(addr=CONFIG.bind_address, port=CONFIG.port, "Starting server");
    HttpServer::new(|| {
        let mut app = App::new()
            .wrap(TracingLogger::default())
            .service(redirect("/", "/index"))
            .service(view::index_page)
            .service(view::element_page)
            .service(view::tag_autocomplete)
            .service(view::dashboard_page)
            .service(view::tag_page)
            .service(view::add_tags)
            .service(view::delete_tag)
            .service(view::edit_tag)
            .service(view::read_log)
            .service(view::import_status)
            .service(view::start_import)
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
