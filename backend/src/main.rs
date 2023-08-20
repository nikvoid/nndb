use std::{time::Duration, path::Path};

use actix_files::Files;
use actix_web::{HttpServer, App};
use config::Config;
use tracing::{info, error};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::fmt::writer::Tee;
use util::LateInit;

use crate::dao::STORAGE;

mod model;
mod dao;
mod import;
mod service;
mod config; 
mod util;
mod api;

/// Spawn periodic import tasks
async fn import_spawner() {
    // Different delays are used here to drive tasks out of sync
    // TODO: Random delays or maybe make this thing more pipelined?..
    
    util::task_with_interval(|| async {
        match service::scan_files().await {
            Ok(count) => info!(count, "added elements to db"),
            Err(e) => error!(?e, "failed to scan files"),
        }
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

    util::blocking_task_with_interval(|| match service::make_thumbnails() {
        Ok(_) => info!("made thumbnails"),
        Err(e) => error!(?e, "failed to make thumbnails"),
    }, Duration::from_secs(330)).await;
}

/// Default config path
const DEF_CONFIG_FILE: &str = "config.toml";

/// Global config
pub static CONFIG: LateInit<Config> = LateInit::new();

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let cfg_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => DEF_CONFIG_FILE.to_string()
    };

    if !Path::new(&cfg_path).exists() {
        std::fs::write(&cfg_path, include_bytes!("../../config.toml"))?;
    }

    let cfg_str = std::fs::read_to_string(cfg_path)?;
    CONFIG.init(toml::from_str(&cfg_str).unwrap());
    
    let log_file = std::fs::File::options()
        .append(true)
        .create(true)
        .open(&CONFIG.log_file)?;
    
    tracing_subscriber::fmt()
        .with_writer(Tee::new(std::io::stdout, log_file))
        .with_file(true)
        .with_line_number(true)
        .with_max_level(CONFIG.log_level)
        .init();

    if CONFIG.auto_scan_files {
        info!("Spawning import tasks");
        import_spawner().await;
    }

    info!("File reads are done {:?}ly", CONFIG.read_files);

    STORAGE.reload_tag_aliases_index().await?;

    info!(addr=CONFIG.bind_address, port=CONFIG.port, "Starting server");
    HttpServer::new(|| {
        let mut app = App::new()
            .wrap(TracingLogger::default())
            .service(api::search_elements)
            .service(api::element)
            .service(api::tag_autocomplete)
            .service(api::tag_data)
            .service(api::tag_edit)
            .service(api::tag_alias)
            .service(api::tags_edit)
            .service(api::read_log)
            .service(api::import_status)
            .service(api::control)
            .service(api::summary)
        ;

        // Serve static folders if needed
        app = if CONFIG.static_folder.serve {
            app.service(Files::new(
                &CONFIG.static_folder.url,
                &CONFIG.static_folder.path
            ))
        } else { app };
        app = if CONFIG.element_pool.serve {
            app.service(Files::new(
                &CONFIG.element_pool.url, 
                &CONFIG.element_pool.path
            ))
        } else { app };
        app = if CONFIG.thumbnails_folder.serve {
            app.service(Files::new(
                &CONFIG.thumbnails_folder.url,
                &CONFIG.thumbnails_folder.path
            ))
        } else { app };

        app
    })
    .bind((CONFIG.bind_address.as_str(), CONFIG.port))?
    .run()
    .await?;

    Ok(())
}
